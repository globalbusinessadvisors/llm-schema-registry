// LLM module integrations

pub mod prompt_management;
pub mod rag_pipeline;
pub mod model_serving;
pub mod training_pipeline;
pub mod evaluation;

pub use prompt_management::PromptManagementIntegration;
pub use rag_pipeline::RAGPipelineIntegration;
pub use model_serving::ModelServingIntegration;
pub use training_pipeline::TrainingPipelineIntegration;
pub use evaluation::EvaluationFrameworkIntegration;

use crate::events::SchemaEvent;
use async_trait::async_trait;
use anyhow::Result;
use schema_registry_core::schema::RegisteredSchema;
use serde_json::Value;
use uuid::Uuid;

/// LLM module integration trait
#[async_trait]
pub trait LLMModuleIntegration: Send + Sync {
    /// Get module name
    fn name(&self) -> &str;

    /// Handle schema event (registered, updated, deprecated, etc.)
    async fn handle_schema_event(&self, event: &SchemaEvent) -> Result<()>;

    /// Validate data against schema
    async fn validate_data(&self, schema_id: Uuid, data: &Value) -> Result<ValidationResult>;

    /// Get schema by ID (with caching)
    async fn get_schema(&self, schema_id: Uuid) -> Result<RegisteredSchema>;

    /// Health check
    async fn health_check(&self) -> Result<()>;
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }
}
