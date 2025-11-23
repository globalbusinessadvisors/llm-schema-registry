//! Core types for schema migration

use chrono::{DateTime, Utc};
use schema_registry_core::versioning::SemanticVersion;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete schema difference analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiff {
    /// Old schema version
    pub old_version: SemanticVersion,
    /// New schema version
    pub new_version: SemanticVersion,
    /// Schema name
    pub schema_name: String,
    /// Schema namespace
    pub namespace: String,
    /// List of all detected changes
    pub changes: Vec<SchemaChange>,
    /// List of breaking changes only
    pub breaking_changes: Vec<BreakingChange>,
    /// Complexity score (0.0 = trivial, 1.0 = extremely complex)
    pub complexity_score: f64,
    /// When this diff was created
    pub created_at: DateTime<Utc>,
}

/// A single schema change
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SchemaChange {
    /// Field added to schema
    FieldAdded {
        /// Name of the added field
        name: String,
        /// Type of the field
        field_type: FieldType,
        /// Default value for existing data
        default: Option<serde_json::Value>,
        /// Whether the field is required
        required: bool,
        /// Field description
        description: Option<String>,
    },
    /// Field removed from schema
    FieldRemoved {
        /// Name of the removed field
        name: String,
        /// Type of the field that was removed
        field_type: FieldType,
        /// Whether to preserve the data
        preserve_data: bool,
    },
    /// Field renamed
    FieldRenamed {
        /// Old field name
        old_name: String,
        /// New field name
        new_name: String,
        /// Field type
        field_type: FieldType,
    },
    /// Field type changed
    TypeChanged {
        /// Field name
        field: String,
        /// Old type
        old_type: FieldType,
        /// New type
        new_type: FieldType,
        /// Conversion function name
        converter: Option<String>,
    },
    /// Nested structure changed
    NestedChanged {
        /// Path to nested field
        path: String,
        /// Changes within nested structure
        changes: Vec<Box<SchemaChange>>,
    },
    /// Array element type changed
    ArrayElementChanged {
        /// Field name
        field: String,
        /// Old element type
        old_element_type: FieldType,
        /// New element type
        new_element_type: FieldType,
    },
    /// Map value type changed
    MapValueChanged {
        /// Field name
        field: String,
        /// Old value type
        old_value_type: FieldType,
        /// New value type
        new_value_type: FieldType,
    },
    /// Constraint added
    ConstraintAdded {
        /// Field name
        field: String,
        /// Constraint type
        constraint: Constraint,
    },
    /// Constraint removed
    ConstraintRemoved {
        /// Field name
        field: String,
        /// Constraint type
        constraint: Constraint,
    },
    /// Enum values changed
    EnumChanged {
        /// Field name
        field: String,
        /// Values added
        added: Vec<String>,
        /// Values removed
        removed: Vec<String>,
    },
}

/// Field type representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldType {
    /// String type
    String,
    /// Integer type
    Integer,
    /// Long/i64 type
    Long,
    /// Float type
    Float,
    /// Double type
    Double,
    /// Boolean type
    Boolean,
    /// Bytes/binary type
    Bytes,
    /// Null type
    Null,
    /// Array type with element type
    Array(Box<FieldType>),
    /// Map type with value type
    Map(Box<FieldType>),
    /// Record/object type with field definitions
    Record {
        /// Record name
        name: String,
        /// Record fields
        fields: Vec<RecordField>,
    },
    /// Enum type with possible values
    Enum {
        /// Enum name
        name: String,
        /// Possible values
        symbols: Vec<String>,
    },
    /// Union type with multiple possibilities
    Union(Vec<FieldType>),
    /// Custom/unknown type
    Custom(String),
}

/// Field definition in a record
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordField {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Whether field is required
    pub required: bool,
    /// Default value
    pub default: Option<String>,
}

