# LLM Schema Registry - API Guide

## Overview

The LLM Schema Registry provides both REST and gRPC APIs for managing, validating, and checking compatibility of schemas across your LLM applications.

## Base URLs

- **REST API:** `http://localhost:8080/api/v1`
- **gRPC API:** `localhost:9090`
- **Swagger UI:** `http://localhost:8080/swagger-ui`
- **Health Check:** `http://localhost:8080/health`
- **Metrics:** `http://localhost:8080/metrics`

## Authentication

### JWT Bearer Token

```bash
curl -H "Authorization: Bearer <your-jwt-token>" \
  http://localhost:8080/api/v1/schemas
```

### API Key

```bash
curl -H "Authorization: llmsr_<your-api-key>" \
  http://localhost:8080/api/v1/schemas
```

## REST API Endpoints

### Schema Management

#### 1. Register New Schema

**POST** `/api/v1/schemas`

Registers a new schema or a new version of an existing schema.

```bash
curl -X POST http://localhost:8080/api/v1/schemas \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "subject": "com.example.user.created",
    "schema": {
      "type": "object",
      "properties": {
        "id": {"type": "string", "format": "uuid"},
        "email": {"type": "string", "format": "email"},
        "name": {"type": "string"},
        "created_at": {"type": "string", "format": "date-time"}
      },
      "required": ["id", "email"]
    },
    "schema_type": "json",
    "compatibility_level": "BACKWARD",
    "description": "User creation event schema",
    "tags": ["user", "events", "v1"],
    "auto_version": true
  }'
```

**Response:**
```json
{
  "schema_id": "550e8400-e29b-41d4-a716-446655440000",
  "version": "1.0.0",
  "subject": "com.example.user.created",
  "created_at": "2024-11-22T10:30:00Z",
  "checksum": "sha256:abc123..."
}
```

#### 2. Get Schema by ID

**GET** `/api/v1/schemas/{id}`

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/schemas/550e8400-e29b-41d4-a716-446655440000
```

**Response:**
```json
{
  "schema": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "subject": "com.example.user.created",
    "version": "1.0.0",
    "schema_type": "json",
    "schema_content": { ... },
    "metadata": {
      "description": "User creation event schema",
      "tags": ["user", "events", "v1"],
      "owner": "user@example.com",
      "compatibility_level": "BACKWARD",
      "custom": {}
    },
    "created_at": "2024-11-22T10:30:00Z",
    "updated_at": "2024-11-22T10:30:00Z",
    "state": "ACTIVE",
    "checksum": "sha256:abc123..."
  }
}
```

#### 3. List Schemas

**GET** `/api/v1/schemas?limit=50&offset=0&subject_prefix=com.example&schema_type=json&state=ACTIVE`

```bash
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/schemas?limit=50&offset=0&subject_prefix=com.example"
```

**Response:**
```json
{
  "schemas": [
    { ... },
    { ... }
  ],
  "total_count": 125,
  "limit": 50,
  "offset": 0
}
```

#### 4. Update Schema Metadata

**PUT** `/api/v1/schemas/{id}`

Updates non-schema fields (description, tags, state, etc.).

```bash
curl -X PUT http://localhost:8080/api/v1/schemas/550e8400-e29b-41d4-a716-446655440000 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "description": "Updated description",
    "tags": ["user", "events", "v2", "production"],
    "state": "DEPRECATED"
  }'
```

#### 5. Delete Schema

**DELETE** `/api/v1/schemas/{id}?soft=true`

```bash
# Soft delete (default)
curl -X DELETE \
  -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/schemas/550e8400-e29b-41d4-a716-446655440000?soft=true"

# Hard delete
curl -X DELETE \
  -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/schemas/550e8400-e29b-41d4-a716-446655440000?soft=false"
```

### Validation

#### 6. Validate Data Against Schema

**POST** `/api/v1/schemas/{id}/validate`

```bash
curl -X POST http://localhost:8080/api/v1/schemas/550e8400-e29b-41d4-a716-446655440000/validate \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "schema_id": "550e8400-e29b-41d4-a716-446655440000",
    "data": {
      "id": "user-123",
      "email": "user@example.com",
      "name": "John Doe",
      "created_at": "2024-11-22T10:30:00Z"
    },
    "strict": true
  }'
