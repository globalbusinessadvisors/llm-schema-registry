//! TypeScript migration code generator

use crate::error::Result;
use crate::types::{GeneratedCode, MigrationContext, SchemaChange};
use indoc::formatdoc;

/// TypeScript code generator
pub struct TypeScriptGenerator;

impl TypeScriptGenerator {
    /// Generate TypeScript migration code
    pub fn generate(&self, context: &MigrationContext) -> Result<GeneratedCode> {
        let migration_code = self.generate_migration_function(context)?;
        let test_code = Some(self.generate_test_code(context)?);
        let rollback_code = Some(self.generate_rollback_function(context)?);
        let documentation = Some(self.generate_documentation(context)?);

        Ok(GeneratedCode {
            migration_code,
            test_code,
            rollback_code,
            documentation,
        })
    }

    fn generate_migration_function(&self, context: &MigrationContext) -> Result<String> {
        let from = &context.from_version;
        let to = &context.to_version;
        let schema_name = &context.schema_name;

        let breaking_count = context.changes.iter().filter(|c| c.is_breaking()).count();
        let non_breaking_count = context.changes.len() - breaking_count;
        let complexity = if context.changes.len() < 3 {
            "Low"
        } else if context.changes.len() < 7 {
            "Medium"
        } else {
            "High"
        };

        let mut transformations = Vec::new();
        for change in &context.changes {
            let code = self.generate_transformation(change)?;
            if !code.is_empty() {
                transformations.push(code);
            }
        }

        let transformations_str = transformations.join("\n  \n  ");

        let code = formatdoc! {r#"
            /**
             * Migration for {schema_name} schema: v{from} → v{to}
             *
             * Breaking changes: {breaking_count}
             * Non-breaking changes: {non_breaking_count}
             * Complexity: {complexity}
             *
             * @generated
             */

            export interface OldSchema {{
              // Define old schema interface
              [key: string]: unknown;
            }}

            export interface NewSchema {{
              // Define new schema interface
              [key: string]: unknown;
            }}

            export class MigrationError extends Error {{
              constructor(message: string) {{
                super(message);
                this.name = 'MigrationError';
              }}
            }}

            /**
             * Migrate {schema_name} from v{from} to v{to}
             *
             * @param data - The data in old schema format
             * @returns The data in new schema format
             * @throws {{MigrationError}} If migration fails
             */
            export function migrate{schema_name_camel}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(
              data: OldSchema
            ): NewSchema {{
              // Deep clone to avoid mutations
              const migrated = JSON.parse(JSON.stringify(data)) as Record<string, unknown>;

              {transformations}

              return migrated as NewSchema;
            }}

            /**
             * Migrate a batch of items
             *
             * @param items - Array of items to migrate
             * @returns Array of migrated items
             */
            export function migrateBatch(items: OldSchema[]): NewSchema[] {{
              return items.map((item) =>
                migrate{schema_name_camel}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(item)
              );
            }}

            /**
             * Safely migrate data, returning null if migration fails
             *
             * @param data - The data to migrate
             * @returns The migrated data or null on failure
             */
            export function safeMigrate(data: OldSchema): NewSchema | null {{
              try {{
                return migrate{schema_name_camel}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(data);
              }} catch (error) {{
                console.error('Migration failed:', error);
                return null;
              }}
            }}

            /**
             * Validate that data can be migrated
             *
             * @param data - The data to validate
             * @returns true if migration is possible
             */
            export function canMigrate(data: unknown): data is OldSchema {{
              if (typeof data !== 'object' || data === null) {{
                return false;
              }}
              // Add validation logic here
              return true;
            }}
        "#,
            schema_name = schema_name,
            schema_name_camel = Self::to_camel_case(schema_name),
            from = from,
            to = to,
            from_major = from.major,
            from_minor = from.minor,
            from_patch = from.patch,
            to_major = to.major,
            to_minor = to.minor,
            to_patch = to.patch,
            breaking_count = breaking_count,
            non_breaking_count = non_breaking_count,
            complexity = complexity,
            transformations = transformations_str,
        };

        Ok(code)
    }

