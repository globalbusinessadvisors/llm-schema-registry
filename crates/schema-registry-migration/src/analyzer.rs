//! Schema difference analyzer

use crate::error::{Error, Result};
use crate::types::{
    BreakingChange, Constraint, FieldType, MigrationStrategy, RecordField, SchemaChange,
    SchemaDiff,
};
use chrono::Utc;
use schema_registry_core::{versioning::SemanticVersion, SerializationFormat};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Analyzes differences between schema versions
pub struct SchemaAnalyzer {
    /// Schema format being analyzed
    format: SerializationFormat,
}

impl SchemaAnalyzer {
    /// Create a new analyzer for the given format
    pub fn new(format: SerializationFormat) -> Self {
        Self { format }
    }

    /// Analyze differences between two schemas
    pub fn analyze(
        &self,
        old_schema: &str,
        new_schema: &str,
        old_version: SemanticVersion,
        new_version: SemanticVersion,
        schema_name: String,
        namespace: String,
    ) -> Result<SchemaDiff> {
        match self.format {
            SerializationFormat::JsonSchema => {
                self.analyze_json_schema(old_schema, new_schema, old_version, new_version, schema_name, namespace)
            }
            SerializationFormat::Avro => {
                self.analyze_avro_schema(old_schema, new_schema, old_version, new_version, schema_name, namespace)
            }
            SerializationFormat::Protobuf => {
                Err(Error::UnsupportedOperation(
                    "Protobuf schema analysis not yet implemented".to_string(),
                ))
            }
        }
    }

    /// Analyze JSON Schema differences
    fn analyze_json_schema(
        &self,
        old_schema: &str,
        new_schema: &str,
        old_version: SemanticVersion,
        new_version: SemanticVersion,
        schema_name: String,
        namespace: String,
    ) -> Result<SchemaDiff> {
        let old: Value = serde_json::from_str(old_schema)?;
        let new: Value = serde_json::from_str(new_schema)?;

        let mut changes = Vec::new();

        // Analyze properties
        if let (Some(old_props), Some(new_props)) = (
            old.get("properties").and_then(|p| p.as_object()),
            new.get("properties").and_then(|p| p.as_object()),
        ) {
            let _old_required = old
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<HashSet<_>>()
                })
                .unwrap_or_default();

