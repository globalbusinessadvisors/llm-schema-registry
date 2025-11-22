//! Core types and traits for the LLM Schema Registry

pub mod error;
pub mod schema;
pub mod types;

pub use error::{Error, Result};
pub use schema::{Schema, SchemaContent, SchemaId, SchemaMetadata, SchemaType, SchemaVersion};
pub use types::{CompatibilityLevel, SchemaState};
