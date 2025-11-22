//! Common types used across the schema registry

use serde::{Deserialize, Serialize};

/// Compatibility level for schema evolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityLevel {
    /// New schema can read old data
    Backward,
    /// Old schema can read new data
    Forward,
    /// Both backward and forward
    Full,
    /// Full compatibility across all versions
    Transitive,
    /// No compatibility checks
    None,
}

impl Default for CompatibilityLevel {
    fn default() -> Self {
        Self::Backward
    }
}

impl std::fmt::Display for CompatibilityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Backward => write!(f, "BACKWARD"),
            Self::Forward => write!(f, "FORWARD"),
            Self::Full => write!(f, "FULL"),
            Self::Transitive => write!(f, "TRANSITIVE"),
            Self::None => write!(f, "NONE"),
        }
    }
}

/// Lifecycle state of a schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SchemaState {
    /// Schema is active and can be used
    Active,
    /// Schema is deprecated but still usable
    Deprecated,
    /// Schema has been soft-deleted
    Deleted,
    /// Schema is in draft/testing state
    Draft,
}

impl Default for SchemaState {
    fn default() -> Self {
        Self::Active
    }
}

impl std::fmt::Display for SchemaState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "ACTIVE"),
            Self::Deprecated => write!(f, "DEPRECATED"),
            Self::Deleted => write!(f, "DELETED"),
            Self::Draft => write!(f, "DRAFT"),
        }
    }
}
