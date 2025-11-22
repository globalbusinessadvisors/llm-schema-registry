//! Protocol Buffers validator
//!
//! Validates Protocol Buffers schemas (proto2 and proto3).

use crate::types::{ValidationError, ValidationResult, ValidationWarning, SchemaFormat};
use anyhow::Result;
use regex::Regex;
use once_cell::sync::Lazy;

// Regex patterns for protobuf validation
static SYNTAX_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"syntax\s*=\s*"(proto[23])"\s*;"#).unwrap()
});

static MESSAGE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"message\s+([A-Z][A-Za-z0-9_]*)\s*\{").unwrap()
});

static FIELD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(required|optional|repeated)?\s*([a-z][a-z0-9_]*)\s+([a-z][A-Za-z0-9_]*)\s*=\s*(\d+)").unwrap()
});

static ENUM_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"enum\s+([A-Z][A-Za-z0-9_]*)\s*\{").unwrap()
});

static PACKAGE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"package\s+([a-z][a-z0-9_.]*)\s*;").unwrap()
});

/// Protocol Buffers validator
pub struct ProtobufValidator;

impl ProtobufValidator {
    /// Creates a new Protobuf validator
    pub fn new() -> Self {
        Self
    }

    /// Validates a Protocol Buffers schema
    pub fn validate(&self, schema: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success(SchemaFormat::Protobuf);

        // Detect and validate syntax declaration
        self.validate_syntax(schema, &mut result);

        // Validate package declaration
        self.validate_package(schema, &mut result);

        // Validate message definitions
        self.validate_messages(schema, &mut result);

        // Validate enum definitions
        self.validate_enums(schema, &mut result);

        // Validate field numbers
        self.validate_field_numbers(schema, &mut result);

        // Validate naming conventions
        self.validate_naming_conventions(schema, &mut result);

        // Check for reserved fields
        self.validate_reserved_fields(schema, &mut result);

        Ok(result)
    }

