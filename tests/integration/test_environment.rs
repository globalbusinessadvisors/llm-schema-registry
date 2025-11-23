//! Test environment setup with testcontainers

use anyhow::Result;
use std::sync::Arc;
use testcontainers::*;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::{postgres::Postgres, redis::Redis, localstack::LocalStack};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Comprehensive test environment with all required services
pub struct TestEnvironment {
    pub postgres_container: Arc<Mutex<ContainerAsync<Postgres>>>,
    pub redis_container: Arc<Mutex<ContainerAsync<Redis>>>,
    pub s3_container: Arc<Mutex<ContainerAsync<LocalStack>>>,
    pub database_url: String,
    pub redis_url: String,
    pub s3_endpoint: String,
    pub s3_bucket: String,
    pub db_pool: Option<sqlx::PgPool>,
    pub redis_client: Option<redis::Client>,
}

impl TestEnvironment {
    /// Create a new test environment with all services
    pub async fn new() -> Result<Self> {
        tracing::info!("Starting test environment...");

        // Start PostgreSQL container
        tracing::info!("Starting PostgreSQL container...");
        let postgres = Postgres::default();
        let postgres_container = postgres.start().await?;
        let postgres_port = postgres_container.get_host_port_ipv4(5432).await?;
        let database_url = format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            postgres_port
        );
        tracing::info!("PostgreSQL running on port {}", postgres_port);

        // Start Redis container
        tracing::info!("Starting Redis container...");
        let redis = Redis::default();
        let redis_container = redis.start().await?;
        let redis_port = redis_container.get_host_port_ipv4(6379).await?;
        let redis_url = format!("redis://localhost:{}", redis_port);
        tracing::info!("Redis running on port {}", redis_port);

        // Start LocalStack (S3) container
        tracing::info!("Starting LocalStack container...");
        let localstack = LocalStack::default();
        let s3_container = localstack.start().await?;
        let s3_port = s3_container.get_host_port_ipv4(4566).await?;
        let s3_endpoint = format!("http://localhost:{}", s3_port);
        let s3_bucket = format!("test-schemas-{}", Uuid::new_v4());
        tracing::info!("LocalStack running on port {}", s3_port);

        // Create database connection pool
        let db_pool = sqlx::PgPool::connect(&database_url).await?;
        tracing::info!("Connected to PostgreSQL");

        // Create Redis client
        let redis_client = redis::Client::open(redis_url.clone())?;
        tracing::info!("Connected to Redis");

        // Initialize database schema
        Self::init_database(&db_pool).await?;
        tracing::info!("Database schema initialized");

        // Initialize S3 bucket
        Self::init_s3(&s3_endpoint, &s3_bucket).await?;
        tracing::info!("S3 bucket created: {}", s3_bucket);

