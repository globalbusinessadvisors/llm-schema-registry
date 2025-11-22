"""
Exception classes for the LLM Schema Registry Python SDK.
"""


class SchemaRegistryError(Exception):
    """Base exception for all schema registry errors."""

    def __init__(self, message: str, status_code: int | None = None):
        super().__init__(message)
        self.message = message
        self.status_code = status_code


class SchemaNotFoundError(SchemaRegistryError):
    """Raised when a schema is not found."""

    def __init__(self, schema_id: str):
        super().__init__(f"Schema not found: {schema_id}", status_code=404)
        self.schema_id = schema_id


class SchemaValidationError(SchemaRegistryError):
    """Raised when schema validation fails."""

    def __init__(self, errors: list[str]):
        message = "Schema validation failed:\n" + "\n".join(f"  - {err}" for err in errors)
        super().__init__(message, status_code=400)
        self.errors = errors


class IncompatibleSchemaError(SchemaRegistryError):
    """Raised when schemas are incompatible."""

    def __init__(self, incompatibilities: list[str]):
        message = "Schema compatibility check failed:\n" + "\n".join(
            f"  - {inc}" for inc in incompatibilities
        )
        super().__init__(message, status_code=409)
        self.incompatibilities = incompatibilities


class AuthenticationError(SchemaRegistryError):
    """Raised when authentication fails."""

    def __init__(self, message: str = "Authentication failed"):
        super().__init__(message, status_code=401)


class AuthorizationError(SchemaRegistryError):
    """Raised when authorization fails."""

    def __init__(self, message: str = "Insufficient permissions"):
        super().__init__(message, status_code=403)


class RateLimitError(SchemaRegistryError):
    """Raised when rate limit is exceeded."""

    def __init__(self, retry_after: int | None = None):
        message = "Rate limit exceeded"
        if retry_after:
            message += f". Retry after {retry_after} seconds"
        super().__init__(message, status_code=429)
        self.retry_after = retry_after


class ServerError(SchemaRegistryError):
    """Raised when server encounters an error."""

    def __init__(self, message: str = "Internal server error"):
        super().__init__(message, status_code=500)
