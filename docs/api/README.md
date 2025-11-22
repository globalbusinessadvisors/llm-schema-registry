# LLM Schema Registry - API Documentation

## Quick Links

- **[API Implementation Report](./API-IMPLEMENTATION-REPORT.md)** - Complete implementation overview
- **[REST API Guide](./API-GUIDE.md)** - Comprehensive REST API documentation with curl examples
- **[gRPC API Guide](./GRPC-GUIDE.md)** - gRPC service documentation with Python and Go examples

## Overview

The LLM Schema Registry provides dual API interfaces:

1. **REST API (Axum)** - HTTP/JSON interface for ease of use
2. **gRPC API (Tonic)** - High-performance binary protocol with streaming support

## Base URLs

- **REST API:** `http://localhost:8080/api/v1`
- **gRPC API:** `localhost:9090`
- **Swagger UI:** `http://localhost:8080/swagger-ui`
- **Health Check:** `http://localhost:8080/health`
- **Metrics:** `http://localhost:8080/metrics`

## Quick Start

### REST API Example

```bash
# Register a schema
curl -X POST http://localhost:8080/api/v1/schemas \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "subject": "com.example.user",
    "schema": {"type": "object", "properties": {"id": {"type": "string"}}},
    "schema_type": "json"
  }'

# Get schema
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/schemas/<schema-id>

# Validate data
curl -X POST http://localhost:8080/api/v1/schemas/<schema-id>/validate \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "schema_id": "<schema-id>",
    "data": {"id": "user-123"},
    "strict": true
  }'
```

### gRPC Example (Python)

```python
import grpc
from schema_registry_pb2_grpc import SchemaRegistryStub
from schema_registry_pb2 import RegisterSchemaRequest, SchemaType

channel = grpc.insecure_channel('localhost:9090')
stub = SchemaRegistryStub(channel)

metadata = [('authorization', 'Bearer <token>')]

request = RegisterSchemaRequest(
    subject="com.example.user",
    schema_content=b'{"type": "object"}',
    schema_type=SchemaType.SCHEMA_TYPE_JSON,
    auto_version=True
)

response = stub.RegisterSchema(request, metadata=metadata)
print(f"Schema ID: {response.schema_id}")
```

## Authentication

### JWT Bearer Token
```bash
curl -H "Authorization: Bearer <jwt-token>" <url>
```

### API Key
```bash
curl -H "Authorization: llmsr_<api-key>" <url>
```

## Features

### REST API Features
- ✅ 17 operational endpoints
- ✅ OpenAPI 3.0 specification
- ✅ Interactive Swagger UI
- ✅ WebSocket support for real-time updates
- ✅ Comprehensive error responses
- ✅ Pagination support
- ✅ CORS enabled

### gRPC API Features
- ✅ 20 service methods
- ✅ Server streaming for lists
- ✅ Bidirectional streaming for batch operations
- ✅ Service reflection support
- ✅ TLS/mTLS support
- ✅ Compression support

### Authentication & Authorization
- ✅ JWT (HS256/RS256)
- ✅ API Keys
- ✅ OAuth 2.0 (Google, Microsoft, Okta, Auth0)
- ✅ mTLS
- ✅ Role-Based Access Control (RBAC)
- ✅ Fine-grained permissions

### Middleware
- ✅ Request logging with correlation IDs
- ✅ Rate limiting
- ✅ CORS
- ✅ Gzip compression
- ✅ Security headers
- ✅ Request timeout enforcement

## Documentation Structure

```
docs/api/
├── README.md                           # This file
├── API-IMPLEMENTATION-REPORT.md        # Complete implementation overview
├── API-GUIDE.md                        # REST API guide
└── GRPC-GUIDE.md                       # gRPC API guide
```

## Implementation Files

```
crates/schema-registry-api/
├── src/
│   ├── models/                        # Request/response models
│   ├── auth/                          # Authentication & authorization
│   ├── middleware/                    # API middleware
│   ├── rest/                          # REST API handlers
│   └── grpc/                          # gRPC service implementation
└── proto/
    └── schema_registry.proto          # Protocol buffer definition
```

## Support

For detailed documentation, see the individual guide files:
- REST API operations: [API-GUIDE.md](./API-GUIDE.md)
- gRPC operations: [GRPC-GUIDE.md](./GRPC-GUIDE.md)
- Implementation details: [API-IMPLEMENTATION-REPORT.md](./API-IMPLEMENTATION-REPORT.md)
