//! SQL migration code generator

use crate::error::Result;
use crate::types::{GeneratedCode, Language, MigrationContext, SchemaChange};
use indoc::formatdoc;

/// SQL code generator
pub struct SqlGenerator;

impl SqlGenerator {
    /// Generate SQL migration code
    pub fn generate(&self, context: &MigrationContext, table_name: Option<&str>) -> Result<GeneratedCode> {
        let table = table_name.unwrap_or(&context.schema_name);
        let migration_code = self.generate_migration_sql(context, table)?;
        let rollback_code = Some(self.generate_rollback_sql(context, table)?);
        let documentation = Some(self.generate_documentation(context, table)?);

        Ok(GeneratedCode {
            migration_code,
            test_code: None,
            rollback_code,
            documentation,
        })
    }

    fn generate_migration_sql(&self, context: &MigrationContext, table_name: &str) -> Result<String> {
        let from = &context.from_version;
        let to = &context.to_version;

        let breaking_count = context.changes.iter().filter(|c| c.is_breaking()).count();
        let non_breaking_count = context.changes.len() - breaking_count;

        let mut statements = Vec::new();

        for change in &context.changes {
            if let Some(sql) = self.generate_statement(change, table_name)? {
                statements.push(sql);
            }
        }

        let statements_str = statements.join("\n\n");

        let code = formatdoc! {r#"
            -- Migration: {table_name} v{from} → v{to}
            -- Generated: {generated_at}
            --
            -- Breaking changes: {breaking_count}
            -- Non-breaking changes: {non_breaking_count}
            --
            -- IMPORTANT: Review this migration carefully before applying to production!

            BEGIN;

            {statements}

            -- Update schema version
            -- UPDATE schema_versions SET version = '{to}' WHERE table_name = '{table_name}';

            COMMIT;

            -- Rollback: Run the rollback script if needed
        "#,
            table_name = table_name,
            from = from,
            to = to,
            generated_at = context.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            breaking_count = breaking_count,
            non_breaking_count = non_breaking_count,
            statements = statements_str,
        };

        Ok(code)
    }

