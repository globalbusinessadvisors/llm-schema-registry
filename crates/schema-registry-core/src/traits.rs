//! Core traits for the schema registry
//!
//! Defines the main abstractions for storage, validation, compatibility checking,
//! and event publishing.

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;
use crate::events::SchemaEvent;
use crate::schema::{RegisteredSchema, SchemaInput};
use crate::types::CompatibilityMode;
use crate::versioning::SemanticVersion;

/// Trait for schema storage operations
#[async_trait]
pub trait SchemaStorage: Send + Sync {
    /// Store a new schema
    async fn store(&self, schema: RegisteredSchema) -> Result<()>;

    /// Retrieve a schema by ID and optionally version
    async fn retrieve(&self, id: Uuid, version: Option<SemanticVersion>) -> Result<RegisteredSchema>;

    /// Retrieve a schema by content hash
    async fn retrieve_by_hash(&self, content_hash: &str) -> Result<Option<RegisteredSchema>>;

    /// Update an existing schema
    async fn update(&self, schema: RegisteredSchema) -> Result<()>;

    /// Delete a schema (soft delete)
    async fn delete(&self, id: Uuid, version: SemanticVersion) -> Result<()>;

    /// List all versions of a schema
    async fn list_versions(&self, id: Uuid) -> Result<Vec<SemanticVersion>>;

    /// Find schemas by namespace and name
    async fn find_by_name(&self, namespace: &str, name: &str) -> Result<Vec<RegisteredSchema>>;
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the schema is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
    /// Metadata from validation
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Validation error
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationError {
    /// Error message
    pub message: String,
    /// Field path where error occurred
    pub field_path: Option<String>,
    /// Error code
    pub code: String,
}

/// Validation warning
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,
    /// Field path where warning applies
    pub field_path: Option<String>,
}

/// Trait for schema validation
#[async_trait]
pub trait SchemaValidator: Send + Sync {
    /// Validate a schema input
    async fn validate(&self, input: &SchemaInput) -> Result<ValidationResult>;

    /// Validate raw schema content
    async fn validate_content(&self, content: &str, format: crate::types::SerializationFormat) -> Result<ValidationResult>;
}

/// Compatibility check result
#[derive(Debug, Clone)]
pub struct CompatibilityResult {
    /// Whether schemas are compatible
    pub is_compatible: bool,
    /// Compatibility mode that was checked
    pub mode: CompatibilityMode,
    /// List of compatibility violations
    pub violations: Vec<CompatibilityViolation>,
    /// Versions that were checked
    pub checked_versions: Vec<SemanticVersion>,
}

/// Compatibility violation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompatibilityViolation {
    /// Type of violation
    pub violation_type: crate::types::ViolationType,
    /// Field path where violation occurred
    pub field_path: String,
    /// Old value
    pub old_value: Option<serde_json::Value>,
    /// New value
    pub new_value: Option<serde_json::Value>,
    /// Severity
    pub severity: crate::types::ViolationSeverity,
    /// Description of the violation
    pub description: String,
}

/// Trait for compatibility checking
#[async_trait]
pub trait CompatibilityChecker: Send + Sync {
    /// Check compatibility between two schemas
    async fn check_compatibility(
        &self,
        new_schema: &RegisteredSchema,
        old_schema: &RegisteredSchema,
        mode: CompatibilityMode,
    ) -> Result<CompatibilityResult>;

    /// Check transitive compatibility
    async fn check_transitive_compatibility(
        &self,
        new_schema: &RegisteredSchema,
        previous_versions: &[RegisteredSchema],
        mode: CompatibilityMode,
    ) -> Result<CompatibilityResult>;
}

/// Trait for event publishing
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a single event
    async fn publish(&self, event: SchemaEvent) -> Result<()>;

    /// Publish multiple events in a batch
    async fn publish_batch(&self, events: Vec<SchemaEvent>) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result() {
        let result = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            metadata: std::collections::HashMap::new(),
        };
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_compatibility_result() {
        let result = CompatibilityResult {
            is_compatible: true,
            mode: CompatibilityMode::Backward,
            violations: vec![],
            checked_versions: vec![],
        };
        assert!(result.is_compatible);
        assert!(result.violations.is_empty());
    }
}
