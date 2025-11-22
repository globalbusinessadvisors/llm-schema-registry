//! # LLM Schema Registry - Compatibility Checker
//!
//! This crate implements the 7-mode compatibility checker as specified in PSEUDOCODE.md § 1.5
//!
//! ## Compatibility Modes
//!
//! - **BACKWARD**: New schema can read old data
//! - **FORWARD**: Old schema can read new data
//! - **FULL**: Both backward and forward compatible
//! - **BACKWARD_TRANSITIVE**: Backward with all previous versions
//! - **FORWARD_TRANSITIVE**: Forward with all previous versions
//! - **FULL_TRANSITIVE**: Full with all previous versions
//! - **NONE**: No compatibility checking
//!
//! ## Performance Target
//!
//! - p95 latency < 25ms for compatibility checks
//! - Support for transitive checks across 100+ versions
//! - Comprehensive breaking change detection

pub mod cache;
pub mod checker;
pub mod dependency;
pub mod formats;
pub mod types;
pub mod violation;

pub use checker::{CompatibilityChecker, CompatibilityCheckerConfig};
pub use types::{CompatibilityMode, CompatibilityResult, SchemaFormat};
pub use violation::{CompatibilityViolation, ViolationType, ViolationSeverity};
