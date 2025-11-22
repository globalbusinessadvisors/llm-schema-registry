//! # LLM Integrations
//!
//! This crate provides integrations between the Schema Registry and various LLM modules:
//!
//! 1. **Prompt Management (LangChain)** - Validates prompt template inputs
//! 2. **RAG Pipeline (LlamaIndex)** - Validates documents and metadata during indexing
//! 3. **Model Serving (vLLM)** - Validates input/output schemas for model inference
//! 4. **Training Data Pipeline** - Validates training datasets and features
//! 5. **Evaluation Framework** - Validates test cases, results, and metrics
//!
//! ## Integration Patterns
//!
//! - **Event-driven**: Via Kafka/RabbitMQ for real-time schema change notifications
//! - **Pull-based**: Via Client SDKs with local caching (5-min TTL)
//! - **Webhook-based**: HTTP callbacks with retry logic and circuit breaker
//!
//! ## Features
//!
//! - Schema validation with caching
//! - Event bus abstraction (Kafka, RabbitMQ, in-memory)
//! - Webhook dispatcher with exponential backoff retry
//! - Circuit breaker pattern for fault tolerance
//! - Comprehensive error handling and logging
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use llm_integrations::modules::PromptManagementIntegration;
//! use llm_integrations::events::{InMemoryEventBus, EventBus};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create integrations
//! let prompt_mgmt = PromptManagementIntegration::new(
//!     "http://localhost:8080".to_string()
//! );
//!
//! // Create event bus
//! let event_bus = InMemoryEventBus::new();
//!
//! // Subscribe to events
//! event_bus.subscribe(move |event| {
//!     println!("Received event: {:?}", event);
//!     Ok(())
//! }).await?;
//! # Ok(())
//! # }
//! ```

pub mod events;
pub mod modules;
pub mod webhooks;

pub use events::{EventBus, InMemoryEventBus, SchemaEvent, SchemaEventType};
pub use modules::{
    LLMModuleIntegration,
    PromptManagementIntegration,
    RAGPipelineIntegration,
    ModelServingIntegration,
    TrainingPipelineIntegration,
    EvaluationFrameworkIntegration,
    ValidationResult,
};
pub use webhooks::{WebhookConfig, WebhookDispatcher};

/// LLM Integrations Result type
pub type Result<T> = anyhow::Result<T>;
