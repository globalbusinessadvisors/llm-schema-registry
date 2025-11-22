//! Schema data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::state::{SchemaLifecycle, SchemaState};
use crate::types::{CompatibilityMode, SerializationFormat};
use crate::versioning::SemanticVersion;

/// Input for registering a new schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInput {
    /// Schema name
    pub name: String,
    /// Namespace for the schema
    pub namespace: String,
    /// Serialization format
    pub format: SerializationFormat,
    /// Raw schema definition content
    pub content: String,
    /// Human-readable description
    pub description: String,
    /// Compatibility mode to enforce
    pub compatibility_mode: CompatibilityMode,
    /// Auto-activate after registration
    pub auto_activate: bool,
    /// Optional version (if not specified, will be auto-calculated)
    pub version: Option<SemanticVersion>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Schema tags for categorization
    pub tags: Vec<String>,
    /// Example instances
    pub examples: Vec<serde_json::Value>,
}

/// Registered schema with full metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredSchema {
    /// Unique schema ID
    pub id: Uuid,
    /// Schema name
    pub name: String,
    /// Namespace
    pub namespace: String,
    /// Version
    pub version: SemanticVersion,
    /// Serialization format
    pub format: SerializationFormat,
    /// Raw schema content
    pub content: String,
    /// SHA-256 hash of content for deduplication
    pub content_hash: String,
    /// Description
    pub description: String,
    /// Compatibility mode
    pub compatibility_mode: CompatibilityMode,
    /// Current lifecycle state
    pub state: SchemaState,
    /// Schema metadata
    pub metadata: SchemaMetadata,
    /// Tags
    pub tags: Vec<String>,
    /// Examples
    pub examples: Vec<serde_json::Value>,
    /// Lifecycle tracker
    pub lifecycle: SchemaLifecycle,
}

/// Schema metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// When the schema was created
    pub created_at: DateTime<Utc>,
    /// Who created the schema
    pub created_by: String,
    /// When the schema was last updated
    pub updated_at: DateTime<Utc>,
    /// Who last updated the schema
    pub updated_by: String,
    /// When the schema was activated (if applicable)
    pub activated_at: Option<DateTime<Utc>>,
    /// Deprecation information
    pub deprecation: Option<DeprecationInfo>,
    /// Deletion information
    pub deletion: Option<DeletionInfo>,
    /// Custom metadata fields
    pub custom: HashMap<String, serde_json::Value>,
}

/// Deprecation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationInfo {
    /// Reason for deprecation
    pub reason: String,
    /// When the schema was deprecated
    pub deprecated_at: DateTime<Utc>,
    /// Who deprecated the schema
    pub deprecated_by: String,
    /// When the schema will be archived/sunset
    pub sunset_date: DateTime<Utc>,
    /// Migration guide URL or text
    pub migration_guide: Option<String>,
    /// Replacement schema reference
    pub replacement_schema: Option<SchemaReference>,
}

/// Deletion information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionInfo {
    /// When the schema was deleted
    pub deleted_at: DateTime<Utc>,
    /// Who deleted the schema
    pub deleted_by: String,
    /// Reason for deletion
    pub reason: String,
}

/// Reference to another schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaReference {
    /// Schema ID
    pub id: Uuid,
    /// Schema version
    pub version: SemanticVersion,
    /// Schema name (for display)
    pub name: String,
}

impl RegisteredSchema {
    /// Calculate the content hash using SHA-256
    pub fn calculate_content_hash(content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Create a schema reference from this schema
    pub fn as_reference(&self) -> SchemaReference {
        SchemaReference {
            id: self.id,
            version: self.version.clone(),
            name: self.name.clone(),
        }
    }

    /// Get the fully qualified name (namespace.name)
    pub fn fully_qualified_name(&self) -> String {
        format!("{}.{}", self.namespace, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash() {
        let content = r#"{"type": "object"}"#;
        let hash = RegisteredSchema::calculate_content_hash(content);
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters

        // Same content should produce same hash
        let hash2 = RegisteredSchema::calculate_content_hash(content);
        assert_eq!(hash, hash2);

        // Different content should produce different hash
        let different_content = r#"{"type": "string"}"#;
        let hash3 = RegisteredSchema::calculate_content_hash(different_content);
        assert_ne!(hash, hash3);
    }
}
