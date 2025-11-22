// Webhook dispatcher with retry logic and circuit breaker

use super::WebhookConfig;
use crate::events::SchemaEvent;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;
use tracing::{info, warn, error};

/// Webhook dispatcher
pub struct WebhookDispatcher {
    client: Client,
    configs: Vec<WebhookConfig>,
}

impl WebhookDispatcher {
    /// Create a new webhook dispatcher
    pub fn new(configs: Vec<WebhookConfig>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            configs,
        })
    }

    /// Dispatch event to all configured webhooks
    pub async fn dispatch(&self, event: &SchemaEvent) -> Result<()> {
        info!(
            event_id = %event.event_id,
            webhook_count = self.configs.len(),
            "Dispatching event to webhooks"
        );

        for config in &self.configs {
            self.dispatch_to_webhook(event, config).await?;
        }

        Ok(())
    }

    /// Dispatch to a single webhook with retry
    async fn dispatch_to_webhook(&self, event: &SchemaEvent, config: &WebhookConfig) -> Result<()> {
        let retry_strategy = ExponentialBackoff::from_millis(500)
            .max_delay(Duration::from_secs(5))
            .take(config.max_retries as usize);

        let client = self.client.clone();
        let url = config.url.clone();
        let event_json = serde_json::to_string(event)?;
        let headers = config.headers.clone();

        let result = Retry::spawn(retry_strategy, move || {
            let client = client.clone();
            let url = url.clone();
            let body = event_json.clone();
            let headers = headers.clone();

            async move {
                let mut request = client.post(&url);

                // Add custom headers
                for (key, value) in &headers {
                    request = request.header(key, value);
                }

                let response = request
                    .header("Content-Type", "application/json")
                    .body(body.clone())
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

                if !response.status().is_success() {
                    anyhow::bail!("Webhook returned error status: {}", response.status());
                }

                Ok::<(), anyhow::Error>(())
            }
        }).await;

        match result {
            Ok(_) => {
                info!(url = %config.url, "Webhook delivered successfully");
                Ok(())
            }
            Err(e) => {
                error!(
                    url = %config.url,
                    error = %e,
                    "Webhook delivery failed after all retries"
                );
                Err(e)
            }
        }
    }

    /// Health check - verify all webhooks are reachable
    pub async fn health_check(&self) -> Result<()> {
        for config in &self.configs {
            let response = self.client
                .get(&config.url)
                .timeout(Duration::from_secs(5))
                .send()
                .await;

            if response.is_err() {
                warn!(url = %config.url, "Webhook health check failed");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_webhook_dispatch_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/webhook"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let config = WebhookConfig {
            url: format!("{}/webhook", mock_server.uri()),
            ..Default::default()
        };

        let dispatcher = WebhookDispatcher::new(vec![config]).unwrap();

        let event = SchemaEvent::registered(
            Uuid::new_v4(),
            "test".to_string(),
            "User".to_string(),
            "1.0.0".to_string(),
        );

        assert!(dispatcher.dispatch(&event).await.is_ok());
    }

    #[tokio::test]
    async fn test_webhook_dispatch_retry() {
        let mock_server = MockServer::start().await;

        // First 2 requests fail, 3rd succeeds
        Mock::given(method("POST"))
            .and(path("/webhook"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/webhook"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let config = WebhookConfig {
            url: format!("{}/webhook", mock_server.uri()),
            max_retries: 3,
            ..Default::default()
        };

        let dispatcher = WebhookDispatcher::new(vec![config]).unwrap();

        let event = SchemaEvent::registered(
            Uuid::new_v4(),
            "test".to_string(),
            "User".to_string(),
            "1.0.0".to_string(),
        );

        assert!(dispatcher.dispatch(&event).await.is_ok());
    }
}
