//! Data models for the LLM Schema Registry SDK.
//!
//! This module contains all the data structures used to interact with the Schema Registry API,
//! including schemas, metadata, validation results, and compatibility information.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported schema formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SchemaFormat {
    /// JSON Schema format
    JsonSchema,
    /// Apache Avro format
    Avro,
    /// Protocol Buffers format
    Protobuf,
}

impl SchemaFormat {
    /// Returns the MIME type for this schema format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            SchemaFormat::JsonSchema => "application/schema+json",
            SchemaFormat::Avro => "application/vnd.apache.avro+json",
            SchemaFormat::Protobuf => "application/protobuf",
        }
    }
}

/// Compatibility checking modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityMode {
    /// New schema can read data written with old schema
    Backward,
    /// Old schema can read data written with new schema
    Forward,
    /// Both backward and forward compatible
    Full,
    /// Backward compatible with all previous versions
    BackwardTransitive,
    /// Forward compatible with all previous versions
    ForwardTransitive,
    /// Full compatibility with all previous versions
    FullTransitive,
    /// No compatibility checking
    None,
}

/// Schema metadata containing administrative information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaMetadata {
    /// Unique schema identifier
    pub schema_id: String,
    /// Schema namespace
    pub namespace: String,
    /// Schema name
    pub name: String,
    /// Schema version (semantic versioning)
    pub version: String,
    /// Schema format
    pub format: SchemaFormat,
    /// Creation timestamp (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Last update timestamp (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Custom metadata tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
}

/// A schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Schema namespace (e.g., "telemetry", "events")
    pub namespace: String,
    /// Schema name (e.g., "InferenceEvent")
    pub name: String,
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    /// Schema format
    pub format: SchemaFormat,
    /// Schema content (JSON string)
    pub content: String,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl Schema {
    /// Creates a new schema with the given parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_schema_registry_sdk::{Schema, SchemaFormat};
    ///
    /// let schema = Schema::new(
    ///     "telemetry",
    ///     "InferenceEvent",
    ///     "1.0.0",
    ///     SchemaFormat::JsonSchema,
    ///     r#"{"type": "object"}"#,
    /// );
    /// ```
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        format: SchemaFormat,
        content: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            version: version.into(),
            format,
            content: content.into(),
            metadata: None,
        }
    }

    /// Sets custom metadata for the schema.
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Returns the fully qualified schema name.
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.namespace, self.name)
    }
}

/// Response from schema registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSchemaResponse {
    /// Unique schema identifier
    pub schema_id: String,
    /// Schema namespace
    pub namespace: String,
    /// Schema name
    pub name: String,
    /// Schema version
    pub version: String,
    /// Whether this is a new schema (true) or existing (false)
    pub created: bool,
}

/// Response from schema retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSchemaResponse {
    /// Schema metadata
    #[serde(flatten)]
    pub metadata: SchemaMetadata,
    /// Schema content
    pub content: String,
}

/// Response from data validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    /// Whether the data is valid
    pub is_valid: bool,
    /// Validation errors (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
}

impl ValidateResponse {
    /// Returns true if validation succeeded.
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Returns validation errors, or an empty vec if none.
    pub fn errors(&self) -> Vec<String> {
        self.errors.clone().unwrap_or_default()
    }
}

/// Response from compatibility checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityResult {
    /// Whether the schemas are compatible
    pub is_compatible: bool,
    /// Compatibility mode used for checking
    pub mode: CompatibilityMode,
    /// Incompatibility details (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<String>>,
}

impl CompatibilityResult {
    /// Returns true if schemas are compatible.
    pub fn is_compatible(&self) -> bool {
        self.is_compatible
    }

    /// Returns compatibility issues, or an empty vec if compatible.
    pub fn issues(&self) -> Vec<String> {
        self.details.clone().unwrap_or_default()
    }
}

/// Request for compatibility checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckCompatibilityRequest {
    /// Schema to check
    pub schema: Schema,
    /// Compatibility mode
    pub mode: CompatibilityMode,
}

/// Schema version information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Version string
    pub version: String,
    /// Schema ID for this version
    pub schema_id: String,
    /// Creation timestamp
    pub created_at: String,
}

/// Response from listing schema versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVersionsResponse {
    /// Namespace
    pub namespace: String,
    /// Name
    pub name: String,
    /// List of versions
    pub versions: Vec<SchemaVersion>,
}

/// Search query for schemas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search query string
    pub query: String,
    /// Optional namespace filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Maximum number of results (default: 10, max: 100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl SearchQuery {
    /// Creates a new search query.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            namespace: None,
            limit: None,
        }
    }

    /// Sets the namespace filter.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Sets the result limit.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit.min(100));
        self
    }
}

