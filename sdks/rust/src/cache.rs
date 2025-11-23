//! Async cache implementation using moka for high-performance schema caching.
//!
//! This module provides a zero-cost abstraction over moka's async cache with TTL support
//! and automatic eviction. The cache is thread-safe and optimized for concurrent access.

use crate::models::GetSchemaResponse;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// Default cache TTL (5 minutes)
const DEFAULT_TTL_SECS: u64 = 300;

/// Default maximum cache entries
const DEFAULT_MAX_CAPACITY: u64 = 1000;

/// Configuration for the schema cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Time-to-live for cache entries
    pub ttl: Duration,
    /// Maximum number of entries
    pub max_capacity: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(DEFAULT_TTL_SECS),
            max_capacity: DEFAULT_MAX_CAPACITY,
        }
    }
}

impl CacheConfig {
    /// Creates a new cache configuration with custom settings.
    pub fn new(ttl_secs: u64, max_capacity: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            max_capacity,
        }
    }

    /// Sets the TTL duration.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Sets the maximum capacity.
    pub fn with_max_capacity(mut self, max_capacity: u64) -> Self {
        self.max_capacity = max_capacity;
        self
    }
}

/// Thread-safe async cache for schema responses.
///
/// This cache provides fast lookups with automatic expiration and eviction.
/// It uses zero-cost abstractions and is optimized for high-throughput scenarios.
#[derive(Clone)]
pub struct SchemaCache {
    cache: Arc<Cache<String, GetSchemaResponse>>,
}

impl SchemaCache {
    /// Creates a new cache with the given configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_schema_registry_sdk::cache::{SchemaCache, CacheConfig};
    /// use std::time::Duration;
    ///
    /// let config = CacheConfig::default()
    ///     .with_ttl(Duration::from_secs(300))
    ///     .with_max_capacity(1000);
    ///
    /// let cache = SchemaCache::new(config);
    /// ```
    pub fn new(config: CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .build();

        Self {
            cache: Arc::new(cache),
        }
    }

    /// Creates a new cache with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Gets a schema from the cache.
    ///
    /// Returns `None` if the schema is not in the cache or has expired.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::cache::SchemaCache;
    /// # async fn example() {
    /// let cache = SchemaCache::with_defaults();
    /// if let Some(schema) = cache.get("schema-id-123").await {
    ///     println!("Found cached schema");
    /// }
    /// # }
    /// ```
    pub async fn get(&self, key: &str) -> Option<GetSchemaResponse> {
        self.cache.get(key).await
    }

    /// Inserts a schema into the cache.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::cache::SchemaCache;
    /// # use llm_schema_registry_sdk::models::{GetSchemaResponse, SchemaMetadata, SchemaFormat};
    /// # async fn example() {
    /// # let cache = SchemaCache::with_defaults();
    /// # let response = GetSchemaResponse {
    /// #     metadata: SchemaMetadata {
    /// #         schema_id: "123".to_string(),
    /// #         namespace: "test".to_string(),
    /// #         name: "Schema".to_string(),
    /// #         version: "1.0.0".to_string(),
    /// #         format: SchemaFormat::JsonSchema,
    /// #         created_at: None,
    /// #         updated_at: None,
    /// #         tags: None,
    /// #     },
    /// #     content: "{}".to_string(),
    /// # };
    /// cache.insert("schema-id-123", response).await;
    /// # }
    /// ```
    pub async fn insert(&self, key: impl Into<String>, value: GetSchemaResponse) {
        self.cache.insert(key.into(), value).await;
    }

    /// Invalidates (removes) a schema from the cache.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::cache::SchemaCache;
    /// # async fn example() {
    /// let cache = SchemaCache::with_defaults();
    /// cache.invalidate("schema-id-123").await;
    /// # }
    /// ```
    pub async fn invalidate(&self, key: &str) {
        self.cache.invalidate(key).await;
    }

