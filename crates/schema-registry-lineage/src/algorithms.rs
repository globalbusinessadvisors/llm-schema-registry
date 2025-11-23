//! Graph algorithms for lineage analysis
//!
//! This module provides graph algorithms including BFS, DFS, cycle detection,
//! shortest paths, and transitive closure calculations.

use crate::error::{LineageError, Result};
use crate::graph_store::GraphStore;
use crate::types::{CircularDependency, SchemaId};
use petgraph::algo::{is_cyclic_directed, kosaraju_scc, toposort};
use petgraph::visit::Dfs;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::{debug, warn};

/// Graph algorithms for lineage analysis
#[derive(Clone)]
pub struct GraphAlgorithms {
    store: GraphStore,
}

impl GraphAlgorithms {
    /// Create a new algorithms instance
    pub fn new(store: GraphStore) -> Self {
        Self { store }
    }

    /// Perform breadth-first search from a schema
    pub fn bfs(&self, start: &SchemaId, max_depth: Option<usize>) -> Result<Vec<SchemaId>> {
        let graph = self.store.get_petgraph();
        let schema_index = self.store.get_schema_index();

        let graph_read = graph.read();
        let index_read = schema_index.read();

        let start_idx = index_read
            .get(start)
            .ok_or_else(|| LineageError::SchemaNotFound(*start))?;

        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back((*start_idx, 0));
        visited.insert(*start_idx);

        while let Some((node_idx, depth)) = queue.pop_front() {
            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
            }

            // Try to get schema ID from node
            if let Some(schema_id) = self.get_schema_id_from_node(&graph_read, &index_read, node_idx) {
                result.push(schema_id);
            }

            for neighbor in graph_read.neighbors(node_idx) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        Ok(result)
    }

    /// Perform depth-first search from a schema
    pub fn dfs(&self, start: &SchemaId, max_depth: Option<usize>) -> Result<Vec<SchemaId>> {
        let graph = self.store.get_petgraph();
        let schema_index = self.store.get_schema_index();

        let graph_read = graph.read();
        let index_read = schema_index.read();

        let start_idx = index_read
            .get(start)
            .ok_or_else(|| LineageError::SchemaNotFound(*start))?;

        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut stack = Vec::new();

        stack.push((*start_idx, 0));

        while let Some((node_idx, depth)) = stack.pop() {
            if visited.contains(&node_idx) {
                continue;
            }

            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
            }

            visited.insert(node_idx);

            if let Some(schema_id) = self.get_schema_id_from_node(&graph_read, &index_read, node_idx) {
                result.push(schema_id);
            }

            for neighbor in graph_read.neighbors(node_idx) {
                if !visited.contains(&neighbor) {
                    stack.push((neighbor, depth + 1));
                }
            }
        }

