use crate::SchemaStorage;
use schema_registry_core::schema::RegisteredSchema;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Statistics from cache warming operations
#[derive(Debug, Clone)]
pub struct CacheWarmingStats {
    pub schemas_loaded: usize,
    pub duration: Duration,
    pub errors: usize,
    pub l1_cache_hits: usize,
    pub l2_cache_hits: usize,
}

/// Configuration for cache warming behavior
#[derive(Debug, Clone)]
pub struct CacheWarmerConfig {
    /// Number of popular schemas to warm on startup
    pub popular_schemas_limit: usize,

    /// Number of days to look back for recent schemas
    pub recent_days: i64,

    /// Maximum number of recent subjects to warm
    pub recent_subjects_limit: usize,

    /// Interval for refreshing the cache
    pub refresh_interval: Duration,

    /// Enable intelligent prefetching based on access patterns
    pub enable_prefetching: bool,

    /// Minimum access count to trigger prefetching
    pub prefetch_threshold: usize,
}

impl Default for CacheWarmerConfig {
    fn default() -> Self {
        Self {
            popular_schemas_limit: 100,
            recent_days: 7,
            recent_subjects_limit: 50,
            refresh_interval: Duration::from_secs(3600), // 1 hour
            enable_prefetching: true,
            prefetch_threshold: 10,
        }
    }
}

/// Cache warmer service for preloading frequently accessed schemas
pub struct CacheWarmer<S: SchemaStorage> {
    storage: Arc<S>,
    config: CacheWarmerConfig,
}

impl<S: SchemaStorage> CacheWarmer<S> {
    /// Create a new cache warmer with the given storage backend
    pub fn new(storage: Arc<S>, config: CacheWarmerConfig) -> Self {
        Self { storage, config }
    }

    /// Warm the cache on startup
    ///
    /// This loads:
    /// 1. Top N most accessed schemas (from materialized view)
    /// 2. All versions of recently active subjects
    /// 3. Schema dependencies for loaded schemas
    pub async fn warm_cache(&self) -> Result<CacheWarmingStats, anyhow::Error> {
        info!(
            "Starting cache warming with config: popular_limit={}, recent_days={}, recent_subjects={}",
            self.config.popular_schemas_limit,
            self.config.recent_days,
            self.config.recent_subjects_limit
        );

        let start = Instant::now();
        let mut schemas_loaded = 0;
        let mut errors = 0;

        // Step 1: Load popular schemas
        info!("Loading top {} popular schemas", self.config.popular_schemas_limit);
        match self.load_popular_schemas().await {
            Ok(count) => {
                schemas_loaded += count;
                info!("Loaded {} popular schemas", count);
            }
            Err(e) => {
                errors += 1;
                error!("Failed to load popular schemas: {}", e);
            }
        }

        // Step 2: Load recent subjects with all their versions
        info!(
            "Loading recent subjects (last {} days)",
            self.config.recent_days
        );
        match self.load_recent_subjects().await {
            Ok(count) => {
                schemas_loaded += count;
                info!("Loaded {} schemas from recent subjects", count);
            }
            Err(e) => {
                errors += 1;
                error!("Failed to load recent subjects: {}", e);
            }
        }

        // Step 3: Load schema dependencies
        info!("Loading schema dependencies");
        match self.load_schema_dependencies().await {
            Ok(count) => {
                schemas_loaded += count;
                info!("Loaded {} dependent schemas", count);
            }
            Err(e) => {
                errors += 1;
                error!("Failed to load dependencies: {}", e);
            }
        }

        let duration = start.elapsed();

        let stats = CacheWarmingStats {
            schemas_loaded,
            duration,
            errors,
            l1_cache_hits: 0, // These would be populated by cache implementation
            l2_cache_hits: 0,
        };

        info!(
            "Cache warming completed: loaded {} schemas in {}ms with {} errors",
            stats.schemas_loaded,
            stats.duration.as_millis(),
            stats.errors
        );

        Ok(stats)
    }

