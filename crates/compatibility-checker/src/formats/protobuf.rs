//! Protocol Buffers compatibility checker
//!
//! Implements compatibility checking for Protocol Buffers
//! Focuses on field numbers, types, and required/optional changes

use crate::checker::CompatibilityError;
use crate::formats::FormatCompatibilityChecker;
use crate::violation::{CompatibilityViolation, ViolationType};

pub struct ProtobufCompatibilityChecker;

impl ProtobufCompatibilityChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse protobuf schema (simplified - in production would use protoc)
    /// For now, we'll do basic validation and structure checks
    fn parse_schema(&self, schema_str: &str) -> Result<ProtoSchema, CompatibilityError> {
        // This is a simplified parser for demonstration
        // In production, you would use prost-build or protoc

        let mut fields = Vec::new();
        let mut in_message = false;
        let mut message_name = String::new();

        for line in schema_str.lines() {
            let line = line.trim();

            if line.starts_with("message ") {
                in_message = true;
                message_name = line
                    .strip_prefix("message ")
                    .and_then(|s| s.strip_suffix(" {"))
                    .unwrap_or("")
                    .trim()
                    .to_string();
            } else if line == "}" {
                in_message = false;
            } else if in_message && !line.is_empty() && !line.starts_with("//") {
                // Parse field: [optional|required|repeated] type name = number;
                if let Some(field) = self.parse_field(line) {
                    fields.push(field);
                }
            }
        }

        Ok(ProtoSchema {
            message_name,
            fields,
        })
    }

    /// Parse a protobuf field line
    fn parse_field(&self, line: &str) -> Option<ProtoField> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 4 {
            return None;
        }

        let mut idx = 0;

        // Check for label (optional, required, repeated)
        let label = if parts[idx] == "optional" || parts[idx] == "required" || parts[idx] == "repeated" {
            let l = parts[idx].to_string();
            idx += 1;
            Some(l)
        } else {
            None
        };

        let field_type = parts[idx].to_string();
        idx += 1;
        let name = parts[idx].to_string();
        idx += 1;

        // Skip '='
        if parts[idx] != "=" {
            return None;
        }
        idx += 1;

        // Get field number
        let number_str = parts[idx].trim_end_matches(';');
        let number = number_str.parse::<u32>().ok()?;

        Some(ProtoField {
            label,
            field_type,
            name,
            number,
        })
    }

    /// Check if proto types are compatible
    fn are_types_compatible(&self, new_type: &str, old_type: &str) -> bool {
        // Exact match
        if new_type == old_type {
            return true;
        }

        // Proto3 compatible type changes
        // See: https://protobuf.dev/programming-guides/proto3/#updating

        // int32, uint32, int64, uint64, and bool are compatible
        let numeric_types = ["int32", "uint32", "int64", "uint64", "bool"];
        if numeric_types.contains(&new_type) && numeric_types.contains(&old_type) {
            return true;
        }

        // sint32 and sint64 are compatible with each other
        if (new_type == "sint32" && old_type == "sint64")
            || (new_type == "sint64" && old_type == "sint32")
        {
            return true;
        }

        // string and bytes are compatible
        if (new_type == "string" && old_type == "bytes")
            || (new_type == "bytes" && old_type == "string")
        {
            return true;
        }

        false
    }
}