        Ok(result)
    }

    /// Get transitive dependencies (all reachable schemas)
    pub fn get_transitive_dependencies(
        &self,
        schema_id: &SchemaId,
        max_depth: Option<usize>,
    ) -> Result<HashMap<SchemaId, usize>> {
        let graph = self.store.get_petgraph();
        let schema_index = self.store.get_schema_index();

        let graph_read = graph.read();
        let index_read = schema_index.read();

        let start_idx = index_read
            .get(schema_id)
            .ok_or_else(|| LineageError::SchemaNotFound(*schema_id))?;

        let mut distances = HashMap::new();
        let mut queue = VecDeque::new();

        queue.push_back((*start_idx, 0));
        distances.insert(*start_idx, 0);

        while let Some((node_idx, depth)) = queue.pop_front() {
            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
            }

            for neighbor in graph_read.neighbors(node_idx) {
                if !distances.contains_key(&neighbor) {
                    distances.insert(neighbor, depth + 1);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        // Convert NodeIndex to SchemaId
        let mut result = HashMap::new();
        for (node_idx, depth) in distances {
            if let Some(schema_id) = self.get_schema_id_from_node(&graph_read, &index_read, node_idx) {
                result.insert(schema_id, depth);
            }
        }

        // Remove the starting schema
        result.remove(schema_id);

        Ok(result)
    }

    /// Get transitive dependents (all schemas that depend on this one)
    pub fn get_transitive_dependents(
        &self,
        schema_id: &SchemaId,
        max_depth: Option<usize>,
    ) -> Result<HashMap<SchemaId, usize>> {
        let graph = self.store.get_petgraph();
        let schema_index = self.store.get_schema_index();

        let graph_read = graph.read();
        let index_read = schema_index.read();

        let start_idx = index_read
            .get(schema_id)
            .ok_or_else(|| LineageError::SchemaNotFound(*schema_id))?;

        let mut distances = HashMap::new();
        let mut queue = VecDeque::new();

        queue.push_back((*start_idx, 0));
        distances.insert(*start_idx, 0);

        while let Some((node_idx, depth)) = queue.pop_front() {
            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
            }

            // Use incoming edges (dependents)
            for neighbor in graph_read.neighbors_directed(node_idx, Direction::Incoming) {
                if !distances.contains_key(&neighbor) {
                    distances.insert(neighbor, depth + 1);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        // Convert NodeIndex to SchemaId
        let mut result = HashMap::new();
        for (node_idx, depth) in distances {
            if let Some(schema_id) = self.get_schema_id_from_node(&graph_read, &index_read, node_idx) {
                result.insert(schema_id, depth);
            }
        }

        // Remove the starting schema
        result.remove(schema_id);

        Ok(result)
    }

    /// Detect all circular dependencies in the graph
    pub fn detect_circular_dependencies(&self) -> Result<Vec<CircularDependency>> {
        let graph = self.store.get_petgraph();
        let graph_read = graph.read();

        if !is_cyclic_directed(&*graph_read) {
            debug!("No cycles detected in graph");
            return Ok(Vec::new());
        }

        warn!("Cycles detected in lineage graph");

        // Find all strongly connected components
        let sccs = kosaraju_scc(&*graph_read);

        let mut circular_deps = Vec::new();
        let schema_index = self.store.get_schema_index();
        let index_read = schema_index.read();

        for scc in sccs {
            // Only interested in SCCs with more than one node (cycles)
            if scc.len() > 1 {
                let mut cycle_ids = Vec::new();

                for node_idx in scc {
                    if let Some(schema_id) = self.get_schema_id_from_node(&graph_read, &index_read, node_idx) {
                        cycle_ids.push(schema_id);
                    }
                }

                if !cycle_ids.is_empty() {
                    circular_deps.push(CircularDependency {
                        cycle: cycle_ids,
                        detected_at: chrono::Utc::now(),
                    });
                }
            }
        }

        Ok(circular_deps)
    }

    /// Perform topological sort (returns error if graph has cycles)
    pub fn topological_sort(&self) -> Result<Vec<SchemaId>> {
        let graph = self.store.get_petgraph();
        let graph_read = graph.read();

        match toposort(&*graph_read, None) {
            Ok(sorted) => {
                let schema_index = self.store.get_schema_index();
                let index_read = schema_index.read();

                let mut result = Vec::new();
                for node_idx in sorted {
                    if let Some(schema_id) = self.get_schema_id_from_node(&graph_read, &index_read, node_idx) {
                        result.push(schema_id);
                    }
                }

                Ok(result)
            }
            Err(_) => Err(LineageError::CircularDependency(
                "Cannot perform topological sort on graph with cycles".to_string(),
            )),
        }
    }

    /// Find shortest path between two schemas
    pub fn shortest_path(&self, from: &SchemaId, to: &SchemaId) -> Result<Option<Vec<SchemaId>>> {
        let graph = self.store.get_petgraph();
        let schema_index = self.store.get_schema_index();

        let graph_read = graph.read();
        let index_read = schema_index.read();

        let from_idx = index_read
            .get(from)
            .ok_or_else(|| LineageError::SchemaNotFound(*from))?;

        let to_idx = index_read
            .get(to)
            .ok_or_else(|| LineageError::SchemaNotFound(*to))?;

        // BFS to find shortest path
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent = HashMap::new();

        queue.push_back(*from_idx);
        visited.insert(*from_idx);

        let mut found = false;

        while let Some(node_idx) = queue.pop_front() {
            if node_idx == *to_idx {
                found = true;
                break;
            }

            for neighbor in graph_read.neighbors(node_idx) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, node_idx);
                    queue.push_back(neighbor);
                }
            }
        }

        if !found {
            return Ok(None);
        }

        // Reconstruct path
        let mut path = Vec::new();
        let mut current = *to_idx;

        while current != *from_idx {
            if let Some(schema_id) = self.get_schema_id_from_node(&graph_read, &index_read, current) {
                path.push(schema_id);
            }

            current = *parent.get(&current).ok_or_else(|| {
                LineageError::GraphOperationFailed("Path reconstruction failed".to_string())
            })?;
        }

        if let Some(schema_id) = self.get_schema_id_from_node(&graph_read, &index_read, *from_idx) {
            path.push(schema_id);
        }

        path.reverse();

        Ok(Some(path))
    }

    /// Check if there's a path between two schemas
    pub fn has_path(&self, from: &SchemaId, to: &SchemaId) -> Result<bool> {
        let graph = self.store.get_petgraph();
        let schema_index = self.store.get_schema_index();

        let graph_read = graph.read();
        let index_read = schema_index.read();

        let from_idx = index_read
            .get(from)
            .ok_or_else(|| LineageError::SchemaNotFound(*from))?;

        let to_idx = index_read
            .get(to)
            .ok_or_else(|| LineageError::SchemaNotFound(*to))?;

        let mut dfs = Dfs::new(&*graph_read, *from_idx);

        while let Some(node) = dfs.next(&*graph_read) {
            if node == *to_idx {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get all root schemas (schemas with no dependencies)
    pub fn get_roots(&self) -> Vec<SchemaId> {
        let graph = self.store.get_petgraph();
        let schema_index = self.store.get_schema_index();

        let graph_read = graph.read();
        let index_read = schema_index.read();

        let mut roots = Vec::new();

        for (schema_id, node_idx) in index_read.iter() {
            // Check if this node has no outgoing edges
            if graph_read.neighbors(*node_idx).count() == 0 {
                roots.push(*schema_id);
            }
        }

        roots
    }

    /// Get all leaf schemas (schemas that no other schema depends on)
    pub fn get_leaves(&self) -> Vec<SchemaId> {
        let graph = self.store.get_petgraph();
        let schema_index = self.store.get_schema_index();

        let graph_read = graph.read();
        let index_read = schema_index.read();

        let mut leaves = Vec::new();

        for (schema_id, node_idx) in index_read.iter() {
            // Check if this node has no incoming edges
            if graph_read.neighbors_directed(*node_idx, Direction::Incoming).count() == 0 {
                leaves.push(*schema_id);
            }
        }

        leaves
    }

    /// Helper: Get schema ID from node index
    fn get_schema_id_from_node(
        &self,
        _graph: &petgraph::graph::DiGraph<crate::graph_store::GraphNode, crate::graph_store::GraphEdge>,
        index_map: &HashMap<SchemaId, petgraph::graph::NodeIndex>,
        node_idx: petgraph::graph::NodeIndex,
    ) -> Option<SchemaId> {
        // Reverse lookup in the index map
        for (schema_id, idx) in index_map.iter() {
            if *idx == node_idx {
                return Some(*schema_id);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DependencyTarget, RelationType, SchemaNode};
    use schema_registry_core::versioning::SemanticVersion;

    fn create_test_schema(id: SchemaId, name: &str) -> SchemaNode {
        SchemaNode::new(
            id,
            SemanticVersion::new(1, 0, 0),
            format!("com.example.{}", name),
        )
    }

    #[test]
    fn test_bfs() {
        let store = GraphStore::new();
        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();
        let id3 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "A");
        let node2 = create_test_schema(id2, "B");
        let node3 = create_test_schema(id3, "C");

        // A -> B -> C
        store
            .add_dependency(node1.clone(), DependencyTarget::Schema(node2.clone()), RelationType::DependsOn)
            .unwrap();
        store
            .add_dependency(node2, DependencyTarget::Schema(node3), RelationType::DependsOn)
            .unwrap();

        let algo = GraphAlgorithms::new(store);
        let result = algo.bfs(&id1, None).unwrap();

        assert!(result.len() >= 2);
        assert!(result.contains(&id1));
    }

    #[test]
    fn test_transitive_dependencies() {
        let store = GraphStore::new();
        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();
        let id3 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "A");
        let node2 = create_test_schema(id2, "B");
        let node3 = create_test_schema(id3, "C");

        // A -> B -> C
        store
            .add_dependency(node1, DependencyTarget::Schema(node2.clone()), RelationType::DependsOn)
            .unwrap();
        store
            .add_dependency(node2, DependencyTarget::Schema(node3), RelationType::DependsOn)
            .unwrap();

        let algo = GraphAlgorithms::new(store);
        let result = algo.get_transitive_dependencies(&id1, None).unwrap();

        // Should find B and C as transitive dependencies
        assert!(result.len() >= 1);
    }

    #[test]
    fn test_has_path() {
        let store = GraphStore::new();
        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "A");
        let node2 = create_test_schema(id2, "B");

        store
            .add_dependency(node1, DependencyTarget::Schema(node2), RelationType::DependsOn)
            .unwrap();

        let algo = GraphAlgorithms::new(store);
        assert!(algo.has_path(&id1, &id2).unwrap());
        assert!(!algo.has_path(&id2, &id1).unwrap());
    }
}
