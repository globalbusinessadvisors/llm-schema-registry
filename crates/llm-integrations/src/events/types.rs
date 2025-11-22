// Event types for schema changes

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Schema change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEvent {
    /// Event ID
    pub event_id: Uuid,

    /// Event type
    pub event_type: SchemaEventType,

    /// Schema ID
    pub schema_id: Uuid,

    /// Schema namespace
    pub namespace: String,

    /// Schema name
    pub name: String,

    /// Schema version
    pub version: String,

    /// Previous version (for updates)
    pub previous_version: Option<String>,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Schema event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaEventType {
    /// Schema was registered (new)
    Registered,

    /// Schema was updated
    Updated,

    /// Schema was deprecated
    Deprecated,

    /// Schema was deleted
    Deleted,

    /// Compatibility violation detected
    CompatibilityViolated,
}

impl SchemaEvent {
    /// Create a new schema registered event
    pub fn registered(
        schema_id: Uuid,
        namespace: String,
        name: String,
        version: String,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type: SchemaEventType::Registered,
            schema_id,
            namespace,
            name,
            version,
            previous_version: None,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }

    /// Create a new schema updated event
    pub fn updated(
        schema_id: Uuid,
        namespace: String,
        name: String,
        version: String,
        previous_version: String,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type: SchemaEventType::Updated,
            schema_id,
            namespace,
            name,
            version,
            previous_version: Some(previous_version),
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }

    /// Create a new schema deprecated event
    pub fn deprecated(
        schema_id: Uuid,
        namespace: String,
        name: String,
        version: String,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type: SchemaEventType::Deprecated,
            schema_id,
            namespace,
            name,
            version,
            previous_version: None,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }

    /// Create a new compatibility violated event
    pub fn compatibility_violated(
        schema_id: Uuid,
        namespace: String,
        name: String,
        version: String,
        violations: Vec<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type: SchemaEventType::CompatibilityViolated,
            schema_id,
            namespace,
            name,
            version,
            previous_version: None,
            timestamp: Utc::now(),
            metadata: serde_json::json!({
                "violations": violations
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_event_registered() {
        let event = SchemaEvent::registered(
            Uuid::new_v4(),
            "com.example".to_string(),
            "User".to_string(),
            "1.0.0".to_string(),
        );

        assert_eq!(event.event_type, SchemaEventType::Registered);
        assert_eq!(event.namespace, "com.example");
        assert_eq!(event.name, "User");
        assert_eq!(event.version, "1.0.0");
        assert!(event.previous_version.is_none());
    }

    #[test]
    fn test_schema_event_updated() {
        let event = SchemaEvent::updated(
            Uuid::new_v4(),
            "com.example".to_string(),
            "User".to_string(),
            "2.0.0".to_string(),
            "1.0.0".to_string(),
        );

        assert_eq!(event.event_type, SchemaEventType::Updated);
        assert_eq!(event.version, "2.0.0");
        assert_eq!(event.previous_version, Some("1.0.0".to_string()));
    }
}
