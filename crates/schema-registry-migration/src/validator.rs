//! Migration validation

use crate::error::{Error, Result};
use crate::types::{MigrationPlan, RiskLevel, SchemaChange, ValidationRule, ValidationRuleType};
use serde_json::Value;

/// Migration validator
pub struct MigrationValidator;

impl MigrationValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self
    }

    /// Validate a migration plan
    pub fn validate(&self, plan: &MigrationPlan) -> Result<ValidationReport> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        let mut info = Vec::new();

        // Check for data loss risks
        let data_loss_risks = self.check_data_loss(&plan.diff.changes);
        warnings.extend(data_loss_risks);

        // Check type compatibility
        let type_issues = self.check_type_compatibility(&plan.diff.changes);
        errors.extend(type_issues);

        // Check constraint satisfaction
        let constraint_issues = self.check_constraints(&plan.diff.changes);
        warnings.extend(constraint_issues);

        // Assess overall risk
        let risk_assessment = self.assess_risk(plan);
        info.push(format!("Risk level: {}", risk_assessment));

        // Check for breaking changes
        if !plan.diff.breaking_changes.is_empty() {
            warnings.push(format!(
                "Migration contains {} breaking changes",
                plan.diff.breaking_changes.len()
            ));
        }

        let valid = errors.is_empty();

        Ok(ValidationReport {
            valid,
            errors,
            warnings,
            info,
            risk_level: risk_assessment,
        })
    }

    /// Perform a dry-run validation with sample data
    pub fn dry_run(&self, plan: &MigrationPlan, sample_data: &[Value]) -> Result<DryRunReport> {
        let mut successful = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for (idx, data) in sample_data.iter().enumerate() {
            match self.simulate_migration(plan, data) {
                Ok(_) => successful += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("Item {}: {}", idx, e));
                }
            }
        }

        Ok(DryRunReport {
            total: sample_data.len(),
            successful,
            failed,
            success_rate: if sample_data.is_empty() {
                0.0
            } else {
                successful as f64 / sample_data.len() as f64
            },
            errors,
        })
    }

    /// Check for data loss risks
    fn check_data_loss(&self, changes: &[SchemaChange]) -> Vec<String> {
        let mut warnings = Vec::new();

        for change in changes {
            match change {
                SchemaChange::FieldRemoved { name, preserve_data, .. } => {
                    if !preserve_data {
                        warnings.push(format!(
                            "Removing field '{}' will result in permanent data loss",
                            name
                        ));
                    }
                }
                SchemaChange::TypeChanged { field, old_type, new_type, .. } => {
                    if !old_type.is_compatible_with(new_type) {
                        warnings.push(format!(
                            "Type change for '{}' from {:?} to {:?} may cause data loss",
                            field, old_type, new_type
                        ));
                    }
                }
                _ => {}
            }
        }

        warnings
    }

    /// Check type compatibility
    fn check_type_compatibility(&self, changes: &[SchemaChange]) -> Vec<String> {
        let mut errors = Vec::new();

        for change in changes {
            if let SchemaChange::TypeChanged { field, old_type, new_type, .. } = change {
                // Check for unsafe type conversions
                if !self.is_safe_conversion(old_type, new_type) {
                    errors.push(format!(
                        "Unsafe type conversion for '{}': {:?} to {:?} requires manual validation",
                        field, old_type, new_type
                    ));
                }
            }
        }

        errors
    }

    /// Check if a type conversion is safe
    fn is_safe_conversion(&self, old_type: &crate::types::FieldType, new_type: &crate::types::FieldType) -> bool {
        use crate::types::FieldType;

        matches!(
            (old_type, new_type),
            (FieldType::Integer, FieldType::Long)
                | (FieldType::Float, FieldType::Double)
                | (FieldType::Integer, FieldType::String)
                | (FieldType::Long, FieldType::String)
                | (FieldType::Boolean, FieldType::String)
        )
    }

    /// Check constraint satisfaction
    fn check_constraints(&self, changes: &[SchemaChange]) -> Vec<String> {
        let mut warnings = Vec::new();

        for change in changes {
            if let SchemaChange::ConstraintAdded { field, constraint } = change {
                warnings.push(format!(
                    "New constraint {:?} on '{}' may reject existing data",
                    constraint, field
                ));
            }
        }

        warnings
    }

    /// Assess overall migration risk
    fn assess_risk(&self, plan: &MigrationPlan) -> RiskLevel {
        let breaking_count = plan.diff.breaking_changes.len();
        let complexity = plan.diff.complexity_score;

        if breaking_count == 0 && complexity < 0.3 {
            RiskLevel::Low
        } else if breaking_count <= 2 && complexity < 0.6 {
            RiskLevel::Medium
        } else if breaking_count <= 5 && complexity < 0.8 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        }
    }

    /// Simulate migration on sample data
    fn simulate_migration(&self, _plan: &MigrationPlan, _data: &Value) -> Result<Value> {
        // This would actually apply the migration transformations
        // For now, we just return the data unchanged
        Ok(_data.clone())
    }

    /// Estimate migration performance
    pub fn estimate_performance(&self, plan: &MigrationPlan, data_size: usize) -> PerformanceEstimate {
        // Simple heuristic: 1ms per change per 1000 items
        let changes_count = plan.diff.changes.len();
        let estimated_ms = (data_size as f64 / 1000.0) * changes_count as f64;

        let estimated_duration = std::time::Duration::from_millis(estimated_ms as u64);

        PerformanceEstimate {
            estimated_duration,
            estimated_memory_mb: (data_size as f64 * 0.001).ceil() as usize,
            parallel_safe: !plan.diff.changes.iter().any(|c| {
                matches!(
                    c,
                    SchemaChange::TypeChanged { .. } | SchemaChange::FieldRemoved { .. }
                )
            }),
        }
    }

    /// Generate validation rules for a migration
    pub fn generate_rules(&self, changes: &[SchemaChange]) -> Vec<ValidationRule> {
        let mut rules = Vec::new();

        for change in changes {
            match change {
                SchemaChange::FieldRemoved { name, field_type: _, preserve_data: _ } => {
                    rules.push(ValidationRule {
                        name: format!("Check data loss for '{}'", name),
                        description: format!("Ensure data from '{}' is preserved or migration path exists", name),
                        fields: vec![name.clone()],
                        rule_type: ValidationRuleType::DataLoss,
                    });
                }
                SchemaChange::TypeChanged { field, old_type, new_type, .. } => {
                    rules.push(ValidationRule {
                        name: format!("Validate type conversion for '{}'", field),
                        description: format!(
                            "Ensure all values can be converted from {:?} to {:?}",
                            old_type, new_type
                        ),
                        fields: vec![field.clone()],
                        rule_type: ValidationRuleType::TypeCompatibility,
                    });
                }
                SchemaChange::ConstraintAdded { field, constraint } => {
                    rules.push(ValidationRule {
                        name: format!("Validate constraint for '{}'", field),
                        description: format!("Ensure all existing values satisfy {:?}", constraint),
                        fields: vec![field.clone()],
                        rule_type: ValidationRuleType::ConstraintSatisfaction,
                    });
                }
                _ => {}
            }
        }

        rules
    }
}