    fn generate_statement(&self, change: &SchemaChange, table_name: &str) -> Result<Option<String>> {
        let sql = match change {
            SchemaChange::FieldAdded { name, field_type, default, required, .. } => {
                let sql_type = field_type.type_name(Language::Sql);
                let nullable = if *required { "NOT NULL" } else { "NULL" };

                let default_clause = if let Some(default_val) = default {
                    format!(" DEFAULT {}", self.format_default_value(default_val))
                } else {
                    String::new()
                };

                Some(formatdoc! {r#"
                    -- Add column '{name}'
                    ALTER TABLE {table_name}
                      ADD COLUMN {name} {sql_type} {nullable}{default_clause};
                "#,
                    name = name,
                    table_name = table_name,
                    sql_type = sql_type,
                    nullable = nullable,
                    default_clause = default_clause,
                })
            }
            SchemaChange::FieldRemoved { name, preserve_data, .. } => {
                if *preserve_data {
                    Some(formatdoc! {r#"
                        -- Archive column '{name}' before removal
                        -- CREATE TABLE {table_name}_archive AS SELECT id, {name} FROM {table_name};

                        -- Remove column '{name}'
                        ALTER TABLE {table_name}
                          DROP COLUMN {name};
                    "#,
                        name = name,
                        table_name = table_name,
                    })
                } else {
                    Some(formatdoc! {r#"
                        -- Remove column '{name}'
                        ALTER TABLE {table_name}
                          DROP COLUMN {name};
                    "#,
                        name = name,
                        table_name = table_name,
                    })
                }
            }
            SchemaChange::FieldRenamed { old_name, new_name, .. } => {
                Some(formatdoc! {r#"
                    -- Rename column '{old_name}' to '{new_name}'
                    ALTER TABLE {table_name}
                      RENAME COLUMN {old_name} TO {new_name};
                "#,
                    old_name = old_name,
                    new_name = new_name,
                    table_name = table_name,
                })
            }
            SchemaChange::TypeChanged { field, old_type, new_type, .. } => {
                let old_sql_type = old_type.type_name(Language::Sql);
                let new_sql_type = new_type.type_name(Language::Sql);
                let using_clause = self.generate_type_conversion(field, old_type, new_type);

                Some(formatdoc! {r#"
                    -- Change type of '{field}' from {old_sql_type} to {new_sql_type}
                    ALTER TABLE {table_name}
                      ALTER COLUMN {field} TYPE {new_sql_type} {using_clause};
                "#,
                    field = field,
                    table_name = table_name,
                    old_sql_type = old_sql_type,
                    new_sql_type = new_sql_type,
                    using_clause = using_clause,
                })
            }
            SchemaChange::ConstraintAdded { field, constraint } => {
                let constraint_sql = match constraint {
                    crate::types::Constraint::NotNull => {
                        formatdoc! {r#"
                            -- Add NOT NULL constraint to '{field}'
                            ALTER TABLE {table_name}
                              ALTER COLUMN {field} SET NOT NULL;
                        "#,
                            field = field,
                            table_name = table_name,
                        }
                    }
                    crate::types::Constraint::Unique => {
                        formatdoc! {r#"
                            -- Add UNIQUE constraint to '{field}'
                            ALTER TABLE {table_name}
                              ADD CONSTRAINT {table_name}_{field}_unique UNIQUE ({field});
                        "#,
                            field = field,
                            table_name = table_name,
                        }
                    }
                    crate::types::Constraint::Minimum(min) => {
                        formatdoc! {r#"
                            -- Add CHECK constraint (minimum value) to '{field}'
                            ALTER TABLE {table_name}
                              ADD CONSTRAINT {table_name}_{field}_min CHECK ({field} >= {min});
                        "#,
                            field = field,
                            table_name = table_name,
                            min = min,
                        }
                    }
                    crate::types::Constraint::Maximum(max) => {
                        formatdoc! {r#"
                            -- Add CHECK constraint (maximum value) to '{field}'
                            ALTER TABLE {table_name}
                              ADD CONSTRAINT {table_name}_{field}_max CHECK ({field} <= {max});
                        "#,
                            field = field,
                            table_name = table_name,
                            max = max,
                        }
                    }
                    crate::types::Constraint::MinLength(len) => {
                        formatdoc! {r#"
                            -- Add CHECK constraint (minimum length) to '{field}'
                            ALTER TABLE {table_name}
                              ADD CONSTRAINT {table_name}_{field}_minlen CHECK (length({field}) >= {len});
                        "#,
                            field = field,
                            table_name = table_name,
                            len = len,
                        }
                    }
                    crate::types::Constraint::Pattern(pattern) => {
                        formatdoc! {r#"
                            -- Add CHECK constraint (pattern) to '{field}'
                            ALTER TABLE {table_name}
                              ADD CONSTRAINT {table_name}_{field}_pattern CHECK ({field} ~ '{pattern}');
                        "#,
                            field = field,
                            table_name = table_name,
                            pattern = pattern.replace('\'', "''"),
                        }
                    }
                    _ => String::new(),
                };
                Some(constraint_sql)
            }
            SchemaChange::ConstraintRemoved { field, constraint } => {
                let constraint_name = format!("{}_{}_constraint", table_name, field);
                Some(formatdoc! {r#"
                    -- Remove constraint from '{field}'
                    ALTER TABLE {table_name}
                      DROP CONSTRAINT IF EXISTS {constraint_name};
                "#,
                    field = field,
                    table_name = table_name,
                    constraint_name = constraint_name,
                })
            }
            _ => None,
        };

        Ok(sql)
    }

