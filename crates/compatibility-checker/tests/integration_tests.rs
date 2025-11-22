//! Integration tests for compatibility checker

use llm_schema_registry_compatibility::*;
use chrono::Utc;
use uuid::Uuid;

fn create_test_schema(
    name: &str,
    version: &str,
    format: SchemaFormat,
    content: &str,
) -> types::Schema {
    types::Schema {
        id: Uuid::new_v4(),
        name: name.to_string(),
        namespace: "test".to_string(),
        version: types::SemanticVersion::parse(version).unwrap(),
        format,
        content: content.to_string(),
        content_hash: types::Schema::calculate_hash(content),
        description: "Test schema".to_string(),
        compatibility_mode: types::CompatibilityMode::Backward,
        created_at: Utc::now(),
        metadata: Default::default(),
    }
}

#[tokio::test]
async fn test_backward_compatibility_json_schema() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

    // Old schema
    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            },
            "required": ["name", "email"]
        }"#,
    );

    // New schema - added optional field
    let new_schema_compatible = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name", "email"]
        }"#,
    );

    let result = checker
        .check_compatibility(&new_schema_compatible, &old_schema, types::CompatibilityMode::Backward)
        .await
        .unwrap();

    assert!(result.is_compatible, "Adding optional field should be backward compatible");
    assert_eq!(result.violations.len(), 0);

    // New schema - removed required field (breaking)
    let new_schema_breaking = create_test_schema(
        "User",
        "2.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        }"#,
    );

    let result = checker
        .check_compatibility(&new_schema_breaking, &old_schema, types::CompatibilityMode::Backward)
        .await
        .unwrap();

    assert!(!result.is_compatible, "Removing required field should break backward compatibility");
    assert!(result.violations.len() > 0);
    assert_eq!(result.violations[0].violation_type, ViolationType::FieldRemoved);
}

#[tokio::test]
async fn test_forward_compatibility_json_schema() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            }
        }"#,
    );

    // New schema removes a field
    let new_schema = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }"#,
    );

    let result = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::Forward)
        .await
        .unwrap();

    // Forward compatibility: old schema reading new data
    // Removing a field is OK for forward compatibility
    assert!(result.is_compatible);
}

#[tokio::test]
async fn test_full_compatibility() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }"#,
    );

    // Full compatibility requires both backward and forward
    let new_schema = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string", "default": ""}
            }
        }"#,
    );

    let result = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::Full)
        .await
        .unwrap();

    assert!(result.is_compatible, "Adding optional field with default should be fully compatible");
}

#[tokio::test]
async fn test_none_compatibility_mode() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{"type": "object"}"#,
    );

    let new_schema = create_test_schema(
        "User",
        "2.0.0",
        SchemaFormat::JsonSchema,
        r#"{"type": "array"}"#, // Completely different
    );

    let result = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::None)
        .await
        .unwrap();

    assert!(result.is_compatible, "NONE mode should always pass");
}

#[tokio::test]
async fn test_avro_backward_compatibility() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::Avro,
        r#"{
            "type": "record",
            "name": "User",
            "fields": [
                {"name": "name", "type": "string"}
            ]
        }"#,
    );

    let new_schema = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::Avro,
        r#"{
            "type": "record",
            "name": "User",
            "fields": [
                {"name": "name", "type": "string"},
                {"name": "email", "type": "string", "default": ""}
            ]
        }"#,
    );

    let result = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::Backward)
        .await
        .unwrap();

    assert!(result.is_compatible);
}

#[tokio::test]
async fn test_protobuf_compatibility() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::Protobuf,
        r#"
            message User {
                optional string name = 1;
            }
        "#,
    );

    let new_schema = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::Protobuf,
        r#"
            message User {
                optional string name = 1;
                optional string email = 2;
            }
        "#,
    );

    let result = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::Backward)
        .await
        .unwrap();

    assert!(result.is_compatible, "Adding optional field should be compatible");
}

#[tokio::test]
async fn test_cache_functionality() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig {
        enable_cache: true,
        ..Default::default()
    });

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#,
    );

    let new_schema = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::JsonSchema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#,
    );

    // First check - should miss cache
    let result1 = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::Backward)
        .await
        .unwrap();

    // Second check - should hit cache
    let result2 = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::Backward)
        .await
        .unwrap();

    assert_eq!(result1.is_compatible, result2.is_compatible);

    // Verify cache stats
    if let Some((hits, misses, hit_rate)) = checker.cache_stats() {
        assert_eq!(hits, 1, "Should have 1 cache hit");
        assert_eq!(misses, 1, "Should have 1 cache miss");
        assert_eq!(hit_rate, 0.5);
    }
}

#[tokio::test]
async fn test_type_changes() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "age": {"type": "string"}
            }
        }"#,
    );

    let new_schema = create_test_schema(
        "User",
        "2.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "age": {"type": "integer"}
            }
        }"#,
    );

    let result = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::Backward)
        .await
        .unwrap();

    assert!(!result.is_compatible, "Type change from string to integer should be breaking");
    assert!(result.violations.iter().any(|v| v.violation_type == ViolationType::TypeChanged));
}

#[tokio::test]
async fn test_required_field_addition() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }"#,
    );

    let new_schema = create_test_schema(
        "User",
        "2.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            },
            "required": ["email"]
        }"#,
    );

    let result = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::Backward)
        .await
        .unwrap();

    assert!(!result.is_compatible, "Adding required field should be breaking");
    assert!(result.violations.iter().any(|v| v.violation_type == ViolationType::RequiredAdded));
}

#[tokio::test]
async fn test_format_mismatch() {
    let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{"type": "object"}"#,
    );

    let new_schema = create_test_schema(
        "User",
        "2.0.0",
        SchemaFormat::Avro,
        r#"{"type": "record", "name": "User", "fields": []}"#,
    );

    let result = checker
        .check_compatibility(&new_schema, &old_schema, types::CompatibilityMode::Backward)
        .await
        .unwrap();

    assert!(!result.is_compatible, "Format change should be breaking");
    assert_eq!(result.violations[0].violation_type, ViolationType::FormatChanged);
}

#[test]
fn test_semantic_version_parsing() {
    let v = types::SemanticVersion::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);

    let v_pre = types::SemanticVersion::parse("1.2.3-alpha.1").unwrap();
    assert_eq!(v_pre.prerelease, Some("alpha.1".to_string()));

    let v_build = types::SemanticVersion::parse("1.2.3+build.123").unwrap();
    assert_eq!(v_build.build_metadata, Some("build.123".to_string()));
}

#[test]
fn test_semantic_version_comparison() {
    let v1 = types::SemanticVersion::new(1, 0, 0);
    let v2 = types::SemanticVersion::new(1, 1, 0);
    let v3 = types::SemanticVersion::new(2, 0, 0);

    assert!(v1 < v2);
    assert!(v2 < v3);
    assert!(v1 < v3);
}