impl Default for MigrationValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Whether the migration is valid
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Informational messages
    pub info: Vec<String>,
    /// Assessed risk level
    pub risk_level: RiskLevel,
}

/// Dry-run report
#[derive(Debug, Clone)]
pub struct DryRunReport {
    /// Total items tested
    pub total: usize,
    /// Successfully migrated
    pub successful: usize,
    /// Failed migrations
    pub failed: usize,
    /// Success rate (0.0-1.0)
    pub success_rate: f64,
    /// Error messages
    pub errors: Vec<String>,
}

/// Performance estimation
#[derive(Debug, Clone)]
pub struct PerformanceEstimate {
    /// Estimated duration
    pub estimated_duration: std::time::Duration,
    /// Estimated memory usage in MB
    pub estimated_memory_mb: usize,
    /// Whether migration can be safely parallelized
    pub parallel_safe: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldType, MigrationPlan, MigrationStrategy, SchemaDiff};
    use chrono::Utc;
    use schema_registry_core::versioning::SemanticVersion;
    use std::collections::HashMap;

    #[test]
    fn test_validate_safe_migration() {
        let validator = MigrationValidator::new();

        let plan = MigrationPlan {
            diff: SchemaDiff {
                old_version: SemanticVersion::new(1, 0, 0),
                new_version: SemanticVersion::new(1, 1, 0),
                schema_name: "test".to_string(),
                namespace: "com.example".to_string(),
                changes: vec![],
                breaking_changes: vec![],
                complexity_score: 0.1,
                created_at: Utc::now(),
            },
            strategy: MigrationStrategy::Safe,
            code_templates: HashMap::new(),
            validation_rules: vec![],
            rollback_plan: None,
            estimated_duration: None,
            risk_level: RiskLevel::Low,
        };

        let report = validator.validate(&plan).unwrap();
        assert!(report.valid);
        assert_eq!(report.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_check_data_loss() {
        let validator = MigrationValidator::new();

        let changes = vec![
            SchemaChange::FieldRemoved {
                name: "old_field".to_string(),
                field_type: FieldType::String,
                preserve_data: false,
            },
        ];

        let warnings = validator.check_data_loss(&changes);
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("data loss"));
    }

    #[test]
    fn test_performance_estimation() {
        let validator = MigrationValidator::new();

        let plan = MigrationPlan {
            diff: SchemaDiff {
                old_version: SemanticVersion::new(1, 0, 0),
                new_version: SemanticVersion::new(2, 0, 0),
                schema_name: "test".to_string(),
                namespace: "com.example".to_string(),
                changes: vec![
                    SchemaChange::FieldAdded {
                        name: "new_field".to_string(),
                        field_type: FieldType::String,
                        default: None,
                        required: false,
                        description: None,
                    },
                ],
                breaking_changes: vec![],
                complexity_score: 0.3,
                created_at: Utc::now(),
            },
            strategy: MigrationStrategy::Safe,
            code_templates: HashMap::new(),
            validation_rules: vec![],
            rollback_plan: None,
            estimated_duration: None,
            risk_level: RiskLevel::Low,
        };

        let estimate = validator.estimate_performance(&plan, 10000);
        assert!(estimate.estimated_duration.as_millis() > 0);
    }
}