            let new_required = new
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<HashSet<_>>()
                })
                .unwrap_or_default();

            // Find added fields
            for (name, schema) in new_props {
                if !old_props.contains_key(name) {
                    let field_type = self.json_schema_to_field_type(schema);
                    let default = schema.get("default").cloned();
                    let required = new_required.contains(name);
                    let description = schema.get("description").and_then(|d| d.as_str()).map(String::from);

                    changes.push(SchemaChange::FieldAdded {
                        name: name.clone(),
                        field_type,
                        default,
                        required,
                        description,
                    });
                }
            }

            // Find removed fields
            for (name, schema) in old_props {
                if !new_props.contains_key(name) {
                    let field_type = self.json_schema_to_field_type(schema);
                    changes.push(SchemaChange::FieldRemoved {
                        name: name.clone(),
                        field_type,
                        preserve_data: false,
                    });
                }
            }

            // Find type changes
            for (name, old_schema) in old_props {
                if let Some(new_schema) = new_props.get(name) {
                    let old_type = self.json_schema_to_field_type(old_schema);
                    let new_type = self.json_schema_to_field_type(new_schema);

                    if old_type != new_type {
                        changes.push(SchemaChange::TypeChanged {
                            field: name.clone(),
                            old_type,
                            new_type,
                            converter: None,
                        });
                    }

                    // Check for constraint changes
                    self.detect_constraint_changes(name, old_schema, new_schema, &mut changes);
                }
            }
        }

        let breaking_changes = self.identify_breaking_changes(&changes);
        let complexity_score = self.calculate_complexity(&changes);

        Ok(SchemaDiff {
            old_version,
            new_version,
            schema_name,
            namespace,
            changes,
            breaking_changes,
            complexity_score,
            created_at: Utc::now(),
        })
    }

    /// Analyze Avro schema differences
    fn analyze_avro_schema(
        &self,
        old_schema: &str,
        new_schema: &str,
        old_version: SemanticVersion,
        new_version: SemanticVersion,
        schema_name: String,
        namespace: String,
    ) -> Result<SchemaDiff> {
        use apache_avro::Schema;

        // Parse schemas to validate they're valid Avro
        let _old = Schema::parse_str(old_schema)?;
        let _new = Schema::parse_str(new_schema)?;

        // For now, simplified Avro analysis (full implementation would inspect schema structure)
        let changes = Vec::new();

        // TODO: Full Avro schema field-by-field comparison
        // This would require working with the apache-avro crate's RecordSchema API

        let breaking_changes = self.identify_breaking_changes(&changes);
        let complexity_score = self.calculate_complexity(&changes);

        Ok(SchemaDiff {
            old_version,
            new_version,
            schema_name,
            namespace,
            changes,
            breaking_changes,
            complexity_score,
            created_at: Utc::now(),
        })
    }

    /// Convert JSON Schema type to FieldType
    fn json_schema_to_field_type(&self, schema: &Value) -> FieldType {
        if let Some(type_str) = schema.get("type").and_then(|t| t.as_str()) {
            match type_str {
                "string" => FieldType::String,
                "integer" => FieldType::Integer,
                "number" => FieldType::Double,
                "boolean" => FieldType::Boolean,
                "null" => FieldType::Null,
                "array" => {
                    if let Some(items) = schema.get("items") {
                        FieldType::Array(Box::new(self.json_schema_to_field_type(items)))
                    } else {
                        FieldType::Array(Box::new(FieldType::Custom("Any".to_string())))
                    }
                }
                "object" => {
                    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                        let fields = props
                            .iter()
                            .map(|(name, schema)| RecordField {
                                name: name.clone(),
                                field_type: self.json_schema_to_field_type(schema),
                                required: false,
                                default: None,
                            })
                            .collect();
                        FieldType::Record {
                            name: "Object".to_string(),
                            fields,
                        }
                    } else {
                        FieldType::Map(Box::new(FieldType::Custom("Any".to_string())))
                    }
                }
                _ => FieldType::Custom(type_str.to_string()),
            }
        } else if let Some(enum_vals) = schema.get("enum").and_then(|e| e.as_array()) {
            let symbols = enum_vals
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            FieldType::Enum {
                name: "Enum".to_string(),
                symbols,
            }
        } else {
            FieldType::Custom("Unknown".to_string())
        }
    }

    /// Convert Avro schema to FieldType
    #[allow(dead_code)]
    fn avro_schema_to_field_type(&self, schema: &apache_avro::Schema) -> FieldType {
        use apache_avro::Schema;

        match schema {
            Schema::String => FieldType::String,
            Schema::Int => FieldType::Integer,
            Schema::Long => FieldType::Long,
            Schema::Float => FieldType::Float,
            Schema::Double => FieldType::Double,
            Schema::Boolean => FieldType::Boolean,
            Schema::Bytes => FieldType::Bytes,
            Schema::Null => FieldType::Null,
            Schema::Array(inner) => FieldType::Array(Box::new(self.avro_schema_to_field_type(inner))),
            Schema::Map(inner) => FieldType::Map(Box::new(self.avro_schema_to_field_type(inner))),
            Schema::Union(_) => FieldType::Custom("Union".to_string()),
            _ => FieldType::Custom("Unknown".to_string()),
        }
    }

    /// Detect constraint changes between schemas
    fn detect_constraint_changes(
        &self,
        field_name: &str,
        old_schema: &Value,
        new_schema: &Value,
        changes: &mut Vec<SchemaChange>,
    ) {
        // Check minimum constraints
        let old_min = old_schema.get("minimum").and_then(|v| v.as_f64());
        let new_min = new_schema.get("minimum").and_then(|v| v.as_f64());
        if new_min.is_some() && old_min != new_min {
            changes.push(SchemaChange::ConstraintAdded {
                field: field_name.to_string(),
                constraint: Constraint::Minimum(new_min.unwrap()),
            });
        }

        // Check maximum constraints
        let old_max = old_schema.get("maximum").and_then(|v| v.as_f64());
        let new_max = new_schema.get("maximum").and_then(|v| v.as_f64());
        if new_max.is_some() && old_max != new_max {
            changes.push(SchemaChange::ConstraintAdded {
                field: field_name.to_string(),
                constraint: Constraint::Maximum(new_max.unwrap()),
            });
        }

        // Check minLength constraints
        let old_min_len = old_schema.get("minLength").and_then(|v| v.as_u64());
        let new_min_len = new_schema.get("minLength").and_then(|v| v.as_u64());
        if new_min_len.is_some() && old_min_len != new_min_len {
            changes.push(SchemaChange::ConstraintAdded {
                field: field_name.to_string(),
                constraint: Constraint::MinLength(new_min_len.unwrap() as usize),
            });
        }

        // Check pattern constraints
        let old_pattern = old_schema.get("pattern").and_then(|v| v.as_str());
        let new_pattern = new_schema.get("pattern").and_then(|v| v.as_str());
        if new_pattern.is_some() && old_pattern != new_pattern {
            changes.push(SchemaChange::ConstraintAdded {
                field: field_name.to_string(),
                constraint: Constraint::Pattern(new_pattern.unwrap().to_string()),
            });
        }
    }

    /// Identify which changes are breaking
    fn identify_breaking_changes(&self, changes: &[SchemaChange]) -> Vec<BreakingChange> {
        changes
            .iter()
            .filter(|c| c.is_breaking())
            .map(|change| {
                let (reason, severity, mitigation) = match change {
                    SchemaChange::FieldRemoved { name, preserve_data, .. } => (
                        format!("Removing field '{}' will break existing consumers", name),
                        if *preserve_data { 0.6 } else { 1.0 },
                        Some(format!(
                            "Consider deprecating '{}' first or providing a migration path",
                            name
                        )),
                    ),
                    SchemaChange::TypeChanged { field, old_type, new_type, .. } => (
                        format!(
                            "Changing type of '{}' from {:?} to {:?} requires data transformation",
                            field, old_type, new_type
                        ),
                        0.9,
                        Some(format!("Provide a conversion function for '{}'", field)),
                    ),
                    SchemaChange::ConstraintAdded { field, constraint } => (
                        format!(
                            "Adding constraint {:?} to '{}' may reject existing data",
                            constraint, field
                        ),
                        0.7,
                        Some(format!("Validate existing data before applying constraint")),
                    ),
                    SchemaChange::EnumChanged { field, removed, .. } => (
                        format!(
                            "Removing {} enum values from '{}' will break existing data",
                            removed.len(),
                            field
                        ),
                        0.8,
                        Some(format!("Migrate existing enum values before removal")),
                    ),
                    _ => ("Unknown breaking change".to_string(), 0.5, None),
                };

                BreakingChange {
                    change: change.clone(),
                    reason,
                    severity,
                    mitigation,
                }
            })
            .collect()
    }

    /// Calculate migration complexity score
    fn calculate_complexity(&self, changes: &[SchemaChange]) -> f64 {
        if changes.is_empty() {
            return 0.0;
        }

        let total_severity: f64 = changes.iter().map(|c| c.severity()).sum();
        let avg_severity = total_severity / changes.len() as f64;

        // Factor in the number of changes
        let count_factor = (changes.len() as f64 / 10.0).min(1.0);

        // Combine average severity with count factor
        (avg_severity * 0.7 + count_factor * 0.3).min(1.0)
    }

    /// Suggest migration strategy based on analysis
    pub fn suggest_strategy(&self, diff: &SchemaDiff) -> MigrationStrategy {
        if diff.breaking_changes.is_empty() && diff.complexity_score < 0.3 {
            MigrationStrategy::Safe
        } else if diff.breaking_changes.is_empty() && diff.complexity_score < 0.6 {
            MigrationStrategy::Risky
        } else if diff.breaking_changes.len() > 5 || diff.complexity_score > 0.8 {
            MigrationStrategy::Manual
        } else {
            MigrationStrategy::DualWrite
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_schema_field_added() {
        let analyzer = SchemaAnalyzer::new(SerializationFormat::JsonSchema);

        let old_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }"#;

        let new_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "default": 0}
            }
        }"#;

        let result = analyzer.analyze(
            old_schema,
            new_schema,
            SemanticVersion::new(1, 0, 0),
            SemanticVersion::new(2, 0, 0),
            "test".to_string(),
            "com.example".to_string(),
        );

        assert!(result.is_ok());
        let diff = result.unwrap();
        assert_eq!(diff.changes.len(), 1);
        assert!(matches!(diff.changes[0], SchemaChange::FieldAdded { .. }));
    }

    #[test]
    fn test_migration_strategy_suggestion() {
        let analyzer = SchemaAnalyzer::new(SerializationFormat::JsonSchema);

        // Safe migration
        let safe_diff = SchemaDiff {
            old_version: SemanticVersion::new(1, 0, 0),
            new_version: SemanticVersion::new(1, 1, 0),
            schema_name: "test".to_string(),
            namespace: "com.example".to_string(),
            changes: vec![],
            breaking_changes: vec![],
            complexity_score: 0.1,
            created_at: Utc::now(),
        };

        assert_eq!(analyzer.suggest_strategy(&safe_diff), MigrationStrategy::Safe);

        // Manual migration
        let manual_diff = SchemaDiff {
            old_version: SemanticVersion::new(1, 0, 0),
            new_version: SemanticVersion::new(2, 0, 0),
            schema_name: "test".to_string(),
            namespace: "com.example".to_string(),
            changes: vec![],
            breaking_changes: vec![BreakingChange {
                change: SchemaChange::FieldRemoved {
                    name: "field".to_string(),
                    field_type: FieldType::String,
                    preserve_data: false,
                },
                reason: "test".to_string(),
                severity: 1.0,
                mitigation: None,
            }; 6],
            complexity_score: 0.9,
            created_at: Utc::now(),
        };

        assert_eq!(analyzer.suggest_strategy(&manual_diff), MigrationStrategy::Manual);
    }
}
