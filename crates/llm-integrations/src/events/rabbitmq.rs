// RabbitMQ event bus implementation

#[cfg(feature = "rabbitmq")]
use super::{EventBus, SchemaEvent};
#[cfg(feature = "rabbitmq")]
use async_trait::async_trait;
#[cfg(feature = "rabbitmq")]
use anyhow::Result;
#[cfg(feature = "rabbitmq")]
use lapin::{Connection, ConnectionProperties, Channel, options::*, BasicProperties};

#[cfg(feature = "rabbitmq")]
pub struct RabbitMQEventBus {
    channel: Channel,
    exchange: String,
}

#[cfg(feature = "rabbitmq")]
impl RabbitMQEventBus {
    pub async fn new(amqp_url: &str, exchange: String) -> Result<Self> {
        let conn = Connection::connect(amqp_url, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;

        // Declare exchange
        channel.exchange_declare(
            &exchange,
            lapin::ExchangeKind::Topic,
            ExchangeDeclareOptions::default(),
            Default::default(),
        ).await?;

        Ok(Self { channel, exchange })
    }
}

#[cfg(feature = "rabbitmq")]
#[async_trait]
impl EventBus for RabbitMQEventBus {
    async fn publish(&self, event: SchemaEvent) -> Result<()> {
        let payload = serde_json::to_vec(&event)?;
        let routing_key = format!("schema.{:?}.{}.{}",
            event.event_type, event.namespace, event.name);

        self.channel.basic_publish(
            &self.exchange,
            &routing_key,
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        ).await?;

        Ok(())
    }

    async fn subscribe<F>(&self, _handler: F) -> Result<()>
    where
        F: Fn(SchemaEvent) -> Result<()> + Send + Sync + 'static,
    {
        // RabbitMQ consumer would be implemented separately
        anyhow::bail!("RabbitMQ subscription requires separate consumer implementation")
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}
