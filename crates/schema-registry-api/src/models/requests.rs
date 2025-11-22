use super::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Schema Management Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterSchemaRequest {
    #[schema(example = "com.example.user.created")]
    pub subject: String,

    #[schema(example = json!({
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "email": {"type": "string", "format": "email"}
        },
        "required": ["id", "email"]
    }))]
    pub schema: serde_json::Value,

    pub schema_type: SchemaType,

    #[serde(default)]
    pub metadata: HashMap<String, String>,

    pub compatibility_level: Option<CompatibilityLevel>,

    #[schema(example = "User creation event schema")]
    pub description: Option<String>,

    #[serde(default)]
    #[schema(example = json!(["user", "events"]))]
    pub tags: Vec<String>,

    #[serde(default = "default_auto_version")]
    pub auto_version: bool,
}

fn default_auto_version() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateSchemaMetadataRequest {
    #[schema(example = "Updated user schema description")]
    pub description: Option<String>,

    #[schema(example = json!(["user", "v2", "production"]))]
    pub tags: Option<Vec<String>>,

    pub metadata: Option<HashMap<String, String>>,

    pub state: Option<SchemaState>,
}

// ============================================================================
// Validation Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidateDataRequest {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub schema_id: Uuid,

    #[schema(example = json!({"id": "123", "email": "user@example.com"}))]
    pub data: serde_json::Value,

    #[serde(default)]
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidateSchemaRequest {
    pub schema: serde_json::Value,
    pub schema_type: SchemaType,
}

// ============================================================================
// Compatibility Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompatibilityCheckRequest {
    #[schema(example = "com.example.user.created")]
    pub subject: String,

    #[schema(example = json!({
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "email": {"type": "string", "format": "email"},
            "name": {"type": "string"}
        },
        "required": ["id", "email"]
    }))]
    pub new_schema: serde_json::Value,

    pub level: CompatibilityLevel,

    #[schema(example = "1.0.0")]
    pub compare_version: Option<String>,
}

// ============================================================================
// Search Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchSchemasRequest {
    #[schema(example = "user")]
    pub query: Option<String>,

    #[schema(example = "com.example.*")]
    pub subject_pattern: Option<String>,

    pub schema_type: Option<SchemaType>,

    #[schema(example = json!(["user", "events"]))]
    pub tags: Option<Vec<String>>,

    pub metadata_filters: Option<HashMap<String, String>>,

    #[serde(flatten)]
    pub pagination: PaginationParams,
}

// ============================================================================
// Subject Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListSubjectsRequest {
    #[schema(example = "com.example")]
    pub prefix: Option<String>,

    #[serde(flatten)]
    pub pagination: PaginationParams,
}

