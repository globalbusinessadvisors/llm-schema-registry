//! Core validation types and structures
//!
//! This module defines the foundational types for schema validation,
//! including validation results, errors, warnings, and metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Represents the format of a schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SchemaFormat {
    /// JSON Schema (Draft 7, 2019-09, 2020-12)
    JsonSchema,
    /// Apache Avro
    Avro,
    /// Protocol Buffers (proto3)
    Protobuf,
}

impl SchemaFormat {
    /// Returns the canonical name of the format
    pub fn as_str(&self) -> &'static str {
        match self {
            SchemaFormat::JsonSchema => "json-schema",
            SchemaFormat::Avro => "avro",
            SchemaFormat::Protobuf => "protobuf",
        }
    }
}

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational message
    Info,
    /// Warning that doesn't prevent validation
    Warning,
    /// Error that causes validation to fail
    Error,
}

/// A validation error with detailed context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationError {
    /// The validation rule that failed
    pub rule: String,
    /// Error message
    pub message: String,
    /// Severity level
    pub severity: Severity,
    /// Location in the schema (e.g., "$.properties.name.type")
    pub location: Option<String>,
    /// Line number (if available)
    pub line: Option<usize>,
    /// Column number (if available)
    pub column: Option<usize>,
    /// Suggested fix (if available)
    pub suggestion: Option<String>,
    /// Additional context
    pub context: HashMap<String, String>,
}

impl ValidationError {
    /// Creates a new validation error
    pub fn new(rule: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            message: message.into(),
            severity: Severity::Error,
            location: None,
            line: None,
            column: None,
            suggestion: None,
            context: HashMap::new(),
        }
    }

    /// Sets the location of the error
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Sets the line and column numbers
    pub fn with_position(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// Adds a suggested fix
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Adds context information
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// A validation warning (non-blocking issue)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// The validation rule that triggered the warning
    pub rule: String,
    /// Warning message
    pub message: String,
    /// Location in the schema
    pub location: Option<String>,
    /// Suggested improvement
    pub suggestion: Option<String>,
}

impl ValidationWarning {
    /// Creates a new validation warning
    pub fn new(rule: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            message: message.into(),
            location: None,
            suggestion: None,
        }
    }

    /// Sets the location
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Adds a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Metrics collected during validation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationMetrics {
    /// Time taken for validation
    pub duration: Duration,
    /// Number of rules applied
    pub rules_applied: usize,
    /// Number of fields validated
    pub fields_validated: usize,
    /// Schema size in bytes
    pub schema_size_bytes: usize,
    /// Maximum recursion depth encountered
    pub max_recursion_depth: usize,
    /// Custom metrics
    pub custom: HashMap<String, String>,
}

impl ValidationMetrics {
    /// Creates new validation metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the validation duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Adds a custom metric
    pub fn add_metric(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom.insert(key.into(), value.into());
    }
}

/// Result of schema validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the schema is valid
    pub is_valid: bool,
    /// List of validation errors
    pub errors: Vec<ValidationError>,
    /// List of validation warnings
    pub warnings: Vec<ValidationWarning>,
    /// Validation metrics
    pub metrics: ValidationMetrics,
    /// The format that was validated
    pub format: SchemaFormat,
}

impl ValidationResult {
    /// Creates a successful validation result
    pub fn success(format: SchemaFormat) -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            metrics: ValidationMetrics::new(),
            format,
        }
    }

    /// Creates a failed validation result
    pub fn failure(format: SchemaFormat, errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
            metrics: ValidationMetrics::new(),
            format,
        }
    }

    /// Adds an error to the result
    pub fn add_error(&mut self, error: ValidationError) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// Adds a warning to the result
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Merges another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        self.is_valid = self.is_valid && other.is_valid;
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.metrics.rules_applied += other.metrics.rules_applied;
        self.metrics.fields_validated += other.metrics.fields_validated;
    }

    /// Returns true if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns true if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Returns the number of errors
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Returns the number of warnings
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