    fn generate_type_conversion(&self, field: &str, old_type: &crate::types::FieldType, new_type: &crate::types::FieldType) -> String {
        use crate::types::FieldType;

        match (old_type, new_type) {
            (FieldType::Integer, FieldType::String) | (FieldType::Long, FieldType::String) => {
                format!("USING {}::VARCHAR", field)
            }
            (FieldType::String, FieldType::Integer) => {
                format!("USING {}::INTEGER", field)
            }
            (FieldType::String, FieldType::Long) => {
                format!("USING {}::BIGINT", field)
            }
            (FieldType::Integer, FieldType::Long) => {
                format!("USING {}::BIGINT", field)
            }
            (FieldType::Float, FieldType::Double) => {
                format!("USING {}::DOUBLE PRECISION", field)
            }
            _ => format!("USING {}", field),
        }
    }

    fn format_default_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Bool(b) => b.to_string().to_uppercase(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
            serde_json::Value::Array(_) => "'{}'::JSONB".to_string(),
            serde_json::Value::Object(_) => "'{}'::JSONB".to_string(),
        }
    }

    fn generate_rollback_sql(&self, context: &MigrationContext, table_name: &str) -> Result<String> {
        let from = &context.from_version;
        let to = &context.to_version;

        let code = formatdoc! {r#"
            -- Rollback Migration: {table_name} v{to} → v{from}
            -- Generated: {generated_at}
            --
            -- WARNING: This rollback may result in data loss!
            -- Review carefully before executing.

            BEGIN;

            -- Reverse the migration changes here
            -- This is a template - customize based on your specific changes

            -- Revert schema version
            -- UPDATE schema_versions SET version = '{from}' WHERE table_name = '{table_name}';

            COMMIT;
        "#,
            table_name = table_name,
            from = from,
            to = to,
            generated_at = context.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
        };

        Ok(code)
    }

    fn generate_documentation(&self, context: &MigrationContext, table_name: &str) -> Result<String> {
        let doc = formatdoc! {r#"
            # SQL Migration Documentation: {table_name} v{from} → v{to}

            ## Overview
            - Generated: {generated_at}
            - Changes: {num_changes}
            - Breaking Changes: {breaking_changes}

            ## Changes
            {changes_list}

            ## Execution Steps

            1. **Backup**: Create a backup of the table before migration
               ```sql
               CREATE TABLE {table_name}_backup AS SELECT * FROM {table_name};
               ```

            2. **Test**: Run migration on a test environment first

            3. **Apply**: Execute the migration script
               ```bash
               psql -U username -d database -f migration.sql
               ```

            4. **Verify**: Check that data migrated correctly
               ```sql
               SELECT * FROM {table_name} LIMIT 10;
               ```

            5. **Rollback** (if needed): Execute rollback script
               ```bash
               psql -U username -d database -f rollback.sql
               ```

            ## Safety Considerations
            - Always test on non-production data first
            - Create backups before running migrations
            - Review breaking changes carefully
            - Consider maintenance windows for large tables
            - Monitor migration performance
        "#,
            table_name = table_name,
            from = &context.from_version,
            to = &context.to_version,
            generated_at = context.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            num_changes = context.changes.len(),
            breaking_changes = context.changes.iter().filter(|c| c.is_breaking()).count(),
            changes_list = context.changes.iter()
                .map(|c| format!("- {}", c.description()))
                .collect::<Vec<_>>()
                .join("\n"),
        };

        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldType, SchemaChange};
    use chrono::Utc;
    use schema_registry_core::versioning::SemanticVersion;

    #[test]
    fn test_generate_sql_migration() {
        let generator = SqlGenerator;
        let context = MigrationContext {
            from_version: SemanticVersion::new(1, 0, 0),
            to_version: SemanticVersion::new(2, 0, 0),
            schema_name: "users".to_string(),
            changes: vec![
                SchemaChange::FieldAdded {
                    name: "email_verified".to_string(),
                    field_type: FieldType::Boolean,
                    default: Some(serde_json::Value::Bool(false)),
                    required: false,
                    description: None,
                },
            ],
            generated_at: Utc::now(),
            options: Default::default(),
        };

        let result = generator.generate(&context, Some("users"));
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.migration_code.contains("ALTER TABLE users"));
        assert!(code.migration_code.contains("email_verified"));
        assert!(code.migration_code.contains("BOOLEAN"));
    }
}
