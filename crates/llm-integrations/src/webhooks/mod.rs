// Webhook dispatcher for pushing events to LLM modules

pub mod dispatcher;

pub use dispatcher::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL
    pub url: String,

    /// HTTP headers to include
    pub headers: HashMap<String, String>,

    /// Max retry attempts (default: 3)
    pub max_retries: u32,

    /// Timeout in seconds (default: 10)
    pub timeout_secs: u64,

    /// Secret for HMAC signature verification
    pub secret: Option<String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            headers: HashMap::new(),
            max_retries: 3,
            timeout_secs: 10,
            secret: None,
        }
    }
}
