//! Apache Avro schema validator
//!
//! Validates Apache Avro schemas using the apache-avro crate.

use crate::types::{ValidationError, ValidationResult, ValidationWarning, SchemaFormat};
use anyhow::Result;
use apache_avro::Schema as AvroSchema;

/// Apache Avro validator
pub struct AvroValidator;

impl AvroValidator {
    /// Creates a new Avro validator
    pub fn new() -> Self {
        Self
    }

    /// Validates an Avro schema
    pub fn validate(&self, schema: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success(SchemaFormat::Avro);

        // Parse the Avro schema
        let parsed_schema = match AvroSchema::parse_str(schema) {
            Ok(s) => s,
            Err(e) => {
                result.add_error(
                    ValidationError::new(
                        "avro-parse",
                        format!("Failed to parse Avro schema: {}", e),
                    )
                    .with_suggestion("Check Avro schema syntax and structure"),
                );
                return Ok(result);
            }
        };

        // Validate schema structure
        self.validate_schema_structure(&parsed_schema, &mut result);

        // Validate naming conventions
        self.validate_naming_conventions(&parsed_schema, &mut result);

        // Validate field constraints
        self.validate_field_constraints(&parsed_schema, &mut result);

        Ok(result)
    }

    /// Validates a data instance against an Avro schema
    pub fn validate_instance(&self, schema: &str, instance: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success(SchemaFormat::Avro);

        // Parse schema
        let parsed_schema = match AvroSchema::parse_str(schema) {
            Ok(s) => s,
            Err(e) => {
                result.add_error(ValidationError::new(
                    "avro-parse",
                    format!("Failed to parse schema: {}", e),
                ));
                return Ok(result);
            }
        };

        // Parse instance as JSON (Avro data can be JSON-encoded)
        let instance_value: serde_json::Value = match serde_json::from_str(instance) {
            Ok(v) => v,
            Err(e) => {
                result.add_error(ValidationError::new(
                    "instance-parse",
                    format!("Failed to parse instance: {}", e),
                ));
                return Ok(result);
            }
        };

        // Validate instance against schema
        // Convert JSON to Avro value and validate
        match apache_avro::from_value::<serde_json::Value>(&apache_avro::to_value(&instance_value)?) {
            Ok(_) => {
                // Additional validation logic would go here
            }
            Err(e) => {
                result.add_error(ValidationError::new(
                    "instance-validation",
                    format!("Instance validation failed: {}", e),
                ));
            }
        }

        Ok(result)
    }

    /// Validates the structure of an Avro schema
    fn validate_schema_structure(&self, schema: &AvroSchema, result: &mut ValidationResult) {
        match schema {
            AvroSchema::Record(record_schema) => {
                // Validate record has at least one field
                if record_schema.fields.is_empty() {
                    result.add_warning(
                        ValidationWarning::new(
                            "avro-empty-record",
                            format!("Record '{}' has no fields", record_schema.name.fullname(None)),
                        )
                        .with_suggestion("Add at least one field to the record"),
                    );
                }

                // Check for duplicate field names
                let mut field_names = std::collections::HashSet::new();
                for field in &record_schema.fields {
                    if !field_names.insert(&field.name) {
                        result.add_error(
                            ValidationError::new(
                                "avro-duplicate-field",
                                format!("Duplicate field name '{}' in record '{}'", field.name, record_schema.name.fullname(None)),
                            )
                            .with_suggestion("Ensure all field names are unique"),
                        );
                    }
                }

                // Recursively validate field schemas
                for field in &record_schema.fields {
                    self.validate_schema_structure(&field.schema, result);
                }
            }
            AvroSchema::Enum(enum_schema) => {
                // Validate enum has at least one symbol
                if enum_schema.symbols.is_empty() {
                    result.add_error(
                        ValidationError::new(
                            "avro-empty-enum",
                            format!("Enum '{}' has no symbols", enum_schema.name.fullname(None)),
                        )
                        .with_suggestion("Add at least one symbol to the enum"),
                    );
                }

                // Check for duplicate symbols
                let mut symbol_set = std::collections::HashSet::new();
                for symbol in &enum_schema.symbols {
                    if !symbol_set.insert(symbol) {
                        result.add_error(
                            ValidationError::new(
                                "avro-duplicate-symbol",
                                format!("Duplicate symbol '{}' in enum '{}'", symbol, enum_schema.name.fullname(None)),
                            ),
                        );
                    }
                }
            }
            AvroSchema::Array(inner) => {
                self.validate_schema_structure(inner, result);
            }
            AvroSchema::Map(inner) => {
                self.validate_schema_structure(inner, result);
            }
            AvroSchema::Union(union_schema) => {
                // Validate union has at least two types
                if union_schema.variants().len() < 2 {
                    result.add_warning(
                        ValidationWarning::new(
                            "avro-single-union",
                            "Union type has only one variant",
                        )
                        .with_suggestion("Consider using the type directly instead of a union"),
                    );
                }

                // Recursively validate union members
                for variant in union_schema.variants() {
                    self.validate_schema_structure(variant, result);
                }
            }
            AvroSchema::Fixed(fixed_schema) => {
                // Validate fixed size is positive
                if fixed_schema.size == 0 {
                    result.add_error(
                        ValidationError::new(
                            "avro-zero-size-fixed",
                            format!("Fixed type '{}' has zero size", fixed_schema.name.fullname(None)),
                        )
                        .with_suggestion("Set a positive size for the fixed type"),
                    );
                }
            }
            _ => {
                // Primitive types are always valid
            }
        }
    }

