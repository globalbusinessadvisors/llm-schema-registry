// Event system for LLM module integrations

pub mod types;
pub mod bus;
pub mod kafka;
pub mod rabbitmq;

pub use types::*;
pub use bus::*;

use async_trait::async_trait;
use std::sync::Arc;
use anyhow::Result;

/// Event bus trait for publishing schema events to LLM modules
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish a schema event
    async fn publish(&self, event: SchemaEvent) -> Result<()>;

    /// Subscribe to schema events
    async fn subscribe<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(SchemaEvent) -> Result<()> + Send + Sync + 'static;

    /// Health check
    async fn health_check(&self) -> Result<()>;
}

pub type DynEventBus = Arc<dyn EventBus>;
