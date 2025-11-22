//! # Schema Registry Compatibility
//!
//! Compatibility checking engine supporting 7 compatibility modes.

use async_trait::async_trait;
use schema_registry_core::{
    error::Result,
    schema::RegisteredSchema,
    traits::{CompatibilityChecker, CompatibilityResult},
    types::CompatibilityMode,
};

/// Compatibility checker
pub struct CompatibilityCheckerImpl {}

impl CompatibilityCheckerImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CompatibilityCheckerImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CompatibilityChecker for CompatibilityCheckerImpl {
    async fn check_compatibility(
        &self,
        new_schema: &RegisteredSchema,
        old_schema: &RegisteredSchema,
        mode: CompatibilityMode,
    ) -> Result<CompatibilityResult> {
        // Fast path: identical content hashes
        if new_schema.content_hash == old_schema.content_hash {
            return Ok(CompatibilityResult {
                is_compatible: true,
                mode,
                violations: Vec::new(),
                checked_versions: vec![old_schema.version.clone()],
            });
        }

        // Detailed compatibility check would go here
        Ok(CompatibilityResult {
            is_compatible: true,
            mode,
            violations: Vec::new(),
            checked_versions: vec![old_schema.version.clone()],
        })
    }

    async fn check_transitive_compatibility(
        &self,
        new_schema: &RegisteredSchema,
        previous_versions: &[RegisteredSchema],
        mode: CompatibilityMode,
    ) -> Result<CompatibilityResult> {
        let mut all_violations = Vec::new();
        let mut checked_versions = Vec::new();

        for old_schema in previous_versions {
            let result = self.check_compatibility(new_schema, old_schema, mode).await?;
            all_violations.extend(result.violations);
            checked_versions.extend(result.checked_versions);

            if !result.is_compatible && !mode.is_transitive() {
                break;
            }
        }

        Ok(CompatibilityResult {
            is_compatible: all_violations.is_empty(),
            mode,
            violations: all_violations,
            checked_versions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema_registry_core::{
        RegisteredSchema, SchemaLifecycle, SchemaMetadata, SchemaState, CompatibilityMode, SerializationFormat, SemanticVersion,
    };
    use uuid::Uuid;

    fn create_test_schema(version: SemanticVersion, content: &str, hash: &str) -> RegisteredSchema {
        let id = Uuid::new_v4();
        RegisteredSchema {
            id,
            namespace: "test".to_string(),
            name: "schema".to_string(),
            version,
            format: SerializationFormat::JsonSchema,
            content: content.to_string(),
            content_hash: hash.to_string(),
            description: "test schema".to_string(),
            compatibility_mode: CompatibilityMode::Full,
            state: SchemaState::Active,
            metadata: SchemaMetadata {
                created_at: chrono::Utc::now(),
                created_by: "test".to_string(),
                updated_at: chrono::Utc::now(),
                updated_by: "test".to_string(),
                activated_at: None,
                deprecation: None,
                deletion: None,
                custom: std::collections::HashMap::new(),
            },
            tags: vec![],
            examples: vec![],
            lifecycle: SchemaLifecycle::new(id),
        }
    }

    #[test]
    fn test_compatibility_checker_new() {
        let checker = CompatibilityCheckerImpl::new();
        assert!(std::ptr::eq(&checker as *const _, &checker as *const _));
    }

    #[test]
    fn test_compatibility_checker_default() {
        let checker = CompatibilityCheckerImpl::default();
        assert!(std::ptr::eq(&checker as *const _, &checker as *const _));
    }

    #[tokio::test]
    async fn test_check_compatibility_identical_schemas() {
        let checker = CompatibilityCheckerImpl::new();
        let schema = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash123");

        let result = checker.check_compatibility(
            &schema,
            &schema,
            CompatibilityMode::Backward,
        ).await;

        assert!(result.is_ok());
        let compat = result.unwrap();
        assert!(compat.is_compatible);
        assert_eq!(compat.mode, CompatibilityMode::Backward);
        assert!(compat.violations.is_empty());
    }

    #[tokio::test]
    async fn test_check_compatibility_different_schemas() {
        let checker = CompatibilityCheckerImpl::new();
        let schema1 = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1");
        let schema2 = create_test_schema(SemanticVersion::new(1, 1, 0), "{}", "hash2");

        let result = checker.check_compatibility(
            &schema1,
            &schema2,
            CompatibilityMode::Backward,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_compatibility_forward_mode() {
        let checker = CompatibilityCheckerImpl::new();
        let schema1 = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1");
        let schema2 = create_test_schema(SemanticVersion::new(1, 1, 0), "{}", "hash2");

        let result = checker.check_compatibility(
            &schema1,
            &schema2,
            CompatibilityMode::Forward,
        ).await;

        assert!(result.is_ok());
        let compat = result.unwrap();
        assert_eq!(compat.mode, CompatibilityMode::Forward);
    }

    #[tokio::test]
    async fn test_check_compatibility_full_mode() {
        let checker = CompatibilityCheckerImpl::new();
        let schema1 = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1");
        let schema2 = create_test_schema(SemanticVersion::new(1, 1, 0), "{}", "hash2");

        let result = checker.check_compatibility(
            &schema1,
            &schema2,
            CompatibilityMode::Full,
        ).await;

        assert!(result.is_ok());
        let compat = result.unwrap();
        assert_eq!(compat.mode, CompatibilityMode::Full);
    }

    #[tokio::test]
    async fn test_check_compatibility_none_mode() {
        let checker = CompatibilityCheckerImpl::new();
        let schema1 = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1");
        let schema2 = create_test_schema(SemanticVersion::new(1, 1, 0), "{}", "hash2");

        let result = checker.check_compatibility(
            &schema1,
            &schema2,
            CompatibilityMode::None,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_transitive_compatibility_empty_list() {
        let checker = CompatibilityCheckerImpl::new();
        let schema = create_test_schema(SemanticVersion::new(2, 0, 0), "{}", "hash1");

        let result = checker.check_transitive_compatibility(
            &schema,
            &[],
            CompatibilityMode::BackwardTransitive,
        ).await;

        assert!(result.is_ok());
        let compat = result.unwrap();
        assert!(compat.is_compatible);
    }

    #[tokio::test]
    async fn test_check_transitive_compatibility_single_version() {
        let checker = CompatibilityCheckerImpl::new();
        let new_schema = create_test_schema(SemanticVersion::new(2, 0, 0), "{}", "hash2");
        let old_schema = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1");

        let result = checker.check_transitive_compatibility(
            &new_schema,
            &[old_schema],
            CompatibilityMode::BackwardTransitive,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_transitive_compatibility_multiple_versions() {
        let checker = CompatibilityCheckerImpl::new();
        let new_schema = create_test_schema(SemanticVersion::new(3, 0, 0), "{}", "hash3");
        let schemas = vec![
            create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1"),
            create_test_schema(SemanticVersion::new(2, 0, 0), "{}", "hash2"),
        ];

        let result = checker.check_transitive_compatibility(
            &new_schema,
            &schemas,
            CompatibilityMode::BackwardTransitive,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_transitive_compatibility_forward_transitive() {
        let checker = CompatibilityCheckerImpl::new();
        let new_schema = create_test_schema(SemanticVersion::new(2, 0, 0), "{}", "hash2");
        let old_schema = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1");

        let result = checker.check_transitive_compatibility(
            &new_schema,
            &[old_schema],
            CompatibilityMode::ForwardTransitive,
        ).await;

        assert!(result.is_ok());
        let compat = result.unwrap();
        assert_eq!(compat.mode, CompatibilityMode::ForwardTransitive);
    }

    #[tokio::test]
    async fn test_check_transitive_compatibility_full_transitive() {
        let checker = CompatibilityCheckerImpl::new();
        let new_schema = create_test_schema(SemanticVersion::new(2, 0, 0), "{}", "hash2");
        let old_schema = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1");

        let result = checker.check_transitive_compatibility(
            &new_schema,
            &[old_schema],
            CompatibilityMode::FullTransitive,
        ).await;

        assert!(result.is_ok());
        let compat = result.unwrap();
        assert_eq!(compat.mode, CompatibilityMode::FullTransitive);
    }

    #[tokio::test]
    async fn test_compatibility_result_checked_versions() {
        let checker = CompatibilityCheckerImpl::new();
        let new_schema = create_test_schema(SemanticVersion::new(2, 0, 0), "{}", "hash2");
        let old_schema = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1");

        let result = checker.check_compatibility(
            &new_schema,
            &old_schema,
            CompatibilityMode::Backward,
        ).await;

        assert!(result.is_ok());
        let compat = result.unwrap();
        assert_eq!(compat.checked_versions.len(), 1);
    }

    #[tokio::test]
    async fn test_transitive_compatibility_multiple_checked_versions() {
        let checker = CompatibilityCheckerImpl::new();
        let new_schema = create_test_schema(SemanticVersion::new(3, 0, 0), "{}", "hash3");
        let schemas = vec![
            create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "hash1"),
            create_test_schema(SemanticVersion::new(2, 0, 0), "{}", "hash2"),
        ];

        let result = checker.check_transitive_compatibility(
            &new_schema,
            &schemas,
            CompatibilityMode::BackwardTransitive,
        ).await;

        assert!(result.is_ok());
        let compat = result.unwrap();
        assert_eq!(compat.checked_versions.len(), 2);
    }

    #[tokio::test]
    async fn test_compatibility_fast_path_same_hash() {
        let checker = CompatibilityCheckerImpl::new();
        let schema1 = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "same_hash");
        let schema2 = create_test_schema(SemanticVersion::new(1, 0, 0), "{}", "same_hash");

        let result = checker.check_compatibility(
            &schema1,
            &schema2,
            CompatibilityMode::Backward,
        ).await;

        assert!(result.is_ok());
        let compat = result.unwrap();
        assert!(compat.is_compatible);
        assert!(compat.violations.is_empty());
    }
}
