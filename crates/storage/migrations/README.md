# Database Migrations

This directory contains SQL migration scripts for the LLM Schema Registry PostgreSQL database.

## Migration Files

Migrations are managed by `sqlx-cli` and are executed in order:

1. **20240101000001_initial_schema.sql** - Initial database schema
   - Core `schemas` table with JSONB storage
   - Schema dependencies tracking
   - Validation history
   - Compatibility check history
   - Subject metadata
   - Materialized views for performance

2. **20240101000002_indexes_optimization.sql** - Performance optimizations
   - Partial indexes for common queries
   - Covering indexes for index-only scans
   - Expression indexes for version ordering
   - Schema access logging for cache optimization
   - Popular schemas materialized view

## Running Migrations

### Prerequisites

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

### Setup Database

```bash
# Create database
createdb schema_registry

# Set environment variable
export DATABASE_URL="postgresql://user:password@localhost/schema_registry"
```

### Run Migrations

```bash
# Run all pending migrations
sqlx migrate run

# Check migration status
sqlx migrate info

# Revert last migration
sqlx migrate revert
```

### Verify Migrations

```bash
# Connect to database
psql $DATABASE_URL

# List tables
\dt

# Check indexes
\di

# View materialized views
\dv
```

## Migration Strategy

### Development
- Migrations run automatically on application startup
- Use `sqlx migrate run` for manual control

### Production
- Run migrations separately before deployment
- Use transaction-based migrations for safety
- Test on staging environment first
- Monitor migration performance

## Performance Considerations

### Indexes
- All critical queries have supporting indexes
- Partial indexes reduce index size and improve performance
- GIN indexes for JSONB queries
- Covering indexes for index-only scans

### Maintenance
```sql
-- Analyze tables for query planner
ANALYZE schemas;
ANALYZE schema_dependencies;
ANALYZE validation_history;

-- Reindex if needed
REINDEX TABLE schemas;

-- Vacuum to reclaim space
VACUUM ANALYZE schemas;
```

### Monitoring
```sql
-- Check index usage
SELECT 
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
ORDER BY idx_scan DESC;

-- Check table statistics
SELECT * FROM get_schema_statistics();

-- View slow queries (requires pg_stat_statements)
SELECT query, mean_exec_time, calls
FROM pg_stat_statements
WHERE query LIKE '%schemas%'
ORDER BY mean_exec_time DESC
LIMIT 10;
```

## Cleanup Scripts

### Cleanup old validation history
```sql
-- Keep last 90 days
SELECT cleanup_old_validation_history(90);
```

### Cleanup deleted schemas (hard delete after retention period)
```sql
-- Delete schemas that have been soft-deleted for > 30 days
DELETE FROM schemas
WHERE deleted_at < NOW() - INTERVAL '30 days';
```

## Rollback Strategy

If a migration fails in production:

1. Check migration error logs
2. Assess data integrity
3. Revert migration: `sqlx migrate revert`
4. Fix migration script
5. Re-run: `sqlx migrate run`

## Extensions Required

- `uuid-ossp` - UUID generation
- `pg_trgm` - Trigram similarity search
- `btree_gin` - GIN indexes on multiple columns

Optional for advanced features:
- `pg_cron` - Scheduled jobs (materialized view refresh)
- `pg_stat_statements` - Query performance monitoring
- `timescaledb` - Time-series optimization for validation history