/// Constraint types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Constraint {
    /// Minimum value constraint
    Minimum(f64),
    /// Maximum value constraint
    Maximum(f64),
    /// Minimum length constraint
    MinLength(usize),
    /// Maximum length constraint
    MaxLength(usize),
    /// Pattern/regex constraint
    Pattern(String),
    /// Unique constraint
    Unique,
    /// Not null constraint
    NotNull,
}

/// Breaking change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    /// The underlying change
    pub change: SchemaChange,
    /// Why this is breaking
    pub reason: String,
    /// Impact severity (0.0 = low, 1.0 = high)
    pub severity: f64,
    /// Suggested mitigation strategy
    pub mitigation: Option<String>,
}

/// Complete migration plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Schema difference
    pub diff: SchemaDiff,
    /// Selected migration strategy
    pub strategy: MigrationStrategy,
    /// Generated code for each language
    pub code_templates: HashMap<Language, GeneratedCode>,
    /// Validation rules to apply
    pub validation_rules: Vec<ValidationRule>,
    /// Rollback plan
    pub rollback_plan: Option<RollbackPlan>,
    /// Estimated migration time
    pub estimated_duration: Option<std::time::Duration>,
    /// Risk assessment
    pub risk_level: RiskLevel,
}

/// Generated migration code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCode {
    /// Main migration code
    pub migration_code: String,
    /// Test code
    pub test_code: Option<String>,
    /// Rollback code
    pub rollback_code: Option<String>,
    /// Documentation
    pub documentation: Option<String>,
}

/// Target programming language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    /// Python
    Python,
    /// TypeScript
    TypeScript,
    /// Java
    Java,
    /// Go
    Go,
    /// SQL
    Sql,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Python => write!(f, "python"),
            Language::TypeScript => write!(f, "typescript"),
            Language::Java => write!(f, "java"),
            Language::Go => write!(f, "go"),
            Language::Sql => write!(f, "sql"),
        }
    }
}

/// Migration strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStrategy {
    /// Fully automated, safe migration
    Safe,
    /// Automated with potential risks
    Risky,
    /// Requires manual intervention
    Manual,
    /// Dual-write strategy
    DualWrite,
    /// Shadow migration (test in parallel)
    Shadow,
}

impl std::fmt::Display for MigrationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationStrategy::Safe => write!(f, "Safe"),
            MigrationStrategy::Risky => write!(f, "Risky"),
            MigrationStrategy::Manual => write!(f, "Manual"),
            MigrationStrategy::DualWrite => write!(f, "Dual-Write"),
            MigrationStrategy::Shadow => write!(f, "Shadow"),
        }
    }
}

/// Validation rule for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Fields this rule applies to
    pub fields: Vec<String>,
    /// Rule type
    pub rule_type: ValidationRuleType,
}

/// Type of validation rule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationRuleType {
    /// Check for data loss
    DataLoss,
    /// Check type compatibility
    TypeCompatibility,
    /// Check constraint satisfaction
    ConstraintSatisfaction,
    /// Custom validation
    Custom(String),
}

/// Rollback plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    /// Rollback strategy
    pub strategy: RollbackStrategy,
    /// Generated rollback code
    pub rollback_code: HashMap<Language, String>,
    /// Estimated rollback time
    pub estimated_duration: Option<std::time::Duration>,
    /// Data backup required
    pub backup_required: bool,
}

/// Rollback strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollbackStrategy {
    /// Simple reverse migration
    Reverse,
    /// Restore from backup
    Backup,
    /// Manual rollback required
    Manual,
    /// No rollback possible
    IrreversibleChanges,
}

/// Risk level assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk, fully automated
    Low,
    /// Medium risk, review recommended
    Medium,
    /// High risk, careful review required
    High,
    /// Critical risk, manual intervention required
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Critical => write!(f, "Critical"),
        }
    }
}

/// Migration context for code generation
#[derive(Debug, Clone)]
pub struct MigrationContext {
    /// Source schema version
    pub from_version: SemanticVersion,
    /// Target schema version
    pub to_version: SemanticVersion,
    /// Schema name
    pub schema_name: String,
    /// Changes to apply
    pub changes: Vec<SchemaChange>,
    /// Generation timestamp
    pub generated_at: DateTime<Utc>,
    /// Custom options
    pub options: HashMap<String, serde_json::Value>,
}

