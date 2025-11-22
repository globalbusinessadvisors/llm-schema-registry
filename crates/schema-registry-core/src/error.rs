//! Error types for the schema registry

use thiserror::Error;

/// Result type alias using our Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for schema registry operations
#[derive(Error, Debug)]
pub enum Error {
    /// Schema not found
    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    /// Schema already exists
    #[error("Schema already exists: {0}")]
    SchemaAlreadyExists(String),

    /// Validation error
    #[error("Validation failed: {0}")]
    ValidationError(String),

    /// Compatibility error
    #[error("Compatibility check failed: {0}")]
    CompatibilityError(String),

    /// State transition error
    #[error("Invalid state transition: {0}")]
    StateTransitionError(String),

    /// Registration error
    #[error("Registration failed: {0}")]
    RegistrationError(String),

    /// Deprecation error
    #[error("Deprecation failed: {0}")]
    DeprecationError(String),

    /// Rollback error
    #[error("Rollback failed: {0}")]
    RollbackError(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Event publishing error
    #[error("Event publish error: {0}")]
    EventPublishError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Security error
    #[error("Security error: {0}")]
    SecurityError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Other error
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl Error {
    /// Check if the error is a not found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::SchemaNotFound(_))
    }

    /// Check if the error is a validation error
    pub fn is_validation_error(&self) -> bool {
        matches!(self, Error::ValidationError(_))
    }

    /// Check if the error is a compatibility error
    pub fn is_compatibility_error(&self) -> bool {
        matches!(self, Error::CompatibilityError(_))
    }
}
