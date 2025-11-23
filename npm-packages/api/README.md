# @llm-dev-ops/llm-schema-api

LLM Schema API - Core API server for schema registry with gRPC support

## Installation

```bash
npm install -g @llm-dev-ops/llm-schema-api
```

## Usage

```bash
# Start the API server
llm-schema-api

# Start with custom configuration
llm-schema-api --config /path/to/config.yaml

# View help
llm-schema-api --help
```

## Configuration

The API server can be configured using environment variables or a configuration file:

```yaml
# Server configuration
server:
  grpc_port: 50051
  host: "0.0.0.0"

# Database
database:
  url: "postgresql://user:password@localhost/schema_registry"
  max_connections: 20

# Storage backend
storage:
  type: "postgres"  # postgres, s3, or hybrid

# Cache settings
cache:
  enabled: true
  redis_url: "redis://localhost:6379"
  ttl_seconds: 300
```

## Features

- High-performance gRPC API
- Protocol Buffer definitions for schemas
- PostgreSQL storage with S3 backup
- Redis caching for fast lookups
- Streaming support for large schemas
- Schema versioning and compatibility checking
- Authentication and authorization
- Metrics and observability

## gRPC Services

The API provides the following gRPC services:

### SchemaRegistry Service

- `RegisterSchema` - Register a new schema version
- `GetSchema` - Retrieve a schema by namespace, name, and version
- `ListSchemas` - List schemas in a namespace
- `DeleteSchema` - Delete a schema version
- `ValidateData` - Validate data against a schema
- `CheckCompatibility` - Check compatibility between schema versions

### Admin Service

- `GetMetrics` - Retrieve server metrics
- `HealthCheck` - Check server health
- `GetConfig` - Get server configuration

## Documentation

For more information, visit: https://github.com/globalbusinessadvisors/llm-schema-registry

## License

Apache-2.0
