//! Export lineage data to various formats
//!
//! This module provides functionality to export the lineage graph to
//! GraphML, DOT (Graphviz), and JSON formats for visualization and analysis.

use crate::error::{LineageError, Result};
use crate::graph_store::GraphStore;
use crate::types::{DependencyGraph, DependencyTarget};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

/// Exporter for lineage data
#[derive(Clone)]
pub struct LineageExporter {
    store: GraphStore,
}

impl LineageExporter {
    /// Create a new lineage exporter
    pub fn new(store: GraphStore) -> Self {
        Self { store }
    }

    /// Export to GraphML format (XML-based graph format)
    pub fn export_graphml(&self) -> Result<String> {
        debug!("Exporting to GraphML format");

        let graph = self.store.to_dependency_graph();

        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\"\n");
        xml.push_str("  xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n");
        xml.push_str("  xsi:schemaLocation=\"http://graphml.graphdrawing.org/xmlns\n");
        xml.push_str("  http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd\">\n");

        // Define attributes
        xml.push_str("  <key id=\"d0\" for=\"node\" attr.name=\"fqn\" attr.type=\"string\"/>\n");
        xml.push_str("  <key id=\"d1\" for=\"node\" attr.name=\"version\" attr.type=\"string\"/>\n");
        xml.push_str("  <key id=\"d2\" for=\"node\" attr.name=\"type\" attr.type=\"string\"/>\n");
        xml.push_str("  <key id=\"d3\" for=\"edge\" attr.name=\"relation\" attr.type=\"string\"/>\n");
        xml.push_str("  <key id=\"d4\" for=\"edge\" attr.name=\"created_at\" attr.type=\"string\"/>\n");

        xml.push_str("  <graph id=\"G\" edgedefault=\"directed\">\n");

        // Add nodes
        for (schema_id, node) in &graph.nodes {
            xml.push_str(&format!("    <node id=\"{}\">\n", schema_id));
            xml.push_str(&format!("      <data key=\"d0\">{}</data>\n", escape_xml(&node.fqn)));
            xml.push_str(&format!("      <data key=\"d1\">{}</data>\n", node.schema_version));
            xml.push_str("      <data key=\"d2\">schema</data>\n");
            xml.push_str("    </node>\n");
        }

        // Add external entities
        for (entity_id, entity) in &graph.external_entities {
            xml.push_str(&format!("    <node id=\"{}\">\n", entity_id));
            xml.push_str(&format!("      <data key=\"d0\">{}</data>\n", escape_xml(&entity.name)));
            xml.push_str("      <data key=\"d1\">N/A</data>\n");
            xml.push_str(&format!("      <data key=\"d2\">{:?}</data>\n", entity.entity_type));
            xml.push_str("    </node>\n");
        }

        // Add edges
        for (idx, edge) in graph.edges.iter().enumerate() {
            let to_id = edge.to.id();
            xml.push_str(&format!(
                "    <edge id=\"e{}\" source=\"{}\" target=\"{}\">\n",
                idx, edge.from.schema_id, to_id
            ));
            xml.push_str(&format!("      <data key=\"d3\">{}</data>\n", edge.relation));
            xml.push_str(&format!("      <data key=\"d4\">{}</data>\n", edge.created_at));
            xml.push_str("    </edge>\n");
        }

        xml.push_str("  </graph>\n");
        xml.push_str("</graphml>\n");