```

**Success Response:**
```json
{
  "valid": true,
  "errors": [],
  "warnings": [],
  "validation_time_ms": 2.3,
  "schema_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Failure Response:**
```json
{
  "valid": false,
  "errors": [
    {
      "path": "/email",
      "message": "Invalid email format",
      "error_type": "format"
    }
  ],
  "warnings": [
    {
      "path": "/created_at",
      "message": "Field uses deprecated format",
      "warning_type": "deprecation"
    }
  ],
  "validation_time_ms": 1.8,
  "schema_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### 7. Validate Schema Structure

**POST** `/api/v1/validate/schema`

```bash
curl -X POST http://localhost:8080/api/v1/validate/schema \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "schema": {
      "type": "object",
      "properties": {
        "id": {"type": "string"}
      }
    },
    "schema_type": "json"
  }'
```

### Compatibility Checking

#### 8. Check Schema Compatibility

**POST** `/api/v1/compatibility/check`

```bash
curl -X POST http://localhost:8080/api/v1/compatibility/check \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "subject": "com.example.user.created",
    "new_schema": {
      "type": "object",
      "properties": {
        "id": {"type": "string", "format": "uuid"},
        "email": {"type": "string", "format": "email"},
        "name": {"type": "string"},
        "phone": {"type": "string"},
        "created_at": {"type": "string", "format": "date-time"}
      },
      "required": ["id", "email"]
    },
    "level": "BACKWARD",
    "compare_version": "1.0.0"
  }'
```

**Compatible Response:**
```json
{
  "compatible": true,
  "level": "BACKWARD",
  "violations": [],
  "compared_versions": ["1.0.0"],
  "message": "Schema is backward compatible"
}
```

**Incompatible Response:**
```json
{
  "compatible": false,
  "level": "BACKWARD",
  "violations": [
    {
      "rule": "new_required_field",
      "path": "/required/phone",
      "message": "New required field 'phone' breaks backward compatibility",
      "severity": "ERROR"
    }
  ],
  "compared_versions": ["1.0.0"],
  "message": "Schema has 1 compatibility violation(s)"
}
```

### Search & Discovery

#### 9. Search Schemas

**POST** `/api/v1/search`

```bash
curl -X POST http://localhost:8080/api/v1/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "query": "user",
    "subject_pattern": "com.example.*",
    "schema_type": "json",
    "tags": ["events", "production"],
    "metadata_filters": {
      "owner": "team-a@example.com"
    },
    "limit": 20,
    "offset": 0
  }'
```

#### 10. Get Schema Dependencies

**GET** `/api/v1/schemas/{id}/dependencies?transitive=true`

```bash
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/schemas/550e8400-e29b-41d4-a716-446655440000/dependencies?transitive=true"
```

**Response:**
```json
{
  "dependencies": [
    {
      "schema_id": "660e8400-e29b-41d4-a716-446655440001",
      "subject": "com.example.address",
      "version": "2.1.0",
      "dependency_type": "reference",
      "depth": 1
    }
  ],
  "total_count": 1
}
```

#### 11. Get Schema Dependents

**GET** `/api/v1/schemas/{id}/dependents?transitive=false`

```bash
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/schemas/550e8400-e29b-41d4-a716-446655440000/dependents?transitive=false"
```

### Subject Management

#### 12. List Subjects

**GET** `/api/v1/subjects?prefix=com.example&limit=100&offset=0`

```bash
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/subjects?prefix=com.example&limit=100"
```

**Response:**
```json
{
  "subjects": [
    "com.example.user.created",
    "com.example.user.updated",
    "com.example.order.placed"
  ],
  "total_count": 3
}
```

#### 13. Get Subject Versions

**GET** `/api/v1/subjects/{subject}/versions`

```bash
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/subjects/com.example.user.created/versions"
```

**Response:**
```json
{
  "subject": "com.example.user.created",
  "versions": [
    {
      "version": "1.0.0",
      "schema_id": "550e8400-e29b-41d4-a716-446655440000",
      "created_at": "2024-01-15T10:00:00Z",
      "state": "ACTIVE"
    },
    {
      "version": "1.1.0",
      "schema_id": "550e8400-e29b-41d4-a716-446655440001",
      "created_at": "2024-03-20T14:30:00Z",
      "state": "ACTIVE"
    },
    {
      "version": "2.0.0",
      "schema_id": "550e8400-e29b-41d4-a716-446655440002",
      "created_at": "2024-11-22T09:15:00Z",
      "state": "ACTIVE"
    }
  ]
}
```

#### 14. Get Schema by Subject and Version

**GET** `/api/v1/subjects/{subject}/versions/{version}`

```bash
# Get specific version
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/subjects/com.example.user.created/versions/1.0.0"

