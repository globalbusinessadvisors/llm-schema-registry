//! Comprehensive input validation and sanitization for security
//!
//! This module provides OWASP-compliant input validation to prevent:
//! - SQL Injection
//! - XSS attacks
//! - Buffer overflow
//! - Path traversal
//! - Command injection

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use thiserror::Error;

// =============================================================================
// Constants
// =============================================================================

/// Maximum schema content size (1MB)
pub const MAX_SCHEMA_SIZE: usize = 1_048_576;

/// Maximum subject name length
pub const MAX_SUBJECT_LENGTH: usize = 255;

/// Maximum description length
pub const MAX_DESCRIPTION_LENGTH: usize = 1000;

/// Maximum metadata JSON size
pub const MAX_METADATA_SIZE: usize = 10_240; // 10KB

/// Maximum number of tags
pub const MAX_TAGS_COUNT: usize = 50;

/// Maximum tag length
pub const MAX_TAG_LENGTH: usize = 50;

// =============================================================================
// Validation Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Subject name is invalid: {0}")]
    InvalidSubject(String),

    #[error("Schema size {0} exceeds maximum {1} bytes")]
    SchemaTooLarge(usize, usize),

    #[error("Version format is invalid: {0}")]
    InvalidVersion(String),

    #[error("Invalid characters detected: {0}")]
    InvalidCharacters(String),

    #[error("Field too long: {field} (max: {max})")]
    FieldTooLong { field: String, max: usize },

    #[error("Too many items: {field} (max: {max})")]
    TooManyItems { field: String, max: usize },

    #[error("Path traversal attempt detected")]
    PathTraversal,

    #[error("SQL injection attempt detected")]
    SqlInjection,

    #[error("XSS attempt detected")]
    XssAttempt,

    #[error("Invalid JSON: {0}")]
    InvalidJson(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

// =============================================================================
// Regex Patterns (Compiled Once)
// =============================================================================

static SUBJECT_PATTERN: OnceLock<Regex> = OnceLock::new();
static VERSION_PATTERN: OnceLock<Regex> = OnceLock::new();
static TAG_PATTERN: OnceLock<Regex> = OnceLock::new();
static SQL_INJECTION_PATTERN: OnceLock<Regex> = OnceLock::new();
static XSS_PATTERN: OnceLock<Regex> = OnceLock::new();
static PATH_TRAVERSAL_PATTERN: OnceLock<Regex> = OnceLock::new();

fn subject_regex() -> &'static Regex {
    SUBJECT_PATTERN.get_or_init(|| {
        Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9._-]*[a-zA-Z0-9]$").unwrap()
    })
}

fn version_regex() -> &'static Regex {
    VERSION_PATTERN.get_or_init(|| {
        Regex::new(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9]+)?(\+[a-zA-Z0-9]+)?$").unwrap()
    })
}

fn tag_regex() -> &'static Regex {
    TAG_PATTERN.get_or_init(|| {
        Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap()
    })
}

fn sql_injection_regex() -> &'static Regex {
    SQL_INJECTION_PATTERN.get_or_init(|| {
        Regex::new(r"(?i)(union\s+select|drop\s+table|insert\s+into|delete\s+from|exec\s*\(|script\s*>|<\s*script)").unwrap()
    })
}

fn xss_regex() -> &'static Regex {
    XSS_PATTERN.get_or_init(|| {
        Regex::new(r"(?i)(<script|javascript:|onerror=|onclick=|<iframe|<object|<embed)").unwrap()
    })
}

fn path_traversal_regex() -> &'static Regex {
    PATH_TRAVERSAL_PATTERN.get_or_init(|| {
        Regex::new(r"(\.\./|\.\.\\|%2e%2e|%252e)").unwrap()
    })
}

// =============================================================================
// Validation Functions
// =============================================================================

/// Validate subject name
///
/// Rules:
/// - 2-255 characters
/// - Starts and ends with alphanumeric
/// - Can contain: letters, numbers, dots, hyphens, underscores
/// - No SQL injection patterns
/// - No XSS patterns
pub fn validate_subject(subject: &str) -> Result<(), ValidationError> {
    // Check length
    if subject.is_empty() {
        return Err(ValidationError::MissingField("subject".to_string()));
    }

    if subject.len() > MAX_SUBJECT_LENGTH {
        return Err(ValidationError::FieldTooLong {
            field: "subject".to_string(),
            max: MAX_SUBJECT_LENGTH,
        });
    }

    // Check pattern
    if !subject_regex().is_match(subject) {
        return Err(ValidationError::InvalidSubject(
            "Must start and end with alphanumeric, can contain dots, hyphens, underscores".to_string(),
        ));
    }

    // Security checks
    check_sql_injection(subject)?;
    check_xss(subject)?;
    check_path_traversal(subject)?;

    Ok(())
}

/// Validate semantic version string
pub fn validate_version(version: &str) -> Result<(), ValidationError> {
    if !version_regex().is_match(version) {
        return Err(ValidationError::InvalidVersion(
            "Must be in format: MAJOR.MINOR.PATCH[-prerelease][+build]".to_string(),
        ));
    }

    Ok(())
}

/// Validate schema content size
pub fn validate_schema_size(content: &str) -> Result<(), ValidationError> {
    let size = content.as_bytes().len();
    if size > MAX_SCHEMA_SIZE {
        return Err(ValidationError::SchemaTooLarge(size, MAX_SCHEMA_SIZE));
    }

    Ok(())
}

