"""
Main client implementation for the LLM Schema Registry Python SDK.
"""

import logging
from typing import Any, Dict, List, Optional
from uuid import UUID

import httpx
from cachetools import TTLCache
from tenacity import (
    retry,
    retry_if_exception_type,
    stop_after_attempt,
    wait_exponential,
)

from .exceptions import (
    AuthenticationError,
    AuthorizationError,
    IncompatibleSchemaError,
    RateLimitError,
    SchemaNotFoundError,
    SchemaRegistryError,
    SchemaValidationError,
    ServerError,
)
from .models import (
    CompatibilityMode,
    CompatibilityResult,
    GetSchemaResponse,
    RegisterSchemaResponse,
    Schema,
    SchemaVersion,
    SearchResult,
    ValidateResponse,
)

logger = logging.getLogger(__name__)


class SchemaRegistryClient:
    """
    Production-ready client for the LLM Schema Registry.

    Features:
    - Async/await support
    - Automatic retries with exponential backoff
    - In-memory caching with TTL
    - Comprehensive error handling
    - Type-safe schema operations

    Example:
        >>> async with SchemaRegistryClient(
        ...     base_url="http://localhost:8080",
        ...     api_key="your-api-key"
        ... ) as client:
        ...     schema = Schema(
        ...         namespace="telemetry",
        ...         name="InferenceEvent",
        ...         version="1.0.0",
        ...         format=SchemaFormat.JSON_SCHEMA,
        ...         content='{"type": "object"}'
        ...     )
        ...     result = await client.register_schema(schema)
        ...     print(f"Schema ID: {result.schema_id}")
    """

    def __init__(
        self,
        base_url: str,
        api_key: Optional[str] = None,
        timeout: float = 30.0,
        max_retries: int = 3,
        cache_ttl: int = 300,
        cache_maxsize: int = 1000,
    ):
        """
        Initialize the schema registry client.

        Args:
            base_url: Base URL of the schema registry (e.g., "http://localhost:8080")
            api_key: Optional API key for authentication
            timeout: Request timeout in seconds (default: 30)
            max_retries: Maximum number of retry attempts (default: 3)
            cache_ttl: Cache time-to-live in seconds (default: 300)
            cache_maxsize: Maximum number of cached items (default: 1000)
        """
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.timeout = timeout
        self.max_retries = max_retries

        # Initialize cache
        self._cache: TTLCache = TTLCache(maxsize=cache_maxsize, ttl=cache_ttl)

        # HTTP client
        headers = {"Content-Type": "application/json"}
        if api_key:
            headers["Authorization"] = f"Bearer {api_key}"

        self._client = httpx.AsyncClient(
            base_url=self.base_url,
            headers=headers,
            timeout=timeout,
        )

    async def __aenter__(self) -> "SchemaRegistryClient":
        """Async context manager entry."""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        """Async context manager exit."""
        await self.close()

    async def close(self) -> None:
        """Close the HTTP client and cleanup resources."""
        await self._client.aclose()

    def _handle_response_error(self, response: httpx.Response) -> None:
        """Handle HTTP error responses."""
        if response.is_success:
            return

        status_code = response.status_code

        try:
            error_data = response.json()
            message = error_data.get("message", response.text)
        except Exception:
            message = response.text

        if status_code == 401:
            raise AuthenticationError(message)
        elif status_code == 403:
            raise AuthorizationError(message)
        elif status_code == 404:
            raise SchemaNotFoundError(schema_id=message)
        elif status_code == 409:
            incompatibilities = error_data.get("incompatibilities", [message])
            raise IncompatibleSchemaError(incompatibilities)
        elif status_code == 429:
            retry_after = response.headers.get("Retry-After")
            raise RateLimitError(retry_after=int(retry_after) if retry_after else None)
        elif status_code >= 500:
            raise ServerError(message)
        else:
            raise SchemaRegistryError(message, status_code=status_code)

    @retry(
        retry=retry_if_exception_type((httpx.TimeoutException, ServerError)),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
    )
    async def register_schema(self, schema: Schema) -> RegisterSchemaResponse:
        """
        Register a new schema.

        Args:
            schema: Schema to register

        Returns:
            RegisterSchemaResponse with schema_id, version, and created_at

        Raises:
            SchemaValidationError: If schema is invalid
            AuthenticationError: If authentication fails
            SchemaRegistryError: For other errors
        """
        logger.info(
            f"Registering schema: {schema.namespace}.{schema.name} v{schema.version}"
        )

        payload = schema.model_dump(mode="json", exclude_none=True)

        response = await self._client.post("/api/v1/schemas", json=payload)
        self._handle_response_error(response)

        data = response.json()
        result = RegisterSchemaResponse(**data)

        # Invalidate cache for this schema
        cache_key = f"{schema.namespace}.{schema.name}"
        if cache_key in self._cache:
            del self._cache[cache_key]

        logger.info(f"Registered schema with ID: {result.schema_id}")
        return result

    @retry(
        retry=retry_if_exception_type((httpx.TimeoutException, ServerError)),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
    )
    async def get_schema(self, schema_id: str, use_cache: bool = True) -> GetSchemaResponse:
        """
        Get a schema by ID.

        Args:
            schema_id: UUID of the schema
            use_cache: Whether to use cached result (default: True)

        Returns:
            GetSchemaResponse with full schema details

        Raises:
            SchemaNotFoundError: If schema doesn't exist
            SchemaRegistryError: For other errors
        """
        # Check cache first
        cache_key = f"schema:{schema_id}"
        if use_cache and cache_key in self._cache:
            logger.debug(f"Cache hit for schema ID: {schema_id}")
            return self._cache[cache_key]

        logger.info(f"Fetching schema: {schema_id}")

        response = await self._client.get(f"/api/v1/schemas/{schema_id}")
        self._handle_response_error(response)

        data = response.json()
        result = GetSchemaResponse(**data)

        # Cache the result
        if use_cache:
            self._cache[cache_key] = result

        return result

    @retry(
        retry=retry_if_exception_type((httpx.TimeoutException, ServerError)),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
    )
    async def get_schema_by_version(
        self, namespace: str, name: str, version: str
    ) -> GetSchemaResponse:
        """
        Get a schema by namespace, name, and version.

        Args:
            namespace: Schema namespace
            name: Schema name
            version: Schema version

        Returns:
            GetSchemaResponse with full schema details

        Raises:
            SchemaNotFoundError: If schema doesn't exist
            SchemaRegistryError: For other errors
        """
        # Check cache
        cache_key = f"schema:{namespace}.{name}:{version}"
        if cache_key in self._cache:
            return self._cache[cache_key]

        logger.info(f"Fetching schema: {namespace}.{name} v{version}")

        response = await self._client.get(
            f"/api/v1/schemas/{namespace}/{name}/versions/{version}"
        )
        self._handle_response_error(response)

        data = response.json()
        result = GetSchemaResponse(**data)

        # Cache the result
        self._cache[cache_key] = result

        return result

    @retry(
        retry=retry_if_exception_type((httpx.TimeoutException, ServerError)),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
    )
    async def validate_data(self, schema_id: str, data: str | dict) -> ValidateResponse:
        """
        Validate data against a schema.

        Args:
            schema_id: UUID of the schema to validate against
            data: Data to validate (JSON string or dict)

        Returns:
            ValidateResponse with is_valid and errors

        Raises:
            SchemaNotFoundError: If schema doesn't exist
            SchemaRegistryError: For other errors
        """
        logger.info(f"Validating data against schema: {schema_id}")

        if isinstance(data, dict):
            import json

            data = json.dumps(data)

        payload = {"schema_id": schema_id, "data": data}

        response = await self._client.post("/api/v1/validate", json=payload)
        self._handle_response_error(response)

        return ValidateResponse(**response.json())

    @retry(
        retry=retry_if_exception_type((httpx.TimeoutException, ServerError)),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
    )
    async def check_compatibility(
        self,
        schema_id: str,
        new_schema_content: str,
        mode: CompatibilityMode = CompatibilityMode.BACKWARD,
    ) -> CompatibilityResult:
        """
        Check if a new schema is compatible with an existing schema.

        Args:
            schema_id: UUID of the existing schema
            new_schema_content: Content of the new schema
            mode: Compatibility mode to use

        Returns:
            CompatibilityResult with is_compatible and incompatibilities

        Raises:
            SchemaNotFoundError: If schema doesn't exist
            SchemaRegistryError: For other errors
        """
        logger.info(f"Checking compatibility for schema: {schema_id} (mode: {mode})")

        payload = {"schema_id": schema_id, "new_schema": new_schema_content, "mode": mode.value}

        response = await self._client.post("/api/v1/compatibility/check", json=payload)
        self._handle_response_error(response)

        return CompatibilityResult(**response.json())

    @retry(
        retry=retry_if_exception_type((httpx.TimeoutException, ServerError)),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
    )
    async def search_schemas(
        self, query: str, limit: int = 10, offset: int = 0
    ) -> List[SearchResult]:
        """
        Search for schemas using full-text search.

        Args:
            query: Search query
            limit: Maximum number of results (default: 10)
            offset: Result offset for pagination (default: 0)

        Returns:
            List of SearchResult objects

        Raises:
            SchemaRegistryError: For errors
        """
        logger.info(f"Searching schemas: query='{query}', limit={limit}, offset={offset}")

        response = await self._client.get(
            "/api/v1/search", params={"q": query, "limit": limit, "offset": offset}
        )
        self._handle_response_error(response)

        results = response.json()
        return [SearchResult(**result) for result in results]

    @retry(
        retry=retry_if_exception_type((httpx.TimeoutException, ServerError)),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
    )
    async def list_versions(self, namespace: str, name: str) -> List[SchemaVersion]:
        """
        List all versions of a schema.

        Args:
            namespace: Schema namespace
            name: Schema name

        Returns:
            List of SchemaVersion objects

        Raises:
            SchemaNotFoundError: If schema doesn't exist
            SchemaRegistryError: For other errors
        """
        logger.info(f"Listing versions for schema: {namespace}.{name}")

        response = await self._client.get(f"/api/v1/schemas/{namespace}/{name}/versions")
        self._handle_response_error(response)

        versions = response.json()
        return [SchemaVersion(**v) for v in versions]

    @retry(
        retry=retry_if_exception_type((httpx.TimeoutException, ServerError)),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
    )
    async def delete_schema(self, schema_id: str) -> None:
        """
        Delete a schema.

        Args:
            schema_id: UUID of the schema to delete

        Raises:
            SchemaNotFoundError: If schema doesn't exist
            AuthorizationError: If user lacks delete permission
            SchemaRegistryError: For other errors
        """
        logger.info(f"Deleting schema: {schema_id}")

        response = await self._client.delete(f"/api/v1/schemas/{schema_id}")
        self._handle_response_error(response)

        # Invalidate cache
        cache_key = f"schema:{schema_id}"
        if cache_key in self._cache:
            del self._cache[cache_key]

        logger.info(f"Deleted schema: {schema_id}")

    @retry(
        retry=retry_if_exception_type((httpx.TimeoutException, ServerError)),
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=1, max=10),
    )
    async def health_check(self) -> Dict[str, Any]:
        """
        Check the health of the schema registry.

        Returns:
            Health status dict

        Raises:
            ServerError: If service is unhealthy
        """
        response = await self._client.get("/health")
        self._handle_response_error(response)
        return response.json()

    def clear_cache(self) -> None:
        """Clear the in-memory cache."""
        self._cache.clear()
        logger.info("Cache cleared")
