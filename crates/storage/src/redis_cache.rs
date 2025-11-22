//! Redis caching layer implementation (L2 cache)

use crate::error::{Result, StorageError};
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use schema_registry_core::{Schema, SchemaId};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Redis cache implementation
pub struct RedisCache {
    client: Client,
    connection: ConnectionManager,
    ttl_seconds: u64,
    key_prefix: String,
}

impl RedisCache {
    /// Create a new Redis cache
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    /// * `ttl_seconds` - Time-to-live for cached entries
    pub async fn new(redis_url: &str, ttl_seconds: u64) -> Result<Self> {
        info!("Connecting to Redis at {}", redis_url);
        
        let client = Client::open(redis_url)
            .map_err(|e| StorageError::CacheError(format!("Failed to create Redis client: {}", e)))?;

        let connection = ConnectionManager::new(client.clone())
            .await
            .map_err(|e| StorageError::CacheError(format!("Failed to connect to Redis: {}", e)))?;

        // Test connection
        let mut conn = connection.clone();
        let _: () = conn.ping()
            .await
            .map_err(|e| StorageError::CacheError(format!("Redis ping failed: {}", e)))?;

        info!("Redis connection established successfully");

        Ok(Self {
            client,
            connection,
            ttl_seconds,
            key_prefix: "schema:".to_string(),
        })
    }

    /// Create with custom key prefix
    pub async fn with_prefix(redis_url: &str, ttl_seconds: u64, prefix: String) -> Result<Self> {
        let mut cache = Self::new(redis_url, ttl_seconds).await?;
        cache.key_prefix = prefix;
        Ok(cache)
    }

    /// Generate Redis key for schema ID
    fn schema_key(&self, id: SchemaId) -> String {
        format!("{}{}", self.key_prefix, id)
    }

    /// Generate Redis key for subject versions
    fn subject_key(&self, subject: &str) -> String {
        format!("{}subject:{}", self.key_prefix, subject)
    }

    /// Get schema from Redis cache
    pub async fn get(&self, id: SchemaId) -> Result<Option<Schema>> {
        let key = self.schema_key(id);
        debug!("Fetching schema from Redis: {}", key);

        let mut conn = self.connection.clone();
        
        match conn.get::<_, Option<Vec<u8>>>(&key).await {
            Ok(Some(bytes)) => {
                debug!("Redis cache hit for schema {}", id);
                match bincode::deserialize::<Schema>(&bytes) {
                    Ok(schema) => {
                        metrics::counter!("schema_registry.cache.redis.hits").increment(1);
                        Ok(Some(schema))
                    }
                    Err(e) => {
                        error!("Failed to deserialize schema from Redis: {}", e);
                        // Invalidate corrupted cache entry
                        let _: Result<(), _> = conn.del(&key).await;
                        metrics::counter!("schema_registry.cache.redis.deserialize_errors").increment(1);
                        Ok(None)
                    }
                }
            }
            Ok(None) => {
                debug!("Redis cache miss for schema {}", id);
                metrics::counter!("schema_registry.cache.redis.misses").increment(1);
                Ok(None)
            }
            Err(e) => {
                warn!("Redis error while fetching schema {}: {}", id, e);
                metrics::counter!("schema_registry.cache.redis.errors").increment(1);
                // Return None instead of error to gracefully degrade
                Ok(None)
            }
        }
    }

    /// Put schema into Redis cache
    pub async fn put(&self, id: SchemaId, schema: &Schema) -> Result<()> {
        let key = self.schema_key(id);
        debug!("Caching schema in Redis: {}", key);

        let bytes = bincode::serialize(schema)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let mut conn = self.connection.clone();

        match conn.set_ex::<_, _, ()>(&key, bytes, self.ttl_seconds).await {
            Ok(_) => {
                metrics::counter!("schema_registry.cache.redis.writes").increment(1);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to cache schema in Redis: {}", e);
                metrics::counter!("schema_registry.cache.redis.write_errors").increment(1);
                // Don't fail the operation if caching fails
                Ok(())
            }
        }
    }

