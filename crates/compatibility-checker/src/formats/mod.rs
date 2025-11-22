//! Format-specific compatibility checkers

mod json_schema;
mod avro;
mod protobuf;

pub use json_schema::JsonSchemaCompatibilityChecker;
pub use avro::AvroCompatibilityChecker;
pub use protobuf::ProtobufCompatibilityChecker;

use crate::violation::CompatibilityViolation;
use crate::checker::CompatibilityError;

/// Trait for format-specific compatibility checking
pub trait FormatCompatibilityChecker: Send + Sync {
    /// Check backward compatibility
    fn check_backward(
        &self,
        new_schema: &str,
        old_schema: &str,
    ) -> Result<Vec<CompatibilityViolation>, CompatibilityError>;

    /// Check forward compatibility
    fn check_forward(
        &self,
        new_schema: &str,
        old_schema: &str,
    ) -> Result<Vec<CompatibilityViolation>, CompatibilityError>;
}
