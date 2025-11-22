// Event bus implementation with retry and circuit breaker

use super::{EventBus, SchemaEvent};
use async_trait::async_trait;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;
use tracing::{info, warn, error};

/// In-memory event bus (for testing and simple deployments)
pub struct InMemoryEventBus {
    handlers: Arc<tokio::sync::RwLock<Vec<EventHandler>>>,
}

type EventHandler = Arc<dyn Fn(SchemaEvent) -> Result<()> + Send + Sync>;

impl InMemoryEventBus {
    /// Create a new in-memory event bus
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish(&self, event: SchemaEvent) -> Result<()> {
        info!(
            event_id = %event.event_id,
            event_type = ?event.event_type,
            schema = %event.name,
            "Publishing schema event"
        );

        let handlers = self.handlers.read().await;

        // Define retry strategy: 3 attempts with exponential backoff
        let retry_strategy = ExponentialBackoff::from_millis(100)
            .max_delay(Duration::from_secs(2))
            .take(3);

        for (idx, handler) in handlers.iter().enumerate() {
            let event_clone = event.clone();
            let handler_clone = Arc::clone(handler);

            // Retry with exponential backoff
            let result = Retry::spawn(retry_strategy.clone(), move || {
                let event = event_clone.clone();
                let handler = Arc::clone(&handler_clone);

                async move {
                    handler(event)
                        .map_err(|e| anyhow::anyhow!("Handler failed: {}", e))
                }
            }).await;

            match result {
                Ok(_) => info!(handler_idx = idx, "Event handler executed successfully"),
                Err(e) => {
                    error!(
                        handler_idx = idx,
                        error = %e,
                        "Event handler failed after retries"
                    );
                    // Continue to next handler even if one fails
                }
            }
        }

        Ok(())
    }

    async fn subscribe<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(SchemaEvent) -> Result<()> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.push(Arc::new(handler));
        info!(total_handlers = handlers.len(), "New event handler subscribed");
        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        let handlers = self.handlers.read().await;
        info!(handler_count = handlers.len(), "Event bus health check");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_in_memory_event_bus() {
        let bus = InMemoryEventBus::new();

        // Subscribe handler
        let received = Arc::new(tokio::sync::RwLock::new(Vec::new()));
        let received_clone = Arc::clone(&received);

        bus.subscribe(move |event: SchemaEvent| {
            let received = Arc::clone(&received_clone);
            tokio::spawn(async move {
                received.write().await.push(event);
            });
            Ok(())
        }).await.unwrap();

        // Publish event
        let event = SchemaEvent::registered(
            Uuid::new_v4(),
            "test".to_string(),
            "User".to_string(),
            "1.0.0".to_string(),
        );

        bus.publish(event).await.unwrap();

        // Give async handler time to run
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify event was received
        let events = received.read().await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_health_check() {
        let bus = InMemoryEventBus::new();
        assert!(bus.health_check().await.is_ok());
    }
}
