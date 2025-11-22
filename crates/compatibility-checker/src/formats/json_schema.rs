//! JSON Schema compatibility checker
//!
//! Implements compatibility checking for JSON Schema as specified in PSEUDOCODE.md § 1.5

use crate::checker::CompatibilityError;
use crate::formats::FormatCompatibilityChecker;
use crate::violation::{CompatibilityViolation, ViolationType};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub struct JsonSchemaCompatibilityChecker;

impl JsonSchemaCompatibilityChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse JSON schema
    fn parse_schema(&self, schema_str: &str) -> Result<Value, CompatibilityError> {
        serde_json::from_str(schema_str)
            .map_err(|e| CompatibilityError::ParseError(format!("JSON parse error: {}", e)))
    }

    /// Extract properties from a schema
    fn extract_properties(&self, schema: &Value) -> HashMap<String, Value> {
        let mut properties = HashMap::new();

        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (name, value) in props {
                properties.insert(name.clone(), value.clone());
            }
        }

        properties
    }

    /// Get required fields from schema
    fn get_required_fields(&self, schema: &Value) -> HashSet<String> {
        schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if a field has a default value
    fn has_default(&self, field: &Value) -> bool {
        field.get("default").is_some()
    }

    /// Check if types are compatible
    fn are_types_compatible(&self, new_type: &Value, old_type: &Value) -> bool {
        // Exact match
        if new_type == old_type {
            return true;
        }

        // Type widening allowed
        let new_type_str = new_type.as_str().unwrap_or("");
        let old_type_str = old_type.as_str().unwrap_or("");

        // Allow number -> number (int to float is allowed in JSON Schema)
        if (old_type_str == "integer" && new_type_str == "number")
            || (old_type_str == "number" && new_type_str == "number")
        {
            return true;
        }

        // Arrays and objects need deeper comparison
        if new_type_str == "array" && old_type_str == "array" {
            // Check items compatibility
            if let (Some(new_items), Some(old_items)) =
                (new_type.get("items"), old_type.get("items"))
            {
                return self.are_types_compatible(new_items, old_items);
            }
            return true;
        }

        false
    }

    /// Check if constraints were tightened
    fn is_constraint_tightened(&self, new_field: &Value, old_field: &Value) -> bool {
        // Check minimum/maximum for numbers
        if let (Some(new_min), Some(old_min)) = (
            new_field.get("minimum").and_then(|v| v.as_f64()),
            old_field.get("minimum").and_then(|v| v.as_f64()),
        ) {
            if new_min > old_min {
                return true;
            }
        }

        if let (Some(new_max), Some(old_max)) = (
            new_field.get("maximum").and_then(|v| v.as_f64()),
            old_field.get("maximum").and_then(|v| v.as_f64()),
        ) {
            if new_max < old_max {
                return true;
            }
        }

        // Check string length constraints
        if let (Some(new_min_len), Some(old_min_len)) = (
            new_field.get("minLength").and_then(|v| v.as_u64()),
            old_field.get("minLength").and_then(|v| v.as_u64()),
        ) {
            if new_min_len > old_min_len {
                return true;
            }
        }

        if let (Some(new_max_len), Some(old_max_len)) = (
            new_field.get("maxLength").and_then(|v| v.as_u64()),
            old_field.get("maxLength").and_then(|v| v.as_u64()),
        ) {
            if new_max_len < old_max_len {
                return true;
            }
        }

        // Check pattern constraints
        if let (Some(new_pattern), Some(old_pattern)) = (
            new_field.get("pattern").and_then(|v| v.as_str()),
            old_field.get("pattern").and_then(|v| v.as_str()),
        ) {
            if new_pattern != old_pattern {
                return true; // Changed pattern is considered tightening
            }
        }

        // Check enum constraints
        if let (Some(new_enum), Some(old_enum)) = (
            new_field.get("enum").and_then(|v| v.as_array()),
            old_field.get("enum").and_then(|v| v.as_array()),
        ) {
            let new_set: HashSet<_> = new_enum.iter().collect();
            let old_set: HashSet<_> = old_enum.iter().collect();

            // If new enum is not a superset of old, it's tightened
            if !old_set.iter().all(|v| new_set.contains(v)) {
                return true;
            }
        }

        false
    }
}

