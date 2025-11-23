use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

// Re-export common types
pub mod requests;
pub mod responses;
pub mod errors;

pub use requests::*;
pub use responses::*;
pub use errors::*;

// ============================================================================
// Core Domain Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Json,
    Avro,
    Protobuf,
    Thrift,
}

impl SchemaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SchemaType::Json => "json",
            SchemaType::Avro => "avro",
            SchemaType::Protobuf => "protobuf",
            SchemaType::Thrift => "thrift",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityLevel {
    Backward,
    Forward,
    Full,
    BackwardTransitive,
    ForwardTransitive,
    FullTransitive,
    None,
}

impl CompatibilityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompatibilityLevel::Backward => "BACKWARD",
            CompatibilityLevel::Forward => "FORWARD",
            CompatibilityLevel::Full => "FULL",
            CompatibilityLevel::BackwardTransitive => "BACKWARD_TRANSITIVE",
            CompatibilityLevel::ForwardTransitive => "FORWARD_TRANSITIVE",
            CompatibilityLevel::FullTransitive => "FULL_TRANSITIVE",
            CompatibilityLevel::None => "NONE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SchemaState {
    Draft,
    Active,
    Deprecated,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SchemaMetadata {
    #[schema(example = "User authentication schema")]
    pub description: Option<String>,

    #[schema(example = json!(["authentication", "user", "v2"]))]
    pub tags: Vec<String>,

    #[schema(example = "user@example.com")]
    pub owner: String,

    pub compatibility_level: CompatibilityLevel,

    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SchemaInfo {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,

    #[schema(example = "com.example.user.created")]
    pub subject: String,

    #[schema(example = "1.2.0")]
    pub version: String,

    pub schema_type: SchemaType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_content: Option<serde_json::Value>,

    pub metadata: SchemaMetadata,

    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,

    pub state: SchemaState,

    #[schema(example = "sha256:abcdef1234567890...")]
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationError {
    #[schema(example = "/properties/email")]
    pub path: String,

    #[schema(example = "Invalid email format")]
    pub message: String,

    #[schema(example = "format")]
    pub error_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationWarning {
    #[schema(example = "/properties/age")]
    pub path: String,

    #[schema(example = "Field is deprecated")]
    pub message: String,

    #[schema(example = "deprecation")]
    pub warning_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompatibilityViolation {
    #[schema(example = "field_removed")]
    pub rule: String,

    #[schema(example = "/properties/username")]
    pub path: String,

    #[schema(example = "Field 'username' was removed, breaking backward compatibility")]
    pub message: String,

    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DependencyInfo {
    pub schema_id: Uuid,
    pub subject: String,
    pub version: String,

    #[schema(example = "reference")]
    pub dependency_type: String,

    #[schema(example = 1)]
    pub depth: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    pub version: String,
    pub schema_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub state: SchemaState,
}

// ============================================================================
// Pagination
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationParams {
    #[schema(example = 50, minimum = 1, maximum = 1000)]
    #[serde(default = "default_limit")]
    pub limit: i32,

    #[schema(example = 0, minimum = 0)]
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    50
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            offset: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_type_as_str() {
        assert_eq!(SchemaType::Json.as_str(), "json");
        assert_eq!(SchemaType::Avro.as_str(), "avro");
        assert_eq!(SchemaType::Protobuf.as_str(), "protobuf");
        assert_eq!(SchemaType::Thrift.as_str(), "thrift");
    }

    #[test]
    fn test_schema_type_serialization() {
        let json = serde_json::to_string(&SchemaType::Json).unwrap();
        assert_eq!(json, "\"json\"");

        let avro = serde_json::to_string(&SchemaType::Avro).unwrap();
        assert_eq!(avro, "\"avro\"");
    }

    #[test]
    fn test_schema_type_deserialization() {
        let json: SchemaType = serde_json::from_str("\"json\"").unwrap();
        assert_eq!(json, SchemaType::Json);

        let avro: SchemaType = serde_json::from_str("\"avro\"").unwrap();
        assert_eq!(avro, SchemaType::Avro);
    }

    #[test]
    fn test_compatibility_level_as_str() {
        assert_eq!(CompatibilityLevel::Backward.as_str(), "BACKWARD");
        assert_eq!(CompatibilityLevel::Forward.as_str(), "FORWARD");
        assert_eq!(CompatibilityLevel::Full.as_str(), "FULL");
        assert_eq!(
            CompatibilityLevel::BackwardTransitive.as_str(),
            "BACKWARD_TRANSITIVE"
        );
        assert_eq!(
            CompatibilityLevel::ForwardTransitive.as_str(),
            "FORWARD_TRANSITIVE"
        );
        assert_eq!(
            CompatibilityLevel::FullTransitive.as_str(),
            "FULL_TRANSITIVE"
        );
        assert_eq!(CompatibilityLevel::None.as_str(), "NONE");
    }

    #[test]
    fn test_compatibility_level_serialization() {
        let backward = serde_json::to_string(&CompatibilityLevel::Backward).unwrap();
        assert_eq!(backward, "\"BACKWARD\"");

        let full_transitive = serde_json::to_string(&CompatibilityLevel::FullTransitive).unwrap();
        assert_eq!(full_transitive, "\"FULL_TRANSITIVE\"");
    }

    #[test]
    fn test_schema_state_serialization() {
        assert_eq!(serde_json::to_string(&SchemaState::Draft).unwrap(), "\"DRAFT\"");
        assert_eq!(
            serde_json::to_string(&SchemaState::Active).unwrap(),
            "\"ACTIVE\""
        );
        assert_eq!(
            serde_json::to_string(&SchemaState::Deprecated).unwrap(),
            "\"DEPRECATED\""
        );
        assert_eq!(
            serde_json::to_string(&SchemaState::Archived).unwrap(),
            "\"ARCHIVED\""
        );
    }

    #[test]
    fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.limit, 50);
        assert_eq!(params.offset, 0);
    }

    #[test]
    fn test_pagination_params_custom_values() {
        let params = PaginationParams {
            limit: 100,
            offset: 200,
        };
        assert_eq!(params.limit, 100);
        assert_eq!(params.offset, 200);
    }

    #[test]
    fn test_pagination_params_serialization() {
        let params = PaginationParams {
            limit: 25,
            offset: 50,
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"limit\":25"));
        assert!(json.contains("\"offset\":50"));
    }

    #[test]
    fn test_severity_serialization() {
        assert_eq!(serde_json::to_string(&Severity::Error).unwrap(), "\"ERROR\"");
        assert_eq!(
            serde_json::to_string(&Severity::Warning).unwrap(),
            "\"WARNING\""
        );
        assert_eq!(serde_json::to_string(&Severity::Info).unwrap(), "\"INFO\"");
    }

    #[test]
    fn test_validation_error_creation() {
        let error = ValidationError {
            path: "/properties/email".to_string(),
            message: "Invalid format".to_string(),
            error_type: "format".to_string(),
        };
        assert_eq!(error.path, "/properties/email");
        assert_eq!(error.message, "Invalid format");
        assert_eq!(error.error_type, "format");
    }

    #[test]
    fn test_validation_warning_creation() {
        let warning = ValidationWarning {
            path: "/properties/age".to_string(),
            message: "Deprecated field".to_string(),
            warning_type: "deprecation".to_string(),
        };
        assert_eq!(warning.path, "/properties/age");
        assert_eq!(warning.message, "Deprecated field");
    }

    #[test]
    fn test_compatibility_violation_creation() {
        let violation = CompatibilityViolation {
            rule: "field_removed".to_string(),
            path: "/properties/username".to_string(),
            message: "Field removed".to_string(),
            severity: Severity::Error,
        };
        assert_eq!(violation.rule, "field_removed");
        assert_eq!(violation.severity, Severity::Error);
    }

    #[test]
    fn test_dependency_info_depth() {
        let dep = DependencyInfo {
            schema_id: Uuid::new_v4(),
            subject: "test.subject".to_string(),
            version: "1.0.0".to_string(),
            dependency_type: "reference".to_string(),
            depth: 2,
        };
        assert_eq!(dep.depth, 2);
        assert_eq!(dep.dependency_type, "reference");
    }

    #[test]
    fn test_schema_metadata_with_custom_fields() {
        let mut custom = HashMap::new();
        custom.insert(
            "custom_field".to_string(),
            serde_json::json!("custom_value"),
        );

        let metadata = SchemaMetadata {
            description: Some("Test schema".to_string()),
            tags: vec!["test".to_string()],
            owner: "test@example.com".to_string(),
            compatibility_level: CompatibilityLevel::Backward,
            custom,
        };

        assert_eq!(metadata.owner, "test@example.com");
        assert_eq!(metadata.tags.len(), 1);
        assert!(metadata.custom.contains_key("custom_field"));
    }

    #[test]
    fn test_schema_type_equality() {
        assert_eq!(SchemaType::Json, SchemaType::Json);
        assert_ne!(SchemaType::Json, SchemaType::Avro);
    }

    #[test]
    fn test_compatibility_level_equality() {
        assert_eq!(CompatibilityLevel::Backward, CompatibilityLevel::Backward);
        assert_ne!(CompatibilityLevel::Backward, CompatibilityLevel::Forward);
    }

    #[test]
    fn test_schema_state_equality() {
        assert_eq!(SchemaState::Active, SchemaState::Active);
        assert_ne!(SchemaState::Active, SchemaState::Draft);
    }
}
