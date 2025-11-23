//! Java migration code generator

use crate::error::Result;
use crate::types::{GeneratedCode, MigrationContext, SchemaChange};
use indoc::formatdoc;

/// Java code generator
pub struct JavaGenerator;

impl JavaGenerator {
    /// Generate Java migration code
    pub fn generate(&self, context: &MigrationContext, package_name: Option<&str>) -> Result<GeneratedCode> {
        let package = package_name.unwrap_or("com.example.migration");
        let migration_code = self.generate_migration_class(context, package)?;
        let test_code = Some(self.generate_test_class(context, package)?);
        let documentation = Some(self.generate_documentation(context)?);

        Ok(GeneratedCode {
            migration_code,
            test_code,
            rollback_code: None,
            documentation,
        })
    }

    fn generate_migration_class(&self, context: &MigrationContext, package: &str) -> Result<String> {
        let from = &context.from_version;
        let to = &context.to_version;
        let class_name = Self::to_class_name(&context.schema_name);

        let breaking_count = context.changes.iter().filter(|c| c.is_breaking()).count();
        let non_breaking_count = context.changes.len() - breaking_count;

        let mut transformations = Vec::new();
        for change in &context.changes {
            let code = self.generate_transformation(change)?;
            if !code.is_empty() {
                transformations.push(code);
            }
        }

        let transformations_str = transformations.join("\n        \n        ");

        let code = formatdoc! {r#"
            package {package};

            import java.util.*;
            import java.util.stream.Collectors;

            /**
             * Migration for {schema_name} schema: v{from} → v{to}
             *
             * <p>Breaking changes: {breaking_count}
             * <p>Non-breaking changes: {non_breaking_count}
             *
             * @generated
             */
            public class {class_name}Migration {{

                /**
                 * Migration exception
                 */
                public static class MigrationException extends RuntimeException {{
                    public MigrationException(String message) {{
                        super(message);
                    }}

                    public MigrationException(String message, Throwable cause) {{
                        super(message, cause);
                    }}
                }}

                /**
                 * Migrate {schema_name} from v{from} to v{to}
                 *
                 * @param data the data in old schema format
                 * @return the data in new schema format
                 * @throws MigrationException if migration fails
                 */
                public static Map<String, Object> migrateV{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(
                        Map<String, Object> data) throws MigrationException {{
                    // Deep copy to avoid mutations
                    Map<String, Object> migrated = new HashMap<>(data);

                    {transformations}

                    return migrated;
                }}

                /**
                 * Migrate a batch of items
                 *
                 * @param items list of items to migrate
                 * @return list of migrated items
                 */
                public static List<Map<String, Object>> migrateBatch(
                        List<Map<String, Object>> items) {{
                    return items.stream()
                            .map({class_name}Migration::migrateV{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch})
                            .collect(Collectors.toList());
                }}

                /**
                 * Safely migrate data, returning Optional.empty() if migration fails
                 *
                 * @param data the data to migrate
                 * @return Optional containing migrated data, or empty if migration fails
                 */
                public static Optional<Map<String, Object>> safeMigrate(
                        Map<String, Object> data) {{
                    try {{
                        return Optional.of(migrateV{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(data));
                    }} catch (Exception e) {{
                        System.err.println("Migration failed: " + e.getMessage());
                        return Optional.empty();
                    }}
                }}

                /**
                 * Deep copy a map
                 */
                @SuppressWarnings("unchecked")
                private static Map<String, Object> deepCopy(Map<String, Object> original) {{
                    Map<String, Object> copy = new HashMap<>();
                    for (Map.Entry<String, Object> entry : original.entrySet()) {{
                        Object value = entry.getValue();
                        if (value instanceof Map) {{
                            copy.put(entry.getKey(), deepCopy((Map<String, Object>) value));
                        }} else if (value instanceof List) {{
                            copy.put(entry.getKey(), new ArrayList<>((List<?>) value));
                        }} else {{
                            copy.put(entry.getKey(), value);
                        }}
                    }}
                    return copy;
                }}
            }}
        "#,
            package = package,
            schema_name = &context.schema_name,
            class_name = class_name,
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
                        if (!migrated.containsKey("{name}")) {{
                            migrated.put("{name}", {default_str});
                        }}
                    "#,
                        name = name,
                        default_str = default_str,
                    }
                } else if *required {
                    formatdoc! {r#"
                        // Add required field '{name}' - manual intervention needed
                        if (!migrated.containsKey("{name}")) {{
                            throw new MigrationException(
                                "Required field '{name}' is missing and has no default value");
                        }}
                    "#,
                        name = name,
                    }
                } else {
                    String::new()
                }
            }
            SchemaChange::FieldRemoved { name, field_type: _, preserve_data: _ } => {
                formatdoc! {r#"
                    // Remove field '{name}'
                    migrated.remove("{name}");
                "#,
                    name = name,
                }
            }
            SchemaChange::FieldRenamed { old_name, new_name, .. } => {
                formatdoc! {r#"
                    // Rename field '{old_name}' to '{new_name}'
                    if (migrated.containsKey("{old_name}")) {{
                        migrated.put("{new_name}", migrated.remove("{old_name}"));
                    }}
                "#,
                    old_name = old_name,
                    new_name = new_name,
                }
            }
            SchemaChange::TypeChanged { field, .. } => {
                formatdoc! {r#"
                    // Convert type of '{field}'
                    if (migrated.containsKey("{field}")) {{
                        // Add type conversion logic here
                    }}
                "#,
                    field = field,
                }
            }
            _ => String::new(),
        };

