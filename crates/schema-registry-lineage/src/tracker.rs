//! Dependency tracking operations
//!
//! This module provides high-level operations for tracking and managing
//! dependencies between schemas and other entities.

use crate::algorithms::GraphAlgorithms;
use crate::error::Result;
use crate::graph_store::GraphStore;
use crate::types::{
    Dependency, DependencyGraph, DependencyTarget, Dependent, RelationType, SchemaId, SchemaNode,
};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

/// Trait for dependency tracking operations
#[async_trait]
pub trait DependencyTracker: Send + Sync {
    /// Track a new dependency
    async fn track_dependency(
        &self,
        from: SchemaNode,
        to: DependencyTarget,
        relation: RelationType,
    ) -> Result<()>;

    /// Remove a dependency
    async fn remove_dependency(&self, from: SchemaId, to: String) -> Result<()>;

    /// Get direct upstream dependencies (what this schema depends on)
    async fn get_upstream(&self, schema_id: SchemaId) -> Result<Vec<Dependency>>;

    /// Get direct downstream dependents (what depends on this schema)
    async fn get_downstream(&self, schema_id: SchemaId) -> Result<Vec<Dependent>>;

    /// Get transitive dependencies up to a certain depth
    async fn get_transitive(
        &self,
        schema_id: SchemaId,
        max_depth: Option<usize>,
    ) -> Result<DependencyGraph>;

    /// Get all dependencies with depth information
    async fn get_dependencies_with_depth(
        &self,
        schema_id: SchemaId,
        max_depth: Option<usize>,
    ) -> Result<HashMap<SchemaId, usize>>;

    /// Get all dependents with depth information
    async fn get_dependents_with_depth(
        &self,
        schema_id: SchemaId,
        max_depth: Option<usize>,
    ) -> Result<HashMap<SchemaId, usize>>;

    /// Check if a dependency path exists
    async fn has_dependency_path(&self, from: SchemaId, to: SchemaId) -> Result<bool>;

    /// Find shortest dependency path
    async fn find_shortest_path(&self, from: SchemaId, to: SchemaId) -> Result<Option<Vec<SchemaId>>>;

    /// Get the complete dependency graph
    async fn get_full_graph(&self) -> Result<DependencyGraph>;

    /// Get all schemas with no dependencies
    async fn get_root_schemas(&self) -> Result<Vec<SchemaId>>;

    /// Get all schemas that nothing depends on
    async fn get_leaf_schemas(&self) -> Result<Vec<SchemaId>>;
}

/// Implementation of dependency tracker
#[derive(Clone)]
pub struct DependencyTrackerImpl {
    store: GraphStore,
    algorithms: GraphAlgorithms,
}

impl DependencyTrackerImpl {
    /// Create a new dependency tracker
    pub fn new(store: GraphStore) -> Self {
        let algorithms = GraphAlgorithms::new(store.clone());
        Self { store, algorithms }
    }

    /// Bulk add dependencies
    pub async fn bulk_track_dependencies(&self, dependencies: Vec<(SchemaNode, DependencyTarget, RelationType)>) -> Result<Vec<Result<()>>> {
        let mut results = Vec::new();

        for (from, to, relation) in dependencies {
            let result = self.track_dependency(from, to, relation).await;
            results.push(result);
        }

        Ok(results)
    }

    /// Get graph statistics
    pub fn get_stats(&self) -> crate::graph_store::GraphStats {
        self.store.stats()
    }

    /// Clear all tracked dependencies
    pub fn clear(&self) {
        self.store.clear();
        info!("All dependencies cleared");
    }
}

#[async_trait]
impl DependencyTracker for DependencyTrackerImpl {
    async fn track_dependency(
        &self,
        from: SchemaNode,
        to: DependencyTarget,
        relation: RelationType,
    ) -> Result<()> {
        debug!(
            "Tracking dependency: {} -> {} ({:?})",
            from.key(),
            to.id(),
            relation
        );

        self.store.add_dependency(from, to, relation)?;

        info!("Dependency tracked successfully");
        Ok(())
    }

    async fn remove_dependency(&self, from: SchemaId, to: String) -> Result<()> {
        debug!("Removing dependency: {} -> {}", from, to);

        self.store.remove_dependency(&from, &to)?;

        info!("Dependency removed successfully");
        Ok(())
    }

    async fn get_upstream(&self, schema_id: SchemaId) -> Result<Vec<Dependency>> {
        debug!("Getting upstream dependencies for schema: {}", schema_id);

        let dependencies = self.store.get_dependencies(&schema_id)?;

        info!("Found {} upstream dependencies", dependencies.len());
        Ok(dependencies)
    }

    async fn get_downstream(&self, schema_id: SchemaId) -> Result<Vec<Dependent>> {
        debug!("Getting downstream dependents for schema: {}", schema_id);

        let dependencies = self.store.get_dependents(&schema_id)?;

        // Convert to Dependent type
        let dependents: Vec<Dependent> = dependencies
            .into_iter()
            .map(|dep| Dependent {
                node: dep.from,
                relation: dep.relation,
                created_at: dep.created_at,
            })
            .collect();

        info!("Found {} downstream dependents", dependents.len());
        Ok(dependents)
    }

