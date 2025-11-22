# Schema Registry Server

A high-performance REST API server for the LLM Schema Registry, built with Axum.

## Features

- **REST API Endpoints**:
  - `POST /api/v1/schemas` - Register a new schema
  - `GET /api/v1/schemas/:id` - Retrieve schema by ID
  - `POST /api/v1/validate/:id` - Validate data against schema
  - `POST /api/v1/compatibility/check` - Check schema compatibility
  - `GET /health` - Health check endpoint

- **Performance Optimizations**:
  - PostgreSQL connection pooling (50 connections)
  - Redis caching with 1-hour TTL
  - Async/await throughout
  - Sub-10ms read latency from cache
  - Sub-100ms write latency

- **Observability**:
  - Prometheus metrics on port 9091 (`GET /metrics`)
  - Structured logging with tracing
  - Request tracing with tower-http

## Configuration

The server is configured via environment variables:

- `DATABASE_URL` - PostgreSQL connection string (default: `postgresql://postgres:postgres@localhost:5432/schema_registry`)
- `REDIS_URL` - Redis connection string (default: `redis://localhost:6379`)
- `SERVER_HOST` - Server bind address (default: `0.0.0.0`)
- `SERVER_PORT` - Server port (default: `8080`)
- `METRICS_PORT` - Prometheus metrics port (default: `9091`)

## Running the Server

### Prerequisites

1. PostgreSQL 14+ running
2. Redis 6+ running

### Start the server

```bash
# Set environment variables
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/schema_registry"
export REDIS_URL="redis://localhost:6379"

# Run the server
cargo run -p schema-registry-server
```

The server will:
1. Connect to PostgreSQL and Redis
2. Run database migrations automatically
3. Start the API server on port 8080
4. Start the metrics server on port 9091

## API Examples

### Register a Schema

```bash
curl -X POST http://localhost:8080/api/v1/schemas \
  -H "Content-Type: application/json" \
  -d '{
    "subject": "test.schema.user",
    "schema": {
      "type": "object",
      "properties": {
        "id": {"type": "string"},
        "name": {"type": "string"}
      },
      "required": ["id"]
    },
    "schema_type": "json"
  }'
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "version": "1.0.0",
  "created_at": "2025-01-15T10:30:00Z"
}
```

### Get Schema by ID

```bash
curl http://localhost:8080/api/v1/schemas/550e8400-e29b-41d4-a716-446655440000
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "namespace": "test.schema",
  "name": "user",
  "version": "1.0.0",
  "format": "JSON",
  "schema": {
    "type": "object",
    "properties": {
      "id": {"type": "string"},
      "name": {"type": "string"}
    },
    "required": ["id"]
  },
  "content": "{\"type\":\"object\",...}",
  "state": "DRAFT",
  "compatibility_mode": "BACKWARD",
  "created_at": "2025-01-15T10:30:00Z",
  "updated_at": "2025-01-15T10:30:00Z"
}
```

### Validate Data

```bash
curl -X POST http://localhost:8080/api/v1/validate/550e8400-e29b-41d4-a716-446655440000 \
  -H "Content-Type: application/json" \
  -d '{
    "id": "user123",
    "name": "John Doe"
  }'
```

Response:
```json
{
  "is_valid": true,
  "errors": []
}
```

### Check Compatibility

```bash
curl -X POST http://localhost:8080/api/v1/compatibility/check \
  -H "Content-Type: application/json" \
  -d '{
    "schema_id": "550e8400-e29b-41d4-a716-446655440000",
    "compared_schema_id": "660e8400-e29b-41d4-a716-446655440001",
    "mode": "BACKWARD"
  }'
```

Response:
```json
{
  "is_compatible": true,
  "mode": "BACKWARD",
  "violations": []
}
```

### Health Check

```bash
curl http://localhost:8080/health
```

Response:
```json
{
  "status": "healthy",
  "components": {
    "database": {
      "status": "up",
      "message": null
    },
    "redis": {
      "status": "up",
      "message": null
    }
  }
}
```

### Prometheus Metrics

```bash
curl http://localhost:9091/metrics
```

## Performance Testing

The server is designed to handle the k6 load tests in `tests/load/`:

```bash
# Run basic load test
k6 run tests/load/basic_load.js

# Run stress test
k6 run tests/load/stress_test.js

# Run spike test
k6 run tests/load/spike_test.js

# Run soak test
k6 run tests/load/soak_test.js
```

### Performance Targets

- **Throughput**: >10,000 req/sec sustained
- **Read Latency**: p95 < 10ms (from cache)
- **Write Latency**: p95 < 100ms
- **Error Rate**: < 1%

## Architecture

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────┐
│     Axum HTTP Server            │
│  - Request routing              │
│  - JSON serialization           │
│  - Error handling               │
│  - Tracing middleware           │
└──────┬──────────────────────────┘
       │
       ├──────────────┬────────────────┐
       │              │                │
       ▼              ▼                ▼
┌────────────┐  ┌──────────┐  ┌──────────────┐
│ PostgreSQL │  │  Redis   │  │ Prometheus   │
│   (L2)     │  │  (L1)    │  │  (Metrics)   │
│            │  │          │  │              │
│ - Schemas  │  │ - Cache  │  │ - /metrics   │
│ - Metadata │  │ - 1h TTL │  │   :9091      │
└────────────┘  └──────────┘  └──────────────┘
```

## Caching Strategy

1. **L1 (Redis)**: Hot cache with 1-hour TTL
   - All schema reads check Redis first
   - Cache misses fallback to PostgreSQL
   - Writes update both PostgreSQL and Redis

2. **L2 (PostgreSQL)**: Persistent storage
   - Source of truth for all schemas
   - Indexed on id, namespace, name, version
   - Connection pooling for performance

## Database Migrations

Migrations are automatically applied on server startup using sqlx. Migration files are in `/migrations/`:

- `001_init.sql` - Initial schema tables
- `002_performance_indexes.sql` - Performance optimizations

## Development

### Build

```bash
cargo build -p schema-registry-server
```

### Test

```bash
cargo test -p schema-registry-server
```

### Run with debug logging

```bash
RUST_LOG=debug cargo run -p schema-registry-server
```

## Production Deployment

For production deployment:

1. Use environment-specific configuration
2. Enable TLS/HTTPS
3. Configure proper connection pool sizes
4. Set up monitoring and alerting on Prometheus metrics
5. Use a reverse proxy (nginx, Envoy) for load balancing
6. Enable rate limiting
7. Configure backup strategies for PostgreSQL

## License

Apache 2.0