        Ok(code)
    }

    fn format_default_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => {
                if n.is_f64() {
                    format!("{}D", n.as_f64().unwrap())
                } else {
                    format!("{}L", n.as_i64().unwrap())
                }
            }
            serde_json::Value::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            _ => "new HashMap<>()".to_string(),
        }
    }

    fn generate_test_class(&self, context: &MigrationContext, package: &str) -> Result<String> {
        let class_name = Self::to_class_name(&context.schema_name);
        let from = &context.from_version;
        let to = &context.to_version;

        let code = formatdoc! {r#"
            package {package};

            import org.junit.jupiter.api.Test;
            import static org.junit.jupiter.api.Assertions.*;

            import java.util.*;

            class {class_name}MigrationTest {{

                @Test
                void testBasicMigration() {{
                    Map<String, Object> oldData = new HashMap<>();
                    // Add test data here

                    Map<String, Object> migrated =
                        {class_name}Migration.migrateV{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(oldData);

                    assertNotNull(migrated);
                    // Add assertions here
                }}

                @Test
                void testBatchMigration() {{
                    List<Map<String, Object>> items = Arrays.asList(
                        new HashMap<>(),
                        new HashMap<>()
                    );

                    List<Map<String, Object>> migrated =
                        {class_name}Migration.migrateBatch(items);

                    assertEquals(items.size(), migrated.size());
                }}

                @Test
                void testSafeMigration() {{
                    Map<String, Object> invalidData = new HashMap<>();
                    invalidData.put("invalid", "data");

                    Optional<Map<String, Object>> result =
                        {class_name}Migration.safeMigrate(invalidData);

                    assertTrue(result.isPresent());
                }}
            }}
        "#,
            package = package,
            class_name = class_name,
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
            # Java Migration Documentation: {schema_name} v{from} → v{to}

            ## Overview
            - Generated: {generated_at}
            - Changes: {num_changes}
            - Breaking Changes: {breaking_changes}

            ## Changes
            {changes_list}

            ## Usage

            ```java
            import {class_name}Migration;
            import java.util.*;

            public class Example {{
                public static void main(String[] args) {{
                    Map<String, Object> oldData = new HashMap<>();
                    oldData.put("field", "value");

                    // Migrate single item
                    Map<String, Object> newData =
                        {class_name}Migration.migrateV{from_major}_{from_minor}_{from_patch}ToV{to_major}_{to_minor}_{to_patch}(oldData);

                    // Migrate batch
                    List<Map<String, Object>> items = Arrays.asList(oldData);
                    List<Map<String, Object>> migratedItems =
                        {class_name}Migration.migrateBatch(items);

                    // Safe migration
                    Optional<Map<String, Object>> result =
                        {class_name}Migration.safeMigrate(oldData);
                    if (!result.isPresent()) {{
                        System.err.println("Migration failed");
                    }}
                }}
            }}
            ```
        "#,
            schema_name = &context.schema_name,
            class_name = Self::to_class_name(&context.schema_name),
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

    fn to_class_name(s: &str) -> String {
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
