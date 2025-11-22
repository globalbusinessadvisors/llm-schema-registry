//! Core validation engine
//!
//! Implements the 7-step validation pipeline:
//! 1. Structural validation (valid syntax)
//! 2. Type validation (correct types)
//! 3. Semantic validation (logical consistency)
//! 4. Compatibility validation (with existing versions)
//! 5. Security validation (no malicious content)
//! 6. Performance validation (complexity limits)
//! 7. Custom rule validation (extensible rules)

use crate::types::{
    SchemaFormat, ValidationConfig, ValidationError, ValidationResult, ValidationWarning, Severity,
};
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;

/// A validation rule that can be applied to a schema
pub trait ValidationRule: Send + Sync {
    /// The name of the validation rule
    fn name(&self) -> &str;

    /// The severity of violations from this rule
    fn severity(&self) -> Severity;

    /// Validates a schema and returns errors/warnings
    fn validate(&self, schema: &str, format: SchemaFormat) -> Result<Vec<ValidationError>>;
}

/// The main validation engine
pub struct ValidationEngine {
    /// Validation configuration
    config: ValidationConfig,
    /// Custom validation rules
    custom_rules: Vec<Arc<dyn ValidationRule>>,
}

impl ValidationEngine {
    /// Creates a new validation engine with default configuration
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
            custom_rules: Vec::new(),
        }
    }

    /// Creates a new validation engine with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            config,
            custom_rules: Vec::new(),
        }
    }

    /// Adds a custom validation rule
    pub fn add_rule(&mut self, rule: Arc<dyn ValidationRule>) {
        self.custom_rules.push(rule);
    }

    /// Validates a schema using the 7-step pipeline
    pub async fn validate(&self, schema: &str, format: SchemaFormat) -> Result<ValidationResult> {
        let start = Instant::now();
        let mut result = ValidationResult::success(format);

        // Check schema size
        let schema_size = schema.len();
        if schema_size > self.config.max_schema_size {
            result.add_error(
                ValidationError::new(
                    "schema-size",
                    format!(
                        "Schema size ({} bytes) exceeds maximum allowed size ({} bytes)",
                        schema_size, self.config.max_schema_size
                    ),
                )
                .with_suggestion("Split the schema into multiple smaller schemas"),
            );
            return Ok(result);
        }
        result.metrics.schema_size_bytes = schema_size;

        // Step 1: Structural validation
        if let Err(errors) = self.validate_structure(schema, format).await {
            result.merge(errors);
            if self.config.fail_fast && result.has_errors() {
                result.metrics.duration = start.elapsed();
                return Ok(result);
            }
        }

        // Step 2: Type validation
        if let Err(errors) = self.validate_types(schema, format).await {
            result.merge(errors);
            if self.config.fail_fast && result.has_errors() {
                result.metrics.duration = start.elapsed();
                return Ok(result);
            }
        }

        // Step 3: Semantic validation
        if let Err(errors) = self.validate_semantics(schema, format).await {
            result.merge(errors);
            if self.config.fail_fast && result.has_errors() {
                result.metrics.duration = start.elapsed();
                return Ok(result);
            }
        }

        // Step 4: Compatibility validation (skipped if no previous version)
        // This would be called separately with previous schema version

        // Step 5: Security validation
        if self.config.security_validation {
            if let Err(errors) = self.validate_security(schema, format).await {
                result.merge(errors);
                if self.config.fail_fast && result.has_errors() {
                    result.metrics.duration = start.elapsed();
                    return Ok(result);
                }
            }
        }

        // Step 6: Performance validation
        if self.config.performance_validation {
            if let Err(errors) = self.validate_performance(schema, format).await {
                result.merge(errors);
                if self.config.fail_fast && result.has_errors() {
                    result.metrics.duration = start.elapsed();
                    return Ok(result);
                }
            }
        }

        // Step 7: Custom rules validation
        for rule in &self.custom_rules {
            match rule.validate(schema, format) {
                Ok(errors) => {
                    for error in errors {
                        result.add_error(error);
                    }
                    result.metrics.rules_applied += 1;

                    if self.config.fail_fast && result.has_errors() {
                        result.metrics.duration = start.elapsed();
                        return Ok(result);
                    }
                }
                Err(e) => {
                    result.add_error(
                        ValidationError::new(rule.name(), format!("Rule execution failed: {}", e)),
                    );
                }
            }
        }

        // Filter warnings if not requested
        if !self.config.include_warnings {
            result.warnings.clear();
        }

        result.metrics.duration = start.elapsed();
        Ok(result)
    }

    /// Step 1: Validates the structural integrity of the schema
    async fn validate_structure(
        &self,
        schema: &str,
        format: SchemaFormat,
    ) -> Result<ValidationResult, ValidationResult> {
        let mut result = ValidationResult::success(format);
        result.metrics.rules_applied += 1;

        match format {
            SchemaFormat::JsonSchema => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(schema) {
                    result.add_error(
                        ValidationError::new(
                            "structural-validity",
                            format!("Invalid JSON: {}", e),
                        )
                        .with_suggestion("Ensure the schema is valid JSON"),
                    );
                }
            }
            SchemaFormat::Avro => {
                if let Err(e) = apache_avro::Schema::parse_str(schema) {
                    result.add_error(
                        ValidationError::new(
                            "structural-validity",
                            format!("Invalid Avro schema: {}", e),
                        )
                        .with_suggestion("Check Avro schema syntax"),
                    );
                }
            }
            SchemaFormat::Protobuf => {
                // Basic syntax check for protobuf
                if !schema.contains("message") && !schema.contains("enum") {
                    result.add_error(
                        ValidationError::new(
                            "structural-validity",
                            "Protobuf schema must contain at least one message or enum definition",
                        )
                        .with_suggestion("Add a message or enum definition"),
                    );
                }
            }
        }

        if result.has_errors() {
            Err(result)
        } else {
            Ok(result)
        }
    }

    /// Step 2: Validates types within the schema
    async fn validate_types(
        &self,
        schema: &str,
        format: SchemaFormat,
    ) -> Result<ValidationResult, ValidationResult> {
        let mut result = ValidationResult::success(format);
        result.metrics.rules_applied += 1;

        match format {
            SchemaFormat::JsonSchema => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(schema) {
                    self.validate_json_schema_types(&json, &mut result);
                }
            }
            SchemaFormat::Avro => {
                if let Ok(avro_schema) = apache_avro::Schema::parse_str(schema) {
                    self.validate_avro_types(&avro_schema, &mut result);
                }
            }
            SchemaFormat::Protobuf => {
                // Type validation for protobuf
                self.validate_protobuf_types(schema, &mut result);
            }
        }

        if result.has_errors() {
            Err(result)
        } else {
            Ok(result)
        }
    }

    /// Step 3: Validates semantic consistency
    async fn validate_semantics(
        &self,
        schema: &str,
        format: SchemaFormat,
    ) -> Result<ValidationResult, ValidationResult> {
        let mut result = ValidationResult::success(format);
        result.metrics.rules_applied += 1;

        match format {
            SchemaFormat::JsonSchema => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(schema) {
                    self.validate_json_schema_semantics(&json, &mut result);
                }
            }
            SchemaFormat::Avro => {
                // Semantic validation for Avro
                if let Ok(avro_schema) = apache_avro::Schema::parse_str(schema) {
                    self.validate_avro_semantics(&avro_schema, &mut result);
                }
            }
            SchemaFormat::Protobuf => {
                // Semantic validation for protobuf
                self.validate_protobuf_semantics(schema, &mut result);
            }
        }

        if result.has_errors() {
            Err(result)
        } else {
            Ok(result)
        }
    }

    /// Step 5: Validates security constraints
    async fn validate_security(
        &self,
        schema: &str,
        format: SchemaFormat,
    ) -> Result<ValidationResult, ValidationResult> {
        let mut result = ValidationResult::success(format);
        result.metrics.rules_applied += 1;

        // Check for suspicious patterns
        let suspicious_patterns = [
            ("eval", "Contains potentially dangerous eval keyword"),
            ("exec", "Contains potentially dangerous exec keyword"),
            ("__proto__", "Contains prototype pollution pattern"),
            ("constructor", "Contains constructor access pattern"),
        ];

        for (pattern, message) in &suspicious_patterns {
            if schema.to_lowercase().contains(pattern) {
                result.add_warning(
                    ValidationWarning::new("security-check", *message)
                        .with_suggestion("Review schema for security implications"),
                );
            }
        }

        // Check schema complexity (potential DoS)
        let nesting_level = self.calculate_nesting_depth(schema, format);
        if nesting_level > self.config.max_recursion_depth {
            result.add_error(
                ValidationError::new(
                    "security-complexity",
                    format!(
                        "Schema nesting depth ({}) exceeds maximum ({})",
                        nesting_level, self.config.max_recursion_depth
                    ),
                )
                .with_suggestion("Reduce schema nesting depth"),
            );
        }

        if result.has_errors() {
            Err(result)
        } else {
            Ok(result)
        }
    }

    /// Step 6: Validates performance constraints
    async fn validate_performance(
        &self,
        schema: &str,
        format: SchemaFormat,
    ) -> Result<ValidationResult, ValidationResult> {
        let mut result = ValidationResult::success(format);
        result.metrics.rules_applied += 1;

        // Check for performance issues
        match format {
            SchemaFormat::JsonSchema => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(schema) {
                    self.validate_json_schema_performance(&json, &mut result);
                }
            }
            _ => {
                // Basic performance checks for other formats
            }
        }

        if result.has_errors() {
            Err(result)
        } else {
            Ok(result)
        }
    }

    // Helper methods for specific format validations

    fn validate_json_schema_types(&self, json: &serde_json::Value, result: &mut ValidationResult) {
        if let Some(properties) = json.get("properties").and_then(|p| p.as_object()) {
            for (name, prop) in properties {
                if let Some(type_value) = prop.get("type") {
                    if let Some(type_str) = type_value.as_str() {
                        // Valid JSON Schema types
                        let valid_types = [
                            "null", "boolean", "object", "array", "number", "string", "integer",
                        ];
                        if !valid_types.contains(&type_str) {
                            result.add_error(
                                ValidationError::new(
                                    "type-validation",
                                    format!("Invalid type '{}' for property '{}'", type_str, name),
                                )
                                .with_location(format!("$.properties.{}.type", name))
                                .with_suggestion(format!(
                                    "Use one of: {}",
                                    valid_types.join(", ")
                                )),
                            );
                        }
                    }
                }
                result.metrics.fields_validated += 1;
            }
        }
    }

    fn validate_avro_types(&self, _schema: &apache_avro::Schema, result: &mut ValidationResult) {
        // Avro schema types are validated during parsing
        // Additional custom type validation can be added here
        result.metrics.fields_validated += 1;
    }

    fn validate_protobuf_types(&self, schema: &str, result: &mut ValidationResult) {
        // Check for valid protobuf types
        let valid_types = [
            "double", "float", "int32", "int64", "uint32", "uint64", "sint32", "sint64",
            "fixed32", "fixed64", "sfixed32", "sfixed64", "bool", "string", "bytes",
        ];

        // Count field definitions
        let field_count = schema.matches("=").count();
        result.metrics.fields_validated = field_count;

        // Basic validation - this is simplified
        // In production, use a proper protobuf parser
        if !schema.contains("message") && !schema.contains("enum") {
            result.add_warning(
                ValidationWarning::new(
                    "type-validation",
                    "No message or enum definitions found",
                ),
            );
        }
    }

    fn validate_json_schema_semantics(
        &self,
        json: &serde_json::Value,
        result: &mut ValidationResult,
    ) {
        // Check for required fields
        if let Some(required) = json.get("required").and_then(|r| r.as_array()) {
            if let Some(properties) = json.get("properties").and_then(|p| p.as_object()) {
                for req in required {
                    if let Some(field_name) = req.as_str() {
                        if !properties.contains_key(field_name) {
                            result.add_error(
                                ValidationError::new(
                                    "semantic-validation",
                                    format!(
                                        "Required field '{}' is not defined in properties",
                                        field_name
                                    ),
                                )
                                .with_location(format!("$.required[{}]", field_name)),
                            );
                        }
                    }
                }
            }
        }

        // LLM-specific validation
        if self.config.llm_validation {
            self.validate_llm_specific(json, result);
        }
    }

    fn validate_llm_specific(&self, json: &serde_json::Value, result: &mut ValidationResult) {
        // Check for description
        if json.get("description").is_none() || json.get("description").and_then(|d| d.as_str()).map(|s| s.is_empty()).unwrap_or(true) {
            result.add_warning(
                ValidationWarning::new(
                    "llm-validation",
                    "Schema lacks description for LLM understanding",
                )
                .with_suggestion("Add a 'description' field to help LLMs understand the schema"),
            );
        }

        // Check properties for descriptions
        if let Some(properties) = json.get("properties").and_then(|p| p.as_object()) {
            for (name, prop) in properties {
                if prop.get("description").is_none() {
                    result.add_warning(
                        ValidationWarning::new(
                            "llm-validation",
                            format!("Field '{}' lacks description for LLM context", name),
                        )
                        .with_location(format!("$.properties.{}", name))
                        .with_suggestion("Add a 'description' field to improve LLM understanding"),
                    );
                }

                // Check for examples
                if prop.get("examples").is_none() && prop.get("example").is_none() {
                    result.add_warning(
                        ValidationWarning::new(
                            "llm-validation",
                            format!("Field '{}' lacks examples for LLM guidance", name),
                        )
                        .with_location(format!("$.properties.{}", name)),
                    );
                }
            }
        }
    }

    fn validate_avro_semantics(&self, _schema: &apache_avro::Schema, _result: &mut ValidationResult) {
        // Avro semantic validation
        // Add custom semantic checks here
    }

    fn validate_protobuf_semantics(&self, schema: &str, result: &mut ValidationResult) {
        // Check for field numbers
        if schema.contains("message") {
            // Ensure field numbers are unique
            // This is a simplified check
            let field_numbers: Vec<&str> = schema.split('=').skip(1).collect();
            if field_numbers.len() > 1 {
                // Check for duplicates would go here
            }
        }
    }

    fn validate_json_schema_performance(
        &self,
        json: &serde_json::Value,
        result: &mut ValidationResult,
    ) {
        // Check for overly complex patterns
        if let Some(properties) = json.get("properties").and_then(|p| p.as_object()) {
            for (name, prop) in properties {
                if let Some(pattern) = prop.get("pattern").and_then(|p| p.as_str()) {
                    // Check pattern complexity
                    if pattern.len() > 500 {
                        result.add_warning(
                            ValidationWarning::new(
                                "performance-validation",
                                format!("Complex regex pattern for field '{}' may impact performance", name),
                            )
                            .with_location(format!("$.properties.{}.pattern", name))
                            .with_suggestion("Simplify the regex pattern"),
                        );
                    }
                }
            }
        }
    }

    fn calculate_nesting_depth(&self, schema: &str, format: SchemaFormat) -> usize {
        match format {
            SchemaFormat::JsonSchema | SchemaFormat::Avro => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(schema) {
                    self.json_nesting_depth(&json, 0)
                } else {
                    0
                }
            }
            SchemaFormat::Protobuf => {
                // Count message nesting
                let open_braces = schema.matches('{').count();
                let close_braces = schema.matches('}').count();
                open_braces.min(close_braces)
            }
        }
    }

    fn json_nesting_depth(&self, value: &serde_json::Value, current_depth: usize) -> usize {
        match value {
            serde_json::Value::Object(map) => {
                let mut max_depth = current_depth;
                for (_key, val) in map {
                    let depth = self.json_nesting_depth(val, current_depth + 1);
                    max_depth = max_depth.max(depth);
                }
                max_depth
            }
            serde_json::Value::Array(arr) => {
                let mut max_depth = current_depth;
                for val in arr {
                    let depth = self.json_nesting_depth(val, current_depth + 1);
                    max_depth = max_depth.max(depth);
                }
                max_depth
            }
            _ => current_depth,
        }
    }
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_valid_json_schema() {
        let engine = ValidationEngine::new();
        let schema = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "description": "A user object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "User's name"
                }
            }
        }"#;

        let result = engine.validate(schema, SchemaFormat::JsonSchema).await.unwrap();
        assert!(result.is_valid);
    }

    #[tokio::test]
    async fn test_validate_invalid_json_schema() {
        let engine = ValidationEngine::new();
        let schema = r#"{ invalid json }"#;

        let result = engine.validate(schema, SchemaFormat::JsonSchema).await.unwrap();
        assert!(!result.is_valid);
        assert!(result.error_count() > 0);
    }

    #[tokio::test]
    async fn test_validate_schema_size_limit() {
        let mut config = ValidationConfig::default();
        config.max_schema_size = 10; // Very small limit

        let engine = ValidationEngine::with_config(config);
        let schema = r#"{"type": "object", "properties": {}}"#;

        let result = engine.validate(schema, SchemaFormat::JsonSchema).await.unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.rule == "schema-size"));
    }

    #[tokio::test]
    async fn test_llm_validation_warnings() {
        let engine = ValidationEngine::new();
        let schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }"#;

        let result = engine.validate(schema, SchemaFormat::JsonSchema).await.unwrap();
        // Should have warnings about missing descriptions
        assert!(result.warning_count() > 0);
    }

    #[tokio::test]
    async fn test_fail_fast_mode() {
        let config = ValidationConfig::default().with_fail_fast(true);
        let engine = ValidationEngine::with_config(config);
        let schema = r#"{ invalid }"#;

        let result = engine.validate(schema, SchemaFormat::JsonSchema).await.unwrap();
        assert!(!result.is_valid);
    }
}