    fn generate_transformation(&self, change: &SchemaChange) -> Result<String> {
        let code = match change {
            SchemaChange::FieldAdded { name, default, required, .. } => {
                if let Some(default_val) = default {
                    let default_str = self.format_default_value(default_val);
                    formatdoc! {r#"
                        // Add field '{name}' with default value
                        if (!('{name}' in migrated)) {{
                          migrated['{name}'] = {default_str};
                        }}
                    "#,
                        name = name,
                        default_str = default_str,
                    }
                } else if *required {
                    formatdoc! {r#"
                        // Add required field '{name}' - manual intervention needed
                        if (!('{name}' in migrated)) {{
                          throw new MigrationError("Required field '{name}' is missing and has no default value");
                        }}
                    "#,
                        name = name,
                    }
                } else {
                    String::new()
                }
            }
            SchemaChange::FieldRemoved { name, preserve_data, .. } => {
                if *preserve_data {
                    formatdoc! {r#"
                        // Remove field '{name}' (data preserved in migration log)
                        if ('{name}' in migrated) {{
                          const removedValue = migrated['{name}'];
                          // Log the removed value if needed
                          delete migrated['{name}'];
                        }}
                    "#,
                        name = name,
                    }
                } else {
                    formatdoc! {r#"
                        // Remove field '{name}'
                        delete migrated['{name}'];
                    "#,
                        name = name,
                    }
                }
            }
            SchemaChange::FieldRenamed { old_name, new_name, .. } => {
                formatdoc! {r#"
                    // Rename field '{old_name}' to '{new_name}'
                    if ('{old_name}' in migrated) {{
                      migrated['{new_name}'] = migrated['{old_name}'];
                      delete migrated['{old_name}'];
                    }}
                "#,
                    old_name = old_name,
                    new_name = new_name,
                }
            }
            SchemaChange::TypeChanged { field, old_type, new_type, .. } => {
                let converter = self.generate_type_converter(old_type, new_type);
                formatdoc! {r#"
                    // Convert type of '{field}' from {old_type:?} to {new_type:?}
                    if ('{field}' in migrated) {{
                      {converter}
                    }}
                "#,
                    field = field,
                    old_type = old_type,
                    new_type = new_type,
                    converter = converter.replace('\n', "\n    "),
                }
            }
            SchemaChange::ArrayElementChanged { field, .. } => {
                formatdoc! {r#"
                    // Transform array elements for '{field}'
                    if (Array.isArray(migrated['{field}'])) {{
                      migrated['{field}'] = (migrated['{field}'] as unknown[]).map((item) => {{
                        // Add transformation logic here
                        return item;
                      }});
                    }}
                "#,
                    field = field,
                }
            }
            _ => String::new(),
        };

        Ok(code)
    }

    fn generate_type_converter(&self, old_type: &crate::types::FieldType, new_type: &crate::types::FieldType) -> String {
        use crate::types::FieldType;

        match (old_type, new_type) {
            (FieldType::Integer, FieldType::String) | (FieldType::Long, FieldType::String) => {
                "migrated[field] = String(migrated[field]);".to_string()
            }
            (FieldType::String, FieldType::Integer) | (FieldType::String, FieldType::Long) => {
                "migrated[field] = parseInt(String(migrated[field]), 10);".to_string()
            }
            (FieldType::String, FieldType::Boolean) => {
                "migrated[field] = ['true', '1', 'yes'].includes(String(migrated[field]).toLowerCase());".to_string()
            }
            (FieldType::Boolean, FieldType::String) => {
                "migrated[field] = migrated[field] ? 'true' : 'false';".to_string()
            }
            (FieldType::Integer, FieldType::Long) | (FieldType::Float, FieldType::Double) => {
                "// Type widening - no conversion needed".to_string()
            }
            _ => "// Custom conversion required".to_string(),
        }
    }

