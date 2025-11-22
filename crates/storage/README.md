# Schema Registry Storage Layer

High-performance storage abstraction layer for the LLM Schema Registry, providing PostgreSQL, Redis, and S3 backends with multi-tier caching.

## Architecture

### Storage Hierarchy

```
┌─────────────────────────────────────────────────────────┐
│                   Application Layer                      │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              StorageBackend Trait                        │
│  (Unified interface for all storage operations)         │
└────────────────────┬────────────────────────────────────┘
                     │
         ┌───────────┼───────────┐
         │           │           │
         ▼           ▼           ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  PostgreSQL  │ │    Redis     │ │      S3      │
│  (Metadata)  │ │   (Cache)    │ │  (Content)   │
└──────────────┘ └──────────────┘ └──────────────┘
```

### Data Flow

```
Read Request:
  1. Check L1 cache (in-memory Moka)
  2. Check L2 cache (Redis) [if configured]
  3. Query PostgreSQL
  4. Store in S3 (if large content)
  5. Populate caches on the way back

Write Request:
  1. Validate schema
  2. Begin PostgreSQL transaction
  3. Store metadata in PostgreSQL
  4. Store large content in S3 (if > threshold)
  5. Invalidate caches
  6. Commit transaction
```

## Components

### 1. StorageBackend Trait

Unified interface for all storage operations:

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn register_schema(&self, schema: &Schema) -> Result<()>;
    async fn get_schema(&self, id: SchemaId) -> Result<Option<Schema>>;
    async fn list_schemas(&self, subject: &str, filter: &SchemaFilter) -> Result<Vec<Schema>>;
    async fn search_schemas(&self, query: &SearchQuery) -> Result<Vec<Schema>>;
    async fn update_schema_state(&self, id: SchemaId, state: SchemaState) -> Result<()>;
    async fn delete_schema(&self, id: SchemaId) -> Result<()>;
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;
    async fn statistics(&self) -> Result<StorageStatistics>;
}
```

### 2. PostgreSQL Backend

Primary metadata store with ACID guarantees.

**Features:**
- Connection pooling (deadpool-postgres)
- Compiled query verification (sqlx macros)
- JSONB storage for flexible schema content
- Full-text search with pg_trgm
- Materialized views for performance
- Automatic migrations

**Performance:**
- Connection pool: 50 max, 10 min connections
- Query timeout: 3 seconds
- Prepared statement caching
- Index-only scans for common queries

**Schema Design:**
```sql
CREATE TABLE schemas (
    id UUID PRIMARY KEY,
    subject VARCHAR(255) NOT NULL,
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL,
    schema_type VARCHAR(50) NOT NULL,
    content JSONB NOT NULL,
    metadata JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    deleted_at TIMESTAMPTZ,
    CONSTRAINT unique_subject_version UNIQUE (...)
);
```

**Indexes:**
- B-tree indexes on subject, version components
- GIN indexes on JSONB fields (metadata, content)
- Partial indexes for active schemas
- Covering indexes for index-only scans

### 3. Redis Cache (L2)

Distributed caching layer with graceful degradation.

**Features:**
- Connection manager for automatic reconnection
- Configurable TTL (default: 3600s)
- Batch operations via pipelining
- Graceful degradation on failures
- Metrics tracking

**Performance:**
- Target hit rate: >95%
- Average latency: <1ms
- Serialization: bincode (compact binary)

**Usage:**
```rust
let cache = RedisCache::new("redis://localhost:6379", 3600).await?;

// Get with fallback
if let Some(schema) = cache.get(id).await? {
    return Ok(schema); // Cache hit
}
// Cache miss - fetch from PostgreSQL and populate
let schema = postgres.get_schema(id).await?;
cache.put(id, &schema).await?;
```

### 4. In-Memory Cache (L1)

High-performance async cache using Moka.

**Features:**
- Time-based eviction (TTL + TTI)
- Size-based eviction (LRU)
- Lock-free concurrent access
- Automatic cleanup
- Statistics tracking

**Configuration:**
```rust
let config = CacheConfig {
    l1_max_entries: 10_000,
    l1_ttl_seconds: 3600,
    enable_statistics: true,
    warm_on_startup: true,
    warm_size: 100,
};
```

**Performance:**
- Capacity: 10,000 entries (configurable)
- Average latency: <100μs
- Concurrency: lock-free
- Memory: ~1MB per 100 schemas

### 5. S3 Backend

Large schema content storage with lifecycle policies.

**Features:**
- Automatic size-based routing
- Presigned URL generation
- Versioned object storage
- Lifecycle policies
- Multipart upload support

**Configuration:**
```rust
let s3 = S3Backend::new(
    "schema-registry-bucket".to_string(),
    "us-east-1",
    Some("schemas/".to_string()),
    1048576, // 1MB threshold
).await?;
```

**Usage:**
```rust
// Store large content
if s3.should_use_s3(&schema.content) {
    let key = s3.put_schema_content(schema.id, &schema.content).await?;
    // Store reference in PostgreSQL
}

