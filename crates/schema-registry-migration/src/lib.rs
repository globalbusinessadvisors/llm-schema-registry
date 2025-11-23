//! # Schema Registry Migration
//!
//! Enterprise-grade schema migration code generator for the LLM Schema Registry.
//!
//! This crate provides comprehensive schema migration capabilities including:
//! - Automatic schema difference detection
//! - Migration code generation for 5 languages (Python, TypeScript, Java, Go, SQL)
//! - Migration validation and dry-run testing
//! - Rollback script generation
//! - Risk assessment and performance estimation
//!
//! ## Features
//!
//! - **Multi-Language Support**: Generate migration code in Python, TypeScript, Java, Go, and SQL
//! - **Smart Analysis**: Detect breaking vs. non-breaking changes automatically
//! - **Safe Migrations**: Validate migrations before applying them
//! - **Rollback Support**: Automatic rollback script generation
//! - **Performance Estimation**: Estimate migration time and resource usage
//!
//! ## Example
//!
//! ```rust
//! use schema_registry_migration::{MigrationEngine, Language};
//! use schema_registry_core::{SerializationFormat, versioning::SemanticVersion};
//!
//! // Create migration engine
//! let engine = MigrationEngine::new(SerializationFormat::JsonSchema);
//!
//! // Generate migration
//! let old_schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
//! let new_schema = r#"{"type": "object", "properties": {"name": {"type": "string"}, "age": {"type": "integer"}}}"#;
//!
//! let plan = engine.generate_migration_from_content(
//!     old_schema,
//!     new_schema,
//!     SemanticVersion::new(1, 0, 0),
//!     SemanticVersion::new(2, 0, 0),
//!     "user".to_string(),
//!     "com.example".to_string(),
//!     vec![Language::Python, Language::TypeScript],
//! ).expect("Failed to generate migration");
//!
//! // Validate migration
//! let validation = engine.validate_migration(&plan).expect("Validation failed");
//! assert!(validation.valid);
//! ```

pub mod analyzer;
pub mod engine;
pub mod error;
pub mod generators;
pub mod types;
pub mod validator;

// Re-export commonly used types
pub use analyzer::SchemaAnalyzer;
pub use engine::{MigrationEngine, MigrationEngineBuilder};
pub use error::{Error, Result};
pub use generators::{GoGenerator, JavaGenerator, PythonGenerator, SqlGenerator, TypeScriptGenerator};
pub use types::{
    Constraint, FieldType, GeneratedCode, Language, MigrationContext, MigrationPlan,
    MigrationStrategy, RiskLevel, RollbackPlan, RollbackStrategy, SchemaChange, SchemaDiff,
    ValidationRule, ValidationRuleType,
};
pub use validator::{DryRunReport, MigrationValidator, PerformanceEstimate, ValidationReport};

/// Version of the migration crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    use schema_registry_core::{versioning::SemanticVersion, SerializationFormat};

    #[test]
    fn test_full_migration_workflow() {
        // Create engine
        let engine = MigrationEngine::new(SerializationFormat::JsonSchema);

        // Define schemas
        let old_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            },
            "required": ["name"]
        }"#;

        let new_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"},
                "age": {"type": "integer", "default": 0},
                "active": {"type": "boolean", "default": true}
            },
            "required": ["name", "email"]
        }"#;

        // Generate migration plan
        let plan = engine
            .generate_migration_from_content(
                old_schema,
                new_schema,
                SemanticVersion::new(1, 0, 0),
                SemanticVersion::new(2, 0, 0),
                "user".to_string(),
                "com.example".to_string(),
                vec![
                    Language::Python,
                    Language::TypeScript,
                    Language::Go,
                    Language::Java,
                    Language::Sql,
                ],
            )
            .expect("Failed to generate migration plan");

        // Verify plan
        assert!(!plan.diff.changes.is_empty());
        assert_eq!(plan.code_templates.len(), 5);

        // Validate migration
        let validation = engine
            .validate_migration(&plan)
            .expect("Failed to validate migration");

        assert!(validation.valid);

        // Check code generation
        assert!(plan.code_templates.contains_key(&Language::Python));
        assert!(plan.code_templates.contains_key(&Language::TypeScript));
        assert!(plan.code_templates.contains_key(&Language::Go));
        assert!(plan.code_templates.contains_key(&Language::Java));
        assert!(plan.code_templates.contains_key(&Language::Sql));

        // Verify Python code
        let python_code = &plan.code_templates[&Language::Python];
        assert!(python_code.migration_code.contains("def migrate"));
        assert!(python_code.test_code.is_some());

        // Verify TypeScript code
        let ts_code = &plan.code_templates[&Language::TypeScript];
        assert!(ts_code.migration_code.contains("export function"));

        // Verify rollback plan exists
        assert!(plan.rollback_plan.is_some());
    }

    #[test]
    fn test_breaking_change_detection() {
        let engine = MigrationEngine::new(SerializationFormat::JsonSchema);

        let old_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "old_field": {"type": "string"}
            }
        }"#;

        let new_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }"#;

        let plan = engine
            .generate_migration_from_content(
                old_schema,
                new_schema,
                SemanticVersion::new(1, 0, 0),
                SemanticVersion::new(2, 0, 0),
                "test".to_string(),
                "com.example".to_string(),
                vec![Language::Python],
            )
            .expect("Failed to generate migration");

        // Should detect field removal as breaking change
        assert!(!plan.diff.breaking_changes.is_empty());
        assert!(plan.risk_level >= RiskLevel::Medium);
    }

    #[test]
    fn test_complexity_scoring() {
        let engine = MigrationEngine::new(SerializationFormat::JsonSchema);

        // Simple change - low complexity
        let simple_old = r#"{"type": "object", "properties": {"a": {"type": "string"}}}"#;
        let simple_new =
            r#"{"type": "object", "properties": {"a": {"type": "string"}, "b": {"type": "integer", "default": 0}}}"#;

        let simple_plan = engine
            .generate_migration_from_content(
                simple_old,
                simple_new,
                SemanticVersion::new(1, 0, 0),
                SemanticVersion::new(1, 1, 0),
                "simple".to_string(),
                "com.example".to_string(),
                vec![],
            )
            .expect("Failed to generate migration");

        assert!(simple_plan.diff.complexity_score < 0.5);

        // Complex change - higher complexity
        let complex_old = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "string"},
                "field2": {"type": "integer"},
                "field3": {"type": "boolean"}
            }
        }"#;

        let complex_new = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "integer"},
                "field4": {"type": "string"}
            }
        }"#;

        let complex_plan = engine
            .generate_migration_from_content(
                complex_old,
                complex_new,
                SemanticVersion::new(1, 0, 0),
                SemanticVersion::new(2, 0, 0),
                "complex".to_string(),
                "com.example".to_string(),
                vec![],
            )
            .expect("Failed to generate migration");

        assert!(complex_plan.diff.complexity_score > simple_plan.diff.complexity_score);
    }
}
