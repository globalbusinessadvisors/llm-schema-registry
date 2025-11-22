"""
LLM Schema Registry Python SDK

A production-ready Python client for the LLM Schema Registry, providing
enterprise-grade schema management, validation, and compatibility checking.

Example:
    >>> from schema_registry import SchemaRegistryClient, Schema, SchemaFormat
    >>>
    >>> client = SchemaRegistryClient(base_url="http://localhost:8080", api_key="your-api-key")
    >>>
    >>> # Register a schema
    >>> schema = Schema(
    ...     namespace="telemetry",
    ...     name="InferenceEvent",
    ...     version="1.0.0",
    ...     format=SchemaFormat.JSON_SCHEMA,
    ...     content='{"type": "object", "properties": {"model": {"type": "string"}}}'
    ... )
    >>> result = await client.register_schema(schema)
    >>> print(f"Registered schema with ID: {result.schema_id}")
    >>>
    >>> # Validate data
    >>> is_valid = await client.validate_data(
    ...     schema_id=result.schema_id,
    ...     data='{"model": "gpt-4"}'
    ... )
    >>> print(f"Data is valid: {is_valid}")
"""

__version__ = "0.1.0"
__author__ = "Schema Registry Team"
__license__ = "Apache-2.0"

from .client import SchemaRegistryClient
from .models import (
    Schema,
    SchemaFormat,
    SchemaMetadata,
    RegisterSchemaResponse,
    ValidateResponse,
    CompatibilityMode,
    CompatibilityResult,
)
from .exceptions import (
    SchemaRegistryError,
    SchemaNotFoundError,
    SchemaValidationError,
    IncompatibleSchemaError,
    AuthenticationError,
    RateLimitError,
)

__all__ = [
    # Client
    "SchemaRegistryClient",
    # Models
    "Schema",
    "SchemaFormat",
    "SchemaMetadata",
    "RegisterSchemaResponse",
    "ValidateResponse",
    "CompatibilityMode",
    "CompatibilityResult",
    # Exceptions
    "SchemaRegistryError",
    "SchemaNotFoundError",
    "SchemaValidationError",
    "IncompatibleSchemaError",
    "AuthenticationError",
    "RateLimitError",
]
