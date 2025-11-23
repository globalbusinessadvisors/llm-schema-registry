//! Error types for the LLM Schema Registry SDK.
//!
//! This module provides comprehensive error handling with strongly-typed error variants
//! for all possible failure modes when interacting with the Schema Registry API.

use thiserror::Error;

/// Result type alias for SDK operations.
pub type Result<T> = std::result::Result<T, SchemaRegistryError>;

/// Main error type for the Schema Registry SDK.
///
/// This enum covers all possible error conditions that can occur during SDK operations,
/// providing detailed error messages and context for debugging and error handling.
#[derive(Error, Debug)]
pub enum SchemaRegistryError {
    /// Schema was not found in the registry.
    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    /// Schema validation failed.
    #[error("Schema validation failed: {0}")]
    ValidationError(String),

    /// Schema is incompatible with existing versions.
    #[error("Incompatible schema: {0}")]
    IncompatibleSchema(String),

    /// Authentication failed (invalid or missing API key).
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    /// Response deserialization failed.
    #[error("Failed to deserialize response: {0}")]
    DeserializationError(String),

    /// Request serialization failed.
    #[error("Failed to serialize request: {0}")]
    SerializationError(String),

    /// Network timeout occurred.
    #[error("Request timeout: {0}")]
    TimeoutError(String),

    /// Server returned an error response.
    #[error("Server error (status {status}): {message}")]
    ServerError {
        /// HTTP status code
        status: u16,
        /// Error message from server
        message: String,
    },

    /// Invalid configuration provided.
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// Invalid URL provided.
    #[error("Invalid URL: {0}")]
    UrlError(String),

    /// Cache operation failed.
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Generic error for unexpected conditions.
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl SchemaRegistryError {
    /// Returns true if this error is retryable.
    ///
    /// Retryable errors include network timeouts, rate limits, and certain server errors.
    pub fn is_retryable(&self) -> bool {
        match self {
            SchemaRegistryError::TimeoutError(_)
            | SchemaRegistryError::RateLimitError(_)
            | SchemaRegistryError::HttpError(_) => true,
            SchemaRegistryError::ServerError { status, .. } => *status >= 500 && *status < 600,
            _ => false,
        }
    }

    /// Returns true if this error is a client error (4xx status codes).
    pub fn is_client_error(&self) -> bool {
        match self {
            SchemaRegistryError::SchemaNotFound(_)
            | SchemaRegistryError::ValidationError(_)
            | SchemaRegistryError::IncompatibleSchema(_)
            | SchemaRegistryError::AuthenticationError(_) => true,
            SchemaRegistryError::ServerError { status, .. } => *status >= 400 && *status < 500,
            _ => false,
        }
    }

    /// Returns true if this error is a server error (5xx status codes).
    pub fn is_server_error(&self) -> bool {
        match self {
            SchemaRegistryError::ServerError { status, .. } => *status >= 500 && *status < 600,
            _ => false,
        }
    }
}

// Conversions from external error types

impl From<reqwest::Error> for SchemaRegistryError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            SchemaRegistryError::TimeoutError(err.to_string())
        } else if err.is_status() {
            if let Some(status) = err.status() {
                let status_code = status.as_u16();
                let message = err.to_string();

                match status_code {
                    404 => SchemaRegistryError::SchemaNotFound(message),
                    401 | 403 => SchemaRegistryError::AuthenticationError(message),
                    429 => SchemaRegistryError::RateLimitError(message),
                    _ => SchemaRegistryError::ServerError {
                        status: status_code,
                        message,
                    },
                }
            } else {
                SchemaRegistryError::HttpError(err.to_string())
            }
        } else {
            SchemaRegistryError::HttpError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for SchemaRegistryError {
    fn from(err: serde_json::Error) -> Self {
        if err.is_io() {
            SchemaRegistryError::InternalError(err.to_string())
        } else if err.is_syntax() || err.is_data() {
            SchemaRegistryError::DeserializationError(err.to_string())
        } else {
            SchemaRegistryError::SerializationError(err.to_string())
        }
    }
}

impl From<url::ParseError> for SchemaRegistryError {
    fn from(err: url::ParseError) -> Self {
        SchemaRegistryError::UrlError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SchemaRegistryError::SchemaNotFound("test.schema".to_string());
        assert_eq!(err.to_string(), "Schema not found: test.schema");

        let err = SchemaRegistryError::ServerError {
            status: 500,
            message: "Internal server error".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Server error (status 500): Internal server error"
        );
    }

    #[test]
    fn test_is_retryable() {
        assert!(SchemaRegistryError::TimeoutError("timeout".to_string()).is_retryable());
        assert!(SchemaRegistryError::RateLimitError("rate limit".to_string()).is_retryable());
        assert!(SchemaRegistryError::ServerError {
            status: 503,
            message: "Service unavailable".to_string()
        }
        .is_retryable());

        assert!(!SchemaRegistryError::SchemaNotFound("not found".to_string()).is_retryable());
        assert!(!SchemaRegistryError::ValidationError("invalid".to_string()).is_retryable());
        assert!(!SchemaRegistryError::AuthenticationError("unauthorized".to_string())
            .is_retryable());
    }

    #[test]
    fn test_is_client_error() {
        assert!(SchemaRegistryError::SchemaNotFound("not found".to_string()).is_client_error());
        assert!(
            SchemaRegistryError::ValidationError("validation failed".to_string())
                .is_client_error()
        );
        assert!(SchemaRegistryError::ServerError {
            status: 404,
            message: "Not found".to_string()
        }
        .is_client_error());

        assert!(!SchemaRegistryError::ServerError {
            status: 500,
            message: "Server error".to_string()
        }
        .is_client_error());
    }

    #[test]
    fn test_is_server_error() {
        assert!(SchemaRegistryError::ServerError {
            status: 500,
            message: "Internal error".to_string()
        }
        .is_server_error());
        assert!(SchemaRegistryError::ServerError {
            status: 503,
            message: "Service unavailable".to_string()
        }
        .is_server_error());

        assert!(!SchemaRegistryError::ServerError {
            status: 404,
            message: "Not found".to_string()
        }
        .is_server_error());
        assert!(!SchemaRegistryError::SchemaNotFound("not found".to_string()).is_server_error());
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let json = r#"{"invalid": json}"#;
        let err: std::result::Result<serde_json::Value, _> = serde_json::from_str(json);
        let sdk_err: SchemaRegistryError = err.unwrap_err().into();

        match sdk_err {
            SchemaRegistryError::DeserializationError(_) => (),
            _ => panic!("Expected DeserializationError"),
        }
    }

    #[test]
    fn test_url_parse_error_conversion() {
        let err = url::Url::parse("not a valid url").unwrap_err();
        let sdk_err: SchemaRegistryError = err.into();

        match sdk_err {
            SchemaRegistryError::UrlError(_) => (),
            _ => panic!("Expected UrlError"),
        }
    }
}
