//! Dependency graph analysis for schemas
//!
//! Tracks schema dependencies and calculates impact radius for breaking changes

use crate::types::{Schema, SemanticVersion};
use crate::violation::CompatibilityViolation;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

/// Represents a schema dependency
#[derive(Debug, Clone)]
pub struct SchemaDependency {
    pub schema_id: Uuid,
    pub version: SemanticVersion,
    pub depends_on_schema_id: Uuid,
    pub depends_on_version: SemanticVersion,
    pub dependency_type: DependencyType,
}

/// Type of dependency between schemas
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// Schema references another schema (e.g., field type)
    Reference,
    /// Schema inherits from another schema
    Inheritance,
    /// Schema includes/embeds another schema
    Embedded,
}

/// Dependency graph for schema analysis
pub struct DependencyGraph {
    /// Map of schema ID to its dependencies
    dependencies: HashMap<Uuid, Vec<SchemaDependency>>,
    /// Reverse map: schema ID to schemas that depend on it
    dependents: HashMap<Uuid, Vec<SchemaDependency>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    /// Add a dependency to the graph
    pub fn add_dependency(&mut self, dependency: SchemaDependency) {
        // Add to dependencies map
        self.dependencies
            .entry(dependency.schema_id)
            .or_insert_with(Vec::new)
            .push(dependency.clone());

        // Add to dependents map (reverse)
        self.dependents
            .entry(dependency.depends_on_schema_id)
            .or_insert_with(Vec::new)
            .push(dependency);
    }

    /// Get direct dependencies of a schema
    pub fn get_dependencies(&self, schema_id: &Uuid) -> Vec<&SchemaDependency> {
        self.dependencies
            .get(schema_id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Get schemas that depend on this schema
    pub fn get_dependents(&self, schema_id: &Uuid) -> Vec<&SchemaDependency> {
        self.dependents
            .get(schema_id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Calculate all transitive dependencies (depth-first)
    pub fn get_transitive_dependencies(&self, schema_id: &Uuid) -> Vec<Uuid> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();

        self.dfs_dependencies(schema_id, &mut visited, &mut result);

        result
    }

    /// Calculate all transitive dependents (breadth-first)
    pub fn get_transitive_dependents(&self, schema_id: &Uuid) -> Vec<Uuid> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(*schema_id);
        visited.insert(*schema_id);

        while let Some(current_id) = queue.pop_front() {
            if let Some(dependents) = self.dependents.get(&current_id) {
                for dep in dependents {
                    if visited.insert(dep.schema_id) {
                        result.push(dep.schema_id);
                        queue.push_back(dep.schema_id);
                    }
                }
            }
        }

        result
    }

    /// Depth-first search for dependencies
    fn dfs_dependencies(&self, schema_id: &Uuid, visited: &mut HashSet<Uuid>, result: &mut Vec<Uuid>) {
        if !visited.insert(*schema_id) {
            return; // Already visited
        }

        if let Some(dependencies) = self.dependencies.get(schema_id) {
            for dep in dependencies {
                result.push(dep.depends_on_schema_id);
                self.dfs_dependencies(&dep.depends_on_schema_id, visited, result);
            }
        }
    }

    /// Detect circular dependencies
    pub fn has_circular_dependency(&self, schema_id: &Uuid) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        self.is_circular(schema_id, &mut visited, &mut rec_stack)
    }

    /// Helper for circular dependency detection
    fn is_circular(
        &self,
        schema_id: &Uuid,
        visited: &mut HashSet<Uuid>,
        rec_stack: &mut HashSet<Uuid>,
    ) -> bool {
        if !visited.contains(schema_id) {
            visited.insert(*schema_id);
            rec_stack.insert(*schema_id);

            if let Some(dependencies) = self.dependencies.get(schema_id) {
                for dep in dependencies {
                    let dep_id = dep.depends_on_schema_id;

                    if !visited.contains(&dep_id) {
                        if self.is_circular(&dep_id, visited, rec_stack) {
                            return true;
                        }
                    } else if rec_stack.contains(&dep_id) {
                        return true;
                    }
                }
            }
        }

        rec_stack.remove(schema_id);
        false
    }
}

/// Impact analysis result
#[derive(Debug, Clone)]
pub struct ImpactAnalysis {
    /// Schema being analyzed
    pub schema_id: Uuid,
    /// Direct dependents affected
    pub direct_dependents: Vec<Uuid>,
    /// All transitive dependents affected
    pub transitive_dependents: Vec<Uuid>,
    /// Breaking changes found
    pub breaking_changes: Vec<CompatibilityViolation>,
    /// Impact radius (number of schemas affected)
    pub impact_radius: usize,
    /// Suggested migration order
    pub migration_order: Vec<Uuid>,
}

impl ImpactAnalysis {
    /// Calculate impact of breaking changes for a schema
    pub fn calculate(
        schema_id: Uuid,
        breaking_changes: Vec<CompatibilityViolation>,
        graph: &DependencyGraph,
    ) -> Self {
        let direct_dependents: Vec<_> = graph
            .get_dependents(&schema_id)
            .iter()
            .map(|dep| dep.schema_id)
            .collect();

        let transitive_dependents = graph.get_transitive_dependents(&schema_id);

        let impact_radius = transitive_dependents.len();

        // Calculate migration order (topological sort)
        let migration_order = Self::calculate_migration_order(&schema_id, graph);

        Self {
            schema_id,
            direct_dependents,
            transitive_dependents,
            breaking_changes,
            impact_radius,
            migration_order,
        }
    }

