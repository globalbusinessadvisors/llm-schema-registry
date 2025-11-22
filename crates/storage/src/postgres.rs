//! PostgreSQL storage backend implementation

use crate::backend::{PoolConfig, StorageBackend, StorageStatistics, Transaction};
use crate::error::{Result, StorageError};
use crate::query::{SchemaFilter, SearchQuery, SortBy};
use async_trait::async_trait;
use schema_registry_core::{Schema, SchemaContent, SchemaId, SchemaMetadata, SchemaState, SchemaType, SchemaVersion};
use serde_json;
use sqlx::postgres::{PgPool, PgPoolOptions, PgQueryResult, PgRow};
use sqlx::{Row, Transaction as SqlxTransaction};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

/// PostgreSQL storage backend
pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection URL
    /// * `pool_config` - Connection pool configuration
    pub async fn new(database_url: &str, pool_config: &PoolConfig) -> Result<Self> {
        info!("Initializing PostgreSQL backend");
        
        let pool = PgPoolOptions::new()
            .max_connections(pool_config.max_connections)
            .min_connections(pool_config.min_connections)
            .acquire_timeout(Duration::from_secs(pool_config.acquire_timeout_secs))
            .idle_timeout(Duration::from_secs(pool_config.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(pool_config.max_lifetime_secs))
            .connect(database_url)
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        info!("PostgreSQL connection pool initialized");
        
        Ok(Self { pool })
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        info!("Running database migrations");
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| StorageError::MigrationError(e.to_string()))?;
        info!("Migrations completed successfully");
        Ok(())
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Convert a database row to a Schema
    fn row_to_schema(row: &PgRow) -> Result<Schema> {
        let id: Uuid = row.try_get("id")?;
        let subject: String = row.try_get("subject")?;
        let version_major: i32 = row.try_get("version_major")?;
        let version_minor: i32 = row.try_get("version_minor")?;
        let version_patch: i32 = row.try_get("version_patch")?;
        let version_prerelease: Option<String> = row.try_get("version_prerelease")?;
        let version_build: Option<String> = row.try_get("version_build")?;
        let schema_type_str: String = row.try_get("schema_type")?;
        let content_json: serde_json::Value = row.try_get("content")?;
        let metadata_json: serde_json::Value = row.try_get("metadata")?;
        let created_at = row.try_get("created_at")?;
        let created_by: Option<String> = row.try_get("created_by")?;
        let deleted_at = row.try_get("deleted_at")?;

        // Parse schema type
        let schema_type = match schema_type_str.as_str() {
            "JSON" => SchemaType::Json,
            "AVRO" => SchemaType::Avro,
            "PROTOBUF" => SchemaType::Protobuf,
            "THRIFT" => SchemaType::Thrift,
            _ => return Err(StorageError::SerializationError(format!("Unknown schema type: {}", schema_type_str))),
        };

        // Parse content
        let content = match schema_type {
            SchemaType::Json => SchemaContent::Json(content_json),
            SchemaType::Avro => {
                let s: String = serde_json::from_value(content_json)?;
                SchemaContent::Avro(s)
            }
            SchemaType::Protobuf => {
                let b: Vec<u8> = serde_json::from_value(content_json)?;
                SchemaContent::Protobuf(b)
            }
            SchemaType::Thrift => {
                let s: String = serde_json::from_value(content_json)?;
                SchemaContent::Thrift(s)
            }
        };

        // Parse metadata
        let metadata: SchemaMetadata = serde_json::from_value(metadata_json)?;

        let version = SchemaVersion {
            major: version_major as u32,
            minor: version_minor as u32,
            patch: version_patch as u32,
            prerelease: version_prerelease,
            build: version_build,
        };

        Ok(Schema {
            id: SchemaId::parse(&id.to_string())?,
            subject,
            version,
            schema_type,
            content,
            metadata,
            created_at,
            created_by,
            deleted_at,
        })
    }
}

