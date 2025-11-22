//! Compatibility check result caching
//!
//! Caches compatibility check results to optimize performance for repeated checks

use crate::types::{CompatibilityMode, CompatibilityResult};
use moka::future::Cache;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Cache key for compatibility checks
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    new_schema_hash: String,
    old_schema_hash: String,
    mode: CompatibilityMode,
}

impl CacheKey {
    fn new(new_schema_hash: String, old_schema_hash: String, mode: CompatibilityMode) -> Self {
        Self {
            new_schema_hash,
            old_schema_hash,
            mode,
        }
    }
}

/// Compatibility cache with statistics
pub struct CompatibilityCache {
    cache: Cache<CacheKey, CompatibilityResult>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl CompatibilityCache {
    /// Create a new compatibility cache
    pub fn new(max_capacity: u64, ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();

        Self {
            cache,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get a cached result
    pub fn get(
        &self,
        new_schema_hash: &str,
        old_schema_hash: &str,
        mode: CompatibilityMode,
    ) -> Option<CompatibilityResult> {
        let key = CacheKey::new(
            new_schema_hash.to_string(),
            old_schema_hash.to_string(),
            mode,
        );

        if let Some(result) = self.cache.get(&key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(result)
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Store a result in the cache
    pub fn put(
        &self,
        new_schema_hash: String,
        old_schema_hash: String,
        mode: CompatibilityMode,
        result: CompatibilityResult,
    ) {
        let key = CacheKey::new(new_schema_hash, old_schema_hash, mode);
        self.cache.insert(key, result);
    }

    /// Invalidate cache entries for a specific schema
    pub async fn invalidate_schema(&self, schema_hash: &str) {
        // We need to iterate through keys and remove matching ones
        // This is expensive, but schema updates are rare
        self.cache
            .invalidate_entries_if(move |key, _| {
                key.new_schema_hash == schema_hash || key.old_schema_hash == schema_hash
            })
            .await;
    }

    /// Get cache statistics (hits, misses, hit_rate)
    pub fn stats(&self) -> (u64, u64, f64) {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        (hits, misses, hit_rate)
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }

    /// Get current cache size
    pub fn size(&self) -> u64 {
        self.cache.entry_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_hit_miss() {
        let cache = CompatibilityCache::new(100, 3600);

        let result = CompatibilityResult::compatible(
            CompatibilityMode::Backward,
            vec![],
            10,
        );

        // First access should be a miss
        assert!(cache.get("hash1", "hash2", CompatibilityMode::Backward).is_none());

        // Store result
        cache.put(
            "hash1".to_string(),
            "hash2".to_string(),
            CompatibilityMode::Backward,
            result.clone(),
        );

        // Second access should be a hit
        assert!(cache.get("hash1", "hash2", CompatibilityMode::Backward).is_some());

        // Check stats
        let (hits, misses, hit_rate) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert_eq!(hit_rate, 0.5);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = CompatibilityCache::new(100, 3600);

        let result = CompatibilityResult::compatible(
            CompatibilityMode::Backward,
            vec![],
            10,
        );

        cache.put(
            "hash1".to_string(),
            "hash2".to_string(),
            CompatibilityMode::Backward,
            result.clone(),
        );

        assert!(cache.get("hash1", "hash2", CompatibilityMode::Backward).is_some());

        // Invalidate schema
        cache.invalidate_schema("hash1").await;

        // Should now be a miss
        assert!(cache.get("hash1", "hash2", CompatibilityMode::Backward).is_none());
    }
}
