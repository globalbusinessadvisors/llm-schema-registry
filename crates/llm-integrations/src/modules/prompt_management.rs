// Prompt Management Integration (LangChain)
// Validates prompt template inputs against schemas

use super::{LLMModuleIntegration, ValidationResult};
use crate::events::SchemaEvent;
use async_trait::async_trait;
use anyhow::Result;
use moka::future::Cache;
use schema_registry_core::schema::RegisteredSchema;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

/// Prompt Management Integration
pub struct PromptManagementIntegration {
    /// Schema cache (5-minute TTL)
    schema_cache: Cache<Uuid, RegisteredSchema>,

    /// Registry API URL
    registry_url: String,

    /// HTTP client
    client: reqwest::Client,
}

impl PromptManagementIntegration {
    /// Create new prompt management integration
    pub fn new(registry_url: String) -> Self {
        // Configure cache: max 10,000 schemas, 5-minute TTL
        let schema_cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(300))
            .build();

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            schema_cache,
            registry_url,
            client,
        }
    }

    /// Identify affected prompts when schema changes
    async fn identify_affected_prompts(&self, event: &SchemaEvent) -> Result<Vec<String>> {
        // In production, this would query a database of registered prompts
        info!(
            schema = %event.name,
            version = %event.version,
            "Identifying affected prompts"
        );
        Ok(Vec::new())
    }

    /// Notify prompt owners of schema changes
    async fn notify_prompt_owners(
        &self,
        event: &SchemaEvent,
        affected_prompts: Vec<String>,
    ) -> Result<()> {
        if affected_prompts.is_empty() {
            return Ok(());
        }
        info!(
            schema = %event.name,
            prompt_count = affected_prompts.len(),
            "Notifying prompt owners of schema change"
        );
        Ok(())
    }
}

#[async_trait]
impl LLMModuleIntegration for PromptManagementIntegration {
    fn name(&self) -> &str {
        "Prompt Management (LangChain)"
    }

    async fn handle_schema_event(&self, event: &SchemaEvent) -> Result<()> {
        info!(
            event_type = ?event.event_type,
            schema = %event.name,
            version = %event.version,
            "Handling schema event in Prompt Management"
        );

        // Invalidate cache for this schema
        self.schema_cache.invalidate(&event.schema_id).await;

        // Identify affected prompts
        let affected = self.identify_affected_prompts(event).await?;

        // Notify owners
        self.notify_prompt_owners(event, affected).await?;

        Ok(())
    }

    async fn validate_data(&self, schema_id: Uuid, _data: &Value) -> Result<ValidationResult> {
        // Get schema (from cache or registry)
        let _schema = self.get_schema(schema_id).await?;

        // TODO: Implement actual validation using schema-registry-validation
        // For now, return a simple validation result
        Ok(ValidationResult::valid())
    }

    async fn get_schema(&self, schema_id: Uuid) -> Result<RegisteredSchema> {
        // Check cache first
        if let Some(schema) = self.schema_cache.get(&schema_id).await {
            return Ok(schema);
        }

        // Fetch from registry
        let url = format!("{}/api/v1/schemas/{}", self.registry_url, schema_id);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch schema: {}", response.status());
        }

        let schema: RegisteredSchema = response.json().await?;

        // Cache it
        self.schema_cache.insert(schema_id, schema.clone()).await;

        Ok(schema)
    }

    async fn health_check(&self) -> Result<()> {
        let url = format!("{}/health", self.registry_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Registry health check failed");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::SchemaEventType;

    #[tokio::test]
    async fn test_prompt_management_integration() {
        let integration = PromptManagementIntegration::new(
            "http://localhost:8080".to_string()
        );

        assert_eq!(integration.name(), "Prompt Management (LangChain)");
    }

    #[tokio::test]
    async fn test_handle_schema_event() {
        let integration = PromptManagementIntegration::new(
            "http://localhost:8080".to_string()
        );

        let event = SchemaEvent {
            event_id: Uuid::new_v4(),
            event_type: SchemaEventType::Updated,
            schema_id: Uuid::new_v4(),
            namespace: "test".to_string(),
            name: "User".to_string(),
            version: "2.0.0".to_string(),
            previous_version: Some("1.0.0".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };

        // Should not fail
        let result = integration.handle_schema_event(&event).await;
        assert!(result.is_ok());
    }
}