// Generate presigned URL for direct access
let url = s3.generate_presigned_url(
    schema.id,
    Duration::from_secs(3600)
).await?;
```

## Performance Optimization

### Cache Hit Rate Optimization

**Strategies:**

1. **Cache Warming**
   - Pre-load popular schemas on startup
   - Uses materialized view `popular_schemas`
   - Tracks access patterns in `schema_access_log`

2. **Pareto Principle**
   - 80% of requests → 20% of schemas
   - Prioritize hot schemas in L1 cache
   - Larger TTL for frequently accessed schemas

3. **Access Pattern Analysis**
   ```sql
   SELECT * FROM popular_schemas
   ORDER BY access_count DESC
   LIMIT 100;
   ```

4. **Metrics**
   - Cache hit rate: >95% target
   - L1 hit rate: ~70%
   - L2 (Redis) hit rate: ~25%
   - Database queries: <5%

### Query Optimization

**Techniques:**

1. **Index Selection**
   - Partial indexes for common filters
   - Covering indexes for index-only scans
   - Expression indexes for computed values

2. **Materialized Views**
   - `subject_versions`: Fast version listings
   - `popular_schemas`: Cache warming
   - Automatic refresh triggers

3. **Query Patterns**
   ```sql
   -- Fast latest version lookup (index-only scan)
   SELECT id, subject, version_major, version_minor, version_patch
   FROM schemas
   WHERE subject = $1 AND deleted_at IS NULL
   ORDER BY version_major DESC, version_minor DESC, version_patch DESC
   LIMIT 1;

   -- Tag search (GIN index)
   SELECT * FROM schemas
   WHERE metadata->'tags' ? 'production'
     AND deleted_at IS NULL;
   ```

### Connection Pooling

**Configuration:**
```rust
PgPoolOptions::new()
    .max_connections(50)      // Handle burst traffic
    .min_connections(10)       // Keep connections warm
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
```

**Benefits:**
- Reduced connection overhead
- Better resource utilization
- Automatic health checks
- Connection recycling

## Performance Targets

| Operation | Target Latency (p95) | Throughput | Notes |
|-----------|---------------------|------------|-------|
| Get schema (cached) | <1ms | 100k ops/sec | L1 cache hit |
| Get schema (Redis) | <5ms | 50k ops/sec | L2 cache hit |
| Get schema (DB) | <10ms | 10k ops/sec | Database query |
| Register schema | <100ms | 1k ops/sec | With validation |
| Search schemas | <200ms | 500 ops/sec | Complex queries |
| Update state | <50ms | 2k ops/sec | Simple update |
| Delete schema | <50ms | 2k ops/sec | Soft delete |

## Benchmarks

Run performance benchmarks:

```bash
cd crates/storage
cargo bench
```

**Benchmark Suites:**

1. **cache_operations**
   - Cache put/get performance
   - Hot vs cold access
   - Different cache sizes

2. **serialization**
   - JSON vs Bincode
   - Serialize vs deserialize
   - Different schema sizes

3. **concurrent_access**
   - 1, 10, 50, 100 concurrent requests
   - Lock contention analysis

4. **cache_hit_rates**
   - Realistic access patterns
   - Pareto distribution simulation

## Migration Management

### Setup

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Set database URL
export DATABASE_URL="postgresql://user:password@localhost/schema_registry"

# Run migrations
sqlx migrate run
```

### Migrations

1. **001_initial_schema.sql**
   - Core tables
   - Indexes
   - Functions
   - Triggers

2. **002_indexes_optimization.sql**
   - Advanced indexes
   - Access logging
   - Popular schemas view

### Rollback

```bash
# Revert last migration
sqlx migrate revert

# Check status
sqlx migrate info
```

## Configuration

### Environment Variables