#[async_trait]
impl StorageBackend for PostgresBackend {
    #[instrument(skip(self, schema), fields(schema_id = %schema.id, subject = %schema.subject))]
    async fn register_schema(&self, schema: &Schema) -> Result<()> {
        debug!("Registering schema");

        // Serialize content and metadata
        let content_json = match &schema.content {
            SchemaContent::Json(v) => v.clone(),
            SchemaContent::Avro(s) => serde_json::to_value(s)?,
            SchemaContent::Protobuf(b) => serde_json::to_value(b)?,
            SchemaContent::Thrift(s) => serde_json::to_value(s)?,
        };
        let metadata_json = serde_json::to_value(&schema.metadata)?;

        sqlx::query!(
            r#"
            INSERT INTO schemas (
                id, subject, version_major, version_minor, version_patch,
                version_prerelease, version_build, schema_type, content, metadata,
                created_at, created_by, compatibility_level
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            schema.id.as_uuid(),
            &schema.subject,
            schema.version.major as i32,
            schema.version.minor as i32,
            schema.version.patch as i32,
            schema.version.prerelease.as_ref(),
            schema.version.build.as_ref(),
            schema.schema_type.as_str(),
            content_json,
            metadata_json,
            schema.created_at,
            schema.created_by.as_ref(),
            schema.metadata.compatibility_level.to_string(),
        )
        .execute(&self.pool)
        .await?;

        info!("Schema registered successfully");
        metrics::counter!("schema_registry.schemas.registered").increment(1);
        Ok(())
    }

    #[instrument(skip(self), fields(schema_id = %id))]
    async fn get_schema(&self, id: SchemaId) -> Result<Option<Schema>> {
        debug!("Fetching schema by ID");

        let row = sqlx::query(
            r#"
            SELECT * FROM schemas WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let schema = Self::row_to_schema(&row)?;
                metrics::counter!("schema_registry.schemas.fetched").increment(1);
                Ok(Some(schema))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(subject = subject, version = %version))]
    async fn get_schema_by_version(
        &self,
        subject: &str,
        version: &SchemaVersion,
        filter: &SchemaFilter,
    ) -> Result<Option<Schema>> {
        debug!("Fetching schema by subject and version");

        let mut query_str = String::from(
            "SELECT * FROM schemas WHERE subject = $1 AND version_major = $2 AND version_minor = $3 AND version_patch = $4"
        );
        
        if !filter.include_deleted {
            query_str.push_str(" AND deleted_at IS NULL");
        }

        let row = sqlx::query(&query_str)
            .bind(subject)
            .bind(version.major as i32)
            .bind(version.minor as i32)
            .bind(version.patch as i32)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(Self::row_to_schema(&row)?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(subject = subject))]
    async fn list_schemas(&self, subject: &str, filter: &SchemaFilter) -> Result<Vec<Schema>> {
        debug!("Listing schemas for subject");

        let mut query_str = String::from(
            "SELECT * FROM schemas WHERE subject = $1"
        );

        if !filter.include_deleted {
            query_str.push_str(" AND deleted_at IS NULL");
        }

        query_str.push_str(" ORDER BY version_major DESC, version_minor DESC, version_patch DESC");

        let rows = sqlx::query(&query_str)
            .bind(subject)
            .fetch_all(&self.pool)
            .await?;

        rows.iter()
            .map(Self::row_to_schema)
            .collect()
    }

    #[instrument(skip(self, query))]
    async fn search_schemas(&self, query: &SearchQuery) -> Result<Vec<Schema>> {
        debug!("Searching schemas");

        let mut sql = String::from("SELECT * FROM schemas WHERE 1=1");
        let mut conditions = Vec::new();

        // Subject pattern
        if let Some(pattern) = &query.subject_pattern {
            conditions.push(format!("subject ILIKE '%{}%'", pattern.replace("'", "''")));
        }

        // Schema type
        if let Some(schema_type) = &query.schema_type {
            conditions.push(format!("schema_type = '{}'", schema_type.as_str()));
        }

        // Owner
        if let Some(owner) = &query.owner {
            conditions.push(format!("metadata->>'owner' = '{}'", owner.replace("'", "''")));
        }

        // State
        if let Some(state) = &query.state {
            conditions.push(format!("metadata->>'state' = '{}'", state.to_string()));
        }

        // Tags
        if let Some(tags) = &query.tags {
            for tag in tags {
                conditions.push(format!("metadata->'tags' ? '{}'", tag.replace("'", "''")));
            }
        }

        // Add all conditions
        for condition in conditions {
            sql.push_str(&format!(" AND {}", condition));
        }

        // Default: exclude deleted
        sql.push_str(" AND deleted_at IS NULL");

        // Sort order
        let sort = match query.sort_by {
            SortBy::CreatedAtAsc => "created_at ASC",
            SortBy::CreatedAtDesc => "created_at DESC",
            SortBy::SubjectAsc => "subject ASC",
            SortBy::SubjectDesc => "subject DESC",
            SortBy::VersionAsc => "version_major ASC, version_minor ASC, version_patch ASC",
            SortBy::VersionDesc => "version_major DESC, version_minor DESC, version_patch DESC",
        };
        sql.push_str(&format!(" ORDER BY {}", sort));

        // Pagination
        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = query.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;

        rows.iter()
            .map(Self::row_to_schema)
            .collect()
    }

    #[instrument(skip(self), fields(schema_id = %id, state = %state))]
    async fn update_schema_state(&self, id: SchemaId, state: SchemaState) -> Result<()> {
        debug!("Updating schema state");

        let result = sqlx::query!(
            r#"
            UPDATE schemas 
            SET metadata = jsonb_set(metadata, '{state}', to_jsonb($2::text), true)
            WHERE id = $1
            "#,
            id.as_uuid(),
            state.to_string(),
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }

        info!("Schema state updated");
        metrics::counter!("schema_registry.schemas.state_updated").increment(1);
        Ok(())
    }

    #[instrument(skip(self), fields(schema_id = %id))]
    async fn delete_schema(&self, id: SchemaId) -> Result<()> {
        debug!("Soft deleting schema");

        let result = sqlx::query!(
            r#"
            UPDATE schemas 
            SET deleted_at = NOW(),
                metadata = jsonb_set(metadata, '{state}', to_jsonb('DELETED'::text), true)
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            id.as_uuid(),
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }

        info!("Schema soft deleted");
        metrics::counter!("schema_registry.schemas.deleted").increment(1);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_subjects(&self) -> Result<Vec<String>> {
        debug!("Listing all subjects");

        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT subject FROM schemas 
            WHERE deleted_at IS NULL 
            ORDER BY subject ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.subject).collect())
    }

