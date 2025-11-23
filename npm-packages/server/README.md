# @llm-dev-ops/llm-schema-registry-server

LLM Schema Registry Server - HTTP/gRPC server for schema management

## Installation

```bash
npm install -g @llm-dev-ops/llm-schema-registry-server
```

## Usage

```bash
# Start the server with default configuration
llm-schema-server

# Start with custom configuration
llm-schema-server --config /path/to/config.yaml

# Start on a specific port
llm-schema-server --port 8080

# View help
llm-schema-server --help
```

## Configuration

The server can be configured using a YAML configuration file. Example:

```yaml
server:
  http_port: 8080
  grpc_port: 50051
  host: "0.0.0.0"

database:
  postgres:
    url: "postgresql://user:password@localhost/schema_registry"
    max_connections: 10

storage:
  type: "postgres"  # or "s3"

cache:
  enabled: true
  ttl_seconds: 300

security:
  auth_enabled: false
  jwt_secret: "your-secret-key"
```

## Features

- HTTP REST API for schema management
- gRPC API for high-performance access
- PostgreSQL/S3 storage backends
- Built-in caching with Redis support
- JWT authentication
- Prometheus metrics
- OpenTelemetry tracing

## API Endpoints

### HTTP REST API (default port 8080)

- `POST /schemas` - Register a new schema
- `GET /schemas/:namespace/:name/:version` - Get a schema
- `POST /schemas/validate` - Validate data against a schema
- `POST /schemas/compatibility` - Check schema compatibility
- `GET /schemas/:namespace` - List schemas in a namespace
- `DELETE /schemas/:namespace/:name/:version` - Delete a schema

### gRPC API (default port 50051)

See the [API documentation](https://github.com/globalbusinessadvisors/llm-schema-registry) for gRPC service definitions.

## Documentation

For more information, visit: https://github.com/globalbusinessadvisors/llm-schema-registry

## License

Apache-2.0
