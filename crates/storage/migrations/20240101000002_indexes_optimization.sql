-- Advanced indexing and performance optimizations
-- Migration: 002

-- Partial indexes for common queries
CREATE INDEX idx_schemas_active_latest ON schemas(
    subject, version_major DESC, version_minor DESC, version_patch DESC
) 
WHERE deleted_at IS NULL AND metadata->>'state' = 'ACTIVE';

-- Index for compatibility level queries
CREATE INDEX idx_schemas_compatibility ON schemas(
    (metadata->>'compatibility_level')
) WHERE deleted_at IS NULL;

-- Covering index for subject listing (index-only scans)
CREATE INDEX idx_schemas_subject_covering ON schemas(
    subject, id, created_at
) WHERE deleted_at IS NULL;

-- Index for tag searches
CREATE INDEX idx_schemas_tags_gin ON schemas USING GIN (
    (metadata->'tags')
) WHERE deleted_at IS NULL;

-- Partial index for recent schemas (hot data)
CREATE INDEX idx_schemas_recent ON schemas(created_at DESC)
WHERE created_at > NOW() - INTERVAL '30 days' AND deleted_at IS NULL;

-- Expression index for version ordering
CREATE INDEX idx_schemas_version_expr ON schemas(
    subject,
    (version_major * 1000000 + version_minor * 1000 + version_patch) DESC
) WHERE deleted_at IS NULL;

-- Add statistics collection for better query planning
ALTER TABLE schemas ALTER COLUMN subject SET STATISTICS 1000;
ALTER TABLE schemas ALTER COLUMN schema_type SET STATISTICS 1000;
ALTER TABLE schemas ALTER COLUMN metadata SET STATISTICS 1000;

-- Create table for tracking schema access patterns (for cache warming)
CREATE TABLE schema_access_log (
    id BIGSERIAL PRIMARY KEY,
    schema_id UUID NOT NULL REFERENCES schemas(id) ON DELETE CASCADE,
    accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    access_count INTEGER NOT NULL DEFAULT 1,
    response_time_ms DOUBLE PRECISION
);

-- Hypertable for time-series data (if using TimescaleDB)
-- SELECT create_hypertable('schema_access_log', 'accessed_at', if_not_exists => TRUE);

CREATE INDEX idx_access_log_schema ON schema_access_log(schema_id, accessed_at DESC);
CREATE INDEX idx_access_log_timestamp ON schema_access_log(accessed_at DESC);

-- Materialized view for popular schemas (cache warming)
CREATE MATERIALIZED VIEW popular_schemas AS
SELECT 
    s.id,
    s.subject,
    s.version_major,
    s.version_minor,
    s.version_patch,
    COUNT(a.id) as access_count,
    AVG(a.response_time_ms) as avg_response_time_ms,
    MAX(a.accessed_at) as last_accessed_at
FROM schemas s
LEFT JOIN schema_access_log a ON s.id = a.schema_id
WHERE s.deleted_at IS NULL
  AND a.accessed_at > NOW() - INTERVAL '7 days'
GROUP BY s.id, s.subject, s.version_major, s.version_minor, s.version_patch
ORDER BY access_count DESC
LIMIT 1000;

CREATE UNIQUE INDEX idx_popular_schemas_id ON popular_schemas(id);

-- Function to refresh popular schemas view
CREATE OR REPLACE FUNCTION refresh_popular_schemas()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY popular_schemas;
END;
$$ LANGUAGE plpgsql;

-- Scheduled job to refresh popular schemas (requires pg_cron extension)
-- SELECT cron.schedule('refresh-popular-schemas', '*/5 * * * *', 'SELECT refresh_popular_schemas()');

COMMENT ON TABLE schema_access_log IS 'Tracks schema access patterns for cache optimization';
COMMENT ON MATERIALIZED VIEW popular_schemas IS 'Most frequently accessed schemas for cache warming';
