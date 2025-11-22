//! Schema format detection
//!
//! Automatically detects whether a schema is JSON Schema, Apache Avro, or Protocol Buffers
//! based on content analysis.

use crate::types::SchemaFormat;
use anyhow::{anyhow, Result};
use serde_json::Value;

/// Detects the format of a schema from its content
pub fn detect_format(content: &str) -> Result<SchemaFormat> {
    // Try to detect based on content patterns
    if is_protobuf(content) {
        return Ok(SchemaFormat::Protobuf);
    }

    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<Value>(content) {
        if is_avro_json(&json) {
            return Ok(SchemaFormat::Avro);
        }
        if is_json_schema(&json) {
            return Ok(SchemaFormat::JsonSchema);
        }

        // Default to JSON Schema if it's valid JSON but we can't determine
        return Ok(SchemaFormat::JsonSchema);
    }

    Err(anyhow!("Unable to detect schema format"))
}

/// Checks if JSON content is an Avro schema
fn is_avro_json(json: &Value) -> bool {
    // Avro schemas have a "type" field that can be:
    // - A primitive type: "null", "boolean", "int", "long", "float", "double", "bytes", "string"
    // - A complex type: "record", "enum", "array", "map", "fixed", "union"

    if let Some(type_field) = json.get("type") {
        if let Some(type_str) = type_field.as_str() {
            // Check for Avro-specific types
            match type_str {
                "record" | "enum" | "fixed" => return true,
                "null" | "boolean" | "int" | "long" | "float" | "double"
                | "bytes" | "string" => {
                    // Could be Avro or JSON Schema, check for Avro-specific fields
                    return json.get("namespace").is_some()
                        || json.get("fields").is_some()
                        || json.get("symbols").is_some();
                }
                _ => {}
            }
        }
    }

    // Check for array type (Avro union)
    if json.is_array() {
        return true;
    }

    false
}

/// Checks if JSON content is a JSON Schema
fn is_json_schema(json: &Value) -> bool {
    // JSON Schema indicators
    if json.get("$schema").is_some() {
        return true;
    }

    if json.get("properties").is_some() {
        return true;
    }

    if json.get("definitions").is_some() || json.get("$defs").is_some() {
        return true;
    }

    // Check for JSON Schema specific keywords
    let json_schema_keywords = [
        "allOf", "anyOf", "oneOf", "not",
        "items", "additionalProperties",
        "required", "minLength", "maxLength",
        "minimum", "maximum", "pattern",
        "enum", "const", "format",
    ];

    for keyword in &json_schema_keywords {
        if json.get(keyword).is_some() {
            return true;
        }
    }

    false
}

/// Checks if content is Protocol Buffers
fn is_protobuf(content: &str) -> bool {
    // Protobuf files typically contain:
    // - syntax declaration: syntax = "proto3";
    // - message definitions
    // - package declarations

    let content_lower = content.to_lowercase();

    // Check for syntax declaration
    if content_lower.contains("syntax") &&
       (content_lower.contains("proto2") || content_lower.contains("proto3")) {
        return true;
    }

    // Check for message keyword
    if content_lower.contains("message") && content_lower.contains("{") {
        return true;
    }

    // Check for package declaration
    if content_lower.contains("package") && content_lower.contains(";") {
        return true;
    }

    false
}

/// Validates that the schema content matches the specified format
pub fn validate_format(content: &str, expected_format: SchemaFormat) -> Result<()> {
    let detected = detect_format(content)?;

    if detected != expected_format {
        return Err(anyhow!(
            "Schema format mismatch: expected {}, detected {}",
            expected_format.as_str(),
            detected.as_str()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_json_schema() {
        let schema = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }"#;

        let format = detect_format(schema).unwrap();
        assert_eq!(format, SchemaFormat::JsonSchema);
    }

    #[test]
    fn test_detect_json_schema_without_dollar_schema() {
        let schema = r#"{
            "type": "object",
            "properties": {
                "age": {"type": "number"}
            },
            "required": ["age"]
        }"#;

        let format = detect_format(schema).unwrap();
        assert_eq!(format, SchemaFormat::JsonSchema);
    }

    #[test]
    fn test_detect_avro_record() {
        let schema = r#"{
            "type": "record",
            "name": "User",
            "namespace": "com.example",
            "fields": [
                {"name": "id", "type": "long"},
                {"name": "username", "type": "string"}
            ]
        }"#;

        let format = detect_format(schema).unwrap();
        assert_eq!(format, SchemaFormat::Avro);
    }

    #[test]
    fn test_detect_avro_enum() {
        let schema = r#"{
            "type": "enum",
            "name": "Status",
            "symbols": ["PENDING", "ACTIVE", "INACTIVE"]
        }"#;

        let format = detect_format(schema).unwrap();
        assert_eq!(format, SchemaFormat::Avro);
    }

    #[test]
    fn test_detect_avro_union() {
        let schema = r#"["null", "string"]"#;

        let format = detect_format(schema).unwrap();
        assert_eq!(format, SchemaFormat::Avro);
    }

    #[test]
    fn test_detect_protobuf_with_syntax() {
        let schema = r#"
syntax = "proto3";

package example;

message User {
  int64 id = 1;
  string username = 2;
}
"#;

        let format = detect_format(schema).unwrap();
        assert_eq!(format, SchemaFormat::Protobuf);
    }

    #[test]
    fn test_detect_protobuf_without_syntax() {
        let schema = r#"
package example;

message User {
  required int64 id = 1;
  optional string username = 2;
}
"#;

        let format = detect_format(schema).unwrap();
        assert_eq!(format, SchemaFormat::Protobuf);
    }

    #[test]
    fn test_validate_format_match() {
        let schema = r#"{"$schema": "http://json-schema.org/draft-07/schema#"}"#;
        let result = validate_format(schema, SchemaFormat::JsonSchema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_format_mismatch() {
        let schema = r#"{"type": "record", "name": "Test", "fields": []}"#;
        let result = validate_format(schema, SchemaFormat::JsonSchema);
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_invalid_schema() {
        let schema = "this is not a valid schema";
        let result = detect_format(schema);
        assert!(result.is_err());
    }
}