    /// Calculate optimal migration order using topological sort
    fn calculate_migration_order(schema_id: &Uuid, graph: &DependencyGraph) -> Vec<Uuid> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();

        // Get all affected schemas
        let mut affected = graph.get_transitive_dependents(schema_id);
        affected.push(*schema_id);

        // Topological sort
        for id in &affected {
            if !visited.contains(id) {
                Self::topological_visit(id, graph, &mut visited, &mut temp_mark, &mut result);
            }
        }

        result.reverse(); // Reverse to get correct order
        result
    }

    /// Visit node for topological sort
    fn topological_visit(
        schema_id: &Uuid,
        graph: &DependencyGraph,
        visited: &mut HashSet<Uuid>,
        temp_mark: &mut HashSet<Uuid>,
        result: &mut Vec<Uuid>,
    ) {
        if temp_mark.contains(schema_id) {
            // Circular dependency detected
            return;
        }

        if !visited.contains(schema_id) {
            temp_mark.insert(*schema_id);

            if let Some(dependencies) = graph.dependencies.get(schema_id) {
                for dep in dependencies {
                    Self::topological_visit(
                        &dep.depends_on_schema_id,
                        graph,
                        visited,
                        temp_mark,
                        result,
                    );
                }
            }

            temp_mark.remove(schema_id);
            visited.insert(*schema_id);
            result.push(*schema_id);
        }
    }

    /// Generate migration guide
    pub fn generate_migration_guide(&self) -> String {
        let mut guide = String::new();

        guide.push_str(&format!(
            "# Migration Guide for Schema {}\n\n",
            self.schema_id
        ));

        guide.push_str(&format!("## Impact Summary\n\n"));
        guide.push_str(&format!(
            "- **Impact Radius**: {} schemas affected\n",
            self.impact_radius
        ));
        guide.push_str(&format!(
            "- **Direct Dependents**: {}\n",
            self.direct_dependents.len()
        ));
        guide.push_str(&format!(
            "- **Breaking Changes**: {}\n\n",
            self.breaking_changes.len()
        ));

        guide.push_str("## Breaking Changes\n\n");
        for (i, change) in self.breaking_changes.iter().enumerate() {
            guide.push_str(&format!(
                "{}. **{}** at `{}`\n   - {}\n\n",
                i + 1,
                format!("{:?}", change.violation_type),
                change.field_path,
                change.description
            ));
        }

        guide.push_str("## Migration Order\n\n");
        guide.push_str("Migrate schemas in the following order:\n\n");
        for (i, schema_id) in self.migration_order.iter().enumerate() {
            guide.push_str(&format!("{}. Schema `{}`\n", i + 1, schema_id));
        }

        guide
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_basic() {
        let mut graph = DependencyGraph::new();

        let schema_a = Uuid::new_v4();
        let schema_b = Uuid::new_v4();

        let dep = SchemaDependency {
            schema_id: schema_a,
            version: SemanticVersion::new(1, 0, 0),
            depends_on_schema_id: schema_b,
            depends_on_version: SemanticVersion::new(1, 0, 0),
            dependency_type: DependencyType::Reference,
        };

        graph.add_dependency(dep);

        assert_eq!(graph.get_dependencies(&schema_a).len(), 1);
        assert_eq!(graph.get_dependents(&schema_b).len(), 1);
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = DependencyGraph::new();

        let schema_a = Uuid::new_v4();
        let schema_b = Uuid::new_v4();
        let schema_c = Uuid::new_v4();

        // A depends on B
        graph.add_dependency(SchemaDependency {
            schema_id: schema_a,
            version: SemanticVersion::new(1, 0, 0),
            depends_on_schema_id: schema_b,
            depends_on_version: SemanticVersion::new(1, 0, 0),
            dependency_type: DependencyType::Reference,
        });

        // B depends on C
        graph.add_dependency(SchemaDependency {
            schema_id: schema_b,
            version: SemanticVersion::new(1, 0, 0),
            depends_on_schema_id: schema_c,
            depends_on_version: SemanticVersion::new(1, 0, 0),
            dependency_type: DependencyType::Reference,
        });

        let transitive = graph.get_transitive_dependencies(&schema_a);
        assert_eq!(transitive.len(), 2); // B and C
        assert!(transitive.contains(&schema_b));
        assert!(transitive.contains(&schema_c));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();

        let schema_a = Uuid::new_v4();
        let schema_b = Uuid::new_v4();

        // A depends on B
        graph.add_dependency(SchemaDependency {
            schema_id: schema_a,
            version: SemanticVersion::new(1, 0, 0),
            depends_on_schema_id: schema_b,
            depends_on_version: SemanticVersion::new(1, 0, 0),
            dependency_type: DependencyType::Reference,
        });

        // B depends on A (circular)
        graph.add_dependency(SchemaDependency {
            schema_id: schema_b,
            version: SemanticVersion::new(1, 0, 0),
            depends_on_schema_id: schema_a,
            depends_on_version: SemanticVersion::new(1, 0, 0),
            dependency_type: DependencyType::Reference,
        });

        assert!(graph.has_circular_dependency(&schema_a));
        assert!(graph.has_circular_dependency(&schema_b));
    }
}
