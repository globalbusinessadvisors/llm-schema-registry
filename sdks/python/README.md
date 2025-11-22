# LLM Schema Registry Python SDK

**Production-ready Python client for the LLM Schema Registry**

[![PyPI](https://img.shields.io/pypi/v/llm-schema-registry-sdk.svg)](https://pypi.org/project/llm-schema-registry-sdk/)
[![Python](https://img.shields.io/pypi/pyversions/llm-schema-registry-sdk.svg)](https://pypi.org/project/llm-schema-registry-sdk/)
[![License](https://img.shields.io/pypi/l/llm-schema-registry-sdk.svg)](https://github.com/llm-schema-registry/llm-schema-registry/blob/main/LICENSE)

## Features

- ✅ **Async/Await Support** - Built on `httpx` for high-performance async operations
- ✅ **Automatic Retries** - Exponential backoff with configurable retry logic
- ✅ **Smart Caching** - In-memory TTL cache for improved performance (5-minute default)
- ✅ **Type Safety** - Full Pydantic models with validation
- ✅ **Comprehensive Error Handling** - Detailed exceptions for all error cases
- ✅ **Production Ready** - Logging, metrics, and enterprise-grade patterns
- ✅ **Multi-Format Support** - JSON Schema, Avro, and Protobuf

## Installation

```bash
# Basic installation
pip install llm-schema-registry-sdk

# With JSON Schema support
pip install llm-schema-registry-sdk[json]

# With Avro support
pip install llm-schema-registry-sdk[avro]

# With Protobuf support
pip install llm-schema-registry-sdk[protobuf]

# With all format support
pip install llm-schema-registry-sdk[all]
```

## Quick Start

```python
import asyncio
from schema_registry import SchemaRegistryClient, Schema, SchemaFormat

async def main():
    # Initialize client
    async with SchemaRegistryClient(
        base_url="http://localhost:8080",
        api_key="your-api-key"
    ) as client:
        # Register a schema
        schema = Schema(
            namespace="telemetry",
            name="InferenceEvent",
            version="1.0.0",
            format=SchemaFormat.JSON_SCHEMA,
            content='{"type": "object", "properties": {"model": {"type": "string"}}}'
        )

        result = await client.register_schema(schema)
        print(f"Registered schema with ID: {result.schema_id}")

        # Validate data
        is_valid = await client.validate_data(
            schema_id=result.schema_id,
            data='{"model": "gpt-4"}'
        )
        print(f"Data is valid: {is_valid.is_valid}")

        # Check compatibility
        new_schema = '{"type": "object", "properties": {"model": {"type": "string"}, "temperature": {"type": "number"}}}'
        compat = await client.check_compatibility(
            schema_id=result.schema_id,
            new_schema_content=new_schema
        )
        print(f"New schema is compatible: {compat.is_compatible}")

if __name__ == "__main__":
    asyncio.run(main())
```

## API Reference

### Client Initialization

```python
client = SchemaRegistryClient(
    base_url="http://localhost:8080",  # Schema registry URL
    api_key="your-api-key",            # Optional API key
    timeout=30.0,                      # Request timeout (seconds)
    max_retries=3,                     # Max retry attempts
    cache_ttl=300,                     # Cache TTL (seconds)
    cache_maxsize=1000                 # Max cached items
)
```

### Schema Operations

#### Register Schema
```python
schema = Schema(
    namespace="telemetry",
    name="InferenceEvent",
    version="1.0.0",
    format=SchemaFormat.JSON_SCHEMA,
    content='{"type": "object"}'
)
result = await client.register_schema(schema)
```

#### Get Schema by ID
```python
schema = await client.get_schema(schema_id="uuid-here")
```

#### Get Schema by Version
```python
schema = await client.get_schema_by_version(
    namespace="telemetry",
    name="InferenceEvent",
    version="1.0.0"
)
```

#### List Versions
```python
versions = await client.list_versions(
    namespace="telemetry",
    name="InferenceEvent"
)
```

### Validation Operations

#### Validate Data
```python
result = await client.validate_data(
    schema_id="uuid-here",
    data='{"model": "gpt-4"}'
)
if not result.is_valid:
    print(f"Validation errors: {result.errors}")
```

#### Check Compatibility
```python
from schema_registry import CompatibilityMode

result = await client.check_compatibility(
    schema_id="uuid-here",
    new_schema_content='{"type": "object"}',
    mode=CompatibilityMode.BACKWARD
)
if not result.is_compatible:
    print(f"Incompatibilities: {result.incompatibilities}")
```

### Search and Discovery

#### Search Schemas
```python
results = await client.search_schemas(
    query="inference",
    limit=10,
    offset=0
)
for result in results:
    print(f"{result.namespace}.{result.name}: {result.score}")
```

### Management Operations

#### Delete Schema
```python
await client.delete_schema(schema_id="uuid-here")
```

#### Health Check
```python
health = await client.health_check()
print(f"Service is healthy: {health['status'] == 'healthy'}")
```

### Cache Management

```python
# Clear the cache
client.clear_cache()
```

## Error Handling

```python
from schema_registry.exceptions import (
    SchemaNotFoundError,
    SchemaValidationError,
    IncompatibleSchemaError,
    AuthenticationError,
    RateLimitError,
)

try:
    schema = await client.get_schema("non-existent-id")
except SchemaNotFoundError as e:
    print(f"Schema not found: {e.schema_id}")
except AuthenticationError:
    print("Authentication failed")
except RateLimitError as e:
    print(f"Rate limited. Retry after {e.retry_after} seconds")
```

## Advanced Usage

### Custom Logging

```python
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("schema_registry")
logger.setLevel(logging.DEBUG)
```

### Disable Caching

```python
# Per-request
schema = await client.get_schema(schema_id="uuid", use_cache=False)

# Clear all cache
client.clear_cache()
```

### Custom Retry Logic

The SDK uses `tenacity` for retries. All operations automatically retry on:
- Network timeouts
- 5xx server errors
- With exponential backoff (1s, 2s, 4s, 8s, 10s)
- Up to 3 attempts by default

## Development

```bash
# Install dependencies
poetry install --with dev

# Run tests
poetry run pytest

# Run tests with coverage
poetry run pytest --cov=schema_registry --cov-report=html

# Type checking
poetry run mypy schema_registry

# Code formatting
poetry run black schema_registry tests
poetry run ruff check schema_registry tests
```

## License

Apache License 2.0 - See [LICENSE](../../LICENSE) for details.