impl SchemaChange {
    /// Check if this change is breaking
    pub fn is_breaking(&self) -> bool {
        match self {
            SchemaChange::FieldRemoved { .. } => true,
            SchemaChange::TypeChanged { .. } => true,
            SchemaChange::ConstraintAdded { .. } => true,
            SchemaChange::EnumChanged { removed, .. } => !removed.is_empty(),
            _ => false,
        }
    }

    /// Get the severity of this change (0.0 = trivial, 1.0 = severe)
    pub fn severity(&self) -> f64 {
        match self {
            SchemaChange::FieldAdded { required: true, default: None, .. } => 0.8,
            SchemaChange::FieldAdded { .. } => 0.1,
            SchemaChange::FieldRemoved { preserve_data: false, .. } => 1.0,
            SchemaChange::FieldRemoved { preserve_data: true, .. } => 0.4,
            SchemaChange::FieldRenamed { .. } => 0.3,
            SchemaChange::TypeChanged { .. } => 0.9,
            SchemaChange::NestedChanged { changes, .. } => {
                changes.iter().map(|c| c.severity()).sum::<f64>() / changes.len() as f64
            }
            SchemaChange::ArrayElementChanged { .. } => 0.7,
            SchemaChange::MapValueChanged { .. } => 0.7,
            SchemaChange::ConstraintAdded { .. } => 0.6,
            SchemaChange::ConstraintRemoved { .. } => 0.2,
            SchemaChange::EnumChanged { added, removed, .. } => {
                if !removed.is_empty() {
                    0.8
                } else if !added.is_empty() {
                    0.2
                } else {
                    0.0
                }
            }
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            SchemaChange::FieldAdded { name, .. } => format!("Add field '{}'", name),
            SchemaChange::FieldRemoved { name, field_type: _, preserve_data: _ } => format!("Remove field '{}'", name),
            SchemaChange::FieldRenamed { old_name, new_name, .. } => {
                format!("Rename field '{}' to '{}'", old_name, new_name)
            }
            SchemaChange::TypeChanged { field, old_type, new_type, .. } => {
                format!("Change type of '{}' from {:?} to {:?}", field, old_type, new_type)
            }
            SchemaChange::NestedChanged { path, changes } => {
                format!("Change nested structure '{}' ({} changes)", path, changes.len())
            }
            SchemaChange::ArrayElementChanged { field, .. } => {
                format!("Change array element type for '{}'", field)
            }
            SchemaChange::MapValueChanged { field, .. } => {
                format!("Change map value type for '{}'", field)
            }
            SchemaChange::ConstraintAdded { field, constraint } => {
                format!("Add constraint {:?} to '{}'", constraint, field)
            }
            SchemaChange::ConstraintRemoved { field, constraint } => {
                format!("Remove constraint {:?} from '{}'", constraint, field)
            }
            SchemaChange::EnumChanged { field, added, removed } => {
                format!(
                    "Change enum '{}' (added: {}, removed: {})",
                    field,
                    added.len(),
                    removed.len()
                )
            }
        }
    }
}

impl FieldType {
    /// Check if this type is compatible with another type
    pub fn is_compatible_with(&self, other: &FieldType) -> bool {
        match (self, other) {
            (a, b) if a == b => true,
            (FieldType::Integer, FieldType::Long) => true,
            (FieldType::Float, FieldType::Double) => true,
            (FieldType::Union(types), other) => types.iter().any(|t| t.is_compatible_with(other)),
            (this, FieldType::Union(types)) => types.iter().any(|t| this.is_compatible_with(t)),
            _ => false,
        }
    }

