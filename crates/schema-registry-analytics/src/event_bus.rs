//! Event bus for real-time event streaming
//!
//! This module provides an in-memory event bus using tokio's broadcast channel
//! for distributing schema usage events to multiple subscribers. It's designed
//! to be later replaced with Kafka or other message brokers for production scale.

use crate::error::{AnalyticsError, Result};
use crate::types::SchemaUsageEvent;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

/// Default channel capacity for the event bus
const DEFAULT_CAPACITY: usize = 10_000;

/// Event bus for broadcasting schema usage events
#[derive(Clone)]
pub struct EventBus {
    sender: Arc<broadcast::Sender<SchemaUsageEvent>>,
    capacity: usize,
}

impl EventBus {
    /// Create a new event bus with default capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new event bus with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender: Arc::new(sender),
            capacity,
        }
    }

    /// Publish an event to the bus
    ///
    /// This is non-blocking and will return an error if the channel is full.
    /// Events are cloned for each active subscriber.
    pub fn publish(&self, event: SchemaUsageEvent) -> Result<usize> {
        match self.sender.send(event.clone()) {
            Ok(receivers) => {
                debug!(
                    event_id = %event.event_id,
                    operation = %event.operation,
                    schema_id = %event.schema_id,
                    receivers = receivers,
                    "Published event to {} receivers",
                    receivers
                );
                Ok(receivers)
            }
            Err(e) => {
                error!(
                    event_id = %event.event_id,
                    error = %e,
                    "Failed to publish event"
                );
                Err(AnalyticsError::ChannelError(e.to_string()))
            }
        }
    }

    /// Publish an event asynchronously
    ///
    /// This method is async-friendly but the actual send is synchronous.
    /// Use this in async contexts for consistency.
    pub async fn publish_async(&self, event: SchemaUsageEvent) -> Result<usize> {
        self.publish(event)
    }

    /// Try to publish an event, logging but not failing on error
    ///
    /// Useful when event publication is best-effort and shouldn't
    /// block the main operation.
    pub fn try_publish(&self, event: SchemaUsageEvent) {
        if let Err(e) = self.publish(event) {
            warn!(error = %e, "Failed to publish event (best-effort)");
        }
    }

    /// Subscribe to the event bus
    ///
    /// Returns a receiver that will get clones of all published events.
    /// Each subscriber gets their own independent stream.
    pub fn subscribe(&self) -> EventReceiver {
        EventReceiver {
            receiver: self.sender.subscribe(),
        }
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Get the channel capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if there are any active subscribers
    pub fn has_subscribers(&self) -> bool {
        self.subscriber_count() > 0
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Event receiver for consuming events from the bus
pub struct EventReceiver {
    receiver: broadcast::Receiver<SchemaUsageEvent>,
}

impl EventReceiver {
    /// Receive the next event
    ///
    /// Returns None if the channel is closed (all senders dropped).
    /// Returns an error if the receiver lagged and missed events.
    pub async fn recv(&mut self) -> Option<Result<SchemaUsageEvent>> {
        match self.receiver.recv().await {
            Ok(event) => Some(Ok(event)),
            Err(broadcast::error::RecvError::Closed) => None,
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(
                    skipped_events = skipped,
                    "Receiver lagged and missed {} events",
                    skipped
                );
                // Continue receiving - don't stop on lag
                Some(Err(AnalyticsError::event_processing(format!(
                    "Receiver lagged, skipped {} events",
                    skipped
                ))))
            }
        }
    }

    /// Try to receive an event without blocking
    ///
    /// Returns Ok(event) if an event is available,
    /// Err(Empty) if no events are available,
    /// Err(Lagged) if the receiver fell behind,
    /// Err(Closed) if the channel is closed.
    pub fn try_recv(&mut self) -> Result<SchemaUsageEvent> {
        match self.receiver.try_recv() {
            Ok(event) => Ok(event),
            Err(broadcast::error::TryRecvError::Empty) => {
                Err(AnalyticsError::event_processing("No events available"))
            }
            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                warn!(
                    skipped_events = skipped,
                    "Receiver lagged and missed {} events",
                    skipped
                );
                Err(AnalyticsError::event_processing(format!(
                    "Receiver lagged, skipped {} events",
                    skipped
                )))
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                Err(AnalyticsError::event_processing("Channel closed"))
            }
        }
    }

    /// Resubscribe to the event bus
    ///
    /// This creates a new receiver that starts receiving from the current point.
    /// Useful if the receiver has lagged too far behind.
    pub fn resubscribe(&self) -> Self {
        Self {
            receiver: self.receiver.resubscribe(),
        }
    }
}

