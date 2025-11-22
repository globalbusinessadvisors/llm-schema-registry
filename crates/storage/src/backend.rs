//! Storage backend trait and configuration

use crate::error::Result;
use crate::query::{SearchQuery, SchemaFilter};
use async_trait::async_trait;
use schema_registry_core::{Schema, SchemaId, SchemaState, SchemaVersion};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Main storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Register a new schema
    ///
    /// # Arguments
    /// * `schema` - The schema to register
    ///
    /// # Returns
    /// The registered schema with generated ID
    ///
    /// # Errors
    /// - `StorageError::AlreadyExists` if a schema with the same subject/version exists
    /// - `StorageError::DatabaseError` on database failures
    async fn register_schema(&self, schema: &Schema) -> Result<()>;

    /// Get a schema by its ID
    ///
    /// # Arguments
    /// * `id` - The schema ID
    ///
    /// # Returns
    /// The schema if found, None otherwise
    async fn get_schema(&self, id: SchemaId) -> Result<Option<Schema>>;

    /// Get a schema by subject and version
    ///
    /// # Arguments
    /// * `subject` - The schema subject
    /// * `version` - The schema version
    /// * `filter` - Additional filters
    ///
    /// # Returns
    /// The schema if found, None otherwise
    async fn get_schema_by_version(
        &self,
        subject: &str,
        version: &SchemaVersion,
        filter: &SchemaFilter,
    ) -> Result<Option<Schema>>;

    /// List all versions of a subject
    ///
    /// # Arguments
    /// * `subject` - The schema subject
    /// * `filter` - Additional filters
    ///
    /// # Returns
    /// List of schemas for the subject, ordered by version descending
    async fn list_schemas(
        &self,
        subject: &str,
        filter: &SchemaFilter,
    ) -> Result<Vec<Schema>>;

    /// Search for schemas
    ///
    /// # Arguments
    /// * `query` - Search query with filters
    ///
    /// # Returns
    /// List of matching schemas
    async fn search_schemas(&self, query: &SearchQuery) -> Result<Vec<Schema>>;

    /// Update schema state (e.g., deprecate, delete)
    ///
    /// # Arguments
    /// * `id` - The schema ID
    /// * `state` - The new state
    ///
    /// # Errors
    /// - `StorageError::NotFound` if schema doesn't exist
    async fn update_schema_state(&self, id: SchemaId, state: SchemaState) -> Result<()>;

    /// Soft delete a schema
    ///
    /// # Arguments
    /// * `id` - The schema ID
    ///
    /// # Errors
    /// - `StorageError::NotFound` if schema doesn't exist
    async fn delete_schema(&self, id: SchemaId) -> Result<()>;

    /// List all subjects
    ///
    /// # Returns
    /// List of unique subject names
    async fn list_subjects(&self) -> Result<Vec<String>>;

    /// Get the latest version for a subject
    ///
    /// # Arguments
    /// * `subject` - The schema subject
    /// * `filter` - Additional filters
    ///
    /// # Returns
    /// The latest schema version if found
    async fn get_latest_version(
        &self,
        subject: &str,
        filter: &SchemaFilter,
    ) -> Result<Option<Schema>>;

    /// Check if a schema exists
    ///
    /// # Arguments
    /// * `id` - The schema ID
    ///
    /// # Returns
    /// True if the schema exists
    async fn exists(&self, id: SchemaId) -> Result<bool> {
        Ok(self.get_schema(id).await?.is_some())
    }

    /// Begin a transaction
    ///
    /// # Returns
    /// A transaction handle
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;

    /// Get storage statistics
    async fn statistics(&self) -> Result<StorageStatistics>;
}

/// Transaction trait for ACID operations
#[async_trait]
pub trait Transaction: Send + Sync {
    /// Register a schema within the transaction
    async fn register_schema(&mut self, schema: &Schema) -> Result<()>;

    /// Update schema state within the transaction
    async fn update_schema_state(&mut self, id: SchemaId, state: SchemaState) -> Result<()>;

    /// Commit the transaction
    async fn commit(self: Box<Self>) -> Result<()>;

    /// Rollback the transaction
    async fn rollback(self: Box<Self>) -> Result<()>;
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Primary storage backend
    pub backend: BackendConfig,

    /// Optional caching configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheBackendConfig>,

    /// Optional S3 configuration for large schemas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3Config>,

    /// Connection pool configuration
    #[serde(default)]
    pub pool: PoolConfig,
}

/// Backend type configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackendConfig {
    Postgres {
        url: String,
    },
    Memory,
}

/// Cache backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CacheBackendConfig {
    Redis {
        url: String,
        ttl_seconds: u64,
    },
    InMemory {
        max_entries: usize,
    },
}

/// S3 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub prefix: Option<String>,
    /// Minimum schema size (bytes) to store in S3
    pub min_size_bytes: usize,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    
    #[serde(default = "default_acquire_timeout_secs")]
    pub acquire_timeout_secs: u64,
    
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    
    #[serde(default = "default_max_lifetime_secs")]
    pub max_lifetime_secs: u64,
}

fn default_max_connections() -> u32 { 50 }
fn default_min_connections() -> u32 { 10 }
fn default_acquire_timeout_secs() -> u64 { 3 }
fn default_idle_timeout_secs() -> u64 { 600 }
fn default_max_lifetime_secs() -> u64 { 1800 }

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            acquire_timeout_secs: default_acquire_timeout_secs(),
            idle_timeout_secs: default_idle_timeout_secs(),
            max_lifetime_secs: default_max_lifetime_secs(),
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStatistics {
    pub total_schemas: i64,
    pub total_subjects: i64,
    pub active_schemas: i64,
    pub deleted_schemas: i64,
    pub storage_size_bytes: Option<i64>,
}

/// Type alias for Arc-wrapped storage backend
pub type StorageBackendRef = Arc<dyn StorageBackend>;