impl FormatCompatibilityChecker for ProtobufCompatibilityChecker {
    /// Check backward compatibility for Protocol Buffers
    ///
    /// Rules:
    /// 1. Cannot change field numbers
    /// 2. Cannot change field types incompatibly
    /// 3. Cannot change field from optional to required
    /// 4. Can add/remove optional fields
    /// 5. Can add/remove repeated fields
    fn check_backward(
        &self,
        new_schema: &str,
        old_schema: &str,
    ) -> Result<Vec<CompatibilityViolation>, CompatibilityError> {
        let new = self.parse_schema(new_schema)?;
        let old = self.parse_schema(old_schema)?;

        let mut violations = Vec::new();

        // Build maps for easy lookup
        let mut new_fields_by_number: std::collections::HashMap<u32, &ProtoField> =
            std::collections::HashMap::new();
        let mut new_fields_by_name: std::collections::HashMap<&str, &ProtoField> =
            std::collections::HashMap::new();

        for field in &new.fields {
            new_fields_by_number.insert(field.number, field);
            new_fields_by_name.insert(&field.name, field);
        }

        // Check old fields
        for old_field in &old.fields {
            if let Some(new_field) = new_fields_by_number.get(&old_field.number) {
                // Field exists with same number

                // Rule 1: Check if name changed (warning, not breaking)
                if old_field.name != new_field.name {
                    violations.push(CompatibilityViolation::warning(
                        ViolationType::NameChanged,
                        format!("field.{}", old_field.number),
                        format!(
                            "Field number {} name changed from '{}' to '{}'",
                            old_field.number, old_field.name, new_field.name
                        ),
                    ));
                }

                // Rule 2: Check type compatibility
                if !self.are_types_compatible(&new_field.field_type, &old_field.field_type) {
                    violations.push(CompatibilityViolation::breaking(
                        ViolationType::TypeChanged,
                        format!("field.{}.type", old_field.number),
                        format!(
                            "Field '{}' type changed from '{}' to '{}'",
                            old_field.name, old_field.field_type, new_field.field_type
                        ),
                    ));
                }

                // Rule 3: Check for optional -> required change
                if let (Some(ref old_label), Some(ref new_label)) =
                    (&old_field.label, &new_field.label)
                {
                    if old_label == "optional" && new_label == "required" {
                        violations.push(CompatibilityViolation::breaking(
                            ViolationType::FieldMadeRequired,
                            format!("field.{}.label", old_field.number),
                            format!("Field '{}' changed from optional to required", old_field.name),
                        ));
                    }
                }
            } else if !new_fields_by_name.contains_key(old_field.name.as_str()) {
                // Field was completely removed (number reused or field deleted)
                // This is OK in proto3 for optional fields
                if let Some(ref label) = old_field.label {
                    if label == "required" {
                        violations.push(CompatibilityViolation::breaking(
                            ViolationType::FieldRemoved,
                            format!("field.{}", old_field.number),
                            format!("Required field '{}' was removed", old_field.name),
                        ));
                    }
                }
            }
        }

        // Check for field number reuse (critical error)
        for new_field in &new.fields {
            if let Some(old_field) = old
                .fields
                .iter()
                .find(|f| f.number == new_field.number && f.name != new_field.name)
            {
                violations.push(CompatibilityViolation::breaking(
                    ViolationType::Custom("FieldNumberReused".to_string()),
                    format!("field.{}", new_field.number),
                    format!(
                        "Field number {} reused: was '{}', now '{}'",
                        new_field.number, old_field.name, new_field.name
                    ),
                ));
            }
        }

        Ok(violations)
    }

    /// Check forward compatibility for Protocol Buffers
    fn check_forward(
        &self,
        new_schema: &str,
        old_schema: &str,
    ) -> Result<Vec<CompatibilityViolation>, CompatibilityError> {
        // Forward: old schema can read new data
        self.check_backward(old_schema, new_schema)
    }
}

/// Simplified protobuf schema representation
#[derive(Debug, Clone)]
struct ProtoSchema {
    message_name: String,
    fields: Vec<ProtoField>,
}

#[derive(Debug, Clone)]
struct ProtoField {
    label: Option<String>, // optional, required, repeated
    field_type: String,
    name: String,
    number: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_proto_schemas_are_compatible() {
        let checker = ProtobufCompatibilityChecker::new();
        let schema = r#"
            message Test {
                optional string field1 = 1;
            }
        "#;

        let violations = checker.check_backward(schema, schema).unwrap();
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_adding_optional_field_is_compatible() {
        let checker = ProtobufCompatibilityChecker::new();

        let old_schema = r#"
            message Test {
                optional string field1 = 1;
            }
        "#;

        let new_schema = r#"
            message Test {
                optional string field1 = 1;
                optional string field2 = 2;
            }
        "#;

        let violations = checker.check_backward(new_schema, old_schema).unwrap();
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_field_number_reuse_is_breaking() {
        let checker = ProtobufCompatibilityChecker::new();

        let old_schema = r#"
            message Test {
                optional string field1 = 1;
            }
        "#;

        let new_schema = r#"
            message Test {
                optional string field2 = 1;
            }
        "#;

        let violations = checker.check_backward(new_schema, old_schema).unwrap();
        assert!(violations.len() > 0);
        assert!(violations
            .iter()
            .any(|v| matches!(v.violation_type, ViolationType::Custom(_))));
    }

    #[test]
    fn test_compatible_type_change() {
        let checker = ProtobufCompatibilityChecker::new();

        let old_schema = r#"
            message Test {
                optional int32 field1 = 1;
            }
        "#;

        let new_schema = r#"
            message Test {
                optional int64 field1 = 1;
            }
        "#;

        let violations = checker.check_backward(new_schema, old_schema).unwrap();
        // int32 to int64 should be compatible
        assert_eq!(violations.len(), 0);
    }
}
