//! Main lineage tracking engine
//!
//! This module provides the main LineageEngine that orchestrates all
//! lineage tracking operations and implements the LineageTracker trait.

use crate::algorithms::GraphAlgorithms;
use crate::error::Result;
use crate::export::LineageExporter;
use crate::graph_store::GraphStore;
use crate::impact::ImpactAnalyzer;
use crate::tracker::{DependencyTracker, DependencyTrackerImpl};
use crate::types::{
    CircularDependency, Dependency, DependencyGraph, DependencyTarget, Dependent, ImpactReport,
    RelationType, SchemaChange, SchemaId, SchemaNode,
};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

/// Main lineage tracking engine
#[derive(Clone)]
pub struct LineageEngine {
    store: GraphStore,
    tracker: DependencyTrackerImpl,
    impact_analyzer: ImpactAnalyzer,
    exporter: LineageExporter,
    algorithms: GraphAlgorithms,
}

impl LineageEngine {
    /// Create a new lineage engine
    pub fn new() -> Self {
        let store = GraphStore::new();
        let tracker = DependencyTrackerImpl::new(store.clone());
        let impact_analyzer = ImpactAnalyzer::new(store.clone());
        let exporter = LineageExporter::new(store.clone());
        let algorithms = GraphAlgorithms::new(store.clone());

        info!("Lineage engine initialized");

        Self {
            store,
            tracker,
            impact_analyzer,
            exporter,
            algorithms,
        }
    }

    /// Create a new lineage engine with an existing store
    pub fn with_store(store: GraphStore) -> Self {
        let tracker = DependencyTrackerImpl::new(store.clone());
        let impact_analyzer = ImpactAnalyzer::new(store.clone());
        let exporter = LineageExporter::new(store.clone());
        let algorithms = GraphAlgorithms::new(store.clone());

        info!("Lineage engine initialized with existing store");

        Self {
            store,
            tracker,
            impact_analyzer,
            exporter,
            algorithms,
        }
    }

    /// Track a new dependency
    pub async fn track_dependency(
        &self,
        from: SchemaNode,
        to: DependencyTarget,
        relation: RelationType,
    ) -> Result<()> {
        self.tracker.track_dependency(from, to, relation).await
    }

    /// Remove a dependency
    pub async fn remove_dependency(&self, from: SchemaId, to: String) -> Result<()> {
        self.tracker.remove_dependency(from, to).await
    }

    /// Get upstream dependencies
    pub async fn get_upstream(&self, schema_id: SchemaId) -> Result<Vec<Dependency>> {
        self.tracker.get_upstream(schema_id).await
    }

    /// Get downstream dependents
    pub async fn get_downstream(&self, schema_id: SchemaId) -> Result<Vec<Dependent>> {
        self.tracker.get_downstream(schema_id).await
    }

    /// Get transitive dependencies with depth limit
    pub async fn get_transitive(
        &self,
        schema_id: SchemaId,
        depth: usize,
    ) -> Result<DependencyGraph> {
        self.tracker.get_transitive(schema_id, Some(depth)).await
    }

    /// Perform impact analysis
    pub async fn impact_analysis(
        &self,
        schema_id: SchemaId,
        proposed_change: SchemaChange,
    ) -> Result<ImpactReport> {
        self.impact_analyzer.analyze_impact(schema_id, proposed_change).await
    }

    /// Detect circular dependencies
    pub async fn detect_circular(&self) -> Result<Vec<CircularDependency>> {
        debug!("Detecting circular dependencies");

        let circular_deps = self.algorithms.detect_circular_dependencies()?;

        if !circular_deps.is_empty() {
            info!("Found {} circular dependencies", circular_deps.len());
        } else {
            info!("No circular dependencies detected");
        }

        Ok(circular_deps)
    }

    /// Export to GraphML format
    pub fn export_graphml(&self) -> Result<String> {
        self.exporter.export_graphml()
    }

    /// Export to DOT format
    pub fn export_dot(&self) -> Result<String> {
        self.exporter.export_dot()
    }

    /// Export to JSON format
    pub fn export_json(&self) -> Result<String> {
        self.exporter.export_json()
    }

    /// Get graph statistics
    pub fn stats(&self) -> crate::graph_store::GraphStats {
        self.store.stats()
    }

    /// Clear all lineage data
    pub fn clear(&self) {
        self.store.clear();
    }

    /// Get all schemas
    pub fn get_all_schemas(&self) -> Vec<SchemaNode> {
        self.store.get_all_schemas()
    }

    /// Check if a schema exists
    pub fn contains_schema(&self, schema_id: &SchemaId) -> bool {
        self.store.contains_schema(schema_id)
    }

    /// Get a schema node by ID
    pub fn get_schema_node(&self, schema_id: &SchemaId) -> Result<SchemaNode> {
        self.store.get_schema_node(schema_id)
    }

    /// Perform breadth-first search
    pub fn bfs(&self, start: &SchemaId, max_depth: Option<usize>) -> Result<Vec<SchemaId>> {
        self.algorithms.bfs(start, max_depth)
    }

