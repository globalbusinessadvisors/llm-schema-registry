//! Storage layer error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Schema not found: {0}")]
    NotFound(String),

    #[error("Schema already exists: {0}")]
    AlreadyExists(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

// Convert from sqlx::Error
impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => StorageError::NotFound("Row not found".to_string()),
            sqlx::Error::Database(db_err) => {
                if let Some(constraint) = db_err.constraint() {
                    if constraint.contains("unique") {
                        return StorageError::AlreadyExists(db_err.message().to_string());
                    }
                }
                StorageError::DatabaseError(db_err.message().to_string())
            }
            sqlx::Error::PoolTimedOut => {
                StorageError::ConnectionError("Connection pool timeout".to_string())
            }
            _ => StorageError::DatabaseError(err.to_string()),
        }
    }
}

// Convert from redis::RedisError
impl From<redis::RedisError> for StorageError {
    fn from(err: redis::RedisError) -> Self {
        StorageError::CacheError(err.to_string())
    }
}

// Convert from serde_json::Error
impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

// Convert from bincode::Error
impl From<bincode::Error> for StorageError {
    fn from(err: bincode::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

// Convert from schema-registry-core::Error
impl From<schema_registry_core::Error> for StorageError {
    fn from(err: schema_registry_core::Error) -> Self {
        StorageError::InternalError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, StorageError>;
