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

    /// Initialize database schema
    async fn init_database(pool: &sqlx::PgPool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schemas (
                id UUID PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                namespace VARCHAR(255) NOT NULL,
                version VARCHAR(50) NOT NULL,
                format VARCHAR(50) NOT NULL,
                content TEXT NOT NULL,
                content_hash VARCHAR(64) NOT NULL,
                description TEXT,
                compatibility_mode VARCHAR(50) NOT NULL,
                state VARCHAR(50) NOT NULL DEFAULT 'ACTIVE',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                metadata JSONB,
                UNIQUE(namespace, name, version)
            );

            CREATE INDEX IF NOT EXISTS idx_schemas_namespace_name ON schemas(namespace, name);
            CREATE INDEX IF NOT EXISTS idx_schemas_content_hash ON schemas(content_hash);
            CREATE INDEX IF NOT EXISTS idx_schemas_created_at ON schemas(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_schemas_state ON schemas(state);

            CREATE TABLE IF NOT EXISTS compatibility_checks (
                id UUID PRIMARY KEY,
                old_schema_id UUID NOT NULL REFERENCES schemas(id),
                new_schema_id UUID NOT NULL REFERENCES schemas(id),
                mode VARCHAR(50) NOT NULL,
                is_compatible BOOLEAN NOT NULL,
                violations JSONB,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE INDEX IF NOT EXISTS idx_compat_old_schema ON compatibility_checks(old_schema_id);
            CREATE INDEX IF NOT EXISTS idx_compat_new_schema ON compatibility_checks(new_schema_id);

            CREATE TABLE IF NOT EXISTS validation_results (
                id UUID PRIMARY KEY,
                schema_id UUID NOT NULL REFERENCES schemas(id),
                data_hash VARCHAR(64) NOT NULL,
                is_valid BOOLEAN NOT NULL,
                errors JSONB,
                validated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE INDEX IF NOT EXISTS idx_validation_schema ON validation_results(schema_id);
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
            sqlx::query("TRUNCATE schemas, compatibility_checks, validation_results CASCADE")
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