    #[instrument(skip(self), fields(subject = subject))]
    async fn get_latest_version(
        &self,
        subject: &str,
        filter: &SchemaFilter,
    ) -> Result<Option<Schema>> {
        debug!("Fetching latest version for subject");

        let mut query_str = String::from(
            "SELECT * FROM schemas WHERE subject = $1"
        );

        if !filter.include_deleted {
            query_str.push_str(" AND deleted_at IS NULL");
        }

        query_str.push_str(" ORDER BY version_major DESC, version_minor DESC, version_patch DESC LIMIT 1");

        let row = sqlx::query(&query_str)
            .bind(subject)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(Self::row_to_schema(&row)?)),
            None => Ok(None),
        }
    }

    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>> {
        let tx = self.pool.begin().await?;
        Ok(Box::new(PostgresTransaction { tx: Some(tx) }))
    }

    #[instrument(skip(self))]
    async fn statistics(&self) -> Result<StorageStatistics> {
        debug!("Fetching storage statistics");

        let row = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_schemas,
                COUNT(DISTINCT subject) as total_subjects,
                COUNT(*) FILTER (WHERE deleted_at IS NULL AND metadata->>'state' = 'ACTIVE') as active_schemas,
                COUNT(*) FILTER (WHERE deleted_at IS NOT NULL OR metadata->>'state' = 'DELETED') as deleted_schemas,
                pg_total_relation_size('schemas') as storage_size
            FROM schemas
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(StorageStatistics {
            total_schemas: row.total_schemas.unwrap_or(0),
            total_subjects: row.total_subjects.unwrap_or(0),
            active_schemas: row.active_schemas.unwrap_or(0),
            deleted_schemas: row.deleted_schemas.unwrap_or(0),
            storage_size_bytes: row.storage_size,
        })
    }
}

/// PostgreSQL transaction implementation
struct PostgresTransaction {
    tx: Option<SqlxTransaction<'static, sqlx::Postgres>>,
}

#[async_trait]
impl Transaction for PostgresTransaction {
    async fn register_schema(&mut self, schema: &Schema) -> Result<()> {
        let tx = self.tx.as_mut().ok_or(StorageError::TransactionError(
            "Transaction already completed".to_string(),
        ))?;

        // Serialize content and metadata
        let content_json = match &schema.content {
            SchemaContent::Json(v) => v.clone(),
            SchemaContent::Avro(s) => serde_json::to_value(s)?,
            SchemaContent::Protobuf(b) => serde_json::to_value(b)?,
            SchemaContent::Thrift(s) => serde_json::to_value(s)?,
        };
        let metadata_json = serde_json::to_value(&schema.metadata)?;

        sqlx::query!(
            r#"
            INSERT INTO schemas (
                id, subject, version_major, version_minor, version_patch,
                version_prerelease, version_build, schema_type, content, metadata,
                created_at, created_by, compatibility_level
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            schema.id.as_uuid(),
            &schema.subject,
            schema.version.major as i32,
            schema.version.minor as i32,
            schema.version.patch as i32,
            schema.version.prerelease.as_ref(),
            schema.version.build.as_ref(),
            schema.schema_type.as_str(),
            content_json,
            metadata_json,
            schema.created_at,
            schema.created_by.as_ref(),
            schema.metadata.compatibility_level.to_string(),
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn update_schema_state(&mut self, id: SchemaId, state: SchemaState) -> Result<()> {
        let tx = self.tx.as_mut().ok_or(StorageError::TransactionError(
            "Transaction already completed".to_string(),
        ))?;

        let result = sqlx::query!(
            r#"
            UPDATE schemas 
            SET metadata = jsonb_set(metadata, '{state}', to_jsonb($2::text), true)
            WHERE id = $1
            "#,
            id.as_uuid(),
            state.to_string(),
        )
        .execute(&mut **tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }

        Ok(())
    }

    async fn commit(mut self: Box<Self>) -> Result<()> {
        let tx = self.tx.take().ok_or(StorageError::TransactionError(
            "Transaction already completed".to_string(),
        ))?;
        tx.commit().await?;
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> Result<()> {
        let tx = self.tx.take().ok_or(StorageError::TransactionError(
            "Transaction already completed".to_string(),
        ))?;
        tx.rollback().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running PostgreSQL instance
    // Run with: cargo test --features test-postgres
    
    #[tokio::test]
    #[ignore]
    async fn test_postgres_connection() {
        let config = PoolConfig::default();
        let backend = PostgresBackend::new("postgresql://localhost/test_registry", &config)
            .await
            .unwrap();
        
        assert!(backend.pool.acquire().await.is_ok());
    }
}
