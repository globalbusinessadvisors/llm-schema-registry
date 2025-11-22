//! Query types for schema search and filtering

use schema_registry_core::{SchemaType, SchemaState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Search query for schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Pattern to match against subject names (supports wildcards)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_pattern: Option<String>,

    /// Filter by schema type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<SchemaType>,

    /// Filter by tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Filter by owner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// Filter by state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<SchemaState>,

    /// Custom metadata filters (JSONB queries)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_filter: Option<HashMap<String, serde_json::Value>>,

    /// Maximum number of results
    #[serde(default = "default_limit")]
    pub limit: Option<i64>,

    /// Offset for pagination
    #[serde(default)]
    pub offset: Option<i64>,

    /// Sort order
    #[serde(default)]
    pub sort_by: SortBy,
}

fn default_limit() -> Option<i64> {
    Some(100)
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            subject_pattern: None,
            schema_type: None,
            tags: None,
            owner: None,
            state: None,
            metadata_filter: None,
            limit: default_limit(),
            offset: None,
            sort_by: SortBy::default(),
        }
    }
}

/// Sort order for search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    CreatedAtAsc,
    CreatedAtDesc,
    SubjectAsc,
    SubjectDesc,
    VersionAsc,
    VersionDesc,
}

impl Default for SortBy {
    fn default() -> Self {
        Self::CreatedAtDesc
    }
}

/// Schema filter for version queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaFilter {
    /// Include deleted schemas
    #[serde(default)]
    pub include_deleted: bool,

    /// Only return schemas in specific states
    #[serde(skip_serializing_if = "Option::is_none")]
    pub states: Option<Vec<SchemaState>>,
}

impl Default for SchemaFilter {
    fn default() -> Self {
        Self {
            include_deleted: false,
            states: None,
        }
    }
}