# Get latest version
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/subjects/com.example.user.created/versions/latest"
```

### WebSocket - Real-time Updates

#### 15. Subscribe to Schema Changes

**WS** `/api/v1/schemas/subscribe`

```javascript
// JavaScript WebSocket example
const ws = new WebSocket('ws://localhost:8080/api/v1/schemas/subscribe');

ws.onopen = () => {
  // Send subscription configuration
  ws.send(JSON.stringify({
    subjects: ["com.example.*", "com.acme.order.*"],
    event_types: ["SCHEMA_REGISTERED", "SCHEMA_UPDATED", "SCHEMA_DEPRECATED"]
  }));
};

ws.onmessage = (event) => {
  const change = JSON.parse(event.data);
  console.log('Schema change:', change);
  /*
  {
    "event_type": "SCHEMA_REGISTERED",
    "schema_id": "550e8400-e29b-41d4-a716-446655440003",
    "subject": "com.example.product.created",
    "version": "1.0.0",
    "timestamp": "2024-11-22T11:00:00Z",
    "metadata": {},
    "changed_by": "user@example.com"
  }
  */
};
```

### Health & Monitoring

#### 16. Health Check

**GET** `/health`

```bash
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "healthy",
  "components": {
    "database": {
      "status": "up",
      "message": "Connected",
      "details": {
        "connection_pool_size": "45",
        "active_connections": "12"
      }
    },
    "cache": {
      "status": "up",
      "message": "Connected",
      "details": {
        "hit_rate": "0.95",
        "entries": "1234"
      }
    },
    "storage": {
      "status": "up",
      "message": "S3 accessible",
      "details": {}
    }
  },
  "version": "0.1.0",
  "timestamp": "2024-11-22T11:00:00Z"
}
```

#### 17. Prometheus Metrics

**GET** `/metrics`

```bash
curl http://localhost:8080/metrics
```

**Response:**
```
# HELP schemas_total Total number of registered schemas
# TYPE schemas_total gauge
schemas_total 1234

# HELP schema_validations_total Total number of validations performed
# TYPE schema_validations_total counter
schema_validations_total 45678

# HELP schema_validation_duration_seconds Schema validation duration
# TYPE schema_validation_duration_seconds histogram
schema_validation_duration_seconds_bucket{le="0.001"} 1234
schema_validation_duration_seconds_bucket{le="0.005"} 5678
schema_validation_duration_seconds_bucket{le="0.01"} 8901
...
```

## Error Responses

All error responses follow this format:

```json
{
  "error_code": "NOT_FOUND",
  "message": "Schema 550e8400-e29b-41d4-a716-446655440000 not found",
  "details": {
    "schema_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "request_id": "req_abc123xyz"
}
```

### Common Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `BAD_REQUEST` | 400 | Invalid request parameters |
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `CONFLICT` | 409 | Resource conflict |
| `INCOMPATIBLE_SCHEMA` | 409 | Schema incompatible with existing versions |
| `INVALID_SCHEMA` | 400 | Schema is malformed |
| `VALIDATION_FAILED` | 400 | Data validation failed |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

## Rate Limiting

Default rate limits:
- **Authenticated users:** 1000 requests/minute
- **Service accounts:** 5000 requests/minute
- **Admin accounts:** 10000 requests/minute

Rate limit headers:
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 995
X-RateLimit-Reset: 1634567890
```

## Pagination

List endpoints support pagination:

```bash
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/schemas?limit=50&offset=100"
```

- `limit`: Number of results (max: 1000, default: 50)
- `offset`: Starting position (default: 0)

## Versioning

The API is versioned via URL path:
- Current version: `/api/v1/`
- Future versions: `/api/v2/`, `/api/v3/`, etc.

Version deprecation will be announced 6 months in advance.

## CORS

CORS is enabled for all origins in development. In production, configure allowed origins via environment variables:

```bash
CORS_ALLOWED_ORIGINS=https://app.example.com,https://dashboard.example.com
```

## Request/Response Headers

### Common Request Headers
- `Authorization`: Bearer token or API key
- `Content-Type`: `application/json`
- `Accept`: `application/json`
- `X-Correlation-ID`: Optional correlation ID for tracing

### Common Response Headers
- `X-Correlation-ID`: Correlation ID for request tracing
- `X-Request-ID`: Unique request identifier
- `X-RateLimit-*`: Rate limiting information

## Next Steps

- [gRPC API Guide](./GRPC-GUIDE.md)
- [Authentication Guide](./AUTHENTICATION.md)
- [SDK Examples](./SDK-EXAMPLES.md)
- [Integration Patterns](./INTEGRATION-PATTERNS.md)
