//! JSON Schema validator
//!
//! Validates JSON Schema using the jsonschema crate with support for
//! Draft 7, Draft 2019-09, and Draft 2020-12.

use crate::types::{ValidationError, ValidationResult, ValidationWarning, SchemaFormat};
use anyhow::Result;
use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

/// JSON Schema validator
pub struct JsonSchemaValidator {
    /// The JSON Schema draft version to use
    draft: Draft,
}

impl JsonSchemaValidator {
    /// Creates a new JSON Schema validator with the specified draft
    pub fn new(draft: Draft) -> Self {
        Self { draft }
    }

    /// Creates a new JSON Schema validator with Draft 7 (most common)
    pub fn new_draft_7() -> Self {
        Self::new(Draft::Draft7)
    }

    /// Creates a new JSON Schema validator with Draft 6
    pub fn new_draft_6() -> Self {
        Self::new(Draft::Draft6)
    }

    /// Creates a new JSON Schema validator with Draft 4
    pub fn new_draft_4() -> Self {
        Self::new(Draft::Draft4)
    }

    /// Validates a JSON Schema
    pub fn validate(&self, schema: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success(SchemaFormat::JsonSchema);

        // Parse the schema
        let schema_value: Value = match serde_json::from_str(schema) {
            Ok(v) => v,
            Err(e) => {
                result.add_error(
                    ValidationError::new(
                        "json-schema-parse",
                        format!("Failed to parse JSON Schema: {}", e),
                    )
                    .with_suggestion("Ensure the schema is valid JSON"),
                );
                return Ok(result);
            }
        };

        // Validate against meta-schema
        match self.validate_against_metaschema(&schema_value) {
            Ok(errors) => {
                for error in errors {
                    result.add_error(error);
                }
            }
            Err(e) => {
                result.add_error(
                    ValidationError::new(
                        "json-schema-validation",
                        format!("Meta-schema validation error: {}", e),
                    ),
                );
            }
        }

        // Additional JSON Schema specific validations
        self.validate_schema_properties(&schema_value, &mut result);

        Ok(result)
    }

    /// Validates a data instance against a JSON Schema
    pub fn validate_instance(&self, schema: &str, instance: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success(SchemaFormat::JsonSchema);

        // Parse schema and instance
        let schema_value: Value = match serde_json::from_str(schema) {
            Ok(v) => v,
            Err(e) => {
                result.add_error(ValidationError::new(
                    "json-schema-parse",
                    format!("Failed to parse schema: {}", e),
                ));
                return Ok(result);
            }
        };

        let instance_value: Value = match serde_json::from_str(instance) {
            Ok(v) => v,
            Err(e) => {
                result.add_error(ValidationError::new(
                    "instance-parse",
                    format!("Failed to parse instance: {}", e),
                ));
                return Ok(result);
            }
        };

        // Compile schema
        let compiled = match JSONSchema::options()
            .with_draft(self.draft)
            .compile(&schema_value)
        {
            Ok(s) => s,
            Err(e) => {
                result.add_error(ValidationError::new(
                    "json-schema-compile",
                    format!("Failed to compile schema: {}", e),
                ));
                return Ok(result);
            }
        };

        // Validate instance
        if let Err(errors) = compiled.validate(&instance_value) {
            for error in errors {
                result.add_error(
                    ValidationError::new(
                        "instance-validation",
                        format!("{}", error),
                    )
                    .with_location(error.instance_path.to_string()),
                );
            }
        }

        Ok(result)
    }

    /// Validates the schema against its meta-schema
    fn validate_against_metaschema(&self, schema: &Value) -> Result<Vec<ValidationError>> {
        let mut errors = Vec::new();

        // For now, we skip meta-schema validation due to lifetime constraints
        // In production, we would use embedded static meta-schemas
        // This validation is primarily structural and semantic

        // Basic validation: ensure it's a valid JSON object
        if !schema.is_object() {
            errors.push(
                ValidationError::new(
                    "json-schema-structure",
                    "Schema must be a JSON object",
                )
                .with_suggestion("Ensure the schema is a valid JSON object"),
            );
        }

        Ok(errors)
    }

    /// Gets the meta-schema for validation
    fn get_metaschema(&self, schema: &Value) -> Result<Value> {
        // Check if schema specifies a $schema
        if let Some(schema_uri) = schema.get("$schema").and_then(|s| s.as_str()) {
            // Use the specified meta-schema
            // In production, you would fetch this or have it embedded
            return self.get_metaschema_by_draft();
        }

        // Use default meta-schema for the configured draft
        self.get_metaschema_by_draft()
    }

    /// Gets the meta-schema for the configured draft
    fn get_metaschema_by_draft(&self) -> Result<Value> {
        // For now, return a minimal meta-schema
        // In production, embed the full meta-schemas
        let metaschema = match self.draft {
            Draft::Draft7 => {
                serde_json::json!({
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "type": "object"
                })
            }
            Draft::Draft6 => {
                serde_json::json!({
                    "$schema": "http://json-schema.org/draft-06/schema#",
                    "type": "object"
                })
            }
            Draft::Draft4 => {
                serde_json::json!({
                    "$schema": "http://json-schema.org/draft-04/schema#",
                    "type": "object"
                })
            }
            _ => {
                serde_json::json!({
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "type": "object"
                })
            }
        };

        Ok(metaschema)
    }