/// Search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Schema metadata
    pub metadata: SchemaMetadata,
    /// Relevance score (0.0 - 1.0)
    pub score: f32,
}

/// Response from schema search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search results
    pub results: Vec<SearchResult>,
    /// Total number of results (may be > results.len() if limit applied)
    pub total: u32,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Service status
    pub status: String,
    /// Service version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Additional info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<HashMap<String, String>>,
}

impl HealthCheckResponse {
    /// Returns true if the service is healthy.
    pub fn is_healthy(&self) -> bool {
        self.status.eq_ignore_ascii_case("healthy") || self.status.eq_ignore_ascii_case("ok")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_format_mime_type() {
        assert_eq!(
            SchemaFormat::JsonSchema.mime_type(),
            "application/schema+json"
        );
        assert_eq!(SchemaFormat::Avro.mime_type(), "application/vnd.apache.avro+json");
        assert_eq!(SchemaFormat::Protobuf.mime_type(), "application/protobuf");
    }

    #[test]
    fn test_schema_builder() {
        let schema = Schema::new(
            "telemetry",
            "InferenceEvent",
            "1.0.0",
            SchemaFormat::JsonSchema,
            r#"{"type": "object"}"#,
        );

        assert_eq!(schema.namespace, "telemetry");
        assert_eq!(schema.name, "InferenceEvent");
        assert_eq!(schema.version, "1.0.0");
        assert_eq!(schema.format, SchemaFormat::JsonSchema);
        assert_eq!(schema.full_name(), "telemetry.InferenceEvent");
    }

    #[test]
    fn test_schema_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("owner".to_string(), "team-a".to_string());

        let schema = Schema::new(
            "events",
            "UserAction",
            "2.0.0",
            SchemaFormat::Avro,
            r#"{"type": "record"}"#,
        )
        .with_metadata(metadata.clone());

        assert_eq!(schema.metadata, Some(metadata));
    }

    #[test]
    fn test_validate_response() {
        let valid = ValidateResponse {
            is_valid: true,
            errors: None,
        };
        assert!(valid.is_valid());
        assert!(valid.errors().is_empty());

        let invalid = ValidateResponse {
            is_valid: false,
            errors: Some(vec!["Field 'name' is required".to_string()]),
        };
        assert!(!invalid.is_valid());
        assert_eq!(invalid.errors().len(), 1);
    }

    #[test]
    fn test_compatibility_result() {
        let compatible = CompatibilityResult {
            is_compatible: true,
            mode: CompatibilityMode::Backward,
            details: None,
        };
        assert!(compatible.is_compatible());
        assert!(compatible.issues().is_empty());

        let incompatible = CompatibilityResult {
            is_compatible: false,
            mode: CompatibilityMode::Full,
            details: Some(vec!["Field removed".to_string()]),
        };
        assert!(!incompatible.is_compatible());
        assert_eq!(incompatible.issues().len(), 1);
    }

    #[test]
    fn test_search_query_builder() {
        let query = SearchQuery::new("inference")
            .with_namespace("telemetry")
            .with_limit(50);

        assert_eq!(query.query, "inference");
        assert_eq!(query.namespace, Some("telemetry".to_string()));
        assert_eq!(query.limit, Some(50));
    }

    #[test]
    fn test_search_query_limit_capped() {
        let query = SearchQuery::new("test").with_limit(200);
        assert_eq!(query.limit, Some(100)); // Should be capped at 100
    }

    #[test]
    fn test_health_check_response() {
        let healthy = HealthCheckResponse {
            status: "healthy".to_string(),
            version: Some("0.1.0".to_string()),
            info: None,
        };
        assert!(healthy.is_healthy());

        let unhealthy = HealthCheckResponse {
            status: "degraded".to_string(),
            version: None,
            info: None,
        };
        assert!(!unhealthy.is_healthy());
    }

    #[test]
    fn test_schema_serialization() {
        let schema = Schema::new(
            "test",
            "MySchema",
            "1.0.0",
            SchemaFormat::JsonSchema,
            r#"{"type": "object"}"#,
        );

        let json = serde_json::to_string(&schema).unwrap();
        let deserialized: Schema = serde_json::from_str(&json).unwrap();

        assert_eq!(schema.namespace, deserialized.namespace);
        assert_eq!(schema.name, deserialized.name);
        assert_eq!(schema.version, deserialized.version);
        assert_eq!(schema.format, deserialized.format);
    }
}