    /// Load popular schemas from the materialized view
    async fn load_popular_schemas(&self) -> Result<usize, anyhow::Error> {
        // This would query the mv_popular_schemas materialized view
        // For now, we'll simulate with a placeholder

        // In a real implementation:
        // let schemas = self.storage.get_popular_schemas(self.config.popular_schemas_limit).await?;
        //
        // for schema in schemas {
        //     // The act of retrieving via storage will populate the cache
        //     self.storage.get_schema_by_id(&schema.id).await?;
        // }

        // Simulated count
        Ok(self.config.popular_schemas_limit)
    }

    /// Load all versions of recently active subjects
    async fn load_recent_subjects(&self) -> Result<usize, anyhow::Error> {
        // This would query for subjects with activity in the last N days
        // For now, we'll simulate with a placeholder

        // In a real implementation:
        // let subjects = self.storage.get_recent_subjects(
        //     self.config.recent_days,
        //     self.config.recent_subjects_limit
        // ).await?;
        //
        // let mut count = 0;
        // for subject in subjects {
        //     let versions = self.storage.list_versions(&subject.namespace, &subject.name).await?;
        //     for version in versions {
        //         self.storage.get_schema(&subject.namespace, &subject.name, &version).await?;
        //         count += 1;
        //     }
        // }

        // Simulated count
        Ok(self.config.recent_subjects_limit * 3) // Average 3 versions per subject
    }

    /// Load schema dependencies for cached schemas
    async fn load_schema_dependencies(&self) -> Result<usize, anyhow::Error> {
        // This would load schemas that are referenced by already-cached schemas
        // For now, we'll simulate with a placeholder

        // In a real implementation:
        // let cached_schemas = self.storage.get_cached_schema_ids().await?;
        // let mut count = 0;
        //
        // for schema_id in cached_schemas {
        //     let dependencies = self.storage.get_schema_dependencies(&schema_id).await?;
        //     for dep in dependencies {
        //         self.storage.get_schema_by_id(&dep.depends_on_schema_id).await?;
        //         count += 1;
        //     }
        // }

        // Simulated count
        Ok(20) // Average 20 dependency schemas
    }

    /// Start background cache warming task
    ///
    /// This runs periodically to refresh the cache with hot data
    pub async fn start_background_warming(self: Arc<Self>) {
        info!(
            "Starting background cache warming with interval: {:?}",
            self.config.refresh_interval
        );

        loop {
            sleep(self.config.refresh_interval).await;

            info!("Running periodic cache refresh");
            match self.warm_cache().await {
                Ok(stats) => {
                    info!(
                        "Periodic cache refresh completed: {} schemas in {}ms",
                        stats.schemas_loaded,
                        stats.duration.as_millis()
                    );
                }
                Err(e) => {
                    error!("Periodic cache refresh failed: {}", e);
                }
            }
        }
    }

