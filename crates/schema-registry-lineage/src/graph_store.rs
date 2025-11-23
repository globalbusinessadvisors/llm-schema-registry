//! In-memory graph storage using petgraph
//!
//! This module provides efficient in-memory storage for the lineage graph using
//! the petgraph library. It supports concurrent access via Arc and RwLock.

use crate::error::{LineageError, Result};
use crate::types::{
    Dependency, DependencyGraph, DependencyTarget, ExternalEntity, RelationType, SchemaId,
    SchemaNode,
};
use parking_lot::RwLock;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Node data in the petgraph
#[derive(Debug, Clone)]
pub(crate) enum GraphNode {
    /// Schema node
    Schema(SchemaNode),
    /// External entity
    External(ExternalEntity),
}

/// Edge data in the petgraph
#[derive(Debug, Clone)]
pub(crate) struct GraphEdge {
    /// Type of relationship
    relation: RelationType,
    /// Creation timestamp
    created_at: chrono::DateTime<chrono::Utc>,
    /// Metadata
    metadata: HashMap<String, String>,
}

/// Thread-safe graph store
#[derive(Clone)]
pub struct GraphStore {
    /// The actual graph
    graph: Arc<RwLock<DiGraph<GraphNode, GraphEdge>>>,
    /// Fast lookup: SchemaId -> NodeIndex
    schema_index: Arc<RwLock<HashMap<SchemaId, NodeIndex>>>,
    /// Fast lookup: External entity ID -> NodeIndex
    entity_index: Arc<RwLock<HashMap<String, NodeIndex>>>,
}