        Ok(Self {
            postgres_container: Arc::new(Mutex::new(postgres_container)),
            redis_container: Arc::new(Mutex::new(redis_container)),
            s3_container: Arc::new(Mutex::new(s3_container)),
            database_url,
            redis_url,
            s3_endpoint,
            s3_bucket,
            db_pool: Some(db_pool),
            redis_client: Some(redis_client),
        })
    }

    /// Initialize database schema (matches production schema)
    async fn init_database(pool: &sqlx::PgPool) -> Result<()> {
        // Create extensions
        sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"")
            .execute(pool)
            .await?;

        sqlx::query("CREATE EXTENSION IF NOT EXISTS \"pg_trgm\"")
            .execute(pool)
            .await?;

        sqlx::query("CREATE EXTENSION IF NOT EXISTS \"btree_gin\"")
            .execute(pool)
            .await?;

        // Main schemas table (production schema)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schemas (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                subject VARCHAR(255) NOT NULL,

                -- Semantic version components
                version_major INTEGER NOT NULL,
                version_minor INTEGER NOT NULL,
                version_patch INTEGER NOT NULL,
                version_prerelease VARCHAR(255),
                version_build VARCHAR(255),

                schema_type VARCHAR(50) NOT NULL,
                content JSONB NOT NULL,
                metadata JSONB NOT NULL DEFAULT '{}',

                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                created_by VARCHAR(255),

                compatibility_level VARCHAR(50) NOT NULL,

                deleted_at TIMESTAMPTZ,

                CONSTRAINT unique_subject_version UNIQUE (
                    subject, version_major, version_minor, version_patch,
                    COALESCE(version_prerelease, ''), COALESCE(version_build, '')
                )
            );

            CREATE INDEX IF NOT EXISTS idx_schemas_subject ON schemas(subject) WHERE deleted_at IS NULL;
            CREATE INDEX IF NOT EXISTS idx_schemas_created_at ON schemas(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_schemas_type ON schemas(schema_type) WHERE deleted_at IS NULL;
            CREATE INDEX IF NOT EXISTS idx_schemas_metadata_gin ON schemas USING GIN (metadata jsonb_path_ops);
            CREATE INDEX IF NOT EXISTS idx_schemas_content_gin ON schemas USING GIN (content jsonb_path_ops);

            CREATE TABLE IF NOT EXISTS compatibility_checks (
                id BIGSERIAL PRIMARY KEY,
                subject VARCHAR(255) NOT NULL,
                old_version VARCHAR(255) NOT NULL,
                new_version VARCHAR(255) NOT NULL,
                compatibility_level VARCHAR(50) NOT NULL,
                compatible BOOLEAN NOT NULL,
                violations JSONB,
                checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE INDEX IF NOT EXISTS idx_compat_subject ON compatibility_checks(subject, checked_at DESC);

            CREATE TABLE IF NOT EXISTS validation_history (
                id BIGSERIAL PRIMARY KEY,
                schema_id UUID NOT NULL REFERENCES schemas(id) ON DELETE CASCADE,
                data_hash VARCHAR(64) NOT NULL,
                valid BOOLEAN NOT NULL,
                error_count INTEGER NOT NULL DEFAULT 0,
                validated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                duration_ms DOUBLE PRECISION NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_validation_schema ON validation_history(schema_id, validated_at DESC);

            CREATE TABLE IF NOT EXISTS subjects (
                name VARCHAR(255) PRIMARY KEY,
                default_compatibility_level VARCHAR(50) NOT NULL,
                description TEXT,
                tags TEXT[] DEFAULT '{}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Initialize S3 bucket
    async fn init_s3(endpoint: &str, bucket: &str) -> Result<()> {
        let config = aws_config::from_env()
            .endpoint_url(endpoint)
            .region("us-east-1")
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                "test",
                "test",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let client = aws_sdk_s3::Client::new(&config);

        client
            .create_bucket()
            .bucket(bucket)
            .send()
            .await?;

        Ok(())
    }

    /// Reset all data between tests
    pub async fn reset(&self) -> Result<()> {
        tracing::info!("Resetting test environment...");

        // Clear PostgreSQL
        if let Some(pool) = &self.db_pool {
            sqlx::query("TRUNCATE schemas, compatibility_checks, validation_history, subjects CASCADE")
                .execute(pool)
                .await?;
            tracing::debug!("PostgreSQL data cleared");
        }

        // Clear Redis
        if let Some(client) = &self.redis_client {
            let mut conn = client.get_connection()?;
            redis::cmd("FLUSHALL").execute(&mut conn);
            tracing::debug!("Redis data cleared");
        }

        // Clear S3 bucket
        self.clear_s3_bucket().await?;
        tracing::debug!("S3 bucket cleared");

        Ok(())
    }

    /// Clear S3 bucket
    async fn clear_s3_bucket(&self) -> Result<()> {
        let config = aws_config::from_env()
            .endpoint_url(&self.s3_endpoint)
            .region("us-east-1")
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                "test",
                "test",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let client = aws_sdk_s3::Client::new(&config);

        // List all objects
        let objects = client
            .list_objects_v2()
            .bucket(&self.s3_bucket)
            .send()
            .await?;

        // Delete all objects
        let contents = objects.contents();
        for obj in contents {
            if let Some(key) = obj.key() {
                client
                    .delete_object()
                    .bucket(&self.s3_bucket)
                    .key(key)
                    .send()
                    .await?;
            }
        }

        Ok(())
    }

    /// Get database pool
    pub fn db_pool(&self) -> &sqlx::PgPool {
        self.db_pool.as_ref().expect("Database pool not initialized")
    }

    /// Get Redis client
    pub fn redis_client(&self) -> &redis::Client {
        self.redis_client.as_ref().expect("Redis client not initialized")
    }

    /// Get S3 client
    pub async fn s3_client(&self) -> aws_sdk_s3::Client {
        let config = aws_config::from_env()
            .endpoint_url(&self.s3_endpoint)
            .region("us-east-1")
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                "test",
                "test",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        aws_sdk_s3::Client::new(&config)
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        tracing::info!("Cleaning up test environment...");
    }
}