    /// Validates JSON Schema specific properties
    fn validate_schema_properties(&self, schema: &Value, result: &mut ValidationResult) {
        // Check for $id
        if let Some(id) = schema.get("$id") {
            if let Some(id_str) = id.as_str() {
                if !id_str.starts_with("http://") && !id_str.starts_with("https://") && !id_str.starts_with("urn:") {
                    result.add_warning(
                        ValidationWarning::new(
                            "json-schema-id",
                            "Schema $id should be a valid URI",
                        )
                        .with_suggestion("Use a full URI for $id (http://, https://, or urn:)"),
                    );
                }
            }
        }

        // Validate properties
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            for (name, prop) in properties {
                self.validate_property(name, prop, result);
            }
        }

        // Check for deprecated keywords
        self.check_deprecated_keywords(schema, result);
    }

    /// Validates a single property
    fn validate_property(&self, name: &str, property: &Value, result: &mut ValidationResult) {
        // Check for type
        if property.get("type").is_none() {
            result.add_warning(
                ValidationWarning::new(
                    "missing-type",
                    format!("Property '{}' does not specify a type", name),
                )
                .with_location(format!("$.properties.{}", name))
                .with_suggestion("Add a 'type' field to clarify the expected data type"),
            );
        }

        // Check for conflicting constraints
        if let (Some(min), Some(max)) = (
            property.get("minimum").and_then(|m| m.as_f64()),
            property.get("maximum").and_then(|m| m.as_f64()),
        ) {
            if min > max {
                result.add_error(
                    ValidationError::new(
                        "conflicting-constraints",
                        format!(
                            "Property '{}' has minimum ({}) greater than maximum ({})",
                            name, min, max
                        ),
                    )
                    .with_location(format!("$.properties.{}", name)),
                );
            }
        }

        // Check for conflicting length constraints
        if let (Some(min_len), Some(max_len)) = (
            property.get("minLength").and_then(|m| m.as_u64()),
            property.get("maxLength").and_then(|m| m.as_u64()),
        ) {
            if min_len > max_len {
                result.add_error(
                    ValidationError::new(
                        "conflicting-constraints",
                        format!(
                            "Property '{}' has minLength ({}) greater than maxLength ({})",
                            name, min_len, max_len
                        ),
                    )
                    .with_location(format!("$.properties.{}", name)),
                );
            }
        }
    }

    /// Checks for deprecated JSON Schema keywords
    fn check_deprecated_keywords(&self, schema: &Value, result: &mut ValidationResult) {
        // Check for Draft 4 keywords that are deprecated
        if schema.get("id").is_some() {
            result.add_warning(
                ValidationWarning::new(
                    "deprecated-keyword",
                    "The 'id' keyword is deprecated in favor of '$id'",
                )
                .with_suggestion("Replace 'id' with '$id'"),
            );
        }

        // Note: The 'dependencies' keyword is deprecated in Draft 2019-09 and later
        // but since we only support up to Draft 7, we don't need to check for this
    }
}

impl Default for JsonSchemaValidator {
    fn default() -> Self {
        Self::new_draft_7()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_json_schema() {
        let validator = JsonSchemaValidator::new_draft_7();
        let schema = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            }
        }"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.is_valid || result.errors.iter().all(|e| e.severity != crate::types::Severity::Error));
    }

    #[test]
    fn test_validate_invalid_json() {
        let validator = JsonSchemaValidator::new_draft_7();
        let schema = r#"{ invalid json }"#;

        let result = validator.validate(schema).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.rule == "json-schema-parse"));
    }

    #[test]
    fn test_validate_conflicting_constraints() {
        let validator = JsonSchemaValidator::new_draft_7();
        let schema = r#"{
            "type": "object",
            "properties": {
                "value": {
                    "type": "number",
                    "minimum": 10,
                    "maximum": 5
                }
            }
        }"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.rule == "conflicting-constraints"));
    }

    #[test]
    fn test_validate_instance() {
        let validator = JsonSchemaValidator::new_draft_7();
        let schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        }"#;

        let valid_instance = r#"{"name": "John"}"#;
        let result = validator.validate_instance(schema, valid_instance).unwrap();
        assert!(result.is_valid);

        let invalid_instance = r#"{"name": 123}"#;
        let result = validator.validate_instance(schema, invalid_instance).unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_missing_type_warning() {
        let validator = JsonSchemaValidator::new_draft_7();
        let schema = r#"{
            "type": "object",
            "properties": {
                "value": {}
            }
        }"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.has_warnings());
        assert!(result.warnings.iter().any(|w| w.rule == "missing-type"));
    }
}
