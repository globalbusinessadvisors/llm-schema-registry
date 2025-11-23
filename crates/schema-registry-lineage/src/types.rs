//! Core type definitions for schema lineage tracking
//!
//! This module defines the data structures used throughout the lineage tracking system,
//! including nodes, edges, graphs, and impact analysis reports.

use chrono::{DateTime, Utc};
use schema_registry_core::versioning::SemanticVersion;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Type alias for Schema ID
pub type SchemaId = Uuid;

/// Represents a node in the lineage graph (a schema at a specific version)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaNode {
    /// Unique schema ID
    pub schema_id: SchemaId,
    /// Schema version
    pub schema_version: SemanticVersion,
    /// Fully qualified name (namespace.name)
    pub fqn: String,
    /// When this node was created in the lineage graph
    pub created_at: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl SchemaNode {
    /// Create a new schema node
    pub fn new(
        schema_id: SchemaId,
        schema_version: SemanticVersion,
        fqn: String,
    ) -> Self {
        Self {
            schema_id,
            schema_version,
            fqn,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a node with metadata
    pub fn with_metadata(
        schema_id: SchemaId,
        schema_version: SemanticVersion,
        fqn: String,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            schema_id,
            schema_version,
            fqn,
            created_at: Utc::now(),
            metadata,
        }
    }

    /// Get a unique key for this node (ID + version)
    pub fn key(&self) -> String {
        format!("{}@{}", self.schema_id, self.schema_version)
    }
}

/// Type of relationship between schemas and other entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationType {
    /// Schema A depends on Schema B (references it)
    DependsOn,
    /// Schema A is used by Application B
    UsedBy,
    /// Schema A is produced by Data Pipeline B
    ProducedBy,
    /// Schema A is consumed by Data Pipeline B
    ConsumedBy,
    /// Schema A is used to train LLM Model B
    TrainsModel,
    /// Schema A inherits from Schema B
    Inherits,
    /// Schema A composes Schema B (contains it as a field)
    Composes,
    /// Schema A derives from Schema B (transformation)
    DerivedFrom,
    /// Schema A is validated by Schema B (JSON Schema meta-schema)
    ValidatedBy,
}

impl RelationType {
    /// Check if this is a schema-to-schema relationship
    pub fn is_schema_relation(&self) -> bool {
        matches!(
            self,
            RelationType::DependsOn
                | RelationType::Inherits
                | RelationType::Composes
                | RelationType::DerivedFrom
                | RelationType::ValidatedBy
        )
    }

    /// Check if this is an application relationship
    pub fn is_application_relation(&self) -> bool {
        matches!(self, RelationType::UsedBy)
    }

    /// Check if this is a pipeline relationship
    pub fn is_pipeline_relation(&self) -> bool {
        matches!(self, RelationType::ProducedBy | RelationType::ConsumedBy)
    }

    /// Check if this is an LLM model relationship
    pub fn is_model_relation(&self) -> bool {
        matches!(self, RelationType::TrainsModel)
    }
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationType::DependsOn => write!(f, "DEPENDS_ON"),
            RelationType::UsedBy => write!(f, "USED_BY"),
            RelationType::ProducedBy => write!(f, "PRODUCED_BY"),
            RelationType::ConsumedBy => write!(f, "CONSUMED_BY"),
            RelationType::TrainsModel => write!(f, "TRAINS_MODEL"),
            RelationType::Inherits => write!(f, "INHERITS"),
            RelationType::Composes => write!(f, "COMPOSES"),
            RelationType::DerivedFrom => write!(f, "DERIVED_FROM"),
            RelationType::ValidatedBy => write!(f, "VALIDATED_BY"),
        }
    }
}

/// Entity type in the lineage graph
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntityType {
    /// Schema entity
    Schema,
    /// Application entity
    Application,
    /// Data pipeline entity
    Pipeline,
    /// LLM model entity
    Model,
}

