//! Python migration code generator

use crate::error::{Error, Result};
use crate::types::{GeneratedCode, Language, MigrationContext, SchemaChange};
use indoc::formatdoc;

/// Python code generator
pub struct PythonGenerator;

impl PythonGenerator {
    /// Generate Python migration code
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

        let transformations_str = transformations.join("\n    \n    ");

        let code = formatdoc! {r#"
            from typing import Any, Dict, Optional, List
            from datetime import datetime
            import copy


            def migrate_{schema_name}_v{from_major}_{from_minor}_{from_patch}_to_v{to_major}_{to_minor}_{to_patch}(data: Dict[str, Any]) -> Dict[str, Any]:
                """
                Migrate {schema_name} schema from v{from} to v{to}

                Breaking changes: {breaking_count}
                Non-breaking changes: {non_breaking_count}
                Complexity: {complexity}

                Args:
                    data: Dictionary containing the old schema data

                Returns:
                    Dictionary with migrated data in new schema format

                Raises:
                    ValueError: If data cannot be migrated
                """
                # Create a deep copy to avoid mutating the original
                migrated = copy.deepcopy(data)

                {transformations}

                return migrated


            def migrate_batch(items: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
                """
                Migrate a batch of items

                Args:
                    items: List of dictionaries to migrate

                Returns:
                    List of migrated dictionaries
                """
                return [migrate_{schema_name}_v{from_major}_{from_minor}_{from_patch}_to_v{to_major}_{to_minor}_{to_patch}(item) for item in items]


            def safe_migrate(data: Dict[str, Any]) -> Optional[Dict[str, Any]]:
                """
                Safely migrate data, returning None if migration fails

                Args:
                    data: Dictionary to migrate

                Returns:
                    Migrated dictionary or None if migration fails
                """
                try:
                    return migrate_{schema_name}_v{from_major}_{from_minor}_{from_patch}_to_v{to_major}_{to_minor}_{to_patch}(data)
                except Exception as e:
                    print(f"Migration failed: {{e}}")
                    return None
        "#,
            schema_name = schema_name.to_lowercase().replace("-", "_"),
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
                        # Add field '{name}' with default value
                        if '{name}' not in migrated:
                            migrated['{name}'] = {default_str}
                    "#,
                        name = name,
                        default_str = default_str,
                    }
                } else if *required {
                    formatdoc! {r#"
                        # Add required field '{name}' - manual intervention needed
                        if '{name}' not in migrated:
                            raise ValueError("Required field '{name}' is missing and has no default value")
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
                        # Remove field '{name}' (data preserved in migration log)
                        if '{name}' in migrated:
                            # Log the removed value if needed
                            removed_value = migrated.pop('{name}')
                    "#,
                        name = name,
                    }
                } else {
                    formatdoc! {r#"
                        # Remove field '{name}'
                        migrated.pop('{name}', None)
                    "#,
                        name = name,
                    }
                }
            }
            SchemaChange::FieldRenamed { old_name, new_name, .. } => {
                formatdoc! {r#"
                    # Rename field '{old_name}' to '{new_name}'
                    if '{old_name}' in migrated:
                        migrated['{new_name}'] = migrated.pop('{old_name}')
                "#,
                    old_name = old_name,
                    new_name = new_name,
                }
            }
            SchemaChange::TypeChanged { field, old_type, new_type, .. } => {
                let converter = self.generate_type_converter(old_type, new_type);
                formatdoc! {r#"
                    # Convert type of '{field}' from {old_type:?} to {new_type:?}
                    if '{field}' in migrated:
                        {converter}
                "#,
                    field = field,
                    old_type = old_type,
                    new_type = new_type,
                    converter = converter.replace('\n', "\n        "),
                }
            }
            SchemaChange::ConstraintAdded { field, constraint } => {
                formatdoc! {r#"
                    # Validate constraint {constraint:?} on '{field}'
                    if '{field}' in migrated:
                        # Add validation logic here
                        pass
                "#,
                    field = field,
                    constraint = constraint,
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
                "migrated[field] = str(migrated[field])".to_string()
            }
            (FieldType::String, FieldType::Integer) | (FieldType::String, FieldType::Long) => {
                "migrated[field] = int(migrated[field])".to_string()
            }
            (FieldType::Integer, FieldType::Long) | (FieldType::Float, FieldType::Double) => {
                "# Type widening - no conversion needed\npass".to_string()
            }
            (FieldType::String, FieldType::Boolean) => {
                "migrated[field] = migrated[field].lower() in ('true', '1', 'yes')".to_string()
            }
            (FieldType::Boolean, FieldType::String) => {
                "migrated[field] = 'true' if migrated[field] else 'false'".to_string()
            }
            _ => "# Custom conversion required\npass".to_string(),
        }
    }

    fn format_default_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "None".to_string(),
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
                format!("{{{}}}", items.join(", "))
            }
        }
    }

    fn generate_test_code(&self, context: &MigrationContext) -> Result<String> {
        let from = &context.from_version;
        let to = &context.to_version;
        let schema_name = &context.schema_name;

        let code = formatdoc! {r#"
            import unittest
            from typing import Dict, Any


            class Test{schema_name}Migration(unittest.TestCase):
                """Test cases for {schema_name} migration from v{from} to v{to}"""

                def test_basic_migration(self):
                    """Test basic migration with minimal data"""
                    old_data = {{
                        # Add test data here
                    }}

                    migrated = migrate_{schema_name}_v{from_major}_{from_minor}_{from_patch}_to_v{to_major}_{to_minor}_{to_patch}(old_data)

                    self.assertIsNotNone(migrated)
                    # Add assertions here

                def test_batch_migration(self):
                    """Test batch migration"""
                    items = [
                        {{}},
                        {{}},
                    ]

                    migrated = migrate_batch(items)

                    self.assertEqual(len(migrated), len(items))

                def test_safe_migration_error_handling(self):
                    """Test error handling in safe migration"""
                    invalid_data = {{"invalid": "data"}}

                    result = safe_migrate(invalid_data)

                    # Should handle errors gracefully
                    self.assertIsNotNone(result)


            if __name__ == '__main__':
                unittest.main()
        "#,
            schema_name = schema_name.replace("-", "_"),
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
        let schema_name = &context.schema_name;

        let code = formatdoc! {r#"
            def rollback_{schema_name}_v{to_major}_{to_minor}_{to_patch}_to_v{from_major}_{from_minor}_{from_patch}(data: Dict[str, Any]) -> Dict[str, Any]:
                """
                Rollback {schema_name} schema from v{to} to v{from}

                WARNING: This is an automated rollback. Data loss may occur.

                Args:
                    data: Dictionary containing the new schema data

                Returns:
                    Dictionary with rolled back data in old schema format
                """
                rolled_back = copy.deepcopy(data)

                # Reverse the migration changes
                # This is a simplified rollback - manual review recommended

                return rolled_back
        "#,
            schema_name = schema_name.to_lowercase().replace("-", "_"),
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
            Generated: {generated_at}
            Changes: {num_changes}
            Breaking Changes: {breaking_changes}

            ## Changes
            {changes_list}

            ## Usage

            ```python
            from migration import migrate_{schema_name}_v{from_major}_{from_minor}_{from_patch}_to_v{to_major}_{to_minor}_{to_patch}

            # Migrate single item
            old_data = {{"field": "value"}}
            new_data = migrate_{schema_name}_v{from_major}_{from_minor}_{from_patch}_to_v{to_major}_{to_minor}_{to_patch}(old_data)

            # Migrate batch
            items = [old_data1, old_data2]
            migrated_items = migrate_batch(items)
            ```

            ## Safety
            - Always test migrations on non-production data first
            - Consider creating backups before running migrations
            - Review breaking changes carefully
        "#,
            schema_name = &context.schema_name,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldType, SchemaChange};
    use chrono::Utc;
    use schema_registry_core::versioning::SemanticVersion;

    #[test]
    fn test_generate_python_migration() {
        let generator = PythonGenerator;
        let context = MigrationContext {
            from_version: SemanticVersion::new(1, 0, 0),
            to_version: SemanticVersion::new(2, 0, 0),
            schema_name: "user_schema".to_string(),
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

        let result = generator.generate(&context);
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.migration_code.contains("def migrate_user_schema"));
        assert!(code.migration_code.contains("email_verified"));
    }
}