    /// Invalidates all entries in the cache.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::cache::SchemaCache;
    /// # async fn example() {
    /// let cache = SchemaCache::with_defaults();
    /// cache.invalidate_all().await;
    /// # }
    /// ```
    pub async fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }

    /// Returns the current number of entries in the cache.
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Returns the weighted size of the cache.
    pub fn weighted_size(&self) -> u64 {
        self.cache.weighted_size()
    }

    /// Runs pending maintenance tasks.
    ///
    /// This method should be called periodically to clean up expired entries.
    /// The cache automatically runs maintenance, but calling this can help
    /// ensure timely cleanup in high-throughput scenarios.
    pub async fn run_pending_tasks(&self) {
        self.cache.run_pending_tasks().await;
    }
}

impl std::fmt::Debug for SchemaCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaCache")
            .field("entry_count", &self.entry_count())
            .field("weighted_size", &self.weighted_size())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SchemaFormat, SchemaMetadata};
    use std::time::Duration;

    fn create_test_response(id: &str) -> GetSchemaResponse {
        GetSchemaResponse {
            metadata: SchemaMetadata {
                schema_id: id.to_string(),
                namespace: "test".to_string(),
                name: "TestSchema".to_string(),
                version: "1.0.0".to_string(),
                format: SchemaFormat::JsonSchema,
                created_at: None,
                updated_at: None,
                tags: None,
            },
            content: r#"{"type": "object"}"#.to_string(),
        }
    }

    #[tokio::test]
    async fn test_cache_insert_and_get() {
        let cache = SchemaCache::with_defaults();
        let response = create_test_response("test-123");

        cache.insert("test-123", response.clone()).await;

        let cached = cache.get("test-123").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().metadata.schema_id, "test-123");
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = SchemaCache::with_defaults();
        let result = cache.get("non-existent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let cache = SchemaCache::with_defaults();
        let response = create_test_response("test-456");

        cache.insert("test-456", response).await;
        assert!(cache.get("test-456").await.is_some());

        cache.invalidate("test-456").await;
        assert!(cache.get("test-456").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate_all() {
        let cache = SchemaCache::with_defaults();

        cache.insert("key1", create_test_response("1")).await;
        cache.insert("key2", create_test_response("2")).await;
        cache.insert("key3", create_test_response("3")).await;

        cache.invalidate_all().await;

        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_none());
        assert!(cache.get("key3").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_entry_count() {
        let cache = SchemaCache::with_defaults();

        cache.insert("key1", create_test_response("1")).await;
        cache.insert("key2", create_test_response("2")).await;

        // Run pending tasks to ensure the cache is up to date
        cache.run_pending_tasks().await;

        assert_eq!(cache.entry_count(), 2);
    }

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let config = CacheConfig::new(1, 100); // 1 second TTL
        let cache = SchemaCache::new(config);

        cache.insert("test-ttl", create_test_response("ttl-1")).await;

        // Should be available immediately
        assert!(cache.get("test-ttl").await.is_some());

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_secs(2)).await;
        cache.run_pending_tasks().await;

        // Should be expired now
        assert!(cache.get("test-ttl").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_max_capacity() {
        let config = CacheConfig::new(300, 2); // Max 2 entries
        let cache = SchemaCache::new(config);

        cache.insert("key1", create_test_response("1")).await;
        cache.insert("key2", create_test_response("2")).await;
        cache.insert("key3", create_test_response("3")).await;

        cache.run_pending_tasks().await;

        // Due to eviction, we should have at most 2 entries
        assert!(cache.entry_count() <= 2);
    }

    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::default()
            .with_ttl(Duration::from_secs(600))
            .with_max_capacity(5000);

        assert_eq!(config.ttl, Duration::from_secs(600));
        assert_eq!(config.max_capacity, 5000);
    }

    #[test]
    fn test_cache_debug() {
        let cache = SchemaCache::with_defaults();
        let debug_str = format!("{:?}", cache);
        assert!(debug_str.contains("SchemaCache"));
        assert!(debug_str.contains("entry_count"));
    }
}