/// External entity (non-schema) in the lineage graph
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalEntity {
    /// Entity identifier
    pub id: String,
    /// Entity type
    pub entity_type: EntityType,
    /// Human-readable name
    pub name: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Represents a dependency edge in the lineage graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Source node (dependent)
    pub from: SchemaNode,
    /// Target node or entity (dependency)
    pub to: DependencyTarget,
    /// Type of relationship
    pub relation: RelationType,
    /// When this dependency was created
    pub created_at: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Target of a dependency (can be a schema or external entity)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DependencyTarget {
    /// Another schema
    Schema(SchemaNode),
    /// External entity (application, pipeline, model)
    External(ExternalEntity),
}

impl DependencyTarget {
    /// Get the identifier for this target
    pub fn id(&self) -> String {
        match self {
            DependencyTarget::Schema(node) => node.key(),
            DependencyTarget::External(entity) => entity.id.clone(),
        }
    }
}

/// A dependent that depends on a schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependent {
    /// The dependent node
    pub node: SchemaNode,
    /// Type of relationship
    pub relation: RelationType,
    /// When this dependency was created
    pub created_at: DateTime<Utc>,
}

/// Complete dependency graph structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// All nodes in the graph (schema nodes only)
    pub nodes: HashMap<SchemaId, SchemaNode>,
    /// All edges in the graph
    pub edges: Vec<Dependency>,
    /// Adjacency list for fast traversal (schema_id -> list of connected schema_ids)
    pub adjacency_list: HashMap<SchemaId, Vec<SchemaId>>,
    /// Reverse adjacency list (for upstream queries)
    pub reverse_adjacency_list: HashMap<SchemaId, Vec<SchemaId>>,
    /// External entities in the graph
    pub external_entities: HashMap<String, ExternalEntity>,
}