    /// Get a type name suitable for code generation
    pub fn type_name(&self, lang: Language) -> String {
        match (self, lang) {
            (FieldType::String, Language::Python) => "str".to_string(),
            (FieldType::String, Language::TypeScript) => "string".to_string(),
            (FieldType::String, Language::Java) => "String".to_string(),
            (FieldType::String, Language::Go) => "string".to_string(),
            (FieldType::String, Language::Sql) => "VARCHAR".to_string(),

            (FieldType::Integer, Language::Python) => "int".to_string(),
            (FieldType::Integer, Language::TypeScript) => "number".to_string(),
            (FieldType::Integer, Language::Java) => "Integer".to_string(),
            (FieldType::Integer, Language::Go) => "int32".to_string(),
            (FieldType::Integer, Language::Sql) => "INTEGER".to_string(),

            (FieldType::Long, Language::Python) => "int".to_string(),
            (FieldType::Long, Language::TypeScript) => "number".to_string(),
            (FieldType::Long, Language::Java) => "Long".to_string(),
            (FieldType::Long, Language::Go) => "int64".to_string(),
            (FieldType::Long, Language::Sql) => "BIGINT".to_string(),

            (FieldType::Float, Language::Python) => "float".to_string(),
            (FieldType::Float, Language::TypeScript) => "number".to_string(),
            (FieldType::Float, Language::Java) => "Float".to_string(),
            (FieldType::Float, Language::Go) => "float32".to_string(),
            (FieldType::Float, Language::Sql) => "REAL".to_string(),

            (FieldType::Double, Language::Python) => "float".to_string(),
            (FieldType::Double, Language::TypeScript) => "number".to_string(),
            (FieldType::Double, Language::Java) => "Double".to_string(),
            (FieldType::Double, Language::Go) => "float64".to_string(),
            (FieldType::Double, Language::Sql) => "DOUBLE PRECISION".to_string(),

            (FieldType::Boolean, Language::Python) => "bool".to_string(),
            (FieldType::Boolean, Language::TypeScript) => "boolean".to_string(),
            (FieldType::Boolean, Language::Java) => "Boolean".to_string(),
            (FieldType::Boolean, Language::Go) => "bool".to_string(),
            (FieldType::Boolean, Language::Sql) => "BOOLEAN".to_string(),

            (FieldType::Array(elem), lang) => match lang {
                Language::Python => format!("list[{}]", elem.type_name(lang)),
                Language::TypeScript => format!("{}[]", elem.type_name(lang)),
                Language::Java => format!("List<{}>", elem.type_name(lang)),
                Language::Go => format!("[]{}", elem.type_name(lang)),
                Language::Sql => format!("{}[]", elem.type_name(lang)),
            },

            (FieldType::Map(val), lang) => match lang {
                Language::Python => format!("dict[str, {}]", val.type_name(lang)),
                Language::TypeScript => format!("Record<string, {}>", val.type_name(lang)),
                Language::Java => format!("Map<String, {}>", val.type_name(lang)),
                Language::Go => format!("map[string]{}", val.type_name(lang)),
                Language::Sql => "JSONB".to_string(),
            },

            _ => format!("{:?}", self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_change_is_breaking() {
        let breaking = SchemaChange::FieldRemoved {
            name: "field".to_string(),
            field_type: FieldType::String,
            preserve_data: false,
        };
        assert!(breaking.is_breaking());

        let non_breaking = SchemaChange::FieldAdded {
            name: "field".to_string(),
            field_type: FieldType::String,
            default: Some(serde_json::Value::String("default".to_string())),
            required: false,
            description: None,
        };
        assert!(!non_breaking.is_breaking());
    }

    #[test]
    fn test_field_type_compatibility() {
        assert!(FieldType::Integer.is_compatible_with(&FieldType::Long));
        assert!(FieldType::Float.is_compatible_with(&FieldType::Double));
        assert!(!FieldType::String.is_compatible_with(&FieldType::Integer));
    }

    #[test]
    fn test_language_display() {
        assert_eq!(Language::Python.to_string(), "python");
        assert_eq!(Language::TypeScript.to_string(), "typescript");
        assert_eq!(Language::Java.to_string(), "java");
        assert_eq!(Language::Go.to_string(), "go");
        assert_eq!(Language::Sql.to_string(), "sql");
    }
}
