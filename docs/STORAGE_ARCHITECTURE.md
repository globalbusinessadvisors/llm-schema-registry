# Storage Layer Architecture

## Overview

The LLM Schema Registry storage layer implements a multi-tier architecture optimized for high-performance schema retrieval with <10ms p95 latency and >95% cache hit rates.

## Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│                         CLIENT REQUESTS                             │
└─────────────────────────────┬──────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     STORAGE ABSTRACTION LAYER                        │
│                      (StorageBackend Trait)                          │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  - register_schema()    - get_schema()                       │  │
│  │  - list_schemas()       - search_schemas()                   │  │
│  │  - update_schema_state() - delete_schema()                   │  │
│  │  - begin_transaction()  - statistics()                       │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────┬──────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
┌──────────────────┐ ┌──────────────┐ ┌──────────────┐
│   L1 CACHE       │ │  L2 CACHE    │ │  PRIMARY     │
│   (In-Memory)    │ │  (Redis)     │ │  STORAGE     │
│                  │ │              │ │ (PostgreSQL) │
│  Moka Cache      │ │ Distributed  │ │              │
│  - 10k entries   │ │ - TTL: 1h    │ │ ACID Store   │
│  - TTL: 1h       │ │ - Bincode    │ │ - Metadata   │
│  - Lock-free     │ │ - Pipelining │ │ - JSONB      │
│  - <100μs        │ │ - <5ms       │ │ - Full-text  │
│                  │ │              │ │ - <10ms      │
└──────────────────┘ └──────────────┘ └──────┬───────┘
                                              │
                                              │
                              ┌───────────────┴────────────┐
                              │                            │
                              ▼                            ▼
                    ┌──────────────┐           ┌──────────────┐
                    │   S3 STORE   │           │  ANALYTICS   │
                    │              │           │              │
                    │ Large Schema │           │ Access Logs  │
                    │ Content      │           │ Metrics      │
                    │ - >1MB       │           │ Statistics   │
                    │ - Versioned  │           │              │
                    │ - Lifecycle  │           │              │
                    └──────────────┘           └──────────────┘
```

## Data Flow

### Read Path (Optimized for Latency)

```
┌─────────┐
│ Request │
└────┬────┘
     │
     ▼
┌────────────────┐
│ L1 Cache Check │ ◄─────────────┐
└────┬───────────┘                │
     │ Miss                       │ Hit (70%)
     ▼                           │
┌────────────────┐               │
│ L2 Cache Check │ ◄─────────┐  │
└────┬───────────┘            │  │
     │ Miss                   │  │
     ▼                       │  │
┌────────────────┐  Hit (25%) │  │
│ PostgreSQL     │ ◄──────────┘  │
│ Query          │               │
└────┬───────────┘               │
     │                           │
     ▼                           │
┌────────────────┐               │
│ S3 Content?    │               │
│ (if large)     │               │
└────┬───────────┘               │
     │                           │
     ▼                           │
┌────────────────┐               │
│ Populate       │ ──────────────┘
│ Caches         │
└────┬───────────┘
     │
     ▼
┌─────────┐
│ Response│ (<10ms p95)
└─────────┘
```

### Write Path (ACID Compliant)

```
┌─────────┐
│ Request │
└────┬────┘
     │
     ▼
┌────────────────┐
│ Validation     │
│ - Schema valid │
│ - Size check   │
└────┬───────────┘
     │
     ▼
┌────────────────┐
│ Begin TX       │
│ (PostgreSQL)   │
└────┬───────────┘
     │
     ▼
┌────────────────┐
│ Store Metadata │
│ in PostgreSQL  │
└────┬───────────┘
     │
     ▼
┌────────────────┐
│ Content >1MB?  │
└────┬───────────┘
     │ Yes
     ▼
┌────────────────┐
│ Upload to S3   │
│ Store ref in   │
│ PostgreSQL     │
└────┬───────────┘
     │
     ▼
┌────────────────┐
│ Invalidate     │
│ All Caches     │
└────┬───────────┘
     │
     ▼
┌────────────────┐
│ Commit TX      │
└────┬───────────┘
     │
     ▼
┌─────────┐
│ Response│ (<100ms p95)
└─────────┘
```

## Database Schema

### Core Tables

```sql
-- Main schemas table (hot data)
schemas (
    id UUID PRIMARY KEY,
    subject VARCHAR(255),
    version_major/minor/patch INTEGER,
    schema_type VARCHAR(50),
    content JSONB,          -- Inline for small schemas
    s3_key VARCHAR(512),    -- Reference for large schemas
    metadata JSONB,
    created_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
)

-- Dependencies graph
schema_dependencies (
    schema_id UUID,
    depends_on_id UUID,
    dependency_type VARCHAR(50)
)

-- Audit trail (warm data)
validation_history (
    id BIGSERIAL,
    schema_id UUID,
    data_hash VARCHAR(64),
    valid BOOLEAN,
    error_count INTEGER,
    validated_at TIMESTAMPTZ,
    duration_ms DOUBLE PRECISION
)

