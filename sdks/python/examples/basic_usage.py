"""
Basic usage examples for the LLM Schema Registry Python SDK.
"""

import asyncio
import json

from schema_registry import (
    CompatibilityMode,
    Schema,
    SchemaFormat,
    SchemaMetadata,
    SchemaRegistryClient,
)


async def register_and_validate_example():
    """Example: Register a schema and validate data against it."""
    async with SchemaRegistryClient(
        base_url="http://localhost:8080", api_key="your-api-key"
    ) as client:
        # Define a JSON Schema for LLM inference events
        inference_schema = {
            "type": "object",
            "required": ["model", "prompt", "timestamp"],
            "properties": {
                "model": {"type": "string", "description": "Model name (e.g., gpt-4)"},
                "prompt": {"type": "string", "description": "Input prompt"},
                "response": {"type": "string", "description": "Model response"},
                "timestamp": {"type": "string", "format": "date-time"},
                "tokens": {
                    "type": "object",
                    "properties": {
                        "prompt": {"type": "integer"},
                        "completion": {"type": "integer"},
                        "total": {"type": "integer"},
                    },
                },
            },
        }

        # Register the schema
        schema = Schema(
            namespace="telemetry",
            name="InferenceEvent",
            version="1.0.0",
            format=SchemaFormat.JSON_SCHEMA,
            content=json.dumps(inference_schema),
            metadata=SchemaMetadata(
                description="Schema for LLM inference telemetry events",
                tags=["telemetry", "inference", "llm"],
                owner="platform-team",
            ),
        )

        result = await client.register_schema(schema)
        print(f"✓ Registered schema: {result.schema_id}")
        print(f"  Version: {result.version}")
        print(f"  Created: {result.created_at}")

        # Validate valid data
        valid_data = {
            "model": "gpt-4",
            "prompt": "What is machine learning?",
            "response": "Machine learning is...",
            "timestamp": "2025-01-01T12:00:00Z",
            "tokens": {"prompt": 10, "completion": 50, "total": 60},
        }

        validation_result = await client.validate_data(
            schema_id=result.schema_id, data=valid_data
        )
        print(f"\n✓ Valid data validation: {validation_result.is_valid}")

        # Validate invalid data (missing required field)
        invalid_data = {"model": "gpt-4"}

        validation_result = await client.validate_data(
            schema_id=result.schema_id, data=invalid_data
        )
        print(f"\n✗ Invalid data validation: {validation_result.is_valid}")
        if not validation_result.is_valid:
            print(f"  Errors: {validation_result.errors}")


async def compatibility_check_example():
    """Example: Check schema compatibility."""
    async with SchemaRegistryClient(
        base_url="http://localhost:8080", api_key="your-api-key"
    ) as client:
        # Original schema
        original_schema = Schema(
            namespace="telemetry",
            name="UserEvent",
            version="1.0.0",
            format=SchemaFormat.JSON_SCHEMA,
            content=json.dumps(
                {
                    "type": "object",
                    "required": ["user_id", "event_type"],
                    "properties": {
                        "user_id": {"type": "string"},
                        "event_type": {"type": "string"},
                    },
                }
            ),
        )

        result = await client.register_schema(original_schema)
        print(f"✓ Registered original schema: {result.schema_id}")

        # BACKWARD compatible change (add optional field)
        new_schema_compatible = json.dumps(
            {
                "type": "object",
                "required": ["user_id", "event_type"],
                "properties": {
                    "user_id": {"type": "string"},
                    "event_type": {"type": "string"},
                    "timestamp": {"type": "string"},  # New optional field
                },
            }
        )

        compat = await client.check_compatibility(
            schema_id=result.schema_id,
            new_schema_content=new_schema_compatible,
            mode=CompatibilityMode.BACKWARD,
        )
        print(f"\n✓ BACKWARD compatible: {compat.is_compatible}")

        # NOT BACKWARD compatible (remove required field)
        new_schema_incompatible = json.dumps(
            {"type": "object", "required": ["user_id"], "properties": {"user_id": {"type": "string"}}}
        )

        compat = await client.check_compatibility(
            schema_id=result.schema_id,
            new_schema_content=new_schema_incompatible,
            mode=CompatibilityMode.BACKWARD,
        )
        print(f"\n✗ BACKWARD compatible: {compat.is_compatible}")
        if not compat.is_compatible:
            print(f"  Incompatibilities: {compat.incompatibilities}")


async def version_management_example():
    """Example: Manage schema versions."""
    async with SchemaRegistryClient(
        base_url="http://localhost:8080", api_key="your-api-key"
    ) as client:
        # Register multiple versions
        for i in range(1, 4):
            schema = Schema(
                namespace="telemetry",
                name="ApiEvent",
                version=f"{i}.0.0",
                format=SchemaFormat.JSON_SCHEMA,
                content=json.dumps({"type": "object", "version": i}),
            )
            result = await client.register_schema(schema)
            print(f"✓ Registered v{i}.0.0: {result.schema_id}")

        # List all versions
        versions = await client.list_versions(namespace="telemetry", name="ApiEvent")
        print(f"\n✓ Found {len(versions)} versions:")
        for v in versions:
            print(f"  - v{v.version} (ID: {v.schema_id})")

        # Get specific version
        schema = await client.get_schema_by_version(
            namespace="telemetry", name="ApiEvent", version="2.0.0"
        )
        print(f"\n✓ Retrieved v2.0.0:")
        print(f"  - Schema ID: {schema.schema_id}")
        print(f"  - Created: {schema.created_at}")


async def search_example():
    """Example: Search for schemas."""
    async with SchemaRegistryClient(
        base_url="http://localhost:8080", api_key="your-api-key"
    ) as client:
        # Search for schemas
        results = await client.search_schemas(query="telemetry", limit=10)

        print(f"✓ Found {len(results)} schemas matching 'telemetry':")
        for result in results:
            print(f"  - {result.namespace}.{result.name} v{result.version}")
            print(f"    Score: {result.score:.2f}")
            if result.description:
                print(f"    Description: {result.description}")
            if result.tags:
                print(f"    Tags: {', '.join(result.tags)}")


async def error_handling_example():
    """Example: Error handling."""
    from schema_registry.exceptions import (
        AuthenticationError,
        SchemaNotFoundError,
        SchemaValidationError,
    )

    async with SchemaRegistryClient(
        base_url="http://localhost:8080", api_key="invalid-key"
    ) as client:
        try:
            # This will raise AuthenticationError
            await client.health_check()
        except AuthenticationError as e:
            print(f"✗ Authentication failed: {e.message}")

        # Fix auth
        client.api_key = "valid-key"

        try:
            # This will raise SchemaNotFoundError
            await client.get_schema("00000000-0000-0000-0000-000000000000")
        except SchemaNotFoundError as e:
            print(f"✗ Schema not found: {e.schema_id}")


async def main():
    """Run all examples."""
    print("=" * 60)
    print("Example 1: Register and Validate")
    print("=" * 60)
    await register_and_validate_example()

    print("\n" + "=" * 60)
    print("Example 2: Compatibility Check")
    print("=" * 60)
    await compatibility_check_example()

    print("\n" + "=" * 60)
    print("Example 3: Version Management")
    print("=" * 60)
    await version_management_example()

    print("\n" + "=" * 60)
    print("Example 4: Search Schemas")
    print("=" * 60)
    await search_example()

    print("\n" + "=" * 60)
    print("Example 5: Error Handling")
    print("=" * 60)
    await error_handling_example()


if __name__ == "__main__":
    asyncio.run(main())
