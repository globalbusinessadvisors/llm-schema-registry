//! Structured logging with correlation IDs and contextual fields
//!
//! This module provides:
//! - JSON-formatted structured logging
//! - Correlation ID propagation
//! - Contextual fields (request_id, schema_id, user_id)
//! - Log level configuration per module
//! - Log sampling for high-volume paths

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::Level;

/// Log sampling configuration
#[derive(Debug, Clone)]
pub struct LogSamplingConfig {
    /// Paths to sample (key: path pattern, value: sample rate 0.0-1.0)
    pub sampled_paths: HashMap<String, f64>,
    /// Default sample rate for non-configured paths
    pub default_sample_rate: f64,
}

impl Default for LogSamplingConfig {
    fn default() -> Self {
        let mut sampled_paths = HashMap::new();
        // Sample health checks at 1%
        sampled_paths.insert("/health".to_string(), 0.01);
        sampled_paths.insert("/ready".to_string(), 0.01);
        sampled_paths.insert("/metrics".to_string(), 0.01);

        Self {
            sampled_paths,
            default_sample_rate: 1.0, // Log everything by default
        }
    }
}

/// Contextual logging fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContext {
    /// Correlation ID for request tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Request ID (unique per request)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Schema ID being operated on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,

    /// User ID making the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Tenant ID for multi-tenancy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// API version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,

    /// Client IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ip: Option<String>,

    /// User agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Additional custom fields
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub custom_fields: HashMap<String, String>,
}

impl Default for LogContext {
    fn default() -> Self {
        Self {
            correlation_id: None,
            request_id: None,
            schema_id: None,
            user_id: None,
            tenant_id: None,
            api_version: None,
            client_ip: None,
            user_agent: None,
            custom_fields: HashMap::new(),
        }
    }
}

impl LogContext {
    /// Creates a new log context from HTTP headers
    pub fn from_headers(headers: &axum::http::HeaderMap) -> Self {
        let mut ctx = Self::default();

        // Extract correlation ID
        ctx.correlation_id = headers
            .get("x-correlation-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Extract request ID
        ctx.request_id = headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Extract user ID (from auth header or custom header)
        ctx.user_id = headers
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Extract tenant ID
        ctx.tenant_id = headers
            .get("x-tenant-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Extract API version
        ctx.api_version = headers
            .get("x-api-version")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Extract client IP
        ctx.client_ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .or_else(|| {
                headers
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            });

        // Extract user agent
        ctx.user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        ctx
    }

    /// Adds a custom field to the context
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_fields.insert(key.into(), value.into());
        self
    }

    /// Sets correlation ID
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Sets request ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Sets schema ID
    pub fn with_schema_id(mut self, id: impl Into<String>) -> Self {
        self.schema_id = Some(id.into());
        self
    }

    /// Sets user ID
    pub fn with_user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }
}

/// Module-specific log level configuration
#[derive(Debug, Clone)]
pub struct ModuleLogLevels {
    levels: HashMap<String, Level>,
    default_level: Level,
}

impl Default for ModuleLogLevels {
    fn default() -> Self {
        let mut levels = HashMap::new();

        // Set specific log levels for different modules
        levels.insert("schema_registry_api".to_string(), Level::INFO);
        levels.insert("schema_registry_storage".to_string(), Level::INFO);
        levels.insert("schema_registry_validation".to_string(), Level::DEBUG);
        levels.insert("schema_registry_compatibility".to_string(), Level::DEBUG);
        levels.insert("sqlx".to_string(), Level::WARN); // Reduce DB query noise
        levels.insert("tower_http".to_string(), Level::INFO);
        levels.insert("hyper".to_string(), Level::WARN);

        Self {
            levels,
            default_level: Level::INFO,
        }
    }
}

impl ModuleLogLevels {
    /// Gets the log level for a specific module
    pub fn get_level(&self, module: &str) -> Level {
        // Try exact match first
        if let Some(&level) = self.levels.get(module) {
            return level;
        }

        // Try prefix match
        for (prefix, &level) in &self.levels {
            if module.starts_with(prefix) {
                return level;
            }
        }

        self.default_level
    }

    /// Sets log level for a module
    pub fn set_level(&mut self, module: impl Into<String>, level: Level) {
        self.levels.insert(module.into(), level);
    }
}

/// Structured log entry
#[derive(Debug, Serialize)]
pub struct StructuredLogEntry {
    #[serde(with = "timestamp_format")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub message: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

mod timestamp_format {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.to_rfc3339())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

/// Helper macros for structured logging
#[macro_export]
macro_rules! log_info {
    ($correlation_id:expr, $msg:expr $(, $key:expr => $value:expr)*) => {
        tracing::info!(
            correlation_id = $correlation_id,
            $($key = ?$value,)*
            $msg
        );
    };
}

#[macro_export]
macro_rules! log_warn {
    ($correlation_id:expr, $msg:expr $(, $key:expr => $value:expr)*) => {
        tracing::warn!(
            correlation_id = $correlation_id,
            $($key = ?$value,)*
            $msg
        );
    };
}

#[macro_export]
macro_rules! log_error {
    ($correlation_id:expr, $msg:expr $(, $key:expr => $value:expr)*) => {
        tracing::error!(
            correlation_id = $correlation_id,
            $($key = ?$value,)*
            $msg
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_log_context_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-correlation-id", "test-corr-id".parse().unwrap());
        headers.insert("x-request-id", "test-req-id".parse().unwrap());
        headers.insert("x-user-id", "user-123".parse().unwrap());

        let ctx = LogContext::from_headers(&headers);
        assert_eq!(ctx.correlation_id.unwrap(), "test-corr-id");
        assert_eq!(ctx.request_id.unwrap(), "test-req-id");
        assert_eq!(ctx.user_id.unwrap(), "user-123");
    }

    #[test]
    fn test_log_context_builder() {
        let ctx = LogContext::default()
            .with_correlation_id("corr-123")
            .with_schema_id("schema-456")
            .with_field("custom", "value");

        assert_eq!(ctx.correlation_id.unwrap(), "corr-123");
        assert_eq!(ctx.schema_id.unwrap(), "schema-456");
        assert_eq!(ctx.custom_fields.get("custom").unwrap(), "value");
    }

    #[test]
    fn test_module_log_levels() {
        let levels = ModuleLogLevels::default();
        assert_eq!(levels.get_level("schema_registry_api"), Level::INFO);
        assert_eq!(levels.get_level("sqlx"), Level::WARN);
        assert_eq!(levels.get_level("unknown_module"), Level::INFO);
    }
}