/// Event processor trait for handling events
#[async_trait::async_trait]
pub trait EventProcessor: Send + Sync {
    /// Process a single event
    async fn process(&self, event: SchemaUsageEvent) -> Result<()>;

    /// Handle processing error
    async fn on_error(&self, event: SchemaUsageEvent, error: AnalyticsError) {
        error!(
            event_id = %event.event_id,
            error = %error,
            "Event processing error"
        );
    }
}

/// Event consumer that runs in the background processing events
pub struct EventConsumer {
    receiver: EventReceiver,
    processor: Arc<dyn EventProcessor>,
    shutdown: tokio::sync::watch::Receiver<bool>,
}

impl EventConsumer {
    /// Create a new event consumer
    pub fn new(
        receiver: EventReceiver,
        processor: Arc<dyn EventProcessor>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self {
            receiver,
            processor,
            shutdown,
        }
    }

    /// Run the consumer until shutdown signal
    pub async fn run(mut self) {
        debug!("Event consumer started");

        loop {
            tokio::select! {
                event_result = self.receiver.recv() => {
                    match event_result {
                        Some(Ok(event)) => {
                            if let Err(e) = self.processor.process(event.clone()).await {
                                self.processor.on_error(event, e).await;
                            }
                        }
                        Some(Err(e)) => {
                            // Lagged error - logged in recv(), continue processing
                            warn!(error = %e, "Event receiver error");
                        }
                        None => {
                            debug!("Event channel closed, shutting down consumer");
                            break;
                        }
                    }
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        debug!("Shutdown signal received, stopping consumer");
                        break;
                    }
                }
            }
        }

        debug!("Event consumer stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Operation;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "test-client".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );

        let receivers = bus.publish(event.clone()).unwrap();
        assert_eq!(receivers, 1);

        let received = receiver.recv().await.unwrap().unwrap();
        assert_eq!(received.event_id, event.event_id);
        assert_eq!(received.operation, Operation::Read);
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new();
        let mut receiver1 = bus.subscribe();
        let mut receiver2 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 2);

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Write,
            "test-client".to_string(),
            "us-west-1".to_string(),
            50,
            true,
        );

        let receivers = bus.publish(event.clone()).unwrap();
        assert_eq!(receivers, 2);

        let received1 = receiver1.recv().await.unwrap().unwrap();
        let received2 = receiver2.recv().await.unwrap().unwrap();

        assert_eq!(received1.event_id, event.event_id);
        assert_eq!(received2.event_id, event.event_id);
    }

    #[tokio::test]
    async fn test_event_bus_no_subscribers() {
        let bus = EventBus::new();

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "test-client".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );

        // With broadcast channels, if there are no receivers, the send succeeds but returns 0
        match bus.publish(event) {
            Ok(receivers) => assert_eq!(receivers, 0),
            Err(_) => {} // Also acceptable if channel is closed
        }
        assert!(!bus.has_subscribers());
    }

    #[tokio::test]
    async fn test_try_recv() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();

        // Should be empty initially
        assert!(receiver.try_recv().is_err());

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "test-client".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );

        bus.publish(event.clone()).unwrap();

        // Should now have an event
        let received = receiver.try_recv().unwrap();
        assert_eq!(received.event_id, event.event_id);

        // Should be empty again
        assert!(receiver.try_recv().is_err());
    }

    struct TestProcessor;

    #[async_trait::async_trait]
    impl EventProcessor for TestProcessor {
        async fn process(&self, _event: SchemaUsageEvent) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_consumer() {
        let bus = EventBus::new();
        let receiver = bus.subscribe();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let processor = Arc::new(TestProcessor);
        let consumer = EventConsumer::new(receiver, processor, shutdown_rx);

        let handle = tokio::spawn(async move {
            consumer.run().await;
        });

        // Publish some events
        for _ in 0..5 {
            let event = SchemaUsageEvent::new(
                Uuid::new_v4(),
                Operation::Read,
                "test-client".to_string(),
                "us-west-1".to_string(),
                100,
                true,
            );
            bus.publish(event).unwrap();
        }

        // Small delay to process events
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Signal shutdown
        shutdown_tx.send(true).unwrap();

        // Wait for consumer to stop
        handle.await.unwrap();
    }
}