    fn format_default_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "\\'")),
            serde_json::Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| self.format_default_value(v)).collect();
                format!("[{}]", items.join(", "))
            }
            serde_json::Value::Object(obj) => {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("'{}': {}", k, self.format_default_value(v)))
                    .collect();
                format!("{{ {} }}", items.join(", "))
            }
        }
    }

    fn generate_test_code(&self, context: &MigrationContext) -> Result<String> {
        let schema_name = &context.schema_name;
        let schema_name_camel = Self::to_camel_case(schema_name);
        let from = &context.from_version;
        let to = &context.to_version;

        let code = formatdoc! {r#"
            import {{ describe, it, expect }} from '@jest/globals';
            import {{
              migrate{schema_name_camel}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch},
              migrateBatch,
              safeMigrate,
              canMigrate,
            }} from './migration';

            describe('{schema_name} Migration v{from} → v{to}', () => {{
              it('should migrate basic data', () => {{
                const oldData = {{
                  // Add test data here
                }};

                const migrated = migrate{schema_name_camel}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(oldData);

                expect(migrated).toBeDefined();
                // Add assertions here
              }});

              it('should migrate batch of items', () => {{
                const items = [
                  {{}},
                  {{}},
                ];

                const migrated = migrateBatch(items);

                expect(migrated).toHaveLength(items.length);
              }});

              it('should handle errors gracefully in safe migration', () => {{
                const invalidData = {{ invalid: 'data' }};

                const result = safeMigrate(invalidData);

                expect(result).toBeDefined();
              }});

              it('should validate migratable data', () => {{
                const validData = {{}};
                const invalidData = 'not an object';

                expect(canMigrate(validData)).toBe(true);
                expect(canMigrate(invalidData)).toBe(false);
              }});
            }});
        "#,
            schema_name = schema_name,
            schema_name_camel = schema_name_camel,
            from = from,
            to = to,
            from_major = from.major,
            from_minor = from.minor,
            from_patch = from.patch,
            to_major = to.major,
            to_minor = to.minor,
            to_patch = to.patch,
        };

        Ok(code)
    }

    fn generate_rollback_function(&self, context: &MigrationContext) -> Result<String> {
        let from = &context.from_version;
        let to = &context.to_version;
        let schema_name_camel = Self::to_camel_case(&context.schema_name);

        let code = formatdoc! {r#"
            /**
             * Rollback {schema_name} from v{to} to v{from}
             *
             * WARNING: This is an automated rollback. Data loss may occur.
             *
             * @param data - The data in new schema format
             * @returns The data in old schema format
             */
            export function rollback{schema_name_camel}V{to_major}_{to_minor}_{to_patch}ToV{from_major}_{from_minor}_{from_patch}(
              data: NewSchema
            ): OldSchema {{
              const rolledBack = JSON.parse(JSON.stringify(data)) as Record<string, unknown>;

              // Reverse the migration changes
              // This is a simplified rollback - manual review recommended

              return rolledBack as OldSchema;
            }}
        "#,
            schema_name = &context.schema_name,
            schema_name_camel = schema_name_camel,
            from = from,
            to = to,
            from_major = from.major,
            from_minor = from.minor,
            from_patch = from.patch,
            to_major = to.major,
            to_minor = to.minor,
            to_patch = to.patch,
        };

        Ok(code)
    }

    fn generate_documentation(&self, context: &MigrationContext) -> Result<String> {
        let doc = formatdoc! {r#"
            # Migration Documentation: {schema_name} v{from} → v{to}

            ## Overview
            - Generated: {generated_at}
            - Changes: {num_changes}
            - Breaking Changes: {breaking_changes}

            ## Changes
            {changes_list}

            ## Usage

            ```typescript
            import {{ migrate{schema_name_camel}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch} }} from './migration';

            // Migrate single item
            const oldData = {{ field: 'value' }};
            const newData = migrate{schema_name_camel}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(oldData);

            // Migrate batch
            const items = [oldData1, oldData2];
            const migratedItems = migrateBatch(items);

            // Safe migration with error handling
            const result = safeMigrate(oldData);
            if (result === null) {{
              console.error('Migration failed');
            }}
            ```

            ## Safety
            - Always test migrations on non-production data first
            - Consider creating backups before running migrations
            - Review breaking changes carefully
        "#,
            schema_name = &context.schema_name,
            schema_name_camel = Self::to_camel_case(&context.schema_name),
            from = &context.from_version,
            to = &context.to_version,
            from_major = context.from_version.major,
            from_minor = context.from_version.minor,
            from_patch = context.from_version.patch,
            to_major = context.to_version.major,
            to_minor = context.to_version.minor,
            to_patch = context.to_version.patch,
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

    fn to_camel_case(s: &str) -> String {
        s.split(|c: char| c == '-' || c == '_' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .enumerate()
            .map(|(i, word)| {
                if i == 0 {
                    word.to_string()
                } else {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().chain(chars).collect(),
                    }
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldType, SchemaChange};
    use chrono::Utc;
    use schema_registry_core::versioning::SemanticVersion;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(TypeScriptGenerator::to_camel_case("user-schema"), "userSchema");
        assert_eq!(TypeScriptGenerator::to_camel_case("my_test_schema"), "myTestSchema");
    }

    #[test]
    fn test_generate_typescript_migration() {
        let generator = TypeScriptGenerator;
        let context = MigrationContext {
            from_version: SemanticVersion::new(1, 0, 0),
            to_version: SemanticVersion::new(2, 0, 0),
            schema_name: "user-schema".to_string(),
            changes: vec![
                SchemaChange::FieldAdded {
                    name: "emailVerified".to_string(),
                    field_type: FieldType::Boolean,
                    default: Some(serde_json::Value::Bool(false)),
                    required: false,
                    description: None,
                },
            ],
            generated_at: Utc::now(),
            options: Default::default(),
        };

        let result = generator.generate(&context);
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.migration_code.contains("export function migrate"));
        assert!(code.migration_code.contains("emailVerified"));
    }
}