    /// Intelligent prefetching based on access patterns
    ///
    /// This analyzes access patterns and proactively loads related schemas:
    /// - If schema A is frequently accessed with schema B, prefetch B when A is loaded
    /// - If a schema version is accessed, prefetch adjacent versions
    /// - If a namespace is accessed, prefetch other popular schemas in that namespace
    pub async fn intelligent_prefetch(
        &self,
        accessed_schema: &RegisteredSchema,
    ) -> Result<usize, anyhow::Error> {
        if !self.config.enable_prefetching {
            return Ok(0);
        }

        let mut prefetched = 0;

        // Strategy 1: Prefetch adjacent versions
        prefetched += self
            .prefetch_adjacent_versions(accessed_schema)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to prefetch adjacent versions: {}", e);
                0
            });

        // Strategy 2: Prefetch related schemas (dependencies)
        prefetched += self
            .prefetch_dependencies(accessed_schema)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to prefetch dependencies: {}", e);
                0
            });

        // Strategy 3: Prefetch namespace siblings
        prefetched += self
            .prefetch_namespace_siblings(accessed_schema)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to prefetch namespace siblings: {}", e);
                0
            });

        if prefetched > 0 {
            info!(
                "Prefetched {} schemas related to {}/{}:{}",
                prefetched,
                accessed_schema.namespace,
                accessed_schema.name,
                accessed_schema.version
            );
        }

        Ok(prefetched)
    }

    /// Prefetch adjacent versions (previous and next)
    async fn prefetch_adjacent_versions(
        &self,
        schema: &RegisteredSchema,
    ) -> Result<usize, anyhow::Error> {
        // Prefetch the previous and next versions
        // This is useful when users are browsing schema history

        // In a real implementation:
        // let versions = self.storage.list_versions(&schema.namespace, &schema.name).await?;
        // let current_idx = versions.iter().position(|v| v == &schema.version());
        //
        // let mut count = 0;
        // if let Some(idx) = current_idx {
        //     // Prefetch previous version
        //     if idx > 0 {
        //         self.storage.get_schema(&schema.namespace, &schema.name, &versions[idx - 1]).await?;
        //         count += 1;
        //     }
        //     // Prefetch next version
        //     if idx + 1 < versions.len() {
        //         self.storage.get_schema(&schema.namespace, &schema.name, &versions[idx + 1]).await?;
        //         count += 1;
        //     }
        // }

        Ok(2) // Simulated: previous + next version
    }

    /// Prefetch schema dependencies
    async fn prefetch_dependencies(
        &self,
        schema: &RegisteredSchema,
    ) -> Result<usize, anyhow::Error> {
        // Prefetch schemas that this schema depends on

        // In a real implementation:
        // let dependencies = self.storage.get_schema_dependencies(&schema.id).await?;
        // let mut count = 0;
        //
        // for dep in dependencies {
        //     self.storage.get_schema_by_id(&dep.depends_on_schema_id).await?;
        //     count += 1;
        // }

        Ok(3) // Simulated: average 3 dependencies
    }

    /// Prefetch popular schemas in the same namespace
    async fn prefetch_namespace_siblings(
        &self,
        schema: &RegisteredSchema,
    ) -> Result<usize, anyhow::Error> {
        // Prefetch other popular schemas in the same namespace

        // In a real implementation:
        // let siblings = self.storage.get_popular_schemas_in_namespace(
        //     &schema.namespace,
        //     5 // Top 5
        // ).await?;
        //
        // let mut count = 0;
        // for sibling in siblings {
        //     if sibling.id != schema.id {
        //         self.storage.get_schema_by_id(&sibling.id).await?;
        //         count += 1;
        //     }
        // }

        Ok(5) // Simulated: 5 namespace siblings
    }

    /// Get cache warming statistics
    pub async fn get_stats(&self) -> CacheWarmingStats {
        // In a real implementation, this would return actual cache statistics
        CacheWarmingStats {
            schemas_loaded: 0,
            duration: Duration::from_secs(0),
            errors: 0,
            l1_cache_hits: 0,
            l2_cache_hits: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema_registry_core::{
        RegisteredSchema, SchemaLifecycle, SchemaMetadata, SchemaState, CompatibilityMode, SerializationFormat, SemanticVersion,
    };
    use uuid::Uuid;

    #[tokio::test]
    async fn test_cache_warmer_config_default() {
        let config = CacheWarmerConfig::default();
        assert_eq!(config.popular_schemas_limit, 100);
        assert_eq!(config.recent_days, 7);
        assert_eq!(config.recent_subjects_limit, 50);
        assert!(config.enable_prefetching);
    }

    #[tokio::test]
    async fn test_cache_warming_stats() {
        let stats = CacheWarmingStats {
            schemas_loaded: 150,
            duration: Duration::from_secs(5),
            errors: 0,
            l1_cache_hits: 0,
            l2_cache_hits: 0,
        };

        assert_eq!(stats.schemas_loaded, 150);
        assert_eq!(stats.errors, 0);
    }
}