    /// Validates the syntax declaration
    fn validate_syntax(&self, schema: &str, result: &mut ValidationResult) {
        if let Some(captures) = SYNTAX_REGEX.captures(schema) {
            let syntax = &captures[1];
            if syntax != "proto2" && syntax != "proto3" {
                result.add_error(
                    ValidationError::new(
                        "protobuf-syntax",
                        format!("Invalid protobuf syntax: {}", syntax),
                    )
                    .with_suggestion("Use 'proto2' or 'proto3'"),
                );
            }

            // Check if it's the first non-comment line
            let trimmed = schema.trim_start();
            if !trimmed.starts_with("syntax") && !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                result.add_warning(
                    ValidationWarning::new(
                        "protobuf-syntax-position",
                        "Syntax declaration should be the first non-comment line",
                    ),
                );
            }
        } else {
            result.add_warning(
                ValidationWarning::new(
                    "protobuf-missing-syntax",
                    "Missing syntax declaration",
                )
                .with_suggestion("Add 'syntax = \"proto3\";' at the beginning of the file"),
            );
        }
    }

    /// Validates the package declaration
    fn validate_package(&self, schema: &str, result: &mut ValidationResult) {
        if let Some(captures) = PACKAGE_REGEX.captures(schema) {
            let package = &captures[1];

            // Validate package name format
            if package.is_empty() {
                result.add_error(
                    ValidationError::new(
                        "protobuf-package",
                        "Package name cannot be empty",
                    ),
                );
            }

            // Check package naming convention (lowercase with dots)
            for part in package.split('.') {
                if !part.chars().all(|c| c.is_lowercase() || c.is_numeric() || c == '_') {
                    result.add_warning(
                        ValidationWarning::new(
                            "protobuf-package-naming",
                            format!("Package part '{}' should be lowercase", part),
                        )
                        .with_suggestion("Use lowercase letters, numbers, and underscores"),
                    );
                }
            }
        } else {
            result.add_warning(
                ValidationWarning::new(
                    "protobuf-missing-package",
                    "Missing package declaration",
                )
                .with_suggestion("Add a package declaration to avoid naming conflicts"),
            );
        }
    }

    /// Validates message definitions
    fn validate_messages(&self, schema: &str, result: &mut ValidationResult) {
        let message_count = MESSAGE_REGEX.captures_iter(schema).count();

        if message_count == 0 {
            result.add_warning(
                ValidationWarning::new(
                    "protobuf-no-messages",
                    "Schema contains no message definitions",
                )
                .with_suggestion("Add at least one message definition"),
            );
        }

        // Validate message names
        for captures in MESSAGE_REGEX.captures_iter(schema) {
            let message_name = &captures[1];

            // Check PascalCase
            if !self.is_pascal_case(message_name) {
                result.add_warning(
                    ValidationWarning::new(
                        "protobuf-message-naming",
                        format!("Message name '{}' should be PascalCase", message_name),
                    ),
                );
            }

            result.metrics.fields_validated += 1;
        }
    }

    /// Validates enum definitions
    fn validate_enums(&self, schema: &str, result: &mut ValidationResult) {
        for captures in ENUM_REGEX.captures_iter(schema) {
            let enum_name = &captures[1];

            // Check PascalCase
            if !self.is_pascal_case(enum_name) {
                result.add_warning(
                    ValidationWarning::new(
                        "protobuf-enum-naming",
                        format!("Enum name '{}' should be PascalCase", enum_name),
                    ),
                );
            }
        }
    }

    /// Validates field numbers
    fn validate_field_numbers(&self, schema: &str, result: &mut ValidationResult) {
        let mut field_numbers: std::collections::HashMap<String, Vec<u32>> = std::collections::HashMap::new();

        // Extract message names and their field numbers
        let lines: Vec<&str> = schema.lines().collect();
        let mut current_message = String::new();
        let mut in_message = false;

        for line in lines {
            let line = line.trim();

            // Check for message start
            if let Some(captures) = MESSAGE_REGEX.captures(line) {
                current_message = captures[1].to_string();
                in_message = true;
                field_numbers.insert(current_message.clone(), Vec::new());
            }

            // Check for message end
            if line.starts_with('}') {
                in_message = false;
            }

            // Extract field numbers
            if in_message && !current_message.is_empty() {
                if let Some(field_num) = self.extract_field_number(line) {
                    field_numbers.get_mut(&current_message).unwrap().push(field_num);

                    // Validate field number range
                    if field_num == 0 {
                        result.add_error(
                            ValidationError::new(
                                "protobuf-field-number",
                                format!("Field number cannot be 0 in message '{}'", current_message),
                            )
                            .with_suggestion("Use field numbers starting from 1"),
                        );
                    }

                    // Reserved range: 19000-19999
                    if (19000..=19999).contains(&field_num) {
                        result.add_error(
                            ValidationError::new(
                                "protobuf-reserved-range",
                                format!(
                                    "Field number {} is in the reserved range (19000-19999) in message '{}'",
                                    field_num, current_message
                                ),
                            )
                            .with_suggestion("Use field numbers outside the reserved range"),
                        );
                    }

                    // Warn about high field numbers (inefficient)
                    if field_num > 536870911 {
                        result.add_error(
                            ValidationError::new(
                                "protobuf-field-number-max",
                                format!("Field number {} exceeds maximum (536870911)", field_num),
                            ),
                        );
                    }
                }
            }
        }

        // Check for duplicate field numbers
        for (message_name, numbers) in field_numbers {
            let mut seen = std::collections::HashSet::new();
            for num in numbers {
                if !seen.insert(num) {
                    result.add_error(
                        ValidationError::new(
                            "protobuf-duplicate-field-number",
                            format!(
                                "Duplicate field number {} in message '{}'",
                                num, message_name
                            ),
                        )
                        .with_suggestion("Ensure all field numbers are unique within a message"),
                    );
                }
            }
        }
    }

    /// Extracts field number from a line
    fn extract_field_number(&self, line: &str) -> Option<u32> {
        // Match field definitions: type name = number;
        let re = Regex::new(r"=\s*(\d+)\s*[;\[]").unwrap();
        if let Some(captures) = re.captures(line) {
            captures[1].parse().ok()
        } else {
            None
        }
    }

    /// Validates naming conventions
    fn validate_naming_conventions(&self, schema: &str, result: &mut ValidationResult) {
        // Validate field names (should be snake_case)
        let field_re = Regex::new(r"\b([a-z][a-z0-9_]*)\s+([a-z][A-Za-z0-9_]*)\s*=\s*\d+").unwrap();

        for captures in field_re.captures_iter(schema) {
            let field_name = &captures[2];

            if !self.is_snake_case(field_name) {
                result.add_warning(
                    ValidationWarning::new(
                        "protobuf-field-naming",
                        format!("Field name '{}' should be snake_case", field_name),
                    )
                    .with_suggestion("Use lowercase letters with underscores"),
                );
            }
        }
    }

    /// Validates reserved fields
    fn validate_reserved_fields(&self, schema: &str, result: &mut ValidationResult) {
        // Check for reserved keyword usage
        let reserved_re = Regex::new(r#"\breserved\s+([\d\s,\-]+|"[^"]+");"#).unwrap();

        for captures in reserved_re.captures_iter(schema) {
            let reserved = &captures[1];

            // Parse reserved field numbers
            if reserved.contains(char::is_numeric) {
                // Validate reserved number format
                for part in reserved.split(',') {
                    let part = part.trim();
                    if part.contains("to") {
                        // Range format: 5 to 10
                        let range_parts: Vec<&str> = part.split("to").collect();
                        if range_parts.len() == 2 {
                            if let (Ok(start), Ok(end)) = (
                                range_parts[0].trim().parse::<u32>(),
                                range_parts[1].trim().parse::<u32>(),
                            ) {
                                if start >= end {
                                    result.add_error(
                                        ValidationError::new(
                                            "protobuf-reserved-range",
                                            format!("Invalid reserved range: {} to {}", start, end),
                                        )
                                        .with_suggestion("Start of range must be less than end"),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Checks if a string is PascalCase
    fn is_pascal_case(&self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        let first_char = s.chars().next().unwrap();
        if !first_char.is_uppercase() {
            return false;
        }

        // Should not contain underscores (use camelCase instead)
        !s.contains('_')
    }

    /// Checks if a string is snake_case
    fn is_snake_case(&self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Should start with lowercase
        let first_char = s.chars().next().unwrap();
        if !first_char.is_lowercase() {
            return false;
        }

        // Should only contain lowercase letters, numbers, and underscores
        s.chars().all(|c| c.is_lowercase() || c.is_numeric() || c == '_')
    }
}

impl Default for ProtobufValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_protobuf() {
        let validator = ProtobufValidator::new();
        let schema = r#"
syntax = "proto3";

package example;

message User {
  int64 id = 1;
  string username = 2;
  string email = 3;
}
"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_missing_syntax() {
        let validator = ProtobufValidator::new();
        let schema = r#"
package example;

message User {
  int64 id = 1;
}
"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.has_warnings());
        assert!(result.warnings.iter().any(|w| w.rule == "protobuf-missing-syntax"));
    }

    #[test]
    fn test_validate_invalid_field_number() {
        let validator = ProtobufValidator::new();
        let schema = r#"
syntax = "proto3";

message Test {
  string field = 0;
}
"#;

        let result = validator.validate(schema).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.rule == "protobuf-field-number"));
    }

    #[test]
    fn test_validate_reserved_range() {
        let validator = ProtobufValidator::new();
        let schema = r#"
syntax = "proto3";

message Test {
  string field = 19500;
}
"#;

        let result = validator.validate(schema).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.rule == "protobuf-reserved-range"));
    }

    #[test]
    fn test_validate_duplicate_field_numbers() {
        let validator = ProtobufValidator::new();
        let schema = r#"
syntax = "proto3";

message Test {
  string field1 = 1;
  string field2 = 1;
}
"#;

        let result = validator.validate(schema).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.rule == "protobuf-duplicate-field-number"));
    }

    #[test]
    fn test_validate_naming_conventions() {
        let validator = ProtobufValidator::new();
        let schema = r#"
syntax = "proto3";

message user_profile {
  string UserName = 1;
}
"#;

        let result = validator.validate(schema).unwrap();
        assert!(result.has_warnings());
    }

    #[test]
    fn test_is_pascal_case() {
        let validator = ProtobufValidator::new();
        assert!(validator.is_pascal_case("UserProfile"));
        assert!(validator.is_pascal_case("User"));
        assert!(!validator.is_pascal_case("userProfile"));
        assert!(!validator.is_pascal_case("user_profile"));
        assert!(!validator.is_pascal_case(""));
    }

    #[test]
    fn test_is_snake_case() {
        let validator = ProtobufValidator::new();
        assert!(validator.is_snake_case("user_name"));
        assert!(validator.is_snake_case("id"));
        assert!(validator.is_snake_case("user_id_123"));
        assert!(!validator.is_snake_case("UserName"));
        assert!(!validator.is_snake_case("userName"));
        assert!(!validator.is_snake_case(""));
    }
}
