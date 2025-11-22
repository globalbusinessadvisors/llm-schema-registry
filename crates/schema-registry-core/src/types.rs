//! Core type definitions

use serde::{Deserialize, Serialize};

/// Serialization format for schemas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SerializationFormat {
    /// JSON Schema format
    JsonSchema,
    /// Apache Avro format
    Avro,
    /// Protocol Buffers format
    Protobuf,
}

impl std::fmt::Display for SerializationFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationFormat::JsonSchema => write!(f, "JSON_SCHEMA"),
            SerializationFormat::Avro => write!(f, "AVRO"),
            SerializationFormat::Protobuf => write!(f, "PROTOBUF"),
        }
    }
}

/// Compatibility mode for schema evolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityMode {
    /// New schema can read old data
    Backward,
    /// Old schema can read new data
    Forward,
    /// Both backward and forward compatible
    Full,
    /// No compatibility required
    None,
    /// Backward compatible with all previous versions
    BackwardTransitive,
    /// Forward compatible with all previous versions
    ForwardTransitive,
    /// Full compatible with all previous versions
    FullTransitive,
}

impl std::fmt::Display for CompatibilityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompatibilityMode::Backward => write!(f, "BACKWARD"),
            CompatibilityMode::Forward => write!(f, "FORWARD"),
            CompatibilityMode::Full => write!(f, "FULL"),
            CompatibilityMode::None => write!(f, "NONE"),
            CompatibilityMode::BackwardTransitive => write!(f, "BACKWARD_TRANSITIVE"),
            CompatibilityMode::ForwardTransitive => write!(f, "FORWARD_TRANSITIVE"),
            CompatibilityMode::FullTransitive => write!(f, "FULL_TRANSITIVE"),
        }
    }
}

impl CompatibilityMode {
    /// Check if this is a transitive mode
    pub fn is_transitive(&self) -> bool {
        matches!(
            self,
            CompatibilityMode::BackwardTransitive
                | CompatibilityMode::ForwardTransitive
                | CompatibilityMode::FullTransitive
        )
    }
}

/// Type of compatibility violation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViolationType {
    /// Field was removed
    FieldRemoved,
    /// Field type changed
    TypeChanged,
    /// Field was made required
    RequiredAdded,
    /// Constraint was tightened
    ConstraintAdded,
    /// Enum value was removed
    EnumValueRemoved,
    /// Schema format changed
    FormatChanged,
}

/// Severity of a compatibility violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViolationSeverity {
    /// Breaking change
    Breaking,
    /// Warning level
    Warning,
    /// Info level
    Info,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_format_display() {
        assert_eq!(SerializationFormat::JsonSchema.to_string(), "JSON_SCHEMA");
        assert_eq!(SerializationFormat::Avro.to_string(), "AVRO");
        assert_eq!(SerializationFormat::Protobuf.to_string(), "PROTOBUF");
    }

    #[test]
    fn test_compatibility_mode_is_transitive() {
        assert!(!CompatibilityMode::Backward.is_transitive());
        assert!(!CompatibilityMode::Forward.is_transitive());
        assert!(!CompatibilityMode::Full.is_transitive());
        assert!(!CompatibilityMode::None.is_transitive());
        assert!(CompatibilityMode::BackwardTransitive.is_transitive());
        assert!(CompatibilityMode::ForwardTransitive.is_transitive());
        assert!(CompatibilityMode::FullTransitive.is_transitive());
    }
}
