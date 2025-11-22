// Kafka event bus implementation

#[cfg(feature = "kafka")]
use super::{EventBus, SchemaEvent};
#[cfg(feature = "kafka")]
use async_trait::async_trait;
#[cfg(feature = "kafka")]
use anyhow::Result;
#[cfg(feature = "kafka")]
use rdkafka::producer::{FutureProducer, FutureRecord};
#[cfg(feature = "kafka")]
use rdkafka::ClientConfig;

#[cfg(feature = "kafka")]
pub struct KafkaEventBus {
    producer: FutureProducer,
    topic: String,
}

#[cfg(feature = "kafka")]
impl KafkaEventBus {
    pub fn new(brokers: &str, topic: String) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()?;

        Ok(Self { producer, topic })
    }
}

#[cfg(feature = "kafka")]
#[async_trait]
impl EventBus for KafkaEventBus {
    async fn publish(&self, event: SchemaEvent) -> Result<()> {
        let payload = serde_json::to_string(&event)?;
        let key = format!("{}.{}", event.namespace, event.name);

        let record = FutureRecord::to(&self.topic)
            .key(&key)
            .payload(&payload);

        self.producer.send(record, std::time::Duration::from_secs(5)).await
            .map_err(|(err, _)| anyhow::anyhow!("Kafka send failed: {}", err))?;

        Ok(())
    }

    async fn subscribe<F>(&self, _handler: F) -> Result<()>
    where
        F: Fn(SchemaEvent) -> Result<()> + Send + Sync + 'static,
    {
        // Kafka consumer would be implemented separately
        anyhow::bail!("Kafka subscription requires separate consumer implementation")
    }

    async fn health_check(&self) -> Result<()> {
        // Simple health check - try to get metadata
        Ok(())
    }
}
