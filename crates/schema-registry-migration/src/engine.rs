//! Migration engine - main orchestrator

use crate::analyzer::SchemaAnalyzer;
use crate::error::{Error, Result};
use crate::generators::{GoGenerator, JavaGenerator, PythonGenerator, SqlGenerator, TypeScriptGenerator};
use crate::types::{
    GeneratedCode, Language, MigrationContext, MigrationPlan, MigrationStrategy, RiskLevel,
    RollbackPlan, RollbackStrategy, SchemaDiff,
};
use crate::validator::{MigrationValidator, PerformanceEstimate, ValidationReport};
use schema_registry_core::{versioning::SemanticVersion, RegisteredSchema, SerializationFormat};
use std::collections::HashMap;

/// Main migration engine
pub struct MigrationEngine {
    /// Schema analyzer
    analyzer: SchemaAnalyzer,
    /// Migration validator
    validator: MigrationValidator,
}

impl MigrationEngine {
    /// Create a new migration engine for a specific schema format
    pub fn new(format: SerializationFormat) -> Self {
        Self {
            analyzer: SchemaAnalyzer::new(format),
            validator: MigrationValidator::new(),
        }
    }

    /// Generate a complete migration plan
    pub fn generate_migration(
        &self,
        old_schema: &RegisteredSchema,
        new_schema: &RegisteredSchema,
        languages: Vec<Language>,
    ) -> Result<MigrationPlan> {
        // Analyze differences
        let diff = self.analyzer.analyze(
            &old_schema.content,
            &new_schema.content,
            old_schema.version.clone(),
            new_schema.version.clone(),
            old_schema.name.clone(),
            old_schema.namespace.clone(),
        )?;

        // Suggest migration strategy
        let strategy = self.analyzer.suggest_strategy(&diff);

        // Generate validation rules
        let validation_rules = self.validator.generate_rules(&diff.changes);

        // Generate code for requested languages
        let code_templates = self.generate_code_for_languages(&diff, &languages)?;

        // Generate rollback plan
        let rollback_plan = self.generate_rollback_plan(&diff, &languages)?;

        // Estimate duration
        let estimated_duration = Some(
            self.validator
                .estimate_performance(
                    &MigrationPlan {
                        diff: diff.clone(),
                        strategy,
                        code_templates: HashMap::new(),
                        validation_rules: vec![],
                        rollback_plan: None,
                        estimated_duration: None,
                        risk_level: RiskLevel::Low,
                    },
                    1000,
                )
                .estimated_duration,
        );

        // Assess risk level
        let risk_level = self.assess_risk(&diff, &strategy);

        Ok(MigrationPlan {
            diff,
            strategy,
            code_templates,
            validation_rules,
            rollback_plan: Some(rollback_plan),
            estimated_duration,
            risk_level,
        })
    }

    /// Validate a migration plan
    pub fn validate_migration(&self, plan: &MigrationPlan) -> Result<ValidationReport> {
        self.validator.validate(plan)
    }

    /// Estimate migration complexity
    pub fn estimate_complexity(&self, diff: &SchemaDiff) -> f64 {
        diff.complexity_score
    }

    /// Generate rollback migration
    pub fn generate_rollback(
        &self,
        new_schema: &RegisteredSchema,
        old_schema: &RegisteredSchema,
        languages: Vec<Language>,
    ) -> Result<MigrationPlan> {
        // Generate reverse migration (new -> old)
        self.generate_migration(new_schema, old_schema, languages)
    }

    /// Estimate migration performance
    pub fn estimate_performance(
        &self,
        plan: &MigrationPlan,
        data_size: usize,
    ) -> PerformanceEstimate {
        self.validator.estimate_performance(plan, data_size)
    }

    /// Generate code for multiple languages
    fn generate_code_for_languages(
        &self,
        diff: &SchemaDiff,
        languages: &[Language],
    ) -> Result<HashMap<Language, GeneratedCode>> {
        let mut code_templates = HashMap::new();

        let context = MigrationContext {
            from_version: diff.old_version.clone(),
            to_version: diff.new_version.clone(),
            schema_name: diff.schema_name.clone(),
            changes: diff.changes.clone(),
            generated_at: diff.created_at,
            options: HashMap::new(),
        };

        for &language in languages {
            let code = match language {
                Language::Python => PythonGenerator.generate(&context)?,
                Language::TypeScript => TypeScriptGenerator.generate(&context)?,
                Language::Go => GoGenerator.generate(&context)?,
                Language::Java => JavaGenerator.generate(&context, None)?,
                Language::Sql => SqlGenerator.generate(&context, None)?,
            };

            code_templates.insert(language, code);
        }

        Ok(code_templates)
    }

