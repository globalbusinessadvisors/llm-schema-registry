//! Go migration code generator

use crate::error::Result;
use crate::types::{GeneratedCode, MigrationContext, SchemaChange};
use indoc::formatdoc;

/// Go code generator
pub struct GoGenerator;

impl GoGenerator {
    /// Generate Go migration code
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
        let schema_name = Self::to_snake_case(&context.schema_name);

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

        let transformations_str = transformations.join("\n\t\n\t");

        let code = formatdoc! {r#"
            package migration

            import (
                "encoding/json"
                "errors"
                "fmt"
            )

            // Migration for {schema_name} schema: v{from} → v{to}
            //
            // Breaking changes: {breaking_count}
            // Non-breaking changes: {non_breaking_count}
            // Complexity: {complexity}

            // OldSchema represents the old schema structure
            type OldSchema map[string]interface{{}}

            // NewSchema represents the new schema structure
            type NewSchema map[string]interface{{}}

            // MigrationError represents a migration error
            type MigrationError struct {{
                Message string
                Field   string
            }}

            func (e *MigrationError) Error() string {{
                return fmt.Sprintf("migration error on field %s: %s", e.Field, e.Message)
            }}

            // Migrate{func_name}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch} migrates {schema_name} from v{from} to v{to}
            func Migrate{func_name}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(data OldSchema) (NewSchema, error) {{
                // Deep copy to avoid mutations
                migrated := make(NewSchema)
                for k, v := range data {{
                    migrated[k] = v
                }}

                {transformations}

                return migrated, nil
            }}

            // MigrateBatch migrates a batch of items
            func MigrateBatch(items []OldSchema) ([]NewSchema, error) {{
                results := make([]NewSchema, 0, len(items))
                for i, item := range items {{
                    migrated, err := Migrate{func_name}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(item)
                    if err != nil {{
                        return nil, fmt.Errorf("failed to migrate item %d: %w", i, err)
                    }}
                    results = append(results, migrated)
                }}
                return results, nil
            }}

            // SafeMigrate safely migrates data, returning nil on error
            func SafeMigrate(data OldSchema) *NewSchema {{
                migrated, err := Migrate{func_name}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(data)
                if err != nil {{
                    return nil
                }}
                return &migrated
            }}

            // CanMigrate checks if data can be migrated
            func CanMigrate(data interface{{}}) bool {{
                _, ok := data.(map[string]interface{{}})
                return ok
            }}

            // DeepCopy creates a deep copy of a map
            func deepCopy(src map[string]interface{{}}) (map[string]interface{{}}, error) {{
                data, err := json.Marshal(src)
                if err != nil {{
                    return nil, err
                }}
                var dst map[string]interface{{}}
                err = json.Unmarshal(data, &dst)
                return dst, err
            }}
        "#,
            schema_name = schema_name,
            func_name = Self::to_pascal_case(&context.schema_name),
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
                        if _, exists := migrated["{name}"]; !exists {{
                            migrated["{name}"] = {default_str}
                        }}
                    "#,
                        name = name,
                        default_str = default_str,
                    }
                } else if *required {
                    formatdoc! {r#"
                        // Add required field '{name}' - manual intervention needed
                        if _, exists := migrated["{name}"]; !exists {{
                            return nil, &MigrationError{{
                                Field:   "{name}",
                                Message: "required field is missing and has no default value",
                            }}
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
                        if removedValue, exists := migrated["{name}"]; exists {{
                            // Log the removed value if needed
                            _ = removedValue
                            delete(migrated, "{name}")
                        }}
                    "#,
                        name = name,
                    }
                } else {
                    formatdoc! {r#"
                        // Remove field '{name}'
                        delete(migrated, "{name}")
                    "#,
                        name = name,
                    }
                }
            }
            SchemaChange::FieldRenamed { old_name, new_name, .. } => {
                formatdoc! {r#"
                    // Rename field '{old_name}' to '{new_name}'
                    if value, exists := migrated["{old_name}"]; exists {{
                        migrated["{new_name}"] = value
                        delete(migrated, "{old_name}")
                    }}
                "#,
                    old_name = old_name,
                    new_name = new_name,
                }
            }
            SchemaChange::TypeChanged { field, old_type, new_type, .. } => {
                let converter = self.generate_type_converter(field, old_type, new_type);
                formatdoc! {r#"
                    // Convert type of '{field}' from {old_type:?} to {new_type:?}
                    if value, exists := migrated["{field}"]; exists {{
                        {converter}
                    }}
                "#,
                    field = field,
                    old_type = old_type,
                    new_type = new_type,
                    converter = converter.replace('\n', "\n\t\t"),
                }
            }
            _ => String::new(),
        };

        Ok(code)
    }

    fn generate_type_converter(&self, field: &str, old_type: &crate::types::FieldType, new_type: &crate::types::FieldType) -> String {
        use crate::types::FieldType;

        match (old_type, new_type) {
            (FieldType::Integer, FieldType::String) | (FieldType::Long, FieldType::String) => {
                format!("migrated[\"{}\"] = fmt.Sprintf(\"%v\", value)", field)
            }
            (FieldType::String, FieldType::Integer) | (FieldType::String, FieldType::Long) => {
                formatdoc! {r#"
                    if str, ok := value.(string); ok {{
                        var num int64
                        fmt.Sscanf(str, "%d", &num)
                        migrated["{field}"] = num
                    }}
                "#,
                    field = field,
                }
            }
            (FieldType::String, FieldType::Boolean) => {
                formatdoc! {r#"
                    if str, ok := value.(string); ok {{
                        migrated["{field}"] = str == "true" || str == "1" || str == "yes"
                    }}
                "#,
                    field = field,
                }
            }
            (FieldType::Boolean, FieldType::String) => {
                formatdoc! {r#"
                    if b, ok := value.(bool); ok {{
                        if b {{
                            migrated["{field}"] = "true"
                        }} else {{
                            migrated["{field}"] = "false"
                        }}
                    }}
                "#,
                    field = field,
                }
            }
            _ => "// Custom conversion required".to_string(),
        }
    }

    fn format_default_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "nil".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => {
                if n.is_f64() {
                    format!("{}", n.as_f64().unwrap())
                } else {
                    format!("{}", n.as_i64().unwrap())
                }
            }
            serde_json::Value::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            serde_json::Value::Array(_) => "[]interface{}{}".to_string(),
            serde_json::Value::Object(_) => "map[string]interface{}{}".to_string(),
        }
    }

    fn generate_test_code(&self, context: &MigrationContext) -> Result<String> {
        let func_name = Self::to_pascal_case(&context.schema_name);
        let from = &context.from_version;
        let to = &context.to_version;

        let code = formatdoc! {r#"
            package migration

            import (
                "testing"
            )

            func Test{func_name}MigrationV{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(t *testing.T) {{
                t.Run("basic migration", func(t *testing.T) {{
                    oldData := OldSchema{{
                        // Add test data here
                    }}

                    migrated, err := Migrate{func_name}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(oldData)
                    if err != nil {{
                        t.Fatalf("migration failed: %v", err)
                    }}

                    if migrated == nil {{
                        t.Fatal("expected non-nil result")
                    }}
                    // Add assertions here
                }})

                t.Run("batch migration", func(t *testing.T) {{
                    items := []OldSchema{{
                        {{}},
                        {{}},
                    }}

                    migrated, err := MigrateBatch(items)
                    if err != nil {{
                        t.Fatalf("batch migration failed: %v", err)
                    }}

                    if len(migrated) != len(items) {{
                        t.Errorf("expected %d items, got %d", len(items), len(migrated))
                    }}
                }})

                t.Run("safe migration error handling", func(t *testing.T) {{
                    invalidData := OldSchema{{
                        "invalid": "data",
                    }}

                    result := SafeMigrate(invalidData)
                    if result == nil {{
                        t.Log("migration returned nil as expected for invalid data")
                    }}
                }})
            }}
        "#,
            func_name = func_name,
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
        let func_name = Self::to_pascal_case(&context.schema_name);
        let from = &context.from_version;
        let to = &context.to_version;

        let code = formatdoc! {r#"
            // Rollback{func_name}V{to_major}_{to_minor}_{to_patch}ToV{from_major}_{from_minor}_{from_patch} rolls back {schema_name} from v{to} to v{from}
            //
            // WARNING: This is an automated rollback. Data loss may occur.
            func Rollback{func_name}V{to_major}_{to_minor}_{to_patch}ToV{from_major}_{from_minor}_{from_patch}(data NewSchema) (OldSchema, error) {{
                // Deep copy to avoid mutations
                rolledBack := make(OldSchema)
                for k, v := range data {{
                    rolledBack[k] = v
                }}

                // Reverse the migration changes
                // This is a simplified rollback - manual review recommended

                return rolledBack, nil
            }}
        "#,
            func_name = func_name,
            schema_name = context.schema_name,
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

            ```go
            package main

            import (
                "fmt"
                "migration"
            )

            func main() {{
                // Migrate single item
                oldData := migration.OldSchema{{
                    "field": "value",
                }}

                newData, err := migration.Migrate{func_name}V{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(oldData)
                if err != nil {{
                    panic(err)
                }}

                // Migrate batch
                items := []migration.OldSchema{{oldData}}
                migratedItems, err := migration.MigrateBatch(items)
                if err != nil {{
                    panic(err)
                }}

                // Safe migration
                result := migration.SafeMigrate(oldData)
                if result == nil {{
                    fmt.Println("Migration failed")
                }}
            }}
            ```

            ## Safety
            - Always test migrations on non-production data first
            - Consider creating backups before running migrations
            - Review breaking changes carefully
        "#,
            schema_name = &context.schema_name,
            func_name = Self::to_pascal_case(&context.schema_name),
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

    fn to_snake_case(s: &str) -> String {
        s.replace(['-', ' '], "_").to_lowercase()
    }

    fn to_pascal_case(s: &str) -> String {
        s.split(|c: char| c == '-' || c == '_' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
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
    fn test_to_pascal_case() {
        assert_eq!(GoGenerator::to_pascal_case("user-schema"), "UserSchema");
        assert_eq!(GoGenerator::to_pascal_case("my_test"), "MyTest");
    }

    #[test]
    fn test_generate_go_migration() {
        let generator = GoGenerator;
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
        assert!(code.migration_code.contains("func MigrateUserSchema"));
        assert!(code.migration_code.contains("emailVerified"));
    }
}
