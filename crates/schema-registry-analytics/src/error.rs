//! Error types for the analytics engine

use thiserror::Error;

/// Result type for analytics operations
pub type Result<T> = std::result::Result<T, AnalyticsError>;

/// Analytics engine errors
#[derive(Debug, Error)]
pub enum AnalyticsError {
    /// Event processing error
    #[error("Event processing error: {0}")]
    EventProcessing(String),

    /// Aggregation error
    #[error("Aggregation error: {0}")]
    Aggregation(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Query error
    #[error("Query error: {0}")]
    Query(String),

    /// Invalid time range
    #[error("Invalid time range: start {start} must be before end {end}")]
    InvalidTimeRange { start: String, end: String },

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Event channel error
    #[error("Event channel error: {0}")]
    ChannelError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Lock error
    #[error("Lock error: unable to acquire lock")]
    LockError,

    /// Event not found
    #[error("Event not found: {0}")]
    NotFound(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AnalyticsError {
    /// Create an event processing error
    pub fn event_processing(msg: impl Into<String>) -> Self {
        Self::EventProcessing(msg.into())
    }

    /// Create an aggregation error
    pub fn aggregation(msg: impl Into<String>) -> Self {
        Self::Aggregation(msg.into())
    }

    /// Create a storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create a query error
    pub fn query(msg: impl Into<String>) -> Self {
        Self::Query(msg.into())
    }

    /// Create an invalid parameter error
    pub fn invalid_parameter(msg: impl Into<String>) -> Self {
        Self::InvalidParameter(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for AnalyticsError {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::ChannelError(err.to_string())
    }
}

impl From<tokio::sync::broadcast::error::SendError<crate::types::SchemaUsageEvent>> for AnalyticsError {
    fn from(err: tokio::sync::broadcast::error::SendError<crate::types::SchemaUsageEvent>) -> Self {
        Self::ChannelError(err.to_string())
    }
}
