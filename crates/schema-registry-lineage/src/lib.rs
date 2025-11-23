//! # Schema Registry Lineage Tracking
//!
//! This crate provides comprehensive lineage tracking capabilities for the LLM Schema Registry.
//! It tracks dependencies between schemas, applications, data pipelines, and LLM models, enabling
//! impact analysis and change propagation understanding.
//!
//! ## Features
//!
//! - **Dependency Graph**: Track schema-to-schema, schema-to-application, and schema-to-pipeline dependencies
//! - **Transitive Dependencies**: Calculate all transitive dependencies with depth control
//! - **Impact Analysis**: Analyze the impact of schema changes on downstream consumers
//! - **Circular Dependency Detection**: Detect and report circular dependencies
//! - **Graph Algorithms**: BFS, DFS, shortest path, topological sort
//! - **Export Formats**: GraphML, DOT (Graphviz), and JSON for visualization
//! - **Thread-Safe**: Concurrent access with Arc and RwLock
//!
//! ## Quick Start
//!
//! ```rust
//! use schema_registry_lineage::{LineageEngine, LineageTracker, RelationType, SchemaChange};
//! use uuid::Uuid;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new lineage engine
//! let engine = LineageEngine::new();
//!
//! // Track dependencies
//! let schema_a = Uuid::new_v4();
//! let schema_b = Uuid::new_v4();
//!
//! engine.track_dependency(schema_a, schema_b, RelationType::DependsOn).await?;
//!
//! // Get upstream dependencies
//! let upstream = engine.get_upstream(schema_a).await?;
//! println!("Schema A depends on {} schemas", upstream.len());
//!
//! // Perform impact analysis
//! let change = SchemaChange::FieldRemoved {
//!     name: "email".to_string(),
//! };
//!
//! let impact = engine.impact_analysis(schema_b, change).await?;
//! println!("Risk level: {:?}", impact.risk_level);
//! println!("Affected schemas: {}", impact.affected_schemas.len());
//!
//! // Detect circular dependencies
//! let circular = engine.detect_circular().await?;
//! if !circular.is_empty() {
//!     println!("WARNING: {} circular dependencies detected", circular.len());
//! }
//!
//! // Export for visualization
//! let graphml = engine.export_graphml()?;
//! std::fs::write("/tmp/lineage.graphml", graphml)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! The lineage tracking system consists of several key components:
//!
//! - **GraphStore**: In-memory graph storage using petgraph
//! - **Algorithms**: Graph algorithms (BFS, DFS, cycles, etc.)
//! - **Tracker**: Dependency tracking operations
//! - **ImpactAnalyzer**: Schema change impact analysis
//! - **Exporter**: Export to GraphML, DOT, and JSON formats
//! - **LineageEngine**: Main orchestrator that combines all components
//!
//! ## Examples
//!
//! ### Tracking Schema Dependencies
//!
//! ```rust
//! use schema_registry_lineage::{LineageEngine, SchemaNode, DependencyTarget, RelationType};
//! use schema_registry_core::versioning::SemanticVersion;
//! use uuid::Uuid;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = LineageEngine::new();
//!
//! let user_schema = SchemaNode::new(
//!     Uuid::new_v4(),
//!     SemanticVersion::new(1, 0, 0),
//!     "com.example.User".to_string(),
//! );
//!
//! let address_schema = SchemaNode::new(
//!     Uuid::new_v4(),
//!     SemanticVersion::new(1, 0, 0),
//!     "com.example.Address".to_string(),
//! );
//!
//! // User composes Address
//! engine.track_dependency(
//!     user_schema.clone(),
//!     DependencyTarget::Schema(address_schema),
//!     RelationType::Composes,
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Impact Analysis
//!
//! ```rust
//! use schema_registry_lineage::{LineageEngine, LineageTracker, SchemaChange};
//! use uuid::Uuid;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = LineageEngine::new();
//! let schema_id = Uuid::new_v4();
//!
//! let change = SchemaChange::FieldTypeChanged {
//!     name: "age".to_string(),
//!     old_type: "int32".to_string(),
//!     new_type: "int64".to_string(),
//! };
//!
//! let impact = engine.impact_analysis(schema_id, change).await?;
//!
//! println!("Impact Report:");
//! println!("  Risk Level: {:?}", impact.risk_level);
//! println!("  Affected Schemas: {}", impact.affected_schemas.len());
//! println!("  Affected Apps: {}", impact.affected_applications.len());
//! println!("  Migration Complexity: {:.2}", impact.migration_complexity);
//! println!("  Estimated Effort: {:.1} hours", impact.estimated_effort_hours);
//!
//! for recommendation in &impact.recommendations {
//!     println!("  - {}", recommendation);
//! }
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod algorithms;
pub mod engine;
pub mod error;
pub mod export;
pub mod graph_store;
pub mod impact;
pub mod tracker;
pub mod types;

// Re-export main types and traits
pub use engine::{LineageEngine, LineageTracker};
pub use error::{LineageError, Result};
pub use export::{JsonEdge, JsonGraph, JsonGraphMetadata, JsonNode, LineageExporter};
pub use graph_store::{GraphStats, GraphStore};
pub use impact::{ImpactAnalyzer, ImpactSummary};
pub use tracker::{DependencyTracker, DependencyTrackerImpl};
pub use types::{
    CircularDependency, Dependency, DependencyGraph, DependencyTarget, Dependent, EntityType,
    ExternalEntity, ImpactReport, LineageFilter, RelationType, RiskLevel, SchemaChange, SchemaId,
    SchemaNode,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_exports() {
        // Verify all main types are exported
        let _engine: Option<LineageEngine> = None;
        let _error: Option<LineageError> = None;
        let _report: Option<ImpactReport> = None;
        let _graph: Option<DependencyGraph> = None;
    }
}