```bash
# PostgreSQL
DATABASE_URL="postgresql://user:password@localhost:5432/schema_registry"
DATABASE_MAX_CONNECTIONS=50
DATABASE_MIN_CONNECTIONS=10

# Redis (optional)
REDIS_URL="redis://localhost:6379"
REDIS_TTL_SECONDS=3600

# S3 (optional)
S3_BUCKET="schema-registry-content"
S3_REGION="us-east-1"
S3_PREFIX="schemas/"
S3_MIN_SIZE_BYTES=1048576

# Cache
CACHE_L1_MAX_ENTRIES=10000
CACHE_L1_TTL_SECONDS=3600
CACHE_WARM_ON_STARTUP=true
CACHE_WARM_SIZE=100
```

### Configuration File

```toml
[storage]
backend = { type = "postgres", url = "postgresql://..." }

[storage.cache]
type = "redis"
url = "redis://localhost:6379"
ttl_seconds = 3600

[storage.s3]
bucket = "schema-registry-content"
region = "us-east-1"
prefix = "schemas/"
min_size_bytes = 1048576

[storage.pool]
max_connections = 50
min_connections = 10
acquire_timeout_secs = 3
idle_timeout_secs = 600
max_lifetime_secs = 1800
```

## Monitoring

### Metrics

All operations emit Prometheus-compatible metrics:

```rust
// Counters
schema_registry.schemas.registered
schema_registry.schemas.fetched
schema_registry.schemas.deleted
schema_registry.cache.l1.hits
schema_registry.cache.l2.hits
schema_registry.cache.misses
schema_registry.s3.uploads
schema_registry.s3.downloads

// Histograms
schema_registry.query.duration_seconds
schema_registry.s3.upload_size_bytes
schema_registry.s3.download_size_bytes
```

### Health Checks

```rust
// PostgreSQL
let is_healthy = backend.pool().acquire().await.is_ok();

// Redis
let is_healthy = cache.ping().await.is_ok();

// S3
let is_healthy = s3.exists(test_id).await.is_ok();
```

### Statistics

```rust
let stats = backend.statistics().await?;
println!("Total schemas: {}", stats.total_schemas);
println!("Active schemas: {}", stats.active_schemas);
println!("Cache hit rate: {:.2}%", cache.hit_rate() * 100.0);
```

## Testing

### Unit Tests

```bash
cargo test --lib
```

### Integration Tests

Requires running services:

```bash
# Start PostgreSQL
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=test postgres:16

# Start Redis
docker run -d -p 6379:6379 redis:7

# Run tests
cargo test --features integration-tests
```

### Load Testing

```bash
# Run benchmarks
cargo bench

# Custom load test
cargo run --example load_test -- --duration 60s --concurrency 100
```

## Troubleshooting

### Connection Pool Exhausted

**Symptoms:** Timeout errors, slow queries

**Solutions:**
1. Increase `max_connections`
2. Reduce `acquire_timeout`
3. Check for connection leaks
4. Review long-running transactions

### Low Cache Hit Rate

**Symptoms:** High database load, slow responses

**Solutions:**
1. Increase L1 cache size
2. Increase TTL
3. Enable cache warming
4. Check access patterns
5. Verify Redis connectivity

### Slow Queries

**Symptoms:** High p95 latency

**Solutions:**
1. Run `ANALYZE` on tables
2. Check index usage
3. Review query plans with `EXPLAIN ANALYZE`
4. Add missing indexes
5. Partition large tables

### S3 Upload Failures

**Symptoms:** 5xx errors from S3

**Solutions:**
1. Check AWS credentials
2. Verify bucket permissions
3. Check region configuration
4. Enable retry logic
5. Monitor S3 quotas

## Production Deployment

### Checklist

- [ ] Database migrations tested on staging
- [ ] Connection pool tuned for load
- [ ] Indexes created and analyzed
- [ ] Cache warming configured
- [ ] S3 lifecycle policies set
- [ ] Monitoring dashboards created
- [ ] Alerting rules configured
- [ ] Backup strategy implemented
- [ ] Disaster recovery tested

### Scaling

**Horizontal Scaling:**
- Application servers scale independently
- Shared PostgreSQL (read replicas for reads)
- Shared Redis (cluster mode)
- S3 scales automatically

**Vertical Scaling:**
- PostgreSQL: increase instance size
- Redis: larger memory instances
- Application: more CPU/memory

### High Availability

**PostgreSQL:**
- Primary + read replicas
- Automatic failover
- Connection pooling
- Read/write splitting

**Redis:**
- Cluster mode
- Sentinel for failover
- Graceful degradation

**S3:**
- Multi-region replication
- Versioning enabled
- Lifecycle policies