/// Configuration for validation behavior
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Stop validation on first error
    pub fail_fast: bool,
    /// Include warnings in the result
    pub include_warnings: bool,
    /// Maximum schema size in bytes (default: 1MB)
    pub max_schema_size: usize,
    /// Maximum recursion depth (default: 100)
    pub max_recursion_depth: usize,
    /// Enable LLM-specific validation rules
    pub llm_validation: bool,
    /// Enable security validation
    pub security_validation: bool,
    /// Enable performance validation
    pub performance_validation: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            fail_fast: false,
            include_warnings: true,
            max_schema_size: 1024 * 1024, // 1MB
            max_recursion_depth: 100,
            llm_validation: true,
            security_validation: true,
            performance_validation: true,
        }
    }
}

impl ValidationConfig {
    /// Creates a new validation configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables fail-fast mode
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// Includes or excludes warnings
    pub fn with_warnings(mut self, include_warnings: bool) -> Self {
        self.include_warnings = include_warnings;
        self
    }

    /// Sets the maximum schema size
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_schema_size = max_size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_builder() {
        let error = ValidationError::new("test-rule", "Test error")
            .with_location("$.properties.name")
            .with_position(10, 5)
            .with_suggestion("Use 'type': 'string' instead")
            .with_context("field", "name");

        assert_eq!(error.rule, "test-rule");
        assert_eq!(error.message, "Test error");
        assert_eq!(error.location, Some("$.properties.name".to_string()));
        assert_eq!(error.line, Some(10));
        assert_eq!(error.column, Some(5));
        assert!(error.suggestion.is_some());
        assert!(error.context.contains_key("field"));
    }

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success(SchemaFormat::JsonSchema);
        assert!(result.is_valid);
        assert_eq!(result.error_count(), 0);
        assert_eq!(result.warning_count(), 0);
    }

    #[test]
    fn test_validation_result_failure() {
        let errors = vec![ValidationError::new("test", "error")];
        let result = ValidationResult::failure(SchemaFormat::JsonSchema, errors);
        assert!(!result.is_valid);
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::success(SchemaFormat::JsonSchema);
        result1.add_warning(ValidationWarning::new("test", "warning"));

        let mut result2 = ValidationResult::success(SchemaFormat::JsonSchema);
        result2.add_error(ValidationError::new("test", "error"));

        result1.merge(result2);
        assert!(!result1.is_valid);
        assert_eq!(result1.error_count(), 1);
        assert_eq!(result1.warning_count(), 1);
    }

    #[test]
    fn test_schema_format_as_str() {
        assert_eq!(SchemaFormat::JsonSchema.as_str(), "json-schema");
        assert_eq!(SchemaFormat::Avro.as_str(), "avro");
        assert_eq!(SchemaFormat::Protobuf.as_str(), "protobuf");
    }

    #[test]
    fn test_validation_config_defaults() {
        let config = ValidationConfig::default();
        assert!(!config.fail_fast);
        assert!(config.include_warnings);
        assert_eq!(config.max_schema_size, 1024 * 1024);
        assert_eq!(config.max_recursion_depth, 100);
    }

    #[test]
    fn test_validation_error_severity() {
        let error = ValidationError::new("test", "message");
        assert_eq!(error.severity, Severity::Error);
    }

    #[test]
    fn test_validation_warning_creation() {
        let warning = ValidationWarning::new("rule", "message");
        assert_eq!(warning.rule, "rule");
        assert_eq!(warning.message, "message");
        assert!(warning.location.is_none());
    }

    #[test]
    fn test_validation_warning_with_location() {
        let warning = ValidationWarning::new("rule", "message")
            .with_location("$.field");
        assert_eq!(warning.location, Some("$.field".to_string()));
    }

    #[test]
    fn test_validation_warning_with_suggestion() {
        let warning = ValidationWarning::new("rule", "message")
            .with_suggestion("fix");
        assert_eq!(warning.suggestion, Some("fix".to_string()));
    }

    #[test]
    fn test_validation_metrics_new() {
        let metrics = ValidationMetrics::new();
        assert_eq!(metrics.rules_applied, 0);
        assert_eq!(metrics.fields_validated, 0);
        assert_eq!(metrics.schema_size_bytes, 0);
    }

    #[test]
    fn test_validation_metrics_with_duration() {
        let metrics = ValidationMetrics::new()
            .with_duration(Duration::from_millis(100));
        assert_eq!(metrics.duration.as_millis(), 100);
    }

    #[test]
    fn test_validation_metrics_add_metric() {
        let mut metrics = ValidationMetrics::new();
        metrics.add_metric("key", "value");
        assert_eq!(metrics.custom.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut result = ValidationResult::success(SchemaFormat::JsonSchema);
        assert!(result.is_valid);

        result.add_error(ValidationError::new("test", "error"));
        assert!(!result.is_valid);
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::success(SchemaFormat::JsonSchema);
        result.add_warning(ValidationWarning::new("test", "warning"));
        assert!(result.is_valid);
        assert_eq!(result.warning_count(), 1);
    }

    #[test]
    fn test_validation_result_has_errors() {
        let mut result = ValidationResult::success(SchemaFormat::JsonSchema);
        assert!(!result.has_errors());

        result.add_error(ValidationError::new("test", "error"));
        assert!(result.has_errors());
    }

    #[test]
    fn test_validation_result_has_warnings() {
        let mut result = ValidationResult::success(SchemaFormat::JsonSchema);
        assert!(!result.has_warnings());

        result.add_warning(ValidationWarning::new("test", "warning"));
        assert!(result.has_warnings());
    }

    #[test]
    fn test_validation_config_with_fail_fast() {
        let config = ValidationConfig::new().with_fail_fast(true);
        assert!(config.fail_fast);
    }

    #[test]
    fn test_validation_config_with_warnings() {
        let config = ValidationConfig::new().with_warnings(false);
        assert!(!config.include_warnings);
    }

    #[test]
    fn test_validation_config_with_max_size() {
        let config = ValidationConfig::new().with_max_size(2048);
        assert_eq!(config.max_schema_size, 2048);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }

    #[test]
    fn test_schema_format_equality() {
        assert_eq!(SchemaFormat::JsonSchema, SchemaFormat::JsonSchema);
        assert_ne!(SchemaFormat::JsonSchema, SchemaFormat::Avro);
    }

    #[test]
    fn test_validation_error_context() {
        let error = ValidationError::new("test", "message")
            .with_context("key1", "value1")
            .with_context("key2", "value2");
        assert_eq!(error.context.len(), 2);
        assert_eq!(error.context.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_validation_result_format() {
        let result = ValidationResult::success(SchemaFormat::Avro);
        assert_eq!(result.format, SchemaFormat::Avro);
    }

    #[test]
    fn test_validation_result_multiple_errors() {
        let mut result = ValidationResult::success(SchemaFormat::JsonSchema);
        result.add_error(ValidationError::new("test1", "error1"));
        result.add_error(ValidationError::new("test2", "error2"));
        assert_eq!(result.error_count(), 2);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validation_result_multiple_warnings() {
        let mut result = ValidationResult::success(SchemaFormat::JsonSchema);
        result.add_warning(ValidationWarning::new("test1", "warning1"));
        result.add_warning(ValidationWarning::new("test2", "warning2"));
        result.add_warning(ValidationWarning::new("test3", "warning3"));
        assert_eq!(result.warning_count(), 3);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validation_config_llm_validation() {
        let config = ValidationConfig::default();
        assert!(config.llm_validation);
    }

    #[test]
    fn test_validation_config_security_validation() {
        let config = ValidationConfig::default();
        assert!(config.security_validation);
    }

    #[test]
    fn test_validation_config_performance_validation() {
        let config = ValidationConfig::default();
        assert!(config.performance_validation);
    }

    #[test]
    fn test_validation_error_with_all_fields() {
        let error = ValidationError::new("test-rule", "Test error")
            .with_location("$.properties.name")
            .with_position(10, 5)
            .with_suggestion("Fix")
            .with_context("field", "name")
            .with_context("type", "string");

        assert_eq!(error.rule, "test-rule");
        assert_eq!(error.message, "Test error");
        assert_eq!(error.severity, Severity::Error);
        assert_eq!(error.location, Some("$.properties.name".to_string()));
        assert_eq!(error.line, Some(10));
        assert_eq!(error.column, Some(5));
        assert_eq!(error.suggestion, Some("Fix".to_string()));
        assert_eq!(error.context.len(), 2);
    }

    #[test]
    fn test_validation_metrics_max_recursion_depth() {
        let mut metrics = ValidationMetrics::new();
        metrics.max_recursion_depth = 50;
        assert_eq!(metrics.max_recursion_depth, 50);
    }

    #[test]
    fn test_validation_config_max_recursion_depth() {
        let config = ValidationConfig::default();
        assert_eq!(config.max_recursion_depth, 100);
    }
}