    /// Perform depth-first search
    pub fn dfs(&self, start: &SchemaId, max_depth: Option<usize>) -> Result<Vec<SchemaId>> {
        self.algorithms.dfs(start, max_depth)
    }

    /// Find shortest path between two schemas
    pub fn shortest_path(&self, from: &SchemaId, to: &SchemaId) -> Result<Option<Vec<SchemaId>>> {
        self.algorithms.shortest_path(from, to)
    }

    /// Check if there's a path between two schemas
    pub fn has_path(&self, from: &SchemaId, to: &SchemaId) -> Result<bool> {
        self.algorithms.has_path(from, to)
    }

    /// Get all root schemas (no dependencies)
    pub fn get_roots(&self) -> Vec<SchemaId> {
        self.algorithms.get_roots()
    }

    /// Get all leaf schemas (nothing depends on them)
    pub fn get_leaves(&self) -> Vec<SchemaId> {
        self.algorithms.get_leaves()
    }

    /// Perform topological sort
    pub fn topological_sort(&self) -> Result<Vec<SchemaId>> {
        self.algorithms.topological_sort()
    }

    /// Bulk track dependencies
    pub async fn bulk_track_dependencies(
        &self,
        dependencies: Vec<(SchemaNode, DependencyTarget, RelationType)>,
    ) -> Result<Vec<Result<()>>> {
        self.tracker.bulk_track_dependencies(dependencies).await
    }

    /// Get dependencies with depth information
    pub async fn get_dependencies_with_depth(
        &self,
        schema_id: SchemaId,
        max_depth: Option<usize>,
    ) -> Result<HashMap<SchemaId, usize>> {
        self.tracker.get_dependencies_with_depth(schema_id, max_depth).await
    }

    /// Get dependents with depth information
    pub async fn get_dependents_with_depth(
        &self,
        schema_id: SchemaId,
        max_depth: Option<usize>,
    ) -> Result<HashMap<SchemaId, usize>> {
        self.tracker.get_dependents_with_depth(schema_id, max_depth).await
    }

    /// Find shortest dependency path
    pub async fn find_shortest_path(
        &self,
        from: SchemaId,
        to: SchemaId,
    ) -> Result<Option<Vec<SchemaId>>> {
        self.tracker.find_shortest_path(from, to).await
    }

    /// Get the complete dependency graph
    pub async fn get_full_graph(&self) -> Result<DependencyGraph> {
        self.tracker.get_full_graph().await
    }

    /// Get all root schemas using tracker
    pub async fn get_root_schemas(&self) -> Result<Vec<SchemaId>> {
        self.tracker.get_root_schemas().await
    }

    /// Get all leaf schemas using tracker
    pub async fn get_leaf_schemas(&self) -> Result<Vec<SchemaId>> {
        self.tracker.get_leaf_schemas().await
    }
}

impl Default for LineageEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for lineage tracking (as specified in requirements)
#[async_trait]
pub trait LineageTracker: Send + Sync {
    /// Track a dependency between schemas or entities
    async fn track_dependency(
        &self,
        from: SchemaId,
        to: SchemaId,
        relation: RelationType,
    ) -> Result<()>;

    /// Get upstream dependencies (what this schema depends on)
    async fn get_upstream(&self, schema_id: SchemaId) -> Result<Vec<Dependency>>;

    /// Get downstream dependents (what depends on this schema)
    async fn get_downstream(&self, schema_id: SchemaId) -> Result<Vec<Dependent>>;

    /// Get transitive dependencies up to a certain depth
    async fn get_transitive(&self, schema_id: SchemaId, depth: usize) -> Result<DependencyGraph>;

    /// Perform impact analysis for a proposed schema change
    async fn impact_analysis(
        &self,
        schema_id: SchemaId,
        proposed_change: SchemaChange,
    ) -> Result<ImpactReport>;

    /// Detect circular dependencies in the graph
    async fn detect_circular(&self) -> Result<Vec<CircularDependency>>;
}

#[async_trait]
impl LineageTracker for LineageEngine {
    async fn track_dependency(
        &self,
        from: SchemaId,
        to: SchemaId,
        relation: RelationType,
    ) -> Result<()> {
        debug!("Tracking dependency: {} -> {} ({:?})", from, to, relation);

        // Get or create nodes
        let from_node = self.store.get_schema_node(&from).unwrap_or_else(|_| {
            SchemaNode::new(
                from,
                schema_registry_core::versioning::SemanticVersion::new(1, 0, 0),
                format!("schema-{}", from),
            )
        });

        let to_node = self.store.get_schema_node(&to).unwrap_or_else(|_| {
            SchemaNode::new(
                to,
                schema_registry_core::versioning::SemanticVersion::new(1, 0, 0),
                format!("schema-{}", to),
            )
        });

        self.tracker
            .track_dependency(from_node, DependencyTarget::Schema(to_node), relation)
            .await
    }

