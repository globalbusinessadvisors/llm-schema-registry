//! Semantic versioning for schemas

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{Error, Result};

/// Semantic version following semver specification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SemanticVersion {
    /// Major version (breaking changes)
    pub major: u32,
    /// Minor version (backward-compatible additions)
    pub minor: u32,
    /// Patch version (backward-compatible bug fixes)
    pub patch: u32,
    /// Pre-release identifier (e.g., "alpha.1", "beta.2")
    pub prerelease: Option<String>,
    /// Build metadata
    pub build_metadata: Option<String>,
}

impl SemanticVersion {
    /// Create a new semantic version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
            build_metadata: None,
        }
    }

    /// Set the prerelease identifier
    pub fn with_prerelease(mut self, prerelease: String) -> Self {
        self.prerelease = Some(prerelease);
        self
    }

    /// Set the build metadata
    pub fn with_build_metadata(mut self, metadata: String) -> Self {
        self.build_metadata = Some(metadata);
        self
    }

    /// Increment the major version (resets minor and patch to 0)
    pub fn increment_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
        self.prerelease = None;
        self.build_metadata = None;
    }

    /// Increment the minor version (resets patch to 0)
    pub fn increment_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
        self.prerelease = None;
        self.build_metadata = None;
    }

    /// Increment the patch version
    pub fn increment_patch(&mut self) {
        self.patch += 1;
        self.prerelease = None;
        self.build_metadata = None;
    }

    /// Check if this is a prerelease version
    pub fn is_prerelease(&self) -> bool {
        self.prerelease.is_some()
    }
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(ref prerelease) = self.prerelease {
            write!(f, "-{}", prerelease)?;
        }

        if let Some(ref metadata) = self.build_metadata {
            write!(f, "+{}", metadata)?;
        }

        Ok(())
    }
}

impl FromStr for SemanticVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let re = regex::Regex::new(
            r"^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z\-\.]+))?(?:\+([0-9A-Za-z\-\.]+))?$"
        ).unwrap();

        let captures = re.captures(s).ok_or_else(|| {
            Error::ParseError(format!("Invalid semantic version format: {}", s))
        })?;

        Ok(Self {
            major: captures[1].parse().unwrap(),
            minor: captures[2].parse().unwrap(),
            patch: captures[3].parse().unwrap(),
            prerelease: captures.get(4).map(|m| m.as_str().to_string()),
            build_metadata: captures.get(5).map(|m| m.as_str().to_string()),
        })
    }
}

impl PartialOrd for SemanticVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemanticVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare major, minor, patch
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        match self.minor.cmp(&other.minor) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        match self.patch.cmp(&other.patch) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        // Handle prerelease: version without prerelease > version with prerelease
        match (&self.prerelease, &other.prerelease) {
            (None, None) => std::cmp::Ordering::Equal,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
        // Build metadata is ignored in precedence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_display() {
        let v = SemanticVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");

        let v = SemanticVersion::new(1, 2, 3).with_prerelease("alpha.1".to_string());
        assert_eq!(v.to_string(), "1.2.3-alpha.1");

        let v = SemanticVersion::new(1, 2, 3)
            .with_prerelease("alpha.1".to_string())
            .with_build_metadata("build.123".to_string());
        assert_eq!(v.to_string(), "1.2.3-alpha.1+build.123");
    }

    #[test]
    fn test_version_parse() {
        let v: SemanticVersion = "1.2.3".parse().unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.prerelease, None);

        let v: SemanticVersion = "1.2.3-alpha.1".parse().unwrap();
        assert_eq!(v.prerelease, Some("alpha.1".to_string()));

        let v: SemanticVersion = "1.2.3-alpha.1+build.123".parse().unwrap();
        assert_eq!(v.prerelease, Some("alpha.1".to_string()));
        assert_eq!(v.build_metadata, Some("build.123".to_string()));
    }

    #[test]
    fn test_version_ordering() {
        let v1 = SemanticVersion::new(1, 0, 0);
        let v2 = SemanticVersion::new(2, 0, 0);
        assert!(v1 < v2);

        let v1 = SemanticVersion::new(1, 0, 0);
        let v2 = SemanticVersion::new(1, 1, 0);
        assert!(v1 < v2);

        let v1 = SemanticVersion::new(1, 0, 0);
        let v2 = SemanticVersion::new(1, 0, 1);
        assert!(v1 < v2);

        // Prerelease versions
        let v1 = SemanticVersion::new(1, 0, 0).with_prerelease("alpha".to_string());
        let v2 = SemanticVersion::new(1, 0, 0);
        assert!(v1 < v2); // Prerelease < release
    }

    #[test]
    fn test_version_increment() {
        let mut v = SemanticVersion::new(1, 2, 3);

        v.increment_patch();
        assert_eq!(v, SemanticVersion::new(1, 2, 4));

        v.increment_minor();
        assert_eq!(v, SemanticVersion::new(1, 3, 0));

        v.increment_major();
        assert_eq!(v, SemanticVersion::new(2, 0, 0));
    }
}