        debug!("GraphML export complete");
        Ok(xml)
    }

    /// Export to DOT format (Graphviz)
    pub fn export_dot(&self) -> Result<String> {
        debug!("Exporting to DOT format");

        let graph = self.store.to_dependency_graph();

        let mut dot = String::new();
        dot.push_str("digraph lineage {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box, style=rounded];\n");

        // Define nodes
        for (schema_id, node) in &graph.nodes {
            let label = format!("{}\\n{}", node.fqn, node.schema_version);
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\", shape=box, style=\"rounded,filled\", fillcolor=lightblue];\n",
                schema_id, escape_dot(&label)
            ));
        }

        // Define external entities
        for (entity_id, entity) in &graph.external_entities {
            let label = format!("{}\\n{:?}", entity.name, entity.entity_type);
            let color = match entity.entity_type {
                crate::types::EntityType::Application => "lightgreen",
                crate::types::EntityType::Pipeline => "lightyellow",
                crate::types::EntityType::Model => "lightcoral",
                _ => "lightgray",
            };

            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\", shape=ellipse, style=filled, fillcolor={}];\n",
                entity_id, escape_dot(&label), color
            ));
        }

        // Define edges
        for edge in &graph.edges {
            let to_id = edge.to.id();
            let label = format!("{:?}", edge.relation);

            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                edge.from.schema_id,
                to_id,
                escape_dot(&label)
            ));
        }

        dot.push_str("}\n");

        debug!("DOT export complete");
        Ok(dot)
    }

    /// Export to JSON format
    pub fn export_json(&self) -> Result<String> {
        debug!("Exporting to JSON format");

        let graph = self.store.to_dependency_graph();
        let json_graph = JsonGraph::from_dependency_graph(&graph);

        serde_json::to_string_pretty(&json_graph)
            .map_err(|e| LineageError::SerializationError(e.to_string()))
    }

    /// Export to compact JSON format
    pub fn export_json_compact(&self) -> Result<String> {
        debug!("Exporting to compact JSON format");

        let graph = self.store.to_dependency_graph();
        let json_graph = JsonGraph::from_dependency_graph(&graph);

        serde_json::to_string(&json_graph)
            .map_err(|e| LineageError::SerializationError(e.to_string()))
    }

    /// Export filtered graph to JSON
    pub fn export_filtered_json(
        &self,
        schema_ids: &[uuid::Uuid],
    ) -> Result<String> {
        debug!("Exporting filtered graph to JSON");

        let full_graph = self.store.to_dependency_graph();
        let mut filtered_graph = DependencyGraph::new();

        // Filter nodes
        for schema_id in schema_ids {
            if let Some(node) = full_graph.nodes.get(schema_id) {
                filtered_graph.nodes.insert(*schema_id, node.clone());
            }
        }

        // Filter edges
        for edge in &full_graph.edges {
            if schema_ids.contains(&edge.from.schema_id) {
                if let DependencyTarget::Schema(target) = &edge.to {
                    if schema_ids.contains(&target.schema_id) {
                        filtered_graph.edges.push(edge.clone());
                    }
                }
            }
        }

        let json_graph = JsonGraph::from_dependency_graph(&filtered_graph);

        serde_json::to_string_pretty(&json_graph)
            .map_err(|e| LineageError::SerializationError(e.to_string()))
    }

    /// Export statistics as JSON
    pub fn export_stats_json(&self) -> Result<String> {
        let stats = self.store.stats();

        let stats_json = serde_json::json!({
            "total_nodes": stats.node_count,
            "total_edges": stats.edge_count,
            "schema_nodes": stats.schema_count,
            "external_entities": stats.entity_count,
        });

        serde_json::to_string_pretty(&stats_json)
            .map_err(|e| LineageError::SerializationError(e.to_string()))
    }
}

/// JSON representation of the lineage graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonGraph {
    /// Graph nodes
    pub nodes: Vec<JsonNode>,
    /// Graph edges
    pub edges: Vec<JsonEdge>,
    /// Graph metadata
    pub metadata: JsonGraphMetadata,
}

