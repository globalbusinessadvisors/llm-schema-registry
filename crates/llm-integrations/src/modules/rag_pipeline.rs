// RAG Pipeline Integration (LlamaIndex)
// Validates documents and metadata during indexing

use super::{LLMModuleIntegration, ValidationResult};
use crate::events::SchemaEvent;
use async_trait::async_trait;
use anyhow::Result;
use moka::future::Cache;
use schema_registry_core::schema::RegisteredSchema;
use serde_json::Value;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

/// RAG Pipeline Integration
pub struct RAGPipelineIntegration {
    schema_cache: Cache<Uuid, RegisteredSchema>,
    registry_url: String,
    client: reqwest::Client,
}

impl RAGPipelineIntegration {
    pub fn new(registry_url: String) -> Self {
        let schema_cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(300))
            .build();
        let client = reqwest::Client::new();

        Self { schema_cache, registry_url, client }
    }
}

#[async_trait]
impl LLMModuleIntegration for RAGPipelineIntegration {
    fn name(&self) -> &str {
        "RAG Pipeline (LlamaIndex)"
    }

    async fn handle_schema_event(&self, event: &SchemaEvent) -> Result<()> {
        info!(schema = %event.name, "Handling schema event in RAG Pipeline");
        self.schema_cache.invalidate(&event.schema_id).await;
        // Trigger reindexing of affected documents
        Ok(())
    }

    async fn validate_data(&self, schema_id: Uuid, _data: &Value) -> Result<ValidationResult> {
        let _schema = self.get_schema(schema_id).await?;

        // TODO: Implement actual validation using schema-registry-validation
        // For now, return a simple validation result
        Ok(ValidationResult::valid())
    }

    async fn get_schema(&self, schema_id: Uuid) -> Result<RegisteredSchema> {
        if let Some(schema) = self.schema_cache.get(&schema_id).await {
            return Ok(schema);
        }
        let url = format!("{}/api/v1/schemas/{}", self.registry_url, schema_id);
        let schema: RegisteredSchema = self.client.get(&url).send().await?.json().await?;
        self.schema_cache.insert(schema_id, schema.clone()).await;
        Ok(schema)
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}
