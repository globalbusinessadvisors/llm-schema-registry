//! Multi-tier caching implementation for schema storage

use crate::error::{Result, StorageError};
use schema_registry_core::{Schema, SchemaId};
use moka::future::Cache as MokaCache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// L1 cache size (in-memory, hot schemas)
    #[serde(default = "default_l1_size")]
    pub l1_max_entries: usize,

    /// L1 cache TTL
    #[serde(default = "default_l1_ttl_secs")]
    pub l1_ttl_seconds: u64,

    /// Compiled validator cache size
    #[serde(default = "default_validator_cache_size")]
    pub validator_cache_size: usize,

    /// Enable cache statistics
    #[serde(default = "default_enable_stats")]
    pub enable_statistics: bool,

    /// Cache warming on startup
    #[serde(default)]
    pub warm_on_startup: bool,

    /// Number of popular schemas to keep warm
    #[serde(default = "default_warm_size")]
    pub warm_size: usize,
}

fn default_l1_size() -> usize { 10_000 }
fn default_l1_ttl_secs() -> u64 { 3600 }
fn default_validator_cache_size() -> usize { 1_000 }
fn default_enable_stats() -> bool { true }
fn default_warm_size() -> usize { 100 }

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_max_entries: default_l1_size(),
            l1_ttl_seconds: default_l1_ttl_secs(),
            validator_cache_size: default_validator_cache_size(),
            enable_statistics: default_enable_stats(),
            warm_on_startup: false,
            warm_size: default_warm_size(),
        }
    }
}

/// Multi-tier cache manager
pub struct CacheManager {
    /// L1: In-memory cache for hot schemas (Moka - high-performance async cache)
    l1_cache: MokaCache<SchemaId, Arc<Schema>>,

    /// Cache statistics
    stats: Arc<CacheStats>,

    /// Configuration
    config: CacheConfig,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(config: CacheConfig) -> Self {
        info!("Initializing cache manager with L1 size: {}", config.l1_max_entries);

        let l1_cache = MokaCache::builder()
            .max_capacity(config.l1_max_entries as u64)
            .time_to_live(Duration::from_secs(config.l1_ttl_seconds))
            .time_to_idle(Duration::from_secs(config.l1_ttl_seconds / 2))
            .build();

        Self {
            l1_cache,
            stats: Arc::new(CacheStats::new(config.enable_statistics)),
            config,
        }
    }

    /// Get schema from cache
    pub async fn get(&self, id: SchemaId) -> Option<Arc<Schema>> {
        // Try L1 cache
        if let Some(schema) = self.l1_cache.get(&id).await {
            debug!("L1 cache hit for schema {}", id);
            self.stats.record_hit(CacheLevel::L1);
            return Some(schema);
        }

        self.stats.record_miss();
        None
    }

    /// Put schema into cache
    pub async fn put(&self, id: SchemaId, schema: Arc<Schema>) {
        debug!("Caching schema {}", id);
        self.l1_cache.insert(id, schema).await;
    }

    /// Invalidate a schema from all cache levels
    pub async fn invalidate(&self, id: SchemaId) {
        debug!("Invalidating cache for schema {}", id);
        self.l1_cache.invalidate(&id).await;
        self.stats.record_invalidation();
    }

    /// Invalidate all schemas for a subject
    pub async fn invalidate_subject(&self, subject: &str) {
        debug!("Invalidating cache for subject {}", subject);
        // We need to iterate and remove matching schemas
        // This is a simplified version - in production you'd maintain a subject->schema_id index
        self.l1_cache.invalidate_all();
        self.stats.record_invalidation();
    }

    /// Clear all caches
    pub async fn clear(&self) {
        info!("Clearing all caches");
        self.l1_cache.invalidate_all();
    }

    /// Get cache statistics
    pub fn statistics(&self) -> CacheStatistics {
        self.stats.get_statistics()
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        self.stats.hit_rate()
    }

    /// Get cache size
    pub fn size(&self) -> u64 {
        self.l1_cache.entry_count()
    }