impl DependencyGraph {
    /// Create an empty dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency_list: HashMap::new(),
            reverse_adjacency_list: HashMap::new(),
            external_entities: HashMap::new(),
        }
    }

    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if the graph contains a specific schema
    pub fn contains_schema(&self, schema_id: &SchemaId) -> bool {
        self.nodes.contains_key(schema_id)
    }

    /// Get all direct dependencies of a schema
    pub fn get_direct_dependencies(&self, schema_id: &SchemaId) -> Vec<&SchemaId> {
        self.adjacency_list
            .get(schema_id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Get all direct dependents of a schema (what depends on this schema)
    pub fn get_direct_dependents(&self, schema_id: &SchemaId) -> Vec<&SchemaId> {
        self.reverse_adjacency_list
            .get(schema_id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of schema change
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SchemaChange {
    /// A field was added
    FieldAdded { name: String },
    /// A field was removed
    FieldRemoved { name: String },
    /// A field's type was changed
    FieldTypeChanged { name: String, old_type: String, new_type: String },
    /// A field was made required
    FieldMadeRequired { name: String },
    /// A field was made optional
    FieldMadeOptional { name: String },
    /// An enum value was added
    EnumValueAdded { enum_name: String, value: String },
    /// An enum value was removed
    EnumValueRemoved { enum_name: String, value: String },
    /// A constraint was added
    ConstraintAdded { field: String, constraint: String },
    /// A constraint was removed
    ConstraintRemoved { field: String, constraint: String },
    /// The schema format changed
    FormatChanged { old_format: String, new_format: String },
    /// Major version change
    MajorVersionChange { old_version: String, new_version: String },
}

/// Risk level for impact analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    /// Low risk - fewer than 10 affected items
    Low,
    /// Medium risk - 10-50 affected items
    Medium,
    /// High risk - 50-200 affected items
    High,
    /// Critical risk - more than 200 affected items
    Critical,
}

impl RiskLevel {
    /// Calculate risk level from affected item count
    pub fn from_count(count: usize) -> Self {
        match count {
            0..=9 => RiskLevel::Low,
            10..=49 => RiskLevel::Medium,
            50..=199 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low risk - minimal impact",
            RiskLevel::Medium => "Medium risk - moderate impact",
            RiskLevel::High => "High risk - significant impact",
            RiskLevel::Critical => "Critical risk - extensive impact",
        }
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
            RiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Impact analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    /// Target schema being analyzed
    pub target_schema: SchemaId,
    /// Proposed change to the schema
    pub proposed_change: SchemaChange,
    /// Affected schemas (downstream dependencies)
    pub affected_schemas: Vec<SchemaId>,
    /// Affected applications
    pub affected_applications: Vec<String>,
    /// Affected data pipelines
    pub affected_pipelines: Vec<String>,
    /// Affected LLM models
    pub affected_models: Vec<String>,
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Estimated migration complexity (0.0 to 1.0)
    pub migration_complexity: f64,
    /// Estimated effort in hours
    pub estimated_effort_hours: f64,
    /// Detailed breakdown by depth
    pub depth_breakdown: HashMap<usize, usize>,
    /// When this report was generated
    pub generated_at: DateTime<Utc>,
    /// Recommendations for migration
    pub recommendations: Vec<String>,
}

impl ImpactReport {
    /// Calculate the total number of affected items
    pub fn total_affected(&self) -> usize {
        self.affected_schemas.len()
            + self.affected_applications.len()
            + self.affected_pipelines.len()
            + self.affected_models.len()
    }

    /// Check if the change is breaking
    pub fn is_breaking(&self) -> bool {
        matches!(
            self.proposed_change,
            SchemaChange::FieldRemoved { .. }
                | SchemaChange::FieldTypeChanged { .. }
                | SchemaChange::FieldMadeRequired { .. }
                | SchemaChange::EnumValueRemoved { .. }
                | SchemaChange::FormatChanged { .. }
                | SchemaChange::MajorVersionChange { .. }
        )
    }
}

/// Circular dependency detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    /// The cycle path (list of schema IDs forming the cycle)
    pub cycle: Vec<SchemaId>,
    /// When this cycle was detected
    pub detected_at: DateTime<Utc>,
}

impl CircularDependency {
    /// Get the length of the cycle
    pub fn length(&self) -> usize {
        self.cycle.len()
    }

    /// Check if a schema is part of this cycle
    pub fn contains(&self, schema_id: &SchemaId) -> bool {
        self.cycle.contains(schema_id)
    }
}

/// Query filter for lineage queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineageFilter {
    /// Filter by relation type
    pub relation_types: Option<Vec<RelationType>>,
    /// Filter by entity type
    pub entity_types: Option<Vec<EntityType>>,
    /// Filter by schema namespace
    pub namespaces: Option<Vec<String>>,
    /// Maximum depth for traversal
    pub max_depth: Option<usize>,
    /// Filter by time range (created after)
    pub created_after: Option<DateTime<Utc>>,
    /// Filter by time range (created before)
    pub created_before: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_node_creation() {
        let id = Uuid::new_v4();
        let version = SemanticVersion::new(1, 0, 0);
        let fqn = "com.example.User".to_string();

        let node = SchemaNode::new(id, version.clone(), fqn.clone());

        assert_eq!(node.schema_id, id);
        assert_eq!(node.schema_version, version);
        assert_eq!(node.fqn, fqn);
        assert!(node.metadata.is_empty());
    }

    #[test]
    fn test_schema_node_key() {
        let id = Uuid::new_v4();
        let version = SemanticVersion::new(1, 2, 3);
        let node = SchemaNode::new(id, version, "test".to_string());

        let key = node.key();
        assert!(key.contains(&id.to_string()));
        assert!(key.contains("1.2.3"));
    }

    #[test]
    fn test_relation_type_categories() {
        assert!(RelationType::DependsOn.is_schema_relation());
        assert!(RelationType::Inherits.is_schema_relation());
        assert!(RelationType::Composes.is_schema_relation());

        assert!(RelationType::UsedBy.is_application_relation());

        assert!(RelationType::ProducedBy.is_pipeline_relation());
        assert!(RelationType::ConsumedBy.is_pipeline_relation());

        assert!(RelationType::TrainsModel.is_model_relation());
    }

    #[test]
    fn test_risk_level_from_count() {
        assert_eq!(RiskLevel::from_count(0), RiskLevel::Low);
        assert_eq!(RiskLevel::from_count(5), RiskLevel::Low);
        assert_eq!(RiskLevel::from_count(9), RiskLevel::Low);
        assert_eq!(RiskLevel::from_count(10), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_count(25), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_count(49), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_count(50), RiskLevel::High);
        assert_eq!(RiskLevel::from_count(100), RiskLevel::High);
        assert_eq!(RiskLevel::from_count(199), RiskLevel::High);
        assert_eq!(RiskLevel::from_count(200), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_count(1000), RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_dependency_graph_creation() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_circular_dependency() {
        let cycle = CircularDependency {
            cycle: vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()],
            detected_at: Utc::now(),
        };

        assert_eq!(cycle.length(), 3);
        assert!(cycle.contains(&cycle.cycle[0]));
        assert!(!cycle.contains(&Uuid::new_v4()));
    }
}