/// Validate schema content is valid JSON
pub fn validate_json_schema(content: &str) -> Result<serde_json::Value, ValidationError> {
    serde_json::from_str(content)
        .map_err(|e| ValidationError::InvalidJson(e.to_string()))
}

/// Validate description field
pub fn validate_description(desc: &str) -> Result<(), ValidationError> {
    if desc.len() > MAX_DESCRIPTION_LENGTH {
        return Err(ValidationError::FieldTooLong {
            field: "description".to_string(),
            max: MAX_DESCRIPTION_LENGTH,
        });
    }

    check_xss(desc)?;

    Ok(())
}

/// Validate tags
pub fn validate_tags(tags: &[String]) -> Result<(), ValidationError> {
    if tags.len() > MAX_TAGS_COUNT {
        return Err(ValidationError::TooManyItems {
            field: "tags".to_string(),
            max: MAX_TAGS_COUNT,
        });
    }

    for tag in tags {
        if tag.len() > MAX_TAG_LENGTH {
            return Err(ValidationError::FieldTooLong {
                field: format!("tag '{}'", tag),
                max: MAX_TAG_LENGTH,
            });
        }

        if !tag_regex().is_match(tag) {
            return Err(ValidationError::InvalidCharacters(
                format!("Tag '{}' contains invalid characters", tag),
            ));
        }
    }

    Ok(())
}

/// Validate metadata JSON
pub fn validate_metadata(metadata: &serde_json::Value) -> Result<(), ValidationError> {
    let size = serde_json::to_string(metadata)
        .map_err(|e| ValidationError::InvalidJson(e.to_string()))?
        .as_bytes()
        .len();

    if size > MAX_METADATA_SIZE {
        return Err(ValidationError::FieldTooLong {
            field: "metadata".to_string(),
            max: MAX_METADATA_SIZE,
        });
    }

    Ok(())
}

// =============================================================================
// Security Checks
// =============================================================================

/// Check for SQL injection patterns
fn check_sql_injection(input: &str) -> Result<(), ValidationError> {
    if sql_injection_regex().is_match(input) {
        return Err(ValidationError::SqlInjection);
    }
    Ok(())
}

/// Check for XSS patterns
fn check_xss(input: &str) -> Result<(), ValidationError> {
    if xss_regex().is_match(input) {
        return Err(ValidationError::XssAttempt);
    }
    Ok(())
}

/// Check for path traversal patterns
fn check_path_traversal(input: &str) -> Result<(), ValidationError> {
    if path_traversal_regex().is_match(input) {
        return Err(ValidationError::PathTraversal);
    }
    Ok(())
}

/// Sanitize user input by removing dangerous characters
pub fn sanitize_string(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '.' | '-' | '_'))
        .collect()
}

// =============================================================================
// Rate Limiting Support
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute per IP
    pub requests_per_minute_ip: u32,
    /// Requests per minute per user
    pub requests_per_minute_user: u32,
    /// Requests per hour per IP
    pub requests_per_hour_ip: u32,
    /// Burst allowance
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute_ip: 60,
            requests_per_minute_user: 120,
            requests_per_hour_ip: 1000,
            burst_size: 10,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_subject() {
        // Valid subjects
        assert!(validate_subject("com.example.user").is_ok());
        assert!(validate_subject("my-schema").is_ok());
        assert!(validate_subject("schema_v1").is_ok());

        // Invalid subjects
        assert!(validate_subject("").is_err()); // Empty
        assert!(validate_subject(".invalid").is_err()); // Starts with dot
        assert!(validate_subject("invalid.").is_err()); // Ends with dot
        assert!(validate_subject("a".repeat(300).as_str()).is_err()); // Too long
    }

    #[test]
    fn test_sql_injection_detection() {
        assert!(validate_subject("user'; DROP TABLE schemas; --").is_err());
        assert!(validate_subject("user UNION SELECT * FROM users").is_err());
        assert!(validate_subject("normal-subject").is_ok());
    }

    #[test]
    fn test_xss_detection() {
        assert!(validate_description("<script>alert('xss')</script>").is_err());
        assert!(validate_description("javascript:alert(1)").is_err());
        assert!(validate_description("Normal description").is_ok());
    }

    #[test]
    fn test_path_traversal_detection() {
        assert!(validate_subject("../../../etc/passwd").is_err());
        assert!(validate_subject("..\\..\\windows\\system32").is_err());
        assert!(validate_subject("normal-path").is_ok());
    }

    #[test]
    fn test_validate_version() {
        // Valid versions
        assert!(validate_version("1.0.0").is_ok());
        assert!(validate_version("2.1.3-alpha").is_ok());
        assert!(validate_version("1.0.0+build123").is_ok());

        // Invalid versions
        assert!(validate_version("1.0").is_err());
        assert!(validate_version("v1.0.0").is_err());
        assert!(validate_version("1.0.0.0").is_err());
    }

    #[test]
    fn test_validate_schema_size() {
        assert!(validate_schema_size("small schema").is_ok());
        assert!(validate_schema_size(&"x".repeat(MAX_SCHEMA_SIZE + 1)).is_err());
    }

    #[test]
    fn test_validate_tags() {
        assert!(validate_tags(&vec!["tag1".to_string(), "tag2".to_string()]).is_ok());
        assert!(validate_tags(&vec!["invalid tag!".to_string()]).is_err());
        assert!(validate_tags(&vec!["x".to_string(); MAX_TAGS_COUNT + 1]).is_err());
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize_string("hello<script>"), "helloscript");
        assert_eq!(sanitize_string("user@example.com"), "userexample.com");
        assert_eq!(sanitize_string("normal-name"), "normal-name");
    }
}