// ============================================================================
// WebSocket Subscription Request
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubscribeRequest {
    #[schema(example = json!(["com.example.user.*", "com.example.order.*"]))]
    pub subjects: Vec<String>,

    #[schema(example = json!(["SCHEMA_REGISTERED", "SCHEMA_UPDATED"]))]
    pub event_types: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_schema_request_serialization() {
        let request = RegisterSchemaRequest {
            subject: "com.example.test".to_string(),
            schema: serde_json::json!({"type": "object"}),
            schema_type: SchemaType::Json,
            metadata: HashMap::new(),
            compatibility_level: Some(CompatibilityLevel::Backward),
            description: Some("Test schema".to_string()),
            tags: vec!["test".to_string()],
            auto_version: true,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("com.example.test"));
        assert!(json.contains("auto_version"));
    }

    #[test]
    fn test_register_schema_request_default_auto_version() {
        let json = r#"{
            "subject": "test",
            "schema": {"type": "object"},
            "schema_type": "json",
            "metadata": {}
        }"#;

        let request: RegisterSchemaRequest = serde_json::from_str(json).unwrap();
        assert!(request.auto_version);
    }

    #[test]
    fn test_update_schema_metadata_request_partial_update() {
        let request = UpdateSchemaMetadataRequest {
            description: Some("Updated description".to_string()),
            tags: None,
            metadata: None,
            state: Some(SchemaState::Active),
        };

        assert!(request.description.is_some());
        assert!(request.tags.is_none());
        assert_eq!(request.state, Some(SchemaState::Active));
    }

    #[test]
    fn test_validate_data_request_strict_mode() {
        let request = ValidateDataRequest {
            schema_id: Uuid::new_v4(),
            data: serde_json::json!({"test": "data"}),
            strict: true,
        };

        assert!(request.strict);
    }

    #[test]
    fn test_validate_data_request_default_not_strict() {
        let json = r#"{
            "schema_id": "550e8400-e29b-41d4-a716-446655440000",
            "data": {"test": "data"}
        }"#;

        let request: ValidateDataRequest = serde_json::from_str(json).unwrap();
        assert!(!request.strict);
    }

    #[test]
    fn test_validate_schema_request_creation() {
        let request = ValidateSchemaRequest {
            schema: serde_json::json!({"type": "string"}),
            schema_type: SchemaType::Json,
        };

        assert_eq!(request.schema_type, SchemaType::Json);
    }

    #[test]
    fn test_compatibility_check_request_with_version() {
        let request = CompatibilityCheckRequest {
            subject: "test.subject".to_string(),
            new_schema: serde_json::json!({"type": "object"}),
            level: CompatibilityLevel::Backward,
            compare_version: Some("1.0.0".to_string()),
        };

        assert_eq!(request.compare_version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_compatibility_check_request_without_version() {
        let request = CompatibilityCheckRequest {
            subject: "test.subject".to_string(),
            new_schema: serde_json::json!({"type": "object"}),
            level: CompatibilityLevel::Full,
            compare_version: None,
        };

        assert!(request.compare_version.is_none());
    }

    #[test]
    fn test_search_schemas_request_all_filters() {
        let mut metadata_filters = HashMap::new();
        metadata_filters.insert("env".to_string(), "prod".to_string());

        let request = SearchSchemasRequest {
            query: Some("user".to_string()),
            subject_pattern: Some("com.example.*".to_string()),
            schema_type: Some(SchemaType::Avro),
            tags: Some(vec!["important".to_string()]),
            metadata_filters: Some(metadata_filters),
            pagination: PaginationParams::default(),
        };

        assert_eq!(request.query, Some("user".to_string()));
        assert!(request.metadata_filters.is_some());
    }

    #[test]
    fn test_search_schemas_request_minimal() {
        let request = SearchSchemasRequest {
            query: None,
            subject_pattern: None,
            schema_type: None,
            tags: None,
            metadata_filters: None,
            pagination: PaginationParams::default(),
        };

        assert!(request.query.is_none());
        assert!(request.subject_pattern.is_none());
    }

    #[test]
    fn test_list_subjects_request_with_prefix() {
        let request = ListSubjectsRequest {
            prefix: Some("com.example".to_string()),
            pagination: PaginationParams {
                limit: 100,
                offset: 0,
            },
        };

        assert_eq!(request.prefix, Some("com.example".to_string()));
        assert_eq!(request.pagination.limit, 100);
    }

    #[test]
    fn test_subscribe_request_multiple_patterns() {
        let request = SubscribeRequest {
            subjects: vec![
                "com.example.user.*".to_string(),
                "com.example.order.*".to_string(),
            ],
            event_types: vec![
                "SCHEMA_REGISTERED".to_string(),
                "SCHEMA_UPDATED".to_string(),
            ],
        };

        assert_eq!(request.subjects.len(), 2);
        assert_eq!(request.event_types.len(), 2);
    }

    #[test]
    fn test_subscribe_request_serialization() {
        let request = SubscribeRequest {
            subjects: vec!["test.*".to_string()],
            event_types: vec!["SCHEMA_REGISTERED".to_string()],
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json["subjects"].is_array());
        assert!(json["event_types"].is_array());
    }
}
