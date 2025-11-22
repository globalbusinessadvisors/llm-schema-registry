//! Schema representation and core types

use crate::error::{Error, Result};
use crate::types::{CompatibilityLevel, SchemaState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaId(Uuid);

impl SchemaId {
    /// Create a new random schema ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a schema ID from a string
    pub fn parse(s: &str) -> Result<Self> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| Error::InvalidSchemaId(e.to_string()))
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for SchemaId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SchemaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Schema type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SchemaType {
    Json,
    Avro,
    Protobuf,
    Thrift,
}

impl SchemaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Avro => "AVRO",
            Self::Protobuf => "PROTOBUF",
            Self::Thrift => "THRIFT",
        }
    }
}

impl std::fmt::Display for SchemaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Schema version with semantic versioning
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerelease: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
}

impl SchemaVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
            build: None,
        }
    }

    pub fn to_string(&self) -> String {
        let mut version = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if let Some(pre) = &self.prerelease {
            version.push_str(&format!("-{}", pre));
        }
        if let Some(build) = &self.build {
            version.push_str(&format!("+{}", build));
        }
        version
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Schema content - can be JSON, Avro, Protobuf, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum SchemaContent {
    Json(serde_json::Value),
    Avro(String),
    Protobuf(Vec<u8>),
    Thrift(String),
}

impl SchemaContent {
    pub fn schema_type(&self) -> SchemaType {
        match self {
            Self::Json(_) => SchemaType::Json,
            Self::Avro(_) => SchemaType::Avro,
            Self::Protobuf(_) => SchemaType::Protobuf,
            Self::Thrift(_) => SchemaType::Thrift,
        }
    }

    pub fn as_json(&self) -> Result<&serde_json::Value> {
        match self {
            Self::Json(v) => Ok(v),
            _ => Err(Error::ValidationError("Schema is not JSON type".to_string())),
        }
    }

    /// Estimate size in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            Self::Json(v) => serde_json::to_string(v).map(|s| s.len()).unwrap_or(0),
            Self::Avro(s) | Self::Thrift(s) => s.len(),
            Self::Protobuf(b) => b.len(),
        }
    }
}

/// Schema metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    #[serde(default)]
    pub tags: Vec<String>,
    
    pub owner: String,
    
    #[serde(default)]
    pub compatibility_level: CompatibilityLevel,
    
    #[serde(default)]
    pub state: SchemaState,
    
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for SchemaMetadata {
    fn default() -> Self {
        Self {
            description: None,
            tags: Vec::new(),
            owner: "system".to_string(),
            compatibility_level: CompatibilityLevel::default(),
            state: SchemaState::default(),
            custom: HashMap::new(),
        }
    }
}

/// Complete schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub id: SchemaId,
    pub subject: String,
    pub version: SchemaVersion,
    pub schema_type: SchemaType,
    pub content: SchemaContent,
    pub metadata: SchemaMetadata,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Schema {
    pub fn new(
        subject: String,
        version: SchemaVersion,
        content: SchemaContent,
        metadata: SchemaMetadata,
    ) -> Self {
        let schema_type = content.schema_type();
        Self {
            id: SchemaId::new(),
            subject,
            version,
            schema_type,
            content,
            metadata,
            created_at: Utc::now(),
            created_by: None,
            deleted_at: None,
        }
    }

    /// Check if schema is deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some() || self.metadata.state == SchemaState::Deleted
    }

    /// Check if schema is active
    pub fn is_active(&self) -> bool {
        !self.is_deleted() && self.metadata.state == SchemaState::Active
    }

    /// Soft delete the schema
    pub fn soft_delete(&mut self) {
        self.deleted_at = Some(Utc::now());
        self.metadata.state = SchemaState::Deleted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_id_creation() {
        let id1 = SchemaId::new();
        let id2 = SchemaId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_schema_id_parse() {
        let id = SchemaId::new();
        let id_str = id.to_string();
        let parsed = SchemaId::parse(&id_str).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_schema_version_display() {
        let version = SchemaVersion::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn test_schema_content_type() {
        let content = SchemaContent::Json(serde_json::json!({"type": "object"}));
        assert_eq!(content.schema_type(), SchemaType::Json);
    }
}
