//! Compatibility violation types

use serde::{Deserialize, Serialize};

/// Type of compatibility violation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViolationType {
    /// Field was removed
    FieldRemoved,
    /// Field type changed incompatibly
    TypeChanged,
    /// New required field added without default
    RequiredAdded,
    /// Constraint was tightened
    ConstraintAdded,
    /// Enum value removed
    EnumValueRemoved,
    /// Schema format changed
    FormatChanged,
    /// Field became required
    FieldMadeRequired,
    /// Array items type changed
    ArrayItemsChanged,
    /// Map value type changed
    MapValueChanged,
    /// Union types incompatible
    UnionTypesIncompatible,
    /// Namespace changed
    NamespaceChanged,
    /// Name changed
    NameChanged,
    /// Custom violation
    Custom(String),
}

/// Severity of violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViolationSeverity {
    /// Breaking change - will cause failures
    Breaking,
    /// Warning - may cause issues
    Warning,
    /// Info - notable but not problematic
    Info,
}

/// A compatibility violation detected during checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityViolation {
    /// Type of violation
    pub violation_type: ViolationType,
    /// Path to the field/element that changed
    pub field_path: String,
    /// Old value (JSON representation)
    pub old_value: Option<serde_json::Value>,
    /// New value (JSON representation)
    pub new_value: Option<serde_json::Value>,
    /// Severity of the violation
    pub severity: ViolationSeverity,
    /// Human-readable description
    pub description: String,
}

impl CompatibilityViolation {
    /// Create a breaking violation
    pub fn breaking(
        violation_type: ViolationType,
        field_path: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            violation_type,
            field_path: field_path.into(),
            old_value: None,
            new_value: None,
            severity: ViolationSeverity::Breaking,
            description: description.into(),
        }
    }

    /// Create a warning violation
    pub fn warning(
        violation_type: ViolationType,
        field_path: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            violation_type,
            field_path: field_path.into(),
            old_value: None,
            new_value: None,
            severity: ViolationSeverity::Warning,
            description: description.into(),
        }
    }

    /// Set old and new values
    pub fn with_values(
        mut self,
        old_value: Option<serde_json::Value>,
        new_value: Option<serde_json::Value>,
    ) -> Self {
        self.old_value = old_value;
        self.new_value = new_value;
        self
    }
}
