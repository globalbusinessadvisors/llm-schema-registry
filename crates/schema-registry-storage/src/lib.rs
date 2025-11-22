//! # Schema Registry Storage
//!
//! Storage abstraction layer for PostgreSQL, Redis, and S3.
//! Implements the SchemaStorage trait from schema-registry-core.

pub mod cache_warmer;
pub mod postgres;
pub mod redis_cache;
pub mod s3;

use async_trait::async_trait;
use schema_registry_core::{error::Result, schema::RegisteredSchema, traits::SchemaStorage, versioning::SemanticVersion};
use uuid::Uuid;

/// Storage backend configuration
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// PostgreSQL configuration
    Postgres {
        connection_string: String,
        max_connections: u32,
    },
    /// Redis configuration
    Redis {
        url: String,
    },
    /// S3 configuration
    S3 {
        bucket: String,
        region: String,
    },
}

/// Multi-tier storage implementation
pub struct MultiTierStorage {
    // Primary storage (PostgreSQL)
    postgres: postgres::PostgresStorage,
    // Cache layer (Redis)
    cache: redis_cache::RedisCache,
    // Archive storage (S3)
    #[allow(dead_code)]
    s3: s3::S3Storage,
}

impl MultiTierStorage {
    /// Create a new multi-tier storage instance
    pub async fn new(postgres_config: StorageConfig, redis_config: StorageConfig, s3_config: StorageConfig) -> Result<Self> {
        Ok(Self {
            postgres: postgres::PostgresStorage::new(postgres_config).await?,
            cache: redis_cache::RedisCache::new(redis_config).await?,
            s3: s3::S3Storage::new(s3_config).await?,
        })
    }
}

#[async_trait]
impl SchemaStorage for MultiTierStorage {
    async fn store(&self, schema: RegisteredSchema) -> Result<()> {
        // Store in PostgreSQL (primary)
        self.postgres.store(schema.clone()).await?;
        // Update cache
        self.cache.store(schema).await?;
        Ok(())
    }

    async fn retrieve(&self, id: Uuid, version: Option<SemanticVersion>) -> Result<RegisteredSchema> {
        // Try cache first
        if let Ok(schema) = self.cache.retrieve(id, version.clone()).await {
            return Ok(schema);
        }
        // Fallback to PostgreSQL
        let schema = self.postgres.retrieve(id, version).await?;
        // Update cache
        let _ = self.cache.store(schema.clone()).await;
        Ok(schema)
    }

    async fn retrieve_by_hash(&self, content_hash: &str) -> Result<Option<RegisteredSchema>> {
        self.postgres.retrieve_by_hash(content_hash).await
    }

    async fn update(&self, schema: RegisteredSchema) -> Result<()> {
        self.postgres.update(schema.clone()).await?;
        self.cache.store(schema).await?;
        Ok(())
    }

    async fn delete(&self, id: Uuid, version: SemanticVersion) -> Result<()> {
        self.postgres.delete(id, version).await?;
        // Invalidate cache
        Ok(())
    }

    async fn list_versions(&self, id: Uuid) -> Result<Vec<SemanticVersion>> {
        self.postgres.list_versions(id).await
    }

    async fn find_by_name(&self, namespace: &str, name: &str) -> Result<Vec<RegisteredSchema>> {
        self.postgres.find_by_name(namespace, name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_config_postgres_creation() {
        let config = StorageConfig::Postgres {
            connection_string: "postgresql://localhost/test".to_string(),
            max_connections: 10,
        };

        match config {
            StorageConfig::Postgres {
                connection_string,
                max_connections,
            } => {
                assert_eq!(connection_string, "postgresql://localhost/test");
                assert_eq!(max_connections, 10);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_storage_config_redis_creation() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        match config {
            StorageConfig::Redis { url } => {
                assert_eq!(url, "redis://localhost:6379");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_storage_config_s3_creation() {
        let config = StorageConfig::S3 {
            bucket: "my-bucket".to_string(),
            region: "us-east-1".to_string(),
        };

        match config {
            StorageConfig::S3 { bucket, region } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(region, "us-east-1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_storage_config_clone() {
        let config = StorageConfig::Postgres {
            connection_string: "test".to_string(),
            max_connections: 5,
        };
        let cloned = config.clone();

        match (config, cloned) {
            (
                StorageConfig::Postgres {
                    connection_string: c1,
                    max_connections: m1,
                },
                StorageConfig::Postgres {
                    connection_string: c2,
                    max_connections: m2,
                },
            ) => {
                assert_eq!(c1, c2);
                assert_eq!(m1, m2);
            }
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_storage_config_debug() {
        let config = StorageConfig::Redis {
            url: "redis://localhost".to_string(),
        };
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Redis"));
        assert!(debug_str.contains("url"));
    }

    #[test]
    fn test_postgres_config_different_max_connections() {
        let config1 = StorageConfig::Postgres {
            connection_string: "test".to_string(),
            max_connections: 10,
        };
        let config2 = StorageConfig::Postgres {
            connection_string: "test".to_string(),
            max_connections: 20,
        };

        if let (
            StorageConfig::Postgres {
                max_connections: m1, ..
            },
            StorageConfig::Postgres {
                max_connections: m2, ..
            },
        ) = (config1, config2)
        {
            assert_ne!(m1, m2);
        }
    }

    #[test]
    fn test_s3_config_different_regions() {
        let config1 = StorageConfig::S3 {
            bucket: "bucket".to_string(),
            region: "us-east-1".to_string(),
        };
        let config2 = StorageConfig::S3 {
            bucket: "bucket".to_string(),
            region: "eu-west-1".to_string(),
        };

        if let (
            StorageConfig::S3 { region: r1, .. },
            StorageConfig::S3 { region: r2, .. },
        ) = (config1, config2)
        {
            assert_ne!(r1, r2);
        }
    }
}
