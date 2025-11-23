//! Error types for schema migration

use thiserror::Error;

/// Result type for migration operations
pub type Result<T> = std::result::Result<T, Error>;

/// Migration error types
#[derive(Error, Debug)]
pub enum Error {
    /// Schema parsing error
    #[error("Failed to parse schema: {0}")]
    SchemaParsing(String),

    /// Invalid schema format
    #[error("Invalid schema format: {0}")]
    InvalidFormat(String),

    /// Schema version error
    #[error("Schema version error: {0}")]
    VersionError(String),

    /// Migration generation error
    #[error("Failed to generate migration: {0}")]
    GenerationFailed(String),

    /// Template rendering error
    #[error("Template rendering error: {0}")]
    TemplateError(String),

    /// Validation error
    #[error("Migration validation failed: {0}")]
    ValidationFailed(String),

    /// Incompatible change detected
    #[error("Incompatible change detected: {0}")]
    IncompatibleChange(String),

    /// Missing field error
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Type conversion error
    #[error("Cannot convert type {from} to {to}: {reason}")]
    TypeConversion {
        from: String,
        to: String,
        reason: String,
    },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Avro error
    #[error("Avro error: {0}")]
    Avro(String),

    /// Core registry error
    #[error("Core registry error: {0}")]
    Core(#[from] schema_registry_core::Error),

    /// Unsupported language
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<apache_avro::Error> for Error {
    fn from(err: apache_avro::Error) -> Self {
        Error::Avro(err.to_string())
    }
}

impl From<tera::Error> for Error {
    fn from(err: tera::Error) -> Self {
        Error::TemplateError(err.to_string())
    }
}