impl JsonGraph {
    /// Convert from DependencyGraph
    fn from_dependency_graph(graph: &DependencyGraph) -> Self {
        let mut nodes = Vec::new();

        // Add schema nodes
        for (schema_id, node) in &graph.nodes {
            nodes.push(JsonNode {
                id: schema_id.to_string(),
                label: node.fqn.clone(),
                node_type: "schema".to_string(),
                version: Some(node.schema_version.to_string()),
                metadata: node.metadata.clone(),
            });
        }

        // Add external entity nodes
        for (entity_id, entity) in &graph.external_entities {
            nodes.push(JsonNode {
                id: entity_id.clone(),
                label: entity.name.clone(),
                node_type: format!("{:?}", entity.entity_type).to_lowercase(),
                version: None,
                metadata: entity.metadata.clone(),
            });
        }

        // Add edges
        let edges: Vec<JsonEdge> = graph
            .edges
            .iter()
            .map(|edge| JsonEdge {
                source: edge.from.schema_id.to_string(),
                target: edge.to.id(),
                relation: format!("{:?}", edge.relation),
                created_at: edge.created_at.to_rfc3339(),
                metadata: edge.metadata.clone(),
            })
            .collect();

        let metadata = JsonGraphMetadata {
            node_count: nodes.len(),
            edge_count: edges.len(),
            schema_count: graph.nodes.len(),
            entity_count: graph.external_entities.len(),
        };

        Self {
            nodes,
            edges,
            metadata,
        }
    }
}

/// JSON node representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonNode {
    /// Node ID
    pub id: String,
    /// Node label
    pub label: String,
    /// Node type
    #[serde(rename = "type")]
    pub node_type: String,
    /// Version (for schemas)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// JSON edge representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEdge {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Relation type
    pub relation: String,
    /// Creation timestamp
    pub created_at: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// JSON graph metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonGraphMetadata {
    /// Total number of nodes
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
    /// Number of schema nodes
    pub schema_count: usize,
    /// Number of external entities
    pub entity_count: usize,
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Escape DOT special characters
fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RelationType, SchemaNode};
    use schema_registry_core::versioning::SemanticVersion;
    use uuid::Uuid;

    fn create_test_schema(id: Uuid, name: &str) -> SchemaNode {
        SchemaNode::new(
            id,
            SemanticVersion::new(1, 0, 0),
            format!("com.example.{}", name),
        )
    }

    #[test]
    fn test_export_graphml() {
        let store = GraphStore::new();
        let exporter = LineageExporter::new(store.clone());

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Profile");

        store
            .add_dependency(
                node1,
                DependencyTarget::Schema(node2),
                RelationType::Composes,
            )
            .unwrap();

        let graphml = exporter.export_graphml().unwrap();

        assert!(graphml.contains("<?xml"));
        assert!(graphml.contains("graphml"));
        assert!(graphml.contains("com.example.User"));
        assert!(graphml.contains("com.example.Profile"));
    }

    #[test]
    fn test_export_dot() {
        let store = GraphStore::new();
        let exporter = LineageExporter::new(store.clone());

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Profile");

        store
            .add_dependency(
                node1,
                DependencyTarget::Schema(node2),
                RelationType::Composes,
            )
            .unwrap();

        let dot = exporter.export_dot().unwrap();

        assert!(dot.contains("digraph lineage"));
        assert!(dot.contains("com.example.User"));
        assert!(dot.contains("com.example.Profile"));
        assert!(dot.contains("->"));
    }

    #[test]
    fn test_export_json() {
        let store = GraphStore::new();
        let exporter = LineageExporter::new(store.clone());

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Profile");

        store
            .add_dependency(
                node1,
                DependencyTarget::Schema(node2),
                RelationType::Composes,
            )
            .unwrap();

        let json = exporter.export_json().unwrap();

        assert!(json.contains("nodes"));
        assert!(json.contains("edges"));
        assert!(json.contains("metadata"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a&b"), "a&amp;b");
        assert_eq!(escape_xml("a<b"), "a&lt;b");
        assert_eq!(escape_xml("a>b"), "a&gt;b");
        assert_eq!(escape_xml("a\"b"), "a&quot;b");
        assert_eq!(escape_xml("a'b"), "a&apos;b");
    }

    #[test]
    fn test_escape_dot() {
        assert_eq!(escape_dot("a\\b"), "a\\\\b");
        assert_eq!(escape_dot("a\"b"), "a\\\"b");
        assert_eq!(escape_dot("a\nb"), "a\\nb");
    }
}
