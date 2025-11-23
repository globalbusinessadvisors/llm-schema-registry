//! Error types for schema lineage tracking

use thiserror::Error;
use uuid::Uuid;

/// Result type for lineage operations
pub type Result<T> = std::result::Result<T, LineageError>;

/// Errors that can occur during lineage tracking operations
#[derive(Debug, Error)]
pub enum LineageError {
    /// Schema not found in the lineage graph
    #[error("Schema not found: {0}")]
    SchemaNotFound(Uuid),

    /// Dependency already exists
    #[error("Dependency already exists: {from} -> {to}")]
    DependencyExists { from: String, to: String },

    /// Dependency not found
    #[error("Dependency not found: {from} -> {to}")]
    DependencyNotFound { from: String, to: String },

    /// Circular dependency detected
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Invalid relationship type
    #[error("Invalid relationship type: {0}")]
    InvalidRelationType(String),

    /// Graph operation failed
    #[error("Graph operation failed: {0}")]
    GraphOperationFailed(String),

    /// Export operation failed
    #[error("Export failed: {0}")]
    ExportFailed(String),

    /// Import operation failed
    #[error("Import failed: {0}")]
    ImportFailed(String),

    /// Invalid query filter
    #[error("Invalid query filter: {0}")]
    InvalidFilter(String),

    /// Maximum depth exceeded
    #[error("Maximum traversal depth exceeded: {0}")]
    MaxDepthExceeded(usize),

    /// Entity not found
    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    /// Invalid entity type
    #[error("Invalid entity type: {0}")]
    InvalidEntityType(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Lock poisoned
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<serde_json::Error> for LineageError {
    fn from(err: serde_json::Error) -> Self {
        LineageError::SerializationError(err.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for LineageError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        LineageError::LockPoisoned(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let id = Uuid::new_v4();
        let err = LineageError::SchemaNotFound(id);
        assert!(err.to_string().contains(&id.to_string()));

        let err = LineageError::DependencyExists {
            from: "A".to_string(),
            to: "B".to_string(),
        };
        assert!(err.to_string().contains("A"));
        assert!(err.to_string().contains("B"));
    }

    #[test]
    fn test_result_type() {
        let success: Result<i32> = Ok(42);
        assert!(success.is_ok());

        let failure: Result<i32> = Err(LineageError::Internal("test".to_string()));
        assert!(failure.is_err());
    }
}
