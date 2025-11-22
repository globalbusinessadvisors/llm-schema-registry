-- Initial database schema for LLM Schema Registry
-- Migration: 001

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";  -- For trigram similarity search
CREATE EXTENSION IF NOT EXISTS "btree_gin"; -- For GIN indexes on multiple columns

-- Main schemas table
CREATE TABLE schemas (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    subject VARCHAR(255) NOT NULL,

    -- Semantic version components
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL,
    version_prerelease VARCHAR(255),
    version_build VARCHAR(255),

    schema_type VARCHAR(50) NOT NULL,  -- 'JSON', 'AVRO', 'PROTOBUF', 'THRIFT'
    content JSONB NOT NULL,            -- Schema content
    metadata JSONB NOT NULL DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255),

    -- Compatibility settings
    compatibility_level VARCHAR(50) NOT NULL,

    -- Soft delete
    deleted_at TIMESTAMPTZ,

    CONSTRAINT unique_subject_version UNIQUE (
        subject, version_major, version_minor, version_patch,
        COALESCE(version_prerelease, ''), COALESCE(version_build, '')
    )
);

-- Comment on table
COMMENT ON TABLE schemas IS 'Stores all schema versions with metadata';
COMMENT ON COLUMN schemas.content IS 'JSONB representation of schema content';
COMMENT ON COLUMN schemas.metadata IS 'Schema metadata including owner, tags, state, and custom fields';
COMMENT ON COLUMN schemas.deleted_at IS 'Soft delete timestamp for schema retention';