-- Cache optimization
schema_access_log (
    id BIGSERIAL,
    schema_id UUID,
    accessed_at TIMESTAMPTZ,
    access_count INTEGER,
    response_time_ms DOUBLE PRECISION
)
```

### Materialized Views

```sql
-- Fast version listings
subject_versions (
    subject,
    version_count,
    latest_version_at,
    versions[]
)

-- Cache warming
popular_schemas (
    id,
    subject,
    version,
    access_count,
    avg_response_time_ms,
    last_accessed_at
)
REFRESH CONCURRENTLY EVERY 5 MINUTES
```

### Indexes Strategy

```sql
-- Primary lookups
CREATE INDEX idx_schemas_subject_version ON schemas(
    subject, version_major DESC, version_minor DESC, version_patch DESC
) WHERE deleted_at IS NULL;

-- Full-text search
CREATE INDEX idx_schemas_subject_trgm ON schemas 
USING GIN (subject gin_trgm_ops);

-- JSONB queries
CREATE INDEX idx_schemas_metadata_gin ON schemas 
USING GIN (metadata jsonb_path_ops);

-- Hot data (partial index)
CREATE INDEX idx_schemas_active ON schemas(subject, created_at DESC)
WHERE deleted_at IS NULL AND metadata->>'state' = 'ACTIVE';

-- Covering index (index-only scan)
CREATE INDEX idx_schemas_covering ON schemas(
    subject, id, created_at, version_major, version_minor, version_patch
) WHERE deleted_at IS NULL;
```

## Cache Strategy

### Multi-Tier Caching

**L1: In-Memory (Moka)**
- Purpose: Ultra-fast access for hot schemas
- Size: 10,000 entries (~10MB)
- TTL: 1 hour
- TTI: 30 minutes
- Eviction: Size + Time based
- Hit Rate Target: 70%
- Latency: <100μs

**L2: Redis**
- Purpose: Distributed cache for multi-instance deployments
- TTL: 1 hour
- Serialization: Bincode (compact binary)
- Pipeline: Batch operations
- Hit Rate Target: 25%
- Latency: <5ms
- Graceful Degradation: Continue without L2 on failure

**Total Cache Hit Rate Target: >95%**

### Cache Warming Strategy

```rust
// On startup, pre-load popular schemas
SELECT * FROM popular_schemas
ORDER BY access_count DESC
LIMIT 100;

// Continuously update based on access patterns
INSERT INTO schema_access_log (schema_id, accessed_at)
VALUES ($1, NOW())
ON CONFLICT (schema_id) 
DO UPDATE SET 
    access_count = schema_access_log.access_count + 1,
    last_accessed_at = NOW();
```

### Cache Invalidation

**On Write:**
```rust
// Invalidate specific schema
cache.invalidate(schema_id).await;

// Invalidate all versions of a subject
cache.invalidate_subject(subject).await;

// Distributed invalidation via Redis pub/sub
redis.publish("schema:invalidate", schema_id).await;
```

## Performance Characteristics

### Latency Distribution

```
Operation            p50      p95      p99      p999
─────────────────────────────────────────────────────
Get (L1 hit)        50μs     100μs    150μs    300μs
Get (L2 hit)        2ms      5ms      8ms      15ms
Get (DB hit)        5ms      10ms     20ms     50ms
Register            30ms     100ms    150ms    300ms
Search              50ms     200ms    400ms    1000ms
Update State        15ms     50ms     75ms     150ms
Delete (soft)       10ms     50ms     75ms     150ms
```

### Throughput Capacity

```
Operation            Throughput     Notes
────────────────────────────────────────────
Cached Reads         100k ops/sec   L1 cache
Redis Reads          50k ops/sec    L2 cache
Database Reads       10k ops/sec    Indexed queries
Writes               1k ops/sec     With validation
Concurrent Conn      5000           PostgreSQL pool
```

### Resource Utilization

```
Component            Memory      CPU         Storage
──────────────────────────────────────────────────────
L1 Cache            ~10MB        Low         -
Redis               ~500MB       Low         Ephemeral
PostgreSQL          ~2GB         Medium      Growing
Application         ~200MB       Medium      -
```

## Scaling Strategy

### Horizontal Scaling

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   App Node 1 │     │   App Node 2 │     │   App Node N │
│              │     │              │     │              │
│  L1 Cache    │     │  L1 Cache    │     │  L1 Cache    │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       └────────────────────┼────────────────────┘
                            │
                            ▼
                    ┌──────────────┐
                    │ Redis Cluster│
                    │ (Shared L2)  │
                    └──────┬───────┘
                            │
                            ▼
                    ┌──────────────┐
                    │  PostgreSQL  │
                    │   Primary +  │
                    │  Replicas    │
                    └──────┬───────┘
                            │
                            ▼
                    ┌──────────────┐
                    │   S3 Bucket  │
                    │ (Auto-scale) │
                    └──────────────┘
```

### Vertical Scaling Recommendations