impl GraphStore {
    /// Create a new empty graph store
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(DiGraph::new())),
            schema_index: Arc::new(RwLock::new(HashMap::new())),
            entity_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a schema node to the graph
    pub fn add_schema_node(&self, node: SchemaNode) -> Result<()> {
        let mut graph = self.graph.write();
        let mut index = self.schema_index.write();

        // Check if already exists
        if index.contains_key(&node.schema_id) {
            debug!("Schema node already exists: {}", node.schema_id);
            return Ok(());
        }

        let node_idx = graph.add_node(GraphNode::Schema(node.clone()));
        index.insert(node.schema_id, node_idx);

        debug!("Added schema node: {} ({})", node.fqn, node.schema_id);
        Ok(())
    }

    /// Add an external entity to the graph
    pub fn add_external_entity(&self, entity: ExternalEntity) -> Result<()> {
        let mut graph = self.graph.write();
        let mut index = self.entity_index.write();

        // Check if already exists
        if index.contains_key(&entity.id) {
            debug!("External entity already exists: {}", entity.id);
            return Ok(());
        }

        let node_idx = graph.add_node(GraphNode::External(entity.clone()));
        index.insert(entity.id.clone(), node_idx);

        debug!("Added external entity: {} ({})", entity.name, entity.id);
        Ok(())
    }

    /// Add a dependency edge between nodes
    pub fn add_dependency(&self, from: SchemaNode, to: DependencyTarget, relation: RelationType) -> Result<()> {
        // Ensure from node exists
        self.add_schema_node(from.clone())?;

        // Ensure to node exists
        match &to {
            DependencyTarget::Schema(schema_node) => {
                self.add_schema_node(schema_node.clone())?;
            }
            DependencyTarget::External(entity) => {
                self.add_external_entity(entity.clone())?;
            }
        }

        let schema_index = self.schema_index.read();
        let entity_index = self.entity_index.read();

        let from_idx = schema_index
            .get(&from.schema_id)
            .ok_or_else(|| LineageError::SchemaNotFound(from.schema_id))?;

        let to_idx = match &to {
            DependencyTarget::Schema(node) => schema_index
                .get(&node.schema_id)
                .ok_or_else(|| LineageError::SchemaNotFound(node.schema_id))?,
            DependencyTarget::External(entity) => entity_index
                .get(&entity.id)
                .ok_or_else(|| LineageError::EntityNotFound(entity.id.clone()))?,
        };

        let mut graph = self.graph.write();

        // Check if edge already exists
        if graph.find_edge(*from_idx, *to_idx).is_some() {
            debug!("Dependency already exists: {} -> {}", from.key(), to.id());
            return Ok(());
        }

        let edge = GraphEdge {
            relation,
            created_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        graph.add_edge(*from_idx, *to_idx, edge);
        debug!("Added dependency: {} -> {} ({:?})", from.key(), to.id(), relation);

        Ok(())
    }

    /// Remove a dependency edge
    pub fn remove_dependency(&self, from_id: &SchemaId, to_id: &str) -> Result<()> {
        let schema_index = self.schema_index.read();
        let entity_index = self.entity_index.read();

        let from_idx = schema_index
            .get(from_id)
            .ok_or_else(|| LineageError::SchemaNotFound(*from_id))?;

        // Try schema index first, then entity index
        let to_idx = schema_index
            .get(&to_id.parse::<SchemaId>().unwrap_or_default())
            .or_else(|| entity_index.get(to_id))
            .ok_or_else(|| LineageError::EntityNotFound(to_id.to_string()))?;

        let mut graph = self.graph.write();

        if let Some(edge_idx) = graph.find_edge(*from_idx, *to_idx) {
            graph.remove_edge(edge_idx);
            debug!("Removed dependency: {} -> {}", from_id, to_id);
            Ok(())
        } else {
            Err(LineageError::DependencyNotFound {
                from: from_id.to_string(),
                to: to_id.to_string(),
            })
        }
    }

    /// Get all dependencies of a schema (outgoing edges)
    pub fn get_dependencies(&self, schema_id: &SchemaId) -> Result<Vec<Dependency>> {
        let graph = self.graph.read();
        let schema_index = self.schema_index.read();

        let node_idx = schema_index
            .get(schema_id)
            .ok_or_else(|| LineageError::SchemaNotFound(*schema_id))?;

        let mut dependencies = Vec::new();

        for edge in graph.edges(*node_idx) {
            let from_node = match graph.node_weight(edge.source()) {
                Some(GraphNode::Schema(node)) => node.clone(),
                _ => continue,
            };

            let to_target = match graph.node_weight(edge.target()) {
                Some(GraphNode::Schema(node)) => DependencyTarget::Schema(node.clone()),
                Some(GraphNode::External(entity)) => DependencyTarget::External(entity.clone()),
                None => continue,
            };

            dependencies.push(Dependency {
                from: from_node,
                to: to_target,
                relation: edge.weight().relation,
                created_at: edge.weight().created_at,
                metadata: edge.weight().metadata.clone(),
            });
        }

        Ok(dependencies)
    }

    /// Get all dependents of a schema (incoming edges)
    pub fn get_dependents(&self, schema_id: &SchemaId) -> Result<Vec<Dependency>> {
        let graph = self.graph.read();
        let schema_index = self.schema_index.read();

        let node_idx = schema_index
            .get(schema_id)
            .ok_or_else(|| LineageError::SchemaNotFound(*schema_id))?;

        let mut dependents = Vec::new();

        // Get all edges pointing TO this node
        for edge in graph.edges_directed(*node_idx, petgraph::Direction::Incoming) {
            let from_node = match graph.node_weight(edge.source()) {
                Some(GraphNode::Schema(node)) => node.clone(),
                _ => continue,
            };

            let to_target = match graph.node_weight(edge.target()) {
                Some(GraphNode::Schema(node)) => DependencyTarget::Schema(node.clone()),
                Some(GraphNode::External(entity)) => DependencyTarget::External(entity.clone()),
                None => continue,
            };

            dependents.push(Dependency {
                from: from_node,
                to: to_target,
                relation: edge.weight().relation,
                created_at: edge.weight().created_at,
                metadata: edge.weight().metadata.clone(),
            });
        }

        Ok(dependents)
    }

    /// Get a schema node by ID
    pub fn get_schema_node(&self, schema_id: &SchemaId) -> Result<SchemaNode> {
        let graph = self.graph.read();
        let schema_index = self.schema_index.read();

        let node_idx = schema_index
            .get(schema_id)
            .ok_or_else(|| LineageError::SchemaNotFound(*schema_id))?;

        match graph.node_weight(*node_idx) {
            Some(GraphNode::Schema(node)) => Ok(node.clone()),
            _ => Err(LineageError::SchemaNotFound(*schema_id)),
        }
    }

    /// Get an external entity by ID
    pub fn get_external_entity(&self, entity_id: &str) -> Result<ExternalEntity> {
        let graph = self.graph.read();
        let entity_index = self.entity_index.read();

        let node_idx = entity_index
            .get(entity_id)
            .ok_or_else(|| LineageError::EntityNotFound(entity_id.to_string()))?;

        match graph.node_weight(*node_idx) {
            Some(GraphNode::External(entity)) => Ok(entity.clone()),
            _ => Err(LineageError::EntityNotFound(entity_id.to_string())),
        }
    }

    /// Check if a schema exists in the graph
    pub fn contains_schema(&self, schema_id: &SchemaId) -> bool {
        let index = self.schema_index.read();
        index.contains_key(schema_id)
    }

    /// Get all schema nodes
    pub fn get_all_schemas(&self) -> Vec<SchemaNode> {
        let graph = self.graph.read();
        graph
            .node_weights()
            .filter_map(|node| match node {
                GraphNode::Schema(schema) => Some(schema.clone()),
                _ => None,
            })
            .collect()
    }

    /// Get all external entities
    pub fn get_all_external_entities(&self) -> Vec<ExternalEntity> {
        let graph = self.graph.read();
        graph
            .node_weights()
            .filter_map(|node| match node {
                GraphNode::External(entity) => Some(entity.clone()),
                _ => None,
            })
            .collect()
    }

    /// Get graph statistics
    pub fn stats(&self) -> GraphStats {
        let graph = self.graph.read();
        GraphStats {
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
            schema_count: self.schema_index.read().len(),
            entity_count: self.entity_index.read().len(),
        }
    }

    /// Export to DependencyGraph (for serialization)
    pub fn to_dependency_graph(&self) -> DependencyGraph {
        let graph = self.graph.read();
        let mut dep_graph = DependencyGraph::new();

        // Add all nodes
        for node in graph.node_weights() {
            match node {
                GraphNode::Schema(schema) => {
                    dep_graph.nodes.insert(schema.schema_id, schema.clone());
                }
                GraphNode::External(entity) => {
                    dep_graph.external_entities.insert(entity.id.clone(), entity.clone());
                }
            }
        }

        // Add all edges
        for edge in graph.raw_edges() {
            if let (Some(GraphNode::Schema(from)), Some(to_node)) =
                (graph.node_weight(edge.source()), graph.node_weight(edge.target()))
            {
                let to_target = match to_node {
                    GraphNode::Schema(node) => DependencyTarget::Schema(node.clone()),
                    GraphNode::External(entity) => DependencyTarget::External(entity.clone()),
                };

                let dependency = Dependency {
                    from: from.clone(),
                    to: to_target.clone(),
                    relation: edge.weight.relation,
                    created_at: edge.weight.created_at,
                    metadata: edge.weight.metadata.clone(),
                };

                dep_graph.edges.push(dependency);

                // Update adjacency lists for schema-to-schema edges
                if let DependencyTarget::Schema(to_schema) = to_target {
                    dep_graph
                        .adjacency_list
                        .entry(from.schema_id)
                        .or_default()
                        .push(to_schema.schema_id);

                    dep_graph
                        .reverse_adjacency_list
                        .entry(to_schema.schema_id)
                        .or_default()
                        .push(from.schema_id);
                }
            }
        }

        dep_graph
    }

    /// Get the underlying petgraph (for algorithms)
    pub fn get_petgraph(&self) -> Arc<RwLock<DiGraph<GraphNode, GraphEdge>>> {
        self.graph.clone()
    }

    /// Get schema index
    pub fn get_schema_index(&self) -> Arc<RwLock<HashMap<SchemaId, NodeIndex>>> {
        self.schema_index.clone()
    }

    /// Clear all data
    pub fn clear(&self) {
        let mut graph = self.graph.write();
        let mut schema_index = self.schema_index.write();
        let mut entity_index = self.entity_index.write();

        graph.clear();
        schema_index.clear();
        entity_index.clear();

        debug!("Graph store cleared");
    }
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStats {
    /// Total number of nodes
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
    /// Number of schema nodes
    pub schema_count: usize,
    /// Number of external entity nodes
    pub entity_count: usize,
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

    #[test]
    fn test_add_schema_node() {
        let store = GraphStore::new();
        let id = SchemaId::new_v4();
        let node = create_test_schema(id, "User");

        assert!(store.add_schema_node(node).is_ok());
        assert!(store.contains_schema(&id));
    }

    #[test]
    fn test_add_dependency() {
        let store = GraphStore::new();
        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Address");

        let result = store.add_dependency(
            node1,
            DependencyTarget::Schema(node2),
            RelationType::Composes,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_get_dependencies() {
        let store = GraphStore::new();
        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Address");

        store
            .add_dependency(
                node1.clone(),
                DependencyTarget::Schema(node2.clone()),
                RelationType::Composes,
            )
            .unwrap();

        let deps = store.get_dependencies(&id1).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].relation, RelationType::Composes);
    }

    #[test]
    fn test_graph_stats() {
        let store = GraphStore::new();
        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Address");

        store
            .add_dependency(
                node1,
                DependencyTarget::Schema(node2),
                RelationType::Composes,
            )
            .unwrap();

        let stats = store.stats();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
        assert_eq!(stats.schema_count, 2);
    }
}