    async fn get_transitive(
        &self,
        schema_id: SchemaId,
        max_depth: Option<usize>,
    ) -> Result<DependencyGraph> {
        debug!(
            "Getting transitive dependencies for schema: {} (max_depth: {:?})",
            schema_id, max_depth
        );

        // Get all reachable schemas
        let reachable = self.algorithms.get_transitive_dependencies(&schema_id, max_depth)?;

        let mut graph = DependencyGraph::new();

        // Add the starting node
        let start_node = self.store.get_schema_node(&schema_id)?;
        graph.nodes.insert(schema_id, start_node);

        // Add all reachable nodes and their edges
        for (dep_id, _depth) in reachable.iter() {
            if let Ok(node) = self.store.get_schema_node(dep_id) {
                graph.nodes.insert(*dep_id, node.clone());

                // Get edges for this node
                if let Ok(dependencies) = self.store.get_dependencies(dep_id) {
                    for dep in dependencies {
                        if let DependencyTarget::Schema(target_node) = &dep.to {
                            if reachable.contains_key(&target_node.schema_id) {
                                graph.edges.push(dep.clone());

                                // Update adjacency lists
                                graph
                                    .adjacency_list
                                    .entry(*dep_id)
                                    .or_default()
                                    .push(target_node.schema_id);

                                graph
                                    .reverse_adjacency_list
                                    .entry(target_node.schema_id)
                                    .or_default()
                                    .push(*dep_id);
                            }
                        }
                    }
                }
            }
        }

        info!(
            "Transitive graph has {} nodes and {} edges",
            graph.nodes.len(),
            graph.edges.len()
        );

        Ok(graph)
    }

    async fn get_dependencies_with_depth(
        &self,
        schema_id: SchemaId,
        max_depth: Option<usize>,
    ) -> Result<HashMap<SchemaId, usize>> {
        debug!(
            "Getting dependencies with depth for schema: {} (max_depth: {:?})",
            schema_id, max_depth
        );

        let result = self.algorithms.get_transitive_dependencies(&schema_id, max_depth)?;

        info!("Found {} dependencies", result.len());
        Ok(result)
    }

    async fn get_dependents_with_depth(
        &self,
        schema_id: SchemaId,
        max_depth: Option<usize>,
    ) -> Result<HashMap<SchemaId, usize>> {
        debug!(
            "Getting dependents with depth for schema: {} (max_depth: {:?})",
            schema_id, max_depth
        );

        let result = self.algorithms.get_transitive_dependents(&schema_id, max_depth)?;

        info!("Found {} dependents", result.len());
        Ok(result)
    }

    async fn has_dependency_path(&self, from: SchemaId, to: SchemaId) -> Result<bool> {
        debug!("Checking dependency path: {} -> {}", from, to);

        let has_path = self.algorithms.has_path(&from, &to)?;

        info!("Dependency path exists: {}", has_path);
        Ok(has_path)
    }

    async fn find_shortest_path(&self, from: SchemaId, to: SchemaId) -> Result<Option<Vec<SchemaId>>> {
        debug!("Finding shortest path: {} -> {}", from, to);

        let path = self.algorithms.shortest_path(&from, &to)?;

        if let Some(ref p) = path {
            info!("Found path with length: {}", p.len());
        } else {
            info!("No path found");
        }

        Ok(path)
    }

    async fn get_full_graph(&self) -> Result<DependencyGraph> {
        debug!("Getting full dependency graph");

        let graph = self.store.to_dependency_graph();

        info!(
            "Full graph has {} nodes and {} edges",
            graph.nodes.len(),
            graph.edges.len()
        );

        Ok(graph)
    }

    async fn get_root_schemas(&self) -> Result<Vec<SchemaId>> {
        debug!("Getting root schemas (no dependencies)");

        let roots = self.algorithms.get_roots();

        info!("Found {} root schemas", roots.len());
        Ok(roots)
    }

    async fn get_leaf_schemas(&self) -> Result<Vec<SchemaId>> {
        debug!("Getting leaf schemas (nothing depends on them)");

        let leaves = self.algorithms.get_leaves();

        info!("Found {} leaf schemas", leaves.len());
        Ok(leaves)
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
    async fn test_track_dependency() {
        let store = GraphStore::new();
        let tracker = DependencyTrackerImpl::new(store);

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Address");

        let result = tracker
            .track_dependency(
                node1,
                DependencyTarget::Schema(node2),
                RelationType::Composes,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_upstream() {
        let store = GraphStore::new();
        let tracker = DependencyTrackerImpl::new(store);

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Address");

        tracker
            .track_dependency(
                node1.clone(),
                DependencyTarget::Schema(node2),
                RelationType::Composes,
            )
            .await
            .unwrap();

        let upstream = tracker.get_upstream(id1).await.unwrap();
        assert_eq!(upstream.len(), 1);
    }

    #[tokio::test]
    async fn test_get_downstream() {
        let store = GraphStore::new();
        let tracker = DependencyTrackerImpl::new(store);

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Address");

        tracker
            .track_dependency(
                node1,
                DependencyTarget::Schema(node2.clone()),
                RelationType::Composes,
            )
            .await
            .unwrap();

        let downstream = tracker.get_downstream(id2).await.unwrap();
        assert_eq!(downstream.len(), 1);
    }

    #[tokio::test]
    async fn test_has_dependency_path() {
        let store = GraphStore::new();
        let tracker = DependencyTrackerImpl::new(store);

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();
        let id3 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "A");
        let node2 = create_test_schema(id2, "B");
        let node3 = create_test_schema(id3, "C");

        // A -> B -> C
        tracker
            .track_dependency(
                node1.clone(),
                DependencyTarget::Schema(node2.clone()),
                RelationType::DependsOn,
            )
            .await
            .unwrap();

        tracker
            .track_dependency(
                node2,
                DependencyTarget::Schema(node3),
                RelationType::DependsOn,
            )
            .await
            .unwrap();

        assert!(tracker.has_dependency_path(id1, id3).await.unwrap());
        assert!(!tracker.has_dependency_path(id3, id1).await.unwrap());
    }
}
