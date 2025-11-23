use super::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Schema Management Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterSchemaResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub schema_id: Uuid,

    #[schema(example = "1.2.0")]
    pub version: String,

    #[schema(example = "com.example.user.created")]
    pub subject: String,

    pub created_at: DateTime<Utc>,

    #[schema(example = "sha256:abcdef1234567890...")]
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetSchemaResponse {
    pub schema: SchemaInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListSchemasResponse {
    pub schemas: Vec<SchemaInfo>,
    pub total_count: i32,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateSchemaMetadataResponse {
    pub schema: SchemaInfo,
}

// ============================================================================
// Version Management Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListVersionsResponse {
    #[schema(example = "com.example.user.created")]
    pub subject: String,

    #[schema(example = json!(["1.0.0", "1.1.0", "1.2.0"]))]
    pub versions: Vec<String>,

    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetSubjectVersionsResponse {
    pub subject: String,
    pub versions: Vec<VersionInfo>,
}

// ============================================================================
// Validation Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationReport {
    pub valid: bool,

    #[serde(default)]
    pub errors: Vec<ValidationError>,

    #[serde(default)]
    pub warnings: Vec<ValidationWarning>,

    #[schema(example = 2.5)]
    pub validation_time_ms: f64,

    pub schema_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SchemaValidationReport {
    pub valid: bool,

    #[serde(default)]
    pub errors: Vec<String>,

    #[serde(default)]
    pub warnings: Vec<String>,
}

// ============================================================================
// Compatibility Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompatibilityReport {
    pub compatible: bool,

    pub level: CompatibilityLevel,

    #[serde(default)]
    pub violations: Vec<CompatibilityViolation>,

    #[schema(example = json!(["1.0.0", "1.1.0"]))]
    pub compared_versions: Vec<String>,

    #[schema(example = "Schema is backward compatible")]
    pub message: String,
}

// ============================================================================
// Search Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchSchemasResponse {
    pub schemas: Vec<SchemaInfo>,
    pub total_count: i32,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DependenciesResponse {
    pub dependencies: Vec<DependencyInfo>,
    pub total_count: i32,
}

// ============================================================================
// Subject Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListSubjectsResponse {
    pub subjects: Vec<String>,
    pub total_count: i32,
}