    /// Invalidate a schema from cache
    pub async fn invalidate(&self, id: SchemaId) -> Result<()> {
        let key = self.schema_key(id);
        debug!("Invalidating Redis cache for schema {}", id);

        let mut conn = self.connection.clone();
        
        match conn.del::<_, ()>(&key).await {
            Ok(_) => {
                metrics::counter!("schema_registry.cache.redis.invalidations").increment(1);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to invalidate Redis cache: {}", e);
                Ok(())
            }
        }
    }

    /// Invalidate all schemas for a subject
    pub async fn invalidate_subject(&self, subject: &str) -> Result<()> {
        let key = self.subject_key(subject);
        debug!("Invalidating Redis cache for subject {}", subject);

        let mut conn = self.connection.clone();

        // Delete subject key
        let _: Result<(), _> = conn.del(&key).await;

        // In a production system, you'd maintain a set of schema IDs per subject
        // and delete all of them here
        
        Ok(())
    }

    /// Clear all cached schemas
    pub async fn clear(&self) -> Result<()> {
        info!("Clearing all Redis cache entries");

        let mut conn = self.connection.clone();
        
        // Scan for all keys with our prefix and delete them
        let pattern = format!("{}*", self.key_prefix);
        
        match redis::cmd("SCAN")
            .arg(0)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(1000)
            .query_async::<_, (u64, Vec<String>)>(&mut conn)
            .await
        {
            Ok((_, keys)) => {
                if !keys.is_empty() {
                    let _: Result<(), _> = conn.del(keys).await;
                }
                Ok(())
            }
            Err(e) => {
                error!("Failed to scan Redis keys: {}", e);
                Ok(())
            }
        }
    }

    /// Get cache statistics from Redis
    pub async fn info(&self) -> Result<String> {
        let mut conn = self.connection.clone();
        
        match redis::cmd("INFO")
            .arg("stats")
            .query_async::<_, String>(&mut conn)
            .await
        {
            Ok(info) => Ok(info),
            Err(e) => Err(StorageError::CacheError(format!("Failed to get Redis info: {}", e))),
        }
    }

    /// Ping Redis to check connection
    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.connection.clone();
        conn.ping()
            .await
            .map_err(|e| StorageError::CacheError(format!("Redis ping failed: {}", e)))
    }

    /// Cache multiple schemas in a pipeline
    pub async fn put_batch(&self, schemas: Vec<(SchemaId, Schema)>) -> Result<usize> {
        if schemas.is_empty() {
            return Ok(0);
        }

        debug!("Batch caching {} schemas in Redis", schemas.len());

        let mut pipe = redis::pipe();
        let mut count = 0;

        for (id, schema) in schemas {
            if let Ok(bytes) = bincode::serialize(&schema) {
                let key = self.schema_key(id);
                pipe.set_ex(&key, bytes, self.ttl_seconds);
                count += 1;
            }
        }

        let mut conn = self.connection.clone();
        
        match pipe.query_async::<_, ()>(&mut conn).await {
            Ok(_) => {
                metrics::counter!("schema_registry.cache.redis.batch_writes").increment(1);
                metrics::counter!("schema_registry.cache.redis.batch_schemas").increment(count);
                Ok(count)
            }
            Err(e) => {
                warn!("Failed to batch cache schemas in Redis: {}", e);
                Ok(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema_registry_core::{SchemaContent, SchemaMetadata, SchemaVersion};

    // Note: These tests require a running Redis instance
    // Run with: docker run -d -p 6379:6379 redis:latest
    
    #[tokio::test]
    #[ignore]
    async fn test_redis_cache_basic() {
        let cache = RedisCache::new("redis://localhost:6379", 60)
            .await
            .unwrap();

        let schema = Schema::new(
            "test.subject".to_string(),
            SchemaVersion::new(1, 0, 0),
            SchemaContent::Json(serde_json::json!({"type": "object"})),
            SchemaMetadata::default(),
        );
        let id = schema.id;

        // Should be empty initially
        assert!(cache.get(id).await.unwrap().is_none());

        // Put schema
        cache.put(id, &schema).await.unwrap();

        // Should now be cached
        let cached = cache.get(id).await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().id, id);

        // Invalidate
        cache.invalidate(id).await.unwrap();
        assert!(cache.get(id).await.unwrap().is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_connection() {
        let cache = RedisCache::new("redis://localhost:6379", 60)
            .await
            .unwrap();
        
        assert!(cache.ping().await.is_ok());
    }
}
