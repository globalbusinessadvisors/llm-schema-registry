//! Error types for schema registry

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    #[error("Invalid schema ID: {0}")]
    InvalidSchemaId(String),

    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