-- Indexes for fast retrieval
CREATE INDEX idx_schemas_subject ON schemas(subject) WHERE deleted_at IS NULL;
CREATE INDEX idx_schemas_created_at ON schemas(created_at DESC);
CREATE INDEX idx_schemas_type ON schemas(schema_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_schemas_metadata_gin ON schemas USING GIN (metadata jsonb_path_ops);
CREATE INDEX idx_schemas_content_gin ON schemas USING GIN (content jsonb_path_ops);
CREATE INDEX idx_schemas_subject_trgm ON schemas USING GIN (subject gin_trgm_ops);

-- Composite index for version lookups (optimized for latest version queries)
CREATE INDEX idx_schemas_subject_version ON schemas(
    subject, version_major DESC, version_minor DESC, version_patch DESC
) WHERE deleted_at IS NULL;

-- Index for state queries
CREATE INDEX idx_schemas_state ON schemas((metadata->>'state')) WHERE deleted_at IS NULL;

-- Schema dependencies (for graph traversal)
CREATE TABLE schema_dependencies (
    schema_id UUID NOT NULL REFERENCES schemas(id) ON DELETE CASCADE,
    depends_on_id UUID NOT NULL REFERENCES schemas(id) ON DELETE CASCADE,
    dependency_type VARCHAR(50) NOT NULL,  -- 'reference', 'import', 'extends'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (schema_id, depends_on_id)
);

COMMENT ON TABLE schema_dependencies IS 'Tracks dependencies between schemas for graph traversal';

CREATE INDEX idx_deps_schema ON schema_dependencies(schema_id);
CREATE INDEX idx_deps_depends_on ON schema_dependencies(depends_on_id);

-- Validation history (for auditing and analytics)
CREATE TABLE validation_history (
    id BIGSERIAL PRIMARY KEY,
    schema_id UUID NOT NULL REFERENCES schemas(id) ON DELETE CASCADE,
    data_hash VARCHAR(64) NOT NULL,  -- SHA-256 of validated data
    valid BOOLEAN NOT NULL,
    error_count INTEGER NOT NULL DEFAULT 0,
    validated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    duration_ms DOUBLE PRECISION NOT NULL
);

COMMENT ON TABLE validation_history IS 'Audit log of all schema validations';

CREATE INDEX idx_validation_schema ON validation_history(schema_id, validated_at DESC);
CREATE INDEX idx_validation_timestamp ON validation_history(validated_at DESC);
CREATE INDEX idx_validation_performance ON validation_history(duration_ms) WHERE duration_ms > 100;

-- Partition by month for better performance
-- This would be set up separately for production with automated partition management

-- Compatibility check history
CREATE TABLE compatibility_checks (
    id BIGSERIAL PRIMARY KEY,
    subject VARCHAR(255) NOT NULL,
    old_version VARCHAR(255) NOT NULL,
    new_version VARCHAR(255) NOT NULL,
    compatibility_level VARCHAR(50) NOT NULL,
    compatible BOOLEAN NOT NULL,
    violations JSONB,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE compatibility_checks IS 'History of compatibility checks between schema versions';

CREATE INDEX idx_compat_subject ON compatibility_checks(subject, checked_at DESC);
CREATE INDEX idx_compat_timestamp ON compatibility_checks(checked_at DESC);

-- Subjects metadata
CREATE TABLE subjects (
    name VARCHAR(255) PRIMARY KEY,
    default_compatibility_level VARCHAR(50) NOT NULL,
    description TEXT,
    tags TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE subjects IS 'Metadata for schema subjects';

-- Trigger to auto-update updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_subjects_updated_at
BEFORE UPDATE ON subjects
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

-- Materialized view for fast subject version listings
CREATE MATERIALIZED VIEW subject_versions AS
SELECT
    subject,
    COUNT(*) as version_count,
    MAX(created_at) as latest_version_at,
    MAX(version_major) as latest_major,
    MAX(version_minor) FILTER (WHERE version_major = MAX(version_major)) as latest_minor,
    MAX(version_patch) FILTER (
        WHERE version_major = MAX(version_major) 
        AND version_minor = MAX(version_minor) FILTER (WHERE version_major = MAX(version_major))
    ) as latest_patch,
    ARRAY_AGG(
        version_major || '.' || version_minor || '.' || version_patch
        ORDER BY version_major DESC, version_minor DESC, version_patch DESC
    ) as versions
FROM schemas
WHERE deleted_at IS NULL
GROUP BY subject;

CREATE UNIQUE INDEX idx_subject_versions ON subject_versions(subject);

COMMENT ON MATERIALIZED VIEW subject_versions IS 'Aggregated view of schema versions per subject for fast queries';

-- Refresh function for materialized view (called after schema changes)
CREATE OR REPLACE FUNCTION refresh_subject_versions()
RETURNS TRIGGER AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY subject_versions;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_refresh_subject_versions
AFTER INSERT OR UPDATE OR DELETE ON schemas
FOR EACH STATEMENT
EXECUTE FUNCTION refresh_subject_versions();

-- Function to get schema statistics
CREATE OR REPLACE FUNCTION get_schema_statistics()
RETURNS TABLE (
    total_schemas BIGINT,
    total_subjects BIGINT,
    active_schemas BIGINT,
    deprecated_schemas BIGINT,
    deleted_schemas BIGINT,
    avg_validations_per_day NUMERIC,
    avg_validation_duration_ms NUMERIC
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        COUNT(*)::BIGINT as total_schemas,
        COUNT(DISTINCT subject)::BIGINT as total_subjects,
        COUNT(*) FILTER (WHERE deleted_at IS NULL AND metadata->>'state' = 'ACTIVE')::BIGINT as active_schemas,
        COUNT(*) FILTER (WHERE metadata->>'state' = 'DEPRECATED')::BIGINT as deprecated_schemas,
        COUNT(*) FILTER (WHERE deleted_at IS NOT NULL OR metadata->>'state' = 'DELETED')::BIGINT as deleted_schemas,
        (SELECT COUNT(*)::NUMERIC / GREATEST(EXTRACT(DAY FROM NOW() - MIN(validated_at)), 1)
         FROM validation_history) as avg_validations_per_day,
        (SELECT AVG(duration_ms)::NUMERIC FROM validation_history) as avg_validation_duration_ms
    FROM schemas;
END;
$$ LANGUAGE plpgsql;

-- Function to clean up old validation history (retention policy)
CREATE OR REPLACE FUNCTION cleanup_old_validation_history(retention_days INTEGER DEFAULT 90)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM validation_history
    WHERE validated_at < NOW() - (retention_days || ' days')::INTERVAL;
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Create a role for read-only access
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'schema_registry_readonly') THEN
        CREATE ROLE schema_registry_readonly;
    END IF;
END
$$;

GRANT SELECT ON ALL TABLES IN SCHEMA public TO schema_registry_readonly;
GRANT SELECT ON subject_versions TO schema_registry_readonly;

-- Grant usage on sequences
GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO schema_registry_readonly;
