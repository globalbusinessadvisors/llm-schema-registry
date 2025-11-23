//! Impact analysis for schema changes
//!
//! This module analyzes the impact of proposed schema changes on downstream
//! dependencies and generates comprehensive impact reports.

use crate::algorithms::GraphAlgorithms;
use crate::error::Result;
use crate::graph_store::GraphStore;
use crate::types::{
    DependencyTarget, EntityType, ImpactReport, RiskLevel, SchemaChange, SchemaId,
};
use chrono::Utc;
use std::collections::HashMap;
use tracing::{debug, info};

/// Impact analyzer for schema changes
#[derive(Clone)]
pub struct ImpactAnalyzer {
    store: GraphStore,
    algorithms: GraphAlgorithms,
}

impl ImpactAnalyzer {
    /// Create a new impact analyzer
    pub fn new(store: GraphStore) -> Self {
        let algorithms = GraphAlgorithms::new(store.clone());
        Self { store, algorithms }
    }

    /// Analyze the impact of a schema change
    pub async fn analyze_impact(
        &self,
        target_schema: SchemaId,
        proposed_change: SchemaChange,
    ) -> Result<ImpactReport> {
        debug!(
            "Analyzing impact for schema {} with change: {:?}",
            target_schema, proposed_change
        );

        // Get all transitive dependents (schemas that depend on this one)
        let dependents = self
            .algorithms
            .get_transitive_dependents(&target_schema, None)?;

        let mut affected_schemas = Vec::new();
        let mut affected_applications = Vec::new();
        let mut affected_pipelines = Vec::new();
        let mut affected_models = Vec::new();
        let mut depth_breakdown: HashMap<usize, usize> = HashMap::new();

        // Process direct dependencies to find external entities
        if let Ok(direct_deps) = self.store.get_dependents(&target_schema) {
            for dep in direct_deps {
                match &dep.to {
                    DependencyTarget::Schema(_) => {
                        // Already handled by transitive dependents
                    }
                    DependencyTarget::External(entity) => {
                        match entity.entity_type {
                            EntityType::Application => {
                                affected_applications.push(entity.name.clone());
                            }
                            EntityType::Pipeline => {
                                affected_pipelines.push(entity.name.clone());
                            }
                            EntityType::Model => {
                                affected_models.push(entity.name.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Collect affected schemas and depth breakdown
        for (schema_id, depth) in dependents {
            affected_schemas.push(schema_id);
            *depth_breakdown.entry(depth).or_insert(0) += 1;
        }

        // Calculate risk level
        let total_affected = affected_schemas.len()
            + affected_applications.len()
            + affected_pipelines.len()
            + affected_models.len();

        let risk_level = RiskLevel::from_count(total_affected);

        // Calculate migration complexity (0.0 to 1.0)
        let migration_complexity = self.calculate_migration_complexity(
            &proposed_change,
            affected_schemas.len(),
            &depth_breakdown,
        );

        // Estimate effort in hours
        let estimated_effort_hours = self.estimate_effort(
            &proposed_change,
            affected_schemas.len(),
            affected_applications.len(),
            affected_pipelines.len(),
        );

        // Generate recommendations
        let recommendations = self.generate_recommendations(
            &proposed_change,
            &risk_level,
            affected_schemas.len(),
        );

        let report = ImpactReport {
            target_schema,
            proposed_change,
            affected_schemas,
            affected_applications,
            affected_pipelines,
            affected_models,
            risk_level,
            migration_complexity,
            estimated_effort_hours,
            depth_breakdown,
            generated_at: Utc::now(),
            recommendations,
        };

        info!(
            "Impact analysis complete: {} total affected, risk level: {:?}",
            total_affected, risk_level
        );

        Ok(report)
    }

    /// Calculate migration complexity score (0.0 to 1.0)
    fn calculate_migration_complexity(
        &self,
        change: &SchemaChange,
        affected_count: usize,
        depth_breakdown: &HashMap<usize, usize>,
    ) -> f64 {
        // Base complexity based on change type
        let base_complexity = match change {
            SchemaChange::FieldAdded { .. } => 0.1,
            SchemaChange::FieldMadeOptional { .. } => 0.2,
            SchemaChange::ConstraintRemoved { .. } => 0.2,
            SchemaChange::EnumValueAdded { .. } => 0.3,
            SchemaChange::FieldMadeRequired { .. } => 0.5,
            SchemaChange::ConstraintAdded { .. } => 0.5,
            SchemaChange::FieldTypeChanged { .. } => 0.7,
            SchemaChange::EnumValueRemoved { .. } => 0.8,
            SchemaChange::FieldRemoved { .. } => 0.9,
            SchemaChange::FormatChanged { .. } => 1.0,
            SchemaChange::MajorVersionChange { .. } => 1.0,
        };

        // Affected count multiplier (logarithmic scale)
        let count_multiplier = if affected_count == 0 {
            0.0
        } else {
            (affected_count as f64).ln() / 10.0
        };

        // Depth complexity (deeper dependencies are harder to migrate)
        let max_depth = depth_breakdown.keys().max().copied().unwrap_or(0);
        let depth_multiplier = (max_depth as f64) / 10.0;

        // Combine factors
        let complexity = base_complexity + count_multiplier + depth_multiplier;

        // Clamp to [0.0, 1.0]
        complexity.min(1.0).max(0.0)
    }

    /// Estimate migration effort in hours
    fn estimate_effort(
        &self,
        change: &SchemaChange,
        schema_count: usize,
        app_count: usize,
        pipeline_count: usize,
    ) -> f64 {
        // Base hours per change type
        let base_hours = match change {
            SchemaChange::FieldAdded { .. } => 2.0,
            SchemaChange::FieldMadeOptional { .. } => 2.0,
            SchemaChange::ConstraintRemoved { .. } => 3.0,
            SchemaChange::EnumValueAdded { .. } => 3.0,
            SchemaChange::FieldMadeRequired { .. } => 5.0,
            SchemaChange::ConstraintAdded { .. } => 5.0,
            SchemaChange::FieldTypeChanged { .. } => 8.0,
            SchemaChange::EnumValueRemoved { .. } => 8.0,
            SchemaChange::FieldRemoved { .. } => 10.0,
            SchemaChange::FormatChanged { .. } => 40.0,
            SchemaChange::MajorVersionChange { .. } => 40.0,
        };

        // Hours per affected schema (decreases with scale due to automation)
        let schema_hours = (schema_count as f64) * 0.5;

        // Hours per affected application
        let app_hours = (app_count as f64) * 4.0;

        // Hours per affected pipeline
        let pipeline_hours = (pipeline_count as f64) * 3.0;

        base_hours + schema_hours + app_hours + pipeline_hours
    }

    /// Generate migration recommendations
    fn generate_recommendations(
        &self,
        change: &SchemaChange,
        risk_level: &RiskLevel,
        affected_count: usize,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Risk-based recommendations
        match risk_level {
            RiskLevel::Low => {
                recommendations.push("Low risk change - proceed with standard testing".to_string());
            }
            RiskLevel::Medium => {
                recommendations.push("Medium risk - deploy to staging environment first".to_string());
                recommendations.push("Run comprehensive integration tests".to_string());
            }
            RiskLevel::High => {
                recommendations.push("High risk - implement gradual rollout strategy".to_string());
                recommendations.push("Create rollback plan before deployment".to_string());
                recommendations.push("Monitor affected systems closely".to_string());
            }
            RiskLevel::Critical => {
                recommendations.push("CRITICAL RISK - consider if change is absolutely necessary".to_string());
                recommendations.push("Implement feature flags for gradual rollout".to_string());
                recommendations.push("Create comprehensive rollback plan".to_string());
                recommendations.push("Schedule maintenance window".to_string());
                recommendations.push("Notify all affected teams in advance".to_string());
            }
        }

        // Change-specific recommendations
        match change {
            SchemaChange::FieldRemoved { name } => {
                recommendations.push(format!(
                    "Breaking change: Field '{}' removal requires version bump",
                    name
                ));
                recommendations.push("Deprecate field first before removing".to_string());
            }
            SchemaChange::FieldTypeChanged { name, .. } => {
                recommendations.push(format!(
                    "Breaking change: Type change for '{}' requires data migration",
                    name
                ));
                recommendations.push("Consider adding new field instead of changing type".to_string());
            }
            SchemaChange::FieldMadeRequired { name } => {
                recommendations.push(format!(
                    "Breaking change: Making '{}' required affects existing data",
                    name
                ));
                recommendations.push("Provide default value or backfill existing data".to_string());
            }
            SchemaChange::EnumValueRemoved { enum_name, value } => {
                recommendations.push(format!(
                    "Breaking change: Removing enum value '{}' from '{}'",
                    value, enum_name
                ));
                recommendations.push("Check for existing data using this enum value".to_string());
            }
            SchemaChange::FormatChanged { .. } => {
                recommendations.push("Format change requires complete system migration".to_string());
                recommendations.push("Plan for dual-format support during transition".to_string());
            }
            _ => {}
        }

        // Scale-based recommendations
        if affected_count > 50 {
            recommendations.push("Large number of affected items - automate migration where possible".to_string());
        }

        if affected_count > 200 {
            recommendations.push("Consider breaking change into smaller incremental updates".to_string());
        }

        recommendations
    }

    /// Analyze impact for multiple changes
    pub async fn analyze_batch_impact(
        &self,
        changes: Vec<(SchemaId, SchemaChange)>,
    ) -> Result<Vec<ImpactReport>> {
        let mut reports = Vec::new();

        for (schema_id, change) in changes {
            let report = self.analyze_impact(schema_id, change).await?;
            reports.push(report);
        }

        Ok(reports)
    }

    /// Get a summary of impact across multiple reports
    pub fn summarize_impacts(&self, reports: &[ImpactReport]) -> ImpactSummary {
        let total_schemas: usize = reports.iter().map(|r| r.affected_schemas.len()).sum();
        let total_applications: usize = reports.iter().map(|r| r.affected_applications.len()).sum();
        let total_pipelines: usize = reports.iter().map(|r| r.affected_pipelines.len()).sum();
        let total_effort: f64 = reports.iter().map(|r| r.estimated_effort_hours).sum();

        let max_risk = reports
            .iter()
            .map(|r| r.risk_level)
            .max()
            .unwrap_or(RiskLevel::Low);

        let breaking_changes = reports.iter().filter(|r| r.is_breaking()).count();

        ImpactSummary {
            total_changes: reports.len(),
            total_affected_schemas: total_schemas,
            total_affected_applications: total_applications,
            total_affected_pipelines: total_pipelines,
            total_estimated_effort_hours: total_effort,
            max_risk_level: max_risk,
            breaking_changes_count: breaking_changes,
        }
    }
}

/// Summary of multiple impact reports
#[derive(Debug, Clone)]
pub struct ImpactSummary {
    /// Total number of changes analyzed
    pub total_changes: usize,
    /// Total affected schemas
    pub total_affected_schemas: usize,
    /// Total affected applications
    pub total_affected_applications: usize,
    /// Total affected pipelines
    pub total_affected_pipelines: usize,
    /// Total estimated effort
    pub total_estimated_effort_hours: f64,
    /// Maximum risk level
    pub max_risk_level: RiskLevel,
    /// Number of breaking changes
    pub breaking_changes_count: usize,
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

    #[tokio::test]
    async fn test_impact_analysis() {
        let store = GraphStore::new();
        let analyzer = ImpactAnalyzer::new(store.clone());

        let id1 = SchemaId::new_v4();
        let id2 = SchemaId::new_v4();

        let node1 = create_test_schema(id1, "User");
        let node2 = create_test_schema(id2, "Profile");

        // Profile depends on User
        store
            .add_dependency(
                node2,
                DependencyTarget::Schema(node1),
                RelationType::DependsOn,
            )
            .unwrap();

        let change = SchemaChange::FieldRemoved {
            name: "email".to_string(),
        };

        let report = analyzer.analyze_impact(id1, change).await.unwrap();

        assert_eq!(report.target_schema, id1);
        assert!(report.is_breaking());
        assert!(!report.recommendations.is_empty());
    }

    #[tokio::test]
    async fn test_migration_complexity() {
        let store = GraphStore::new();
        let analyzer = ImpactAnalyzer::new(store);

        let change = SchemaChange::FieldAdded {
            name: "new_field".to_string(),
        };

        let complexity = analyzer.calculate_migration_complexity(&change, 5, &HashMap::new());

        assert!(complexity >= 0.0 && complexity <= 1.0);
    }

    #[tokio::test]
    async fn test_risk_levels() {
        assert_eq!(RiskLevel::from_count(5), RiskLevel::Low);
        assert_eq!(RiskLevel::from_count(25), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_count(100), RiskLevel::High);
        assert_eq!(RiskLevel::from_count(500), RiskLevel::Critical);
    }
}