**Database:**
- Start: 2 vCPU, 4GB RAM
- Medium: 4 vCPU, 16GB RAM
- Large: 8 vCPU, 32GB RAM
- Monitor: `pg_stat_statements`, connection pool usage

**Redis:**
- Start: 1GB memory
- Medium: 4GB memory  
- Large: 16GB memory
- Monitor: Hit rate, eviction rate

**Application:**
- Start: 1 vCPU, 512MB RAM
- Medium: 2 vCPU, 2GB RAM
- Large: 4 vCPU, 4GB RAM
- Monitor: Request latency, cache hit rates

## Monitoring & Observability

### Key Metrics

```rust
// Throughput
schema_registry.operations.total (counter)
schema_registry.operations.errors (counter)

// Latency  
schema_registry.operation.duration_seconds (histogram)

// Cache Performance
schema_registry.cache.hit_rate (gauge)
schema_registry.cache.l1.hits (counter)
schema_registry.cache.l2.hits (counter)
schema_registry.cache.misses (counter)

// Storage
schema_registry.storage.size_bytes (gauge)
schema_registry.storage.schemas.total (gauge)
schema_registry.storage.schemas.active (gauge)

// Database
schema_registry.db.connections.active (gauge)
schema_registry.db.connections.idle (gauge)
schema_registry.db.query.duration_seconds (histogram)

// S3
schema_registry.s3.operations.total (counter)
schema_registry.s3.upload_size_bytes (histogram)
schema_registry.s3.download_size_bytes (histogram)
```

### Health Checks

```rust
/health/live  - Process alive
/health/ready - All dependencies ready
  - PostgreSQL connection pool
  - Redis connectivity (if configured)
  - S3 bucket access (if configured)

/health/storage - Storage-specific health
  - Database query time
  - Cache hit rate
  - Active connections
  - Storage statistics
```

### Alerting Rules

```yaml
- Alert: HighCacheMissRate
  Expr: schema_registry.cache.hit_rate < 0.90
  Duration: 5m
  Severity: warning

- Alert: DatabaseConnectionPoolExhausted
  Expr: schema_registry.db.connections.active / max_connections > 0.90
  Duration: 2m
  Severity: critical

- Alert: SlowQueries
  Expr: histogram_quantile(0.95, schema_registry.db.query.duration_seconds) > 0.1
  Duration: 5m
  Severity: warning

- Alert: HighErrorRate
  Expr: rate(schema_registry.operations.errors[5m]) > 0.01
  Duration: 2m
  Severity: critical
```

## Disaster Recovery

### Backup Strategy

**PostgreSQL:**
```bash
# Daily full backup
pg_dump -Fc schema_registry > backup_$(date +%Y%m%d).dump

# Point-in-time recovery
# Enable WAL archiving
archive_mode = on
archive_command = 'cp %p /backups/wal/%f'
```

**S3:**
- Versioning enabled
- Cross-region replication
- Lifecycle policies (retain 90 days)

**Redis:**
- RDB snapshots every 6 hours
- AOF for durability (optional)
- Not critical (cache can be rebuilt)

### Recovery Procedures

**Database Failure:**
1. Promote read replica to primary
2. Update connection strings
3. Warm cache from new primary
4. Estimated RTO: <5 minutes

**Redis Failure:**
1. Application continues with L1 cache only
2. Restore Redis from backup
3. Cache rebuilds automatically
4. Estimated RTO: <1 minute

**S3 Failure:**
1. New schemas stored in PostgreSQL temporarily
2. Failover to backup region
3. Update S3 configuration
4. Estimated RTO: <10 minutes

## Security Considerations

### Access Control

**Database:**
- Separate read/write roles
- Row-level security (RLS) for multi-tenancy
- SSL/TLS connections required
- IP whitelisting

**Redis:**
- AUTH password required
- TLS encryption
- Network isolation

**S3:**
- Bucket policies
- IAM roles
- Encryption at rest (SSE-S3)
- Encryption in transit

### Data Protection

**Encryption:**
- Database: Transparent data encryption (TDE)
- Redis: TLS in transit
- S3: Server-side encryption
- Application: Sensitive fields encrypted

**Audit Trail:**
```sql
-- All operations logged
SELECT * FROM validation_history
WHERE schema_id = $1
ORDER BY validated_at DESC;

-- Access patterns tracked
SELECT * FROM schema_access_log
WHERE schema_id = $1;
```

## Future Enhancements

### Planned Features

1. **Geo-Distributed Storage**
   - Multi-region PostgreSQL
   - Regional Redis clusters
   - S3 cross-region replication

2. **Advanced Caching**
   - Predictive cache warming
   - ML-based eviction policies
   - Edge caching (CDN)

3. **Query Optimization**
   - Query result caching
   - Prepared statement pooling
   - Read replica routing

4. **Data Tiering**
   - Hot/warm/cold data separation
   - Automatic archival
   - Compressed storage

### Performance Goals

- p95 latency: <5ms (currently <10ms)
- Cache hit rate: >98% (currently >95%)
- Throughput: 200k ops/sec (currently 100k)
- Storage efficiency: 10:1 compression ratio
