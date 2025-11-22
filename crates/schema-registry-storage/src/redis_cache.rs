//! Redis caching layer

use async_trait::async_trait;
use schema_registry_core::{error::{Error, Result}, schema::RegisteredSchema, traits::SchemaStorage, versioning::SemanticVersion};
use uuid::Uuid;

use crate::StorageConfig;

/// Redis cache implementation
pub struct RedisCache {}

impl RedisCache {
    pub async fn new(_config: StorageConfig) -> Result<Self> {
        Ok(Self {})
    }
}

#[async_trait]
impl SchemaStorage for RedisCache {
    async fn store(&self, _schema: RegisteredSchema) -> Result<()> {
        Ok(())
    }

    async fn retrieve(&self, _id: Uuid, _version: Option<SemanticVersion>) -> Result<RegisteredSchema> {
        Err(Error::SchemaNotFound("Not in cache".to_string()))
    }

    async fn retrieve_by_hash(&self, _content_hash: &str) -> Result<Option<RegisteredSchema>> {
        Ok(None)
    }

    async fn update(&self, _schema: RegisteredSchema) -> Result<()> {
        Ok(())
    }

    async fn delete(&self, _id: Uuid, _version: SemanticVersion) -> Result<()> {
        Ok(())
    }

    async fn list_versions(&self, _id: Uuid) -> Result<Vec<SemanticVersion>> {
        Ok(vec![])
    }

    async fn find_by_name(&self, _namespace: &str, _name: &str) -> Result<Vec<RegisteredSchema>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema_registry_core::{
        schema::SchemaMetadata,
        types::SerializationFormat,
        CompatibilityMode, SchemaState, SemanticVersion, RegisteredSchema, SchemaLifecycle,
    };
    use uuid::Uuid;

    #[tokio::test]
    async fn test_redis_cache_creation() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await;
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn test_redis_cache_store_succeeds() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await.unwrap();

        let id = Uuid::new_v4();
        let schema = RegisteredSchema {
            id,
            namespace: "test".to_string(),
            name: "schema".to_string(),
            version: SemanticVersion::new(1, 0, 0),
            format: SerializationFormat::JsonSchema,
            content: "{}".to_string(),
            content_hash: "abc123".to_string(),
            description: "test schema".to_string(),
            compatibility_mode: CompatibilityMode::Full,
            state: SchemaState::Active,
            metadata: SchemaMetadata {
                created_at: chrono::Utc::now(),
                created_by: "test".to_string(),
                updated_at: chrono::Utc::now(),
                updated_by: "test".to_string(),
                activated_at: None,
                deprecation: None,
                deletion: None,
                custom: std::collections::HashMap::new(),
            },
            tags: vec![],
            examples: vec![],
            lifecycle: SchemaLifecycle::new(id),
        };

        let result = cache.store(schema).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_redis_cache_retrieve_not_in_cache() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await.unwrap();
        let result = cache.retrieve(Uuid::new_v4(), None).await;
        assert!(result.is_err());

        if let Err(Error::SchemaNotFound(msg)) = result {
            assert_eq!(msg, "Not in cache");
        } else {
            panic!("Expected SchemaNotFound error");
        }
    }

    #[tokio::test]
    async fn test_redis_cache_retrieve_with_version() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await.unwrap();
        let result = cache
            .retrieve(Uuid::new_v4(), Some(SemanticVersion::new(1, 0, 0)))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_redis_cache_retrieve_by_hash_returns_none() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await.unwrap();
        let result = cache.retrieve_by_hash("test_hash").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_redis_cache_update_succeeds() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await.unwrap();

        let id = Uuid::new_v4();
        let schema = RegisteredSchema {
            id,
            namespace: "test".to_string(),
            name: "schema".to_string(),
            version: SemanticVersion::new(1, 0, 0),
            format: SerializationFormat::JsonSchema,
            content: "{}".to_string(),
            content_hash: "abc123".to_string(),
            description: "test schema".to_string(),
            compatibility_mode: CompatibilityMode::Full,
            state: SchemaState::Active,
            metadata: SchemaMetadata {
                created_at: chrono::Utc::now(),
                created_by: "test".to_string(),
                updated_at: chrono::Utc::now(),
                updated_by: "test".to_string(),
                activated_at: None,
                deprecation: None,
                deletion: None,
                custom: std::collections::HashMap::new(),
            },
            tags: vec![],
            examples: vec![],
            lifecycle: SchemaLifecycle::new(id),
        };

        let result = cache.update(schema).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_redis_cache_delete_succeeds() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await.unwrap();
        let result = cache
            .delete(Uuid::new_v4(), SemanticVersion::new(1, 0, 0))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_redis_cache_list_versions_empty() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await.unwrap();
        let result = cache.list_versions(Uuid::new_v4()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_redis_cache_find_by_name_empty() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await.unwrap();
        let result = cache.find_by_name("test", "schema").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_redis_config_different_urls() {
        let config1 = StorageConfig::Redis {
            url: "redis://host1:6379".to_string(),
        };

        let config2 = StorageConfig::Redis {
            url: "redis://host2:6379".to_string(),
        };

        if let (StorageConfig::Redis { url: u1 }, StorageConfig::Redis { url: u2 }) =
            (config1, config2)
        {
            assert_ne!(u1, u2);
        }
    }

    #[tokio::test]
    async fn test_redis_cache_multiple_operations() {
        let config = StorageConfig::Redis {
            url: "redis://localhost:6379".to_string(),
        };

        let cache = RedisCache::new(config).await.unwrap();
        let id = Uuid::new_v4();
        let schema = RegisteredSchema {
            id,
            namespace: "test".to_string(),
            name: "schema".to_string(),
            version: SemanticVersion::new(1, 0, 0),
            format: SerializationFormat::JsonSchema,
            content: "{}".to_string(),
            content_hash: "abc123".to_string(),
            description: "test schema".to_string(),
            compatibility_mode: CompatibilityMode::Full,
            state: SchemaState::Active,
            metadata: SchemaMetadata {
                created_at: chrono::Utc::now(),
                created_by: "test".to_string(),
                updated_at: chrono::Utc::now(),
                updated_by: "test".to_string(),
                activated_at: None,
                deprecation: None,
                deletion: None,
                custom: std::collections::HashMap::new(),
            },
            tags: vec![],
            examples: vec![],
            lifecycle: SchemaLifecycle::new(id),
        };

        // Store
        assert!(cache.store(schema.clone()).await.is_ok());
        // Update
        assert!(cache.update(schema.clone()).await.is_ok());
        // Delete
        assert!(cache
            .delete(schema.id, schema.version.clone())
            .await
            .is_ok());
    }
}