// ============================================================================
// Health Check Response
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthCheckResponse {
    pub status: HealthStatus,
    pub components: HashMap<String, ComponentHealth>,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    pub status: ComponentStatus,
    pub message: Option<String>,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus {
    Up,
    Down,
    Degraded,
}

// ============================================================================
// WebSocket Events
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SchemaChangeEvent {
    pub event_type: EventType,
    pub schema_id: Uuid,
    pub subject: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub changed_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    SchemaRegistered,
    SchemaUpdated,
    SchemaDeleted,
    SchemaDeprecated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        assert_eq!(
            serde_json::to_string(&HealthStatus::Healthy).unwrap(),
            "\"healthy\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Degraded).unwrap(),
            "\"degraded\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Unhealthy).unwrap(),
            "\"unhealthy\""
        );
    }

    #[test]
    fn test_component_status_serialization() {
        assert_eq!(
            serde_json::to_string(&ComponentStatus::Up).unwrap(),
            "\"up\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentStatus::Down).unwrap(),
            "\"down\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentStatus::Degraded).unwrap(),
            "\"degraded\""
        );
    }

    #[test]
    fn test_event_type_serialization() {
        assert_eq!(
            serde_json::to_string(&EventType::SchemaRegistered).unwrap(),
            "\"SCHEMA_REGISTERED\""
        );
        assert_eq!(
            serde_json::to_string(&EventType::SchemaUpdated).unwrap(),
            "\"SCHEMA_UPDATED\""
        );
        assert_eq!(
            serde_json::to_string(&EventType::SchemaDeleted).unwrap(),
            "\"SCHEMA_DELETED\""
        );
        assert_eq!(
            serde_json::to_string(&EventType::SchemaDeprecated).unwrap(),
            "\"SCHEMA_DEPRECATED\""
        );
    }

    #[test]
    fn test_register_schema_response_creation() {
        let response = RegisterSchemaResponse {
            schema_id: Uuid::new_v4(),
            version: "1.0.0".to_string(),
            subject: "test.subject".to_string(),
            created_at: Utc::now(),
            checksum: "sha256:abc123".to_string(),
        };

        assert_eq!(response.version, "1.0.0");
        assert_eq!(response.subject, "test.subject");
        assert!(response.checksum.starts_with("sha256:"));
    }

    #[test]
    fn test_validation_report_valid() {
        let report = ValidationReport {
            valid: true,
            errors: vec![],
            warnings: vec![],
            validation_time_ms: 1.5,
            schema_id: Uuid::new_v4(),
        };

        assert!(report.valid);
        assert!(report.errors.is_empty());
        assert_eq!(report.validation_time_ms, 1.5);
    }

    #[test]
    fn test_validation_report_with_errors() {
        use crate::models::ValidationError;

        let report = ValidationReport {
            valid: false,
            errors: vec![ValidationError {
                path: "/field".to_string(),
                message: "Error".to_string(),
                error_type: "type".to_string(),
            }],
            warnings: vec![],
            validation_time_ms: 2.0,
            schema_id: Uuid::new_v4(),
        };

        assert!(!report.valid);
        assert_eq!(report.errors.len(), 1);
    }

    #[test]
    fn test_compatibility_report_compatible() {
        use crate::models::CompatibilityLevel;

        let report = CompatibilityReport {
            compatible: true,
            level: CompatibilityLevel::Backward,
            violations: vec![],
            compared_versions: vec!["1.0.0".to_string()],
            message: "Compatible".to_string(),
        };

        assert!(report.compatible);
        assert_eq!(report.level, CompatibilityLevel::Backward);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_compatibility_report_with_violations() {
        use crate::models::{CompatibilityLevel, CompatibilityViolation, Severity};

        let report = CompatibilityReport {
            compatible: false,
            level: CompatibilityLevel::Full,
            violations: vec![CompatibilityViolation {
                rule: "field_removed".to_string(),
                path: "/field".to_string(),
                message: "Field removed".to_string(),
                severity: Severity::Error,
            }],
            compared_versions: vec!["1.0.0".to_string(), "1.1.0".to_string()],
            message: "Incompatible".to_string(),
        };

        assert!(!report.compatible);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.compared_versions.len(), 2);
    }

    #[test]
    fn test_list_schemas_response_pagination() {
        use crate::models::SchemaInfo;

        let response = ListSchemasResponse {
            schemas: vec![],
            total_count: 100,
            limit: 50,
            offset: 0,
        };

        assert_eq!(response.total_count, 100);
        assert_eq!(response.limit, 50);
        assert_eq!(response.offset, 0);
    }

    #[test]
    fn test_list_versions_response() {
        let response = ListVersionsResponse {
            subject: "test.subject".to_string(),
            versions: vec!["1.0.0".to_string(), "1.1.0".to_string(), "2.0.0".to_string()],
            total_count: 3,
        };

        assert_eq!(response.subject, "test.subject");
        assert_eq!(response.versions.len(), 3);
        assert_eq!(response.total_count, 3);
    }

    #[test]
    fn test_list_subjects_response() {
        let response = ListSubjectsResponse {
            subjects: vec!["subject1".to_string(), "subject2".to_string()],
            total_count: 2,
        };

        assert_eq!(response.subjects.len(), 2);
        assert_eq!(response.total_count, 2);
    }

    #[test]
    fn test_health_check_response_creation() {
        let mut components = HashMap::new();
        components.insert(
            "database".to_string(),
            ComponentHealth {
                status: ComponentStatus::Up,
                message: Some("OK".to_string()),
                details: HashMap::new(),
            },
        );

        let response = HealthCheckResponse {
            status: HealthStatus::Healthy,
            components,
            version: "1.0.0".to_string(),
            timestamp: Utc::now(),
        };

        assert_eq!(response.status, HealthStatus::Healthy);
        assert_eq!(response.version, "1.0.0");
        assert!(response.components.contains_key("database"));
    }

    #[test]
    fn test_component_health_with_details() {
        let mut details = HashMap::new();
        details.insert("latency_ms".to_string(), "5".to_string());

        let health = ComponentHealth {
            status: ComponentStatus::Up,
            message: Some("Healthy".to_string()),
            details,
        };

        assert_eq!(health.status, ComponentStatus::Up);
        assert!(health.details.contains_key("latency_ms"));
    }

    #[test]
    fn test_schema_change_event_creation() {
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), "value".to_string());

        let event = SchemaChangeEvent {
            event_type: EventType::SchemaRegistered,
            schema_id: Uuid::new_v4(),
            subject: "test.subject".to_string(),
            version: "1.0.0".to_string(),
            timestamp: Utc::now(),
            metadata,
            changed_by: Some("user@example.com".to_string()),
        };

        assert_eq!(event.event_type, EventType::SchemaRegistered);
        assert_eq!(event.version, "1.0.0");
        assert!(event.changed_by.is_some());
    }

    #[test]
    fn test_search_schemas_response_empty() {
        let response = SearchSchemasResponse {
            schemas: vec![],
            total_count: 0,
            limit: 50,
            offset: 0,
        };

        assert!(response.schemas.is_empty());
        assert_eq!(response.total_count, 0);
    }

    #[test]
    fn test_dependencies_response() {
        use crate::models::DependencyInfo;

        let response = DependenciesResponse {
            dependencies: vec![DependencyInfo {
                schema_id: Uuid::new_v4(),
                subject: "dep.subject".to_string(),
                version: "1.0.0".to_string(),
                dependency_type: "reference".to_string(),
                depth: 1,
            }],
            total_count: 1,
        };

        assert_eq!(response.dependencies.len(), 1);
        assert_eq!(response.total_count, 1);
    }
}