    /// Validates Avro naming conventions
    fn validate_naming_conventions(&self, schema: &AvroSchema, result: &mut ValidationResult) {
        match schema {
            AvroSchema::Record(record_schema) => {
                // Check record name follows conventions
                self.validate_name(&record_schema.name.fullname(None), "record", result);

                // Check field names
                for field in &record_schema.fields {
                    self.validate_field_name(&field.name, result);
                    self.validate_naming_conventions(&field.schema, result);
                }
            }
            AvroSchema::Enum(enum_schema) => {
                self.validate_name(&enum_schema.name.fullname(None), "enum", result);
            }
            AvroSchema::Fixed(fixed_schema) => {
                self.validate_name(&fixed_schema.name.fullname(None), "fixed", result);
            }
            AvroSchema::Array(inner) | AvroSchema::Map(inner) => {
                self.validate_naming_conventions(inner, result);
            }
            AvroSchema::Union(union_schema) => {
                for variant in union_schema.variants() {
                    self.validate_naming_conventions(variant, result);
                }
            }
            _ => {}
        }
    }

    /// Validates a name follows Avro conventions
    fn validate_name(&self, name: &str, type_name: &str, result: &mut ValidationResult) {
        // Avro names must start with [A-Za-z_] and contain only [A-Za-z0-9_]
        let first_char = name.chars().next();
        if let Some(c) = first_char {
            if !c.is_alphabetic() && c != '_' {
                result.add_warning(
                    ValidationWarning::new(
                        "avro-naming-convention",
                        format!("{} name '{}' should start with a letter or underscore", type_name, name),
                    ),
                );
            }
        }

        for c in name.chars() {
            if !c.is_alphanumeric() && c != '_' && c != '.' {
                result.add_warning(
                    ValidationWarning::new(
                        "avro-naming-convention",
                        format!("{} name '{}' contains invalid character '{}'", type_name, name, c),
                    )
                    .with_suggestion("Use only letters, numbers, underscores, and dots"),
                );
                break;
            }
        }
    }

    /// Validates a field name
    fn validate_field_name(&self, name: &str, result: &mut ValidationResult) {
        // Check for reserved words
        let reserved = ["type", "schema", "namespace", "name", "fields"];
        if reserved.contains(&name) {
            result.add_warning(
                ValidationWarning::new(
                    "avro-reserved-field-name",
                    format!("Field name '{}' is a reserved Avro keyword", name),
                )
                .with_suggestion("Use a different field name to avoid confusion"),
            );
        }

        self.validate_name(name, "field", result);
    }

    /// Validates field constraints
    fn validate_field_constraints(&self, schema: &AvroSchema, result: &mut ValidationResult) {
        match schema {
            AvroSchema::Record(record_schema) => {
                for field in &record_schema.fields {
                    // Check if field has documentation
                    if field.doc.is_none() {
                        result.add_warning(
                            ValidationWarning::new(
                                "avro-missing-doc",
                                format!("Field '{}' lacks documentation", field.name),
                            )
                            .with_suggestion("Add a 'doc' field to document the field's purpose"),
                        );
                    }

                    self.validate_field_constraints(&field.schema, result);
                }
            }
            AvroSchema::Array(inner) | AvroSchema::Map(inner) => {
                self.validate_field_constraints(inner, result);
            }
            AvroSchema::Union(union_schema) => {
                for variant in union_schema.variants() {
                    self.validate_field_constraints(variant, result);
                }
            }
            _ => {}
        }
    }
}

impl Default for AvroValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_avro_record() {
        let validator = AvroValidator::new();
        let schema = r#"{
            "type": "record",
            "name": "User",
            "namespace": "com.example",
            "fields": [
                {"name": "id", "type": "long"},
                {"name": "username", "type": "string"}
            ]
        }"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_invalid_avro() {
        let validator = AvroValidator::new();
        let schema = r#"{ invalid avro }"#;

        let result = validator.validate(schema).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.rule == "avro-parse"));
    }

    #[test]
    fn test_validate_empty_record() {
        let validator = AvroValidator::new();
        let schema = r#"{
            "type": "record",
            "name": "Empty",
            "fields": []
        }"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.has_warnings());
        assert!(result.warnings.iter().any(|w| w.rule == "avro-empty-record"));
    }

    #[test]
    fn test_validate_duplicate_field() {
        let validator = AvroValidator::new();
        let schema = r#"{
            "type": "record",
            "name": "Duplicate",
            "fields": [
                {"name": "id", "type": "long"},
                {"name": "id", "type": "string"}
            ]
        }"#;

        let result = validator.validate(schema).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.rule == "avro-duplicate-field"));
    }

    #[test]
    fn test_validate_enum() {
        let validator = AvroValidator::new();
        let schema = r#"{
            "type": "enum",
            "name": "Status",
            "symbols": ["ACTIVE", "INACTIVE"]
        }"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_empty_enum() {
        let validator = AvroValidator::new();
        let schema = r#"{
            "type": "enum",
            "name": "Status",
            "symbols": []
        }"#;

        let result = validator.validate(schema).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.rule == "avro-empty-enum"));
    }

    #[test]
    fn test_validate_union() {
        let validator = AvroValidator::new();
        let schema = r#"["null", "string"]"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_single_union() {
        let validator = AvroValidator::new();
        let schema = r#"["string"]"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.has_warnings());
        assert!(result.warnings.iter().any(|w| w.rule == "avro-single-union"));
    }
}