    /// Warm up cache with frequently accessed schemas
    pub async fn warm_up<F, Fut>(&self, load_fn: F) -> Result<usize>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Schema>>>,
    {
        if !self.config.warm_on_startup {
            return Ok(0);
        }

        info!("Warming up cache with {} popular schemas", self.config.warm_size);
        
        let schemas = load_fn().await?;
        let count = schemas.len();

        for schema in schemas {
            self.put(schema.id, Arc::new(schema)).await;
        }

        info!("Cache warmed with {} schemas", count);
        Ok(count)
    }
}

/// Cache level enumeration
#[derive(Debug, Clone, Copy)]
pub enum CacheLevel {
    L1, // In-memory
    L2, // Redis (if configured)
}

/// Cache statistics tracker
pub struct CacheStats {
    l1_hits: AtomicU64,
    l2_hits: AtomicU64,
    misses: AtomicU64,
    invalidations: AtomicU64,
    enabled: bool,
}

impl CacheStats {
    fn new(enabled: bool) -> Self {
        Self {
            l1_hits: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
            enabled,
        }
    }

    fn record_hit(&self, level: CacheLevel) {
        if !self.enabled {
            return;
        }
        match level {
            CacheLevel::L1 => {
                self.l1_hits.fetch_add(1, Ordering::Relaxed);
                metrics::counter!("schema_registry.cache.l1.hits").increment(1);
            }
            CacheLevel::L2 => {
                self.l2_hits.fetch_add(1, Ordering::Relaxed);
                metrics::counter!("schema_registry.cache.l2.hits").increment(1);
            }
        }
    }

    fn record_miss(&self) {
        if !self.enabled {
            return;
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        metrics::counter!("schema_registry.cache.misses").increment(1);
    }

    fn record_invalidation(&self) {
        if !self.enabled {
            return;
        }
        self.invalidations.fetch_add(1, Ordering::Relaxed);
        metrics::counter!("schema_registry.cache.invalidations").increment(1);
    }

    fn get_statistics(&self) -> CacheStatistics {
        CacheStatistics {
            l1_hits: self.l1_hits.load(Ordering::Relaxed),
            l2_hits: self.l2_hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
            hit_rate: self.hit_rate(),
        }
    }

    fn hit_rate(&self) -> f64 {
        let l1_hits = self.l1_hits.load(Ordering::Relaxed) as f64;
        let l2_hits = self.l2_hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        let total = l1_hits + l2_hits + misses;
        
        if total == 0.0 {
            0.0
        } else {
            (l1_hits + l2_hits) / total
        }
    }
}

/// Cache statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub l1_hits: u64,
    pub l2_hits: u64,
    pub misses: u64,
    pub invalidations: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema_registry_core::{SchemaContent, SchemaMetadata, SchemaVersion};

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let config = CacheConfig::default();
        let cache = CacheManager::new(config);

        let schema = Schema::new(
            "test.subject".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaContent::Json(serde_json::json!({"type": "object"})),
            SchemaMetadata::default(),
        );
        let id = schema.id;

        // Should be empty initially
        assert!(cache.get(id).await.is_none());

        // Put schema
        cache.put(id, Arc::new(schema)).await;

        // Should now be cached
        assert!(cache.get(id).await.is_some());

        // Invalidate
        cache.invalidate(id).await;
        assert!(cache.get(id).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let mut config = CacheConfig::default();
        config.enable_statistics = true;
        let cache = CacheManager::new(config);

        let schema = Schema::new(
            "test.subject".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaContent::Json(serde_json::json!({"type": "object"})),
            SchemaMetadata::default(),
        );
        let id = schema.id;

        // Miss
        cache.get(id).await;
        
        // Put and hit
        cache.put(id, Arc::new(schema)).await;
        cache.get(id).await;

        let stats = cache.statistics();
        assert_eq!(stats.l1_hits, 1);
        assert_eq!(stats.misses, 1);
        assert!(stats.hit_rate > 0.0 && stats.hit_rate < 1.0);
    }
}