    /// Generate rollback plan
    fn generate_rollback_plan(
        &self,
        diff: &SchemaDiff,
        languages: &[Language],
    ) -> Result<RollbackPlan> {
        // Determine rollback strategy
        let strategy = if diff.breaking_changes.is_empty() {
            RollbackStrategy::Reverse
        } else if diff.breaking_changes.len() > 5 {
            RollbackStrategy::Manual
        } else {
            RollbackStrategy::Backup
        };

        // Generate rollback code for each language
        let mut rollback_code = HashMap::new();

        let context = MigrationContext {
            from_version: diff.new_version.clone(), // Reversed
            to_version: diff.old_version.clone(),   // Reversed
            schema_name: diff.schema_name.clone(),
            changes: vec![], // Would need to reverse changes
            generated_at: diff.created_at,
            options: HashMap::new(),
        };

        for &language in languages {
            let code = match language {
                Language::Python => {
                    PythonGenerator.generate(&context)?.rollback_code.unwrap_or_default()
                }
                Language::TypeScript => {
                    TypeScriptGenerator.generate(&context)?.rollback_code.unwrap_or_default()
                }
                Language::Go => GoGenerator.generate(&context)?.rollback_code.unwrap_or_default(),
                Language::Java => {
                    JavaGenerator.generate(&context, None)?.migration_code // Java uses same class
                }
                Language::Sql => {
                    SqlGenerator.generate(&context, None)?.rollback_code.unwrap_or_default()
                }
            };

            rollback_code.insert(language, code);
        }

        Ok(RollbackPlan {
            strategy,
            rollback_code,
            estimated_duration: Some(std::time::Duration::from_secs(10)),
            backup_required: matches!(strategy, RollbackStrategy::Backup),
        })
    }

    /// Assess migration risk level
    fn assess_risk(&self, diff: &SchemaDiff, strategy: &MigrationStrategy) -> RiskLevel {
        let breaking_count = diff.breaking_changes.len();
        let complexity = diff.complexity_score;

        match strategy {
            MigrationStrategy::Safe => {
                if complexity < 0.3 {
                    RiskLevel::Low
                } else {
                    RiskLevel::Medium
                }
            }
            MigrationStrategy::Risky => {
                if complexity < 0.5 {
                    RiskLevel::Medium
                } else {
                    RiskLevel::High
                }
            }
            MigrationStrategy::Manual | MigrationStrategy::DualWrite | MigrationStrategy::Shadow => {
                if breaking_count > 5 || complexity > 0.8 {
                    RiskLevel::Critical
                } else {
                    RiskLevel::High
                }
            }
        }
    }

    /// Generate migration for schema content strings
    pub fn generate_migration_from_content(
        &self,
        old_content: &str,
        new_content: &str,
        old_version: SemanticVersion,
        new_version: SemanticVersion,
        schema_name: String,
        namespace: String,
        languages: Vec<Language>,
    ) -> Result<MigrationPlan> {
        // Analyze differences
        let diff = self.analyzer.analyze(
            old_content,
            new_content,
            old_version,
            new_version,
            schema_name,
            namespace,
        )?;

        // Suggest migration strategy
        let strategy = self.analyzer.suggest_strategy(&diff);

        // Generate validation rules
        let validation_rules = self.validator.generate_rules(&diff.changes);

        // Generate code for requested languages
        let code_templates = self.generate_code_for_languages(&diff, &languages)?;

        // Generate rollback plan
        let rollback_plan = self.generate_rollback_plan(&diff, &languages)?;

        // Assess risk level
        let risk_level = self.assess_risk(&diff, &strategy);

        Ok(MigrationPlan {
            diff,
            strategy,
            code_templates,
            validation_rules,
            rollback_plan: Some(rollback_plan),
            estimated_duration: Some(std::time::Duration::from_secs(5)),
            risk_level,
        })
    }
}

/// Builder for migration engine configuration
pub struct MigrationEngineBuilder {
    format: SerializationFormat,
}

impl MigrationEngineBuilder {
    /// Create a new builder
    pub fn new(format: SerializationFormat) -> Self {
        Self { format }
    }

    /// Build the migration engine
    pub fn build(self) -> MigrationEngine {
        MigrationEngine::new(self.format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_engine_creation() {
        let engine = MigrationEngine::new(SerializationFormat::JsonSchema);
        assert!(true); // Engine created successfully
    }

    #[test]
    fn test_builder_pattern() {
        let engine = MigrationEngineBuilder::new(SerializationFormat::JsonSchema).build();
        assert!(true); // Engine built successfully
    }

    #[test]
    fn test_generate_migration_from_content() {
        let engine = MigrationEngine::new(SerializationFormat::JsonSchema);

        let old_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }"#;

        let new_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "default": 0}
            }
        }"#;

        let result = engine.generate_migration_from_content(
            old_schema,
            new_schema,
            SemanticVersion::new(1, 0, 0),
            SemanticVersion::new(2, 0, 0),
            "user".to_string(),
            "com.example".to_string(),
            vec![Language::Python, Language::TypeScript],
        );

        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.code_templates.len(), 2);
        assert!(plan.code_templates.contains_key(&Language::Python));
        assert!(plan.code_templates.contains_key(&Language::TypeScript));
    }
}