impl FormatCompatibilityChecker for JsonSchemaCompatibilityChecker {
    /// Check backward compatibility for JSON Schema
    ///
    /// Rules:
    /// 1. Cannot remove fields without default values
    /// 2. Cannot change field types incompatibly
    /// 3. Cannot add new required fields without defaults
    /// 4. Cannot tighten constraints
    fn check_backward(
        &self,
        new_schema: &str,
        old_schema: &str,
    ) -> Result<Vec<CompatibilityViolation>, CompatibilityError> {
        let new = self.parse_schema(new_schema)?;
        let old = self.parse_schema(old_schema)?;

        let mut violations = Vec::new();

        let new_properties = self.extract_properties(&new);
        let old_properties = self.extract_properties(&old);
        let new_required = self.get_required_fields(&new);
        let old_required = self.get_required_fields(&old);

        // Rule 1: Cannot remove fields without default values
        for (old_field_name, old_field_value) in &old_properties {
            if !new_properties.contains_key(old_field_name) {
                // Field was removed
                if !self.has_default(old_field_value) {
                    violations.push(
                        CompatibilityViolation::breaking(
                            ViolationType::FieldRemoved,
                            format!("properties.{}", old_field_name),
                            format!(
                                "Field '{}' removed without default value",
                                old_field_name
                            ),
                        )
                        .with_values(Some(old_field_value.clone()), None),
                    );
                }
            }
        }

        // Rule 2: Cannot change field types incompatibly
        for (new_field_name, new_field_value) in &new_properties {
            if let Some(old_field_value) = old_properties.get(new_field_name) {
                let new_type = new_field_value.get("type");
                let old_type = old_field_value.get("type");

                if let (Some(new_t), Some(old_t)) = (new_type, old_type) {
                    if !self.are_types_compatible(new_t, old_t) {
                        violations.push(
                            CompatibilityViolation::breaking(
                                ViolationType::TypeChanged,
                                format!("properties.{}.type", new_field_name),
                                format!(
                                    "Type changed from {:?} to {:?} for field '{}'",
                                    old_t, new_t, new_field_name
                                ),
                            )
                            .with_values(Some(old_t.clone()), Some(new_t.clone())),
                        );
                    }
                }
            }
        }

        // Rule 3: Cannot add new required fields without defaults
        for new_field_name in &new_required {
            if !old_required.contains(new_field_name) {
                // Field became required or is a new required field
                if let Some(new_field_value) = new_properties.get(new_field_name) {
                    if !old_properties.contains_key(new_field_name) {
                        // New field that is required
                        if !self.has_default(new_field_value) {
                            violations.push(
                                CompatibilityViolation::breaking(
                                    ViolationType::RequiredAdded,
                                    format!("properties.{}", new_field_name),
                                    format!(
                                        "New required field '{}' added without default",
                                        new_field_name
                                    ),
                                )
                                .with_values(None, Some(new_field_value.clone())),
                            );
                        }
                    } else {
                        // Existing field became required
                        violations.push(
                            CompatibilityViolation::breaking(
                                ViolationType::FieldMadeRequired,
                                format!("required.{}", new_field_name),
                                format!("Field '{}' made required", new_field_name),
                            )
                            .with_values(
                                Some(serde_json::json!(false)),
                                Some(serde_json::json!(true)),
                            ),
                        );
                    }
                }
            }
        }

        // Rule 4: Cannot tighten constraints
        for (new_field_name, new_field_value) in &new_properties {
            if let Some(old_field_value) = old_properties.get(new_field_name) {
                if self.is_constraint_tightened(new_field_value, old_field_value) {
                    violations.push(
                        CompatibilityViolation::breaking(
                            ViolationType::ConstraintAdded,
                            format!("properties.{}", new_field_name),
                            format!("Constraints tightened on field '{}'", new_field_name),
                        )
                        .with_values(Some(old_field_value.clone()), Some(new_field_value.clone())),
                    );
                }
            }
        }

        Ok(violations)
    }

    /// Check forward compatibility for JSON Schema
    ///
    /// Forward compatibility is checked by swapping new and old schemas
    /// and checking backward compatibility
    fn check_forward(
        &self,
        new_schema: &str,
        old_schema: &str,
    ) -> Result<Vec<CompatibilityViolation>, CompatibilityError> {
        // Forward compatibility: old schema can read new data
        // This is equivalent to checking if new schema is backward compatible with old
        self.check_backward(old_schema, new_schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_schemas_are_compatible() {
        let checker = JsonSchemaCompatibilityChecker::new();
        let schema = r#"{"type": "object", "properties": {"field1": {"type": "string"}}}"#;

        let violations = checker.check_backward(schema, schema).unwrap();
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_field_removal_without_default_is_breaking() {
        let checker = JsonSchemaCompatibilityChecker::new();

        let old_schema = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "string"},
                "field2": {"type": "string"}
            }
        }"#;

        let new_schema = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "string"}
            }
        }"#;

        let violations = checker.check_backward(new_schema, old_schema).unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::FieldRemoved);
    }

    #[test]
    fn test_adding_optional_field_is_compatible() {
        let checker = JsonSchemaCompatibilityChecker::new();

        let old_schema = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "string"}
            }
        }"#;

        let new_schema = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "string"},
                "field2": {"type": "string"}
            }
        }"#;

        let violations = checker.check_backward(new_schema, old_schema).unwrap();
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_adding_required_field_without_default_is_breaking() {
        let checker = JsonSchemaCompatibilityChecker::new();

        let old_schema = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "string"}
            }
        }"#;

        let new_schema = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "string"},
                "field2": {"type": "string"}
            },
            "required": ["field2"]
        }"#;

        let violations = checker.check_backward(new_schema, old_schema).unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::RequiredAdded);
    }

    #[test]
    fn test_type_change_is_breaking() {
        let checker = JsonSchemaCompatibilityChecker::new();

        let old_schema = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "string"}
            }
        }"#;

        let new_schema = r#"{
            "type": "object",
            "properties": {
                "field1": {"type": "number"}
            }
        }"#;

        let violations = checker.check_backward(new_schema, old_schema).unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::TypeChanged);
    }
}
