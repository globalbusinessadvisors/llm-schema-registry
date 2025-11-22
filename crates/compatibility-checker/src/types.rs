//! Core types for compatibility checking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::violation::CompatibilityViolation;

/// Compatibility mode as specified in PSEUDOCODE.md § 1.5
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityMode {
    /// New schema can read old data
    Backward,
    /// Old schema can read new data
    Forward,
    /// Both backward and forward
    Full,
    /// No compatibility required
    None,
    /// Backward with all previous versions
    BackwardTransitive,
    /// Forward with all previous versions
    ForwardTransitive,
    /// Full with all previous versions
    FullTransitive,
}

impl CompatibilityMode {
    /// Check if this mode requires transitive checking
    pub fn is_transitive(&self) -> bool {
        matches!(
            self,
            CompatibilityMode::BackwardTransitive
                | CompatibilityMode::ForwardTransitive
                | CompatibilityMode::FullTransitive
        )
    }

    /// Get the base mode (non-transitive equivalent)
    pub fn base_mode(&self) -> CompatibilityMode {
        match self {
            CompatibilityMode::BackwardTransitive => CompatibilityMode::Backward,
            CompatibilityMode::ForwardTransitive => CompatibilityMode::Forward,
            CompatibilityMode::FullTransitive => CompatibilityMode::Full,
            other => *other,
        }
    }
}

/// Schema format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SchemaFormat {
    JsonSchema,
    Avro,
    Protobuf,
}

/// Semantic version
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
    pub build_metadata: Option<String>,
}

impl SemanticVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
            build_metadata: None,
        }
    }

    /// Parse semantic version from string
    pub fn parse(s: &str) -> Result<Self, String> {
        let re = regex::Regex::new(
            r"^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z\-\.]+))?(?:\+([0-9A-Za-z\-\.]+))?$",
        )
        .unwrap();

        let caps = re
            .captures(s)
            .ok_or_else(|| format!("Invalid semantic version format: {}", s))?;

        Ok(Self {
            major: caps[1].parse().unwrap(),
            minor: caps[2].parse().unwrap(),
            patch: caps[3].parse().unwrap(),
            prerelease: caps.get(4).map(|m| m.as_str().to_string()),
            build_metadata: caps.get(5).map(|m| m.as_str().to_string()),
        })
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.prerelease {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build_metadata {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl PartialOrd for SemanticVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemanticVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        // Compare major
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // Compare minor
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // Compare patch
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // Compare prerelease (version without prerelease > version with prerelease)
        match (&self.prerelease, &other.prerelease) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

/// Schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub id: Uuid,
    pub name: String,
    pub namespace: String,
    pub version: SemanticVersion,
    pub format: SchemaFormat,
    pub content: String,
    pub content_hash: String,
    pub description: String,
    pub compatibility_mode: CompatibilityMode,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Schema {
    /// Calculate SHA-256 hash of schema content
    pub fn calculate_hash(content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Result of compatibility check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityResult {
    /// Whether schemas are compatible
    pub is_compatible: bool,
    /// Compatibility mode used
    pub mode: CompatibilityMode,
    /// List of violations found (empty if compatible)
    pub violations: Vec<CompatibilityViolation>,
    /// Versions checked against
    pub checked_versions: Vec<SemanticVersion>,
    /// Time taken for check (milliseconds)
    pub check_duration_ms: u64,
    /// Metadata about the check
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CompatibilityResult {
    /// Create a compatible result
    pub fn compatible(
        mode: CompatibilityMode,
        checked_versions: Vec<SemanticVersion>,
        check_duration_ms: u64,
    ) -> Self {
        Self {
            is_compatible: true,
            mode,
            violations: Vec::new(),
            checked_versions,
            check_duration_ms,
            metadata: HashMap::new(),
        }
    }

    /// Create an incompatible result
    pub fn incompatible(
        mode: CompatibilityMode,
        violations: Vec<CompatibilityViolation>,
        checked_versions: Vec<SemanticVersion>,
        check_duration_ms: u64,
    ) -> Self {
        Self {
            is_compatible: false,
            mode,
            violations,
            checked_versions,
            check_duration_ms,
            metadata: HashMap::new(),
        }
    }

    /// Get breaking violations
    pub fn breaking_violations(&self) -> Vec<&CompatibilityViolation> {
        use crate::violation::ViolationSeverity;
        self.violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Breaking)
            .collect()
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}