    async fn get_upstream(&self, schema_id: SchemaId) -> Result<Vec<Dependency>> {
        self.tracker.get_upstream(schema_id).await
    }

    async fn get_downstream(&self, schema_id: SchemaId) -> Result<Vec<Dependent>> {
        self.tracker.get_downstream(schema_id).await
    }

    async fn get_transitive(&self, schema_id: SchemaId, depth: usize) -> Result<DependencyGraph> {
        self.tracker.get_transitive(schema_id, Some(depth)).await
    }

    async fn impact_analysis(
        &self,
        schema_id: SchemaId,
        proposed_change: SchemaChange,
    ) -> Result<ImpactReport> {
        self.impact_analyzer.analyze_impact(schema_id, proposed_change).await
    }

    async fn detect_circular(&self) -> Result<Vec<CircularDependency>> {
        self.algorithms.detect_circular_dependencies()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema_registry_core::versioning::SemanticVersion;

    fn create_test_schema(id: SchemaId, name: &str) -> SchemaNode {
        SchemaNode::new(
            id,
            SemanticVersion::new(1, 0, 0),
            format!("com.example.{}", name),
        )
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = LineageEngine::new();
        let stats = engine.stats();

        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
    }

    #[tokio::test]
    async fn test_track_dependency() {
        let engine = LineageEngine::new();

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Profile");

        let result = engine.track_dependency(
            node1,
            DependencyTarget::Schema(node2),
            RelationType::DependsOn
        ).await;

        assert!(result.is_ok());
        assert_eq!(engine.stats().node_count, 2);
        assert_eq!(engine.stats().edge_count, 1);
    }

    #[tokio::test]
    async fn test_get_upstream_downstream() {
        let engine = LineageEngine::new();

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Profile");

        engine.track_dependency(
            node1,
            DependencyTarget::Schema(node2),
            RelationType::DependsOn
        ).await.unwrap();

        let upstream = engine.get_upstream(id1).await.unwrap();
        assert_eq!(upstream.len(), 1);

        let downstream = engine.get_downstream(id2).await.unwrap();
        assert_eq!(downstream.len(), 1);
    }

    #[tokio::test]
    async fn test_impact_analysis() {
        let engine = LineageEngine::new();

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Profile");

        engine.track_dependency(
            node1,
            DependencyTarget::Schema(node2),
            RelationType::DependsOn
        ).await.unwrap();

        let change = SchemaChange::FieldRemoved {
            name: "test_field".to_string(),
        };

        let report = engine.impact_analysis(id2, change).await.unwrap();

        assert_eq!(report.target_schema, id2);
        assert!(report.is_breaking());
    }

    #[tokio::test]
    async fn test_detect_circular() {
        let engine = LineageEngine::new();

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();
        let id3 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "A");
        let node2 = create_test_schema(id2, "B");
        let node3 = create_test_schema(id3, "C");

        // Create a cycle: 1 -> 2 -> 3 -> 1
        engine.track_dependency(node1.clone(), DependencyTarget::Schema(node2.clone()), RelationType::DependsOn).await.unwrap();
        engine.track_dependency(node2, DependencyTarget::Schema(node3.clone()), RelationType::DependsOn).await.unwrap();
        engine.track_dependency(node3, DependencyTarget::Schema(node1), RelationType::DependsOn).await.unwrap();

        let circular = engine.detect_circular().await.unwrap();

        assert!(!circular.is_empty());
    }

    #[tokio::test]
    async fn test_export_formats() {
        let engine = LineageEngine::new();

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Profile");

        engine.track_dependency(
            node1,
            DependencyTarget::Schema(node2),
            RelationType::DependsOn
        ).await.unwrap();

        let graphml = engine.export_graphml().unwrap();
        assert!(graphml.contains("graphml"));

        let dot = engine.export_dot().unwrap();
        assert!(dot.contains("digraph"));

        let json = engine.export_json().unwrap();
        assert!(json.contains("nodes"));
    }

    #[tokio::test]
    async fn test_graph_algorithms() {
        let engine = LineageEngine::new();

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();
        let id3 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "A");
        let node2 = create_test_schema(id2, "B");
        let node3 = create_test_schema(id3, "C");

        engine.track_dependency(node1.clone(), DependencyTarget::Schema(node2.clone()), RelationType::DependsOn).await.unwrap();
        engine.track_dependency(node2, DependencyTarget::Schema(node3), RelationType::DependsOn).await.unwrap();

        // BFS
        let bfs_result = engine.bfs(&id1, None).unwrap();
        assert!(bfs_result.len() >= 2);

        // DFS
        let dfs_result = engine.dfs(&id1, None).unwrap();
        assert!(dfs_result.len() >= 2);

        // Has path
        assert!(engine.has_path(&id1, &id3).unwrap());
        assert!(!engine.has_path(&id3, &id1).unwrap());

        // Shortest path
        let path = engine.shortest_path(&id1, &id3).unwrap();
        assert!(path.is_some());
    }
}
