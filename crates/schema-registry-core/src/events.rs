//! Event system for schema registry
//!
//! Implements event sourcing and pub/sub for schema lifecycle events

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::schema::{DeprecationInfo, SchemaReference};
use crate::versioning::SemanticVersion;

/// Event types for schema lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    /// Schema was created
    SchemaCreated,
    /// Schema passed validation
    SchemaValidated,
    /// Schema was registered
    SchemaRegistered,
    /// Schema was activated
    SchemaActivated,
    /// Schema was deprecated
    SchemaDeprecated,
    /// Schema was archived
    SchemaArchived,
    /// Schema was deleted
    SchemaDeleted,
    /// Schema was rolled back
    SchemaRolledBack,
    /// Compatibility check failed
    CompatibilityCheckFailed,
    /// Validation failed
    ValidationFailed,
    /// Consumer registered for schema
    ConsumerRegistered,
    /// Consumer unregistered from schema
    ConsumerUnregistered,
    /// Usage threshold exceeded
    UsageThresholdExceeded,
}

/// Base schema event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEvent {
    /// Unique event ID
    pub event_id: Uuid,
    /// Event type
    pub event_type: EventType,
    /// Schema ID this event relates to
    pub schema_id: Uuid,
    /// Schema version
    pub schema_version: SemanticVersion,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Actor who triggered the event (user ID, service name, etc.)
    pub actor: String,
    /// Correlation ID for tracing related events
    pub correlation_id: Uuid,
    /// Event payload
    pub payload: EventPayload,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SchemaEvent {
    /// Create a new schema event
    pub fn new(
        event_type: EventType,
        schema_id: Uuid,
        schema_version: SemanticVersion,
        actor: String,
        payload: EventPayload,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type,
            schema_id,
            schema_version,
            timestamp: Utc::now(),
            actor,
            correlation_id: Uuid::new_v4(),
            payload,
            metadata: HashMap::new(),
        }
    }

    /// Set a correlation ID for this event
    pub fn with_correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = correlation_id;
        self
    }

    /// Add metadata to this event
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Event payload (event-specific data)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    /// Schema registered event
    SchemaRegistered {
        /// Schema name
        schema_name: String,
        /// Namespace
        namespace: String,
        /// Validation result
        validation_result: Option<serde_json::Value>,
        /// Compatibility result
        compatibility_result: Option<serde_json::Value>,
    },

    /// Schema deprecated event
    SchemaDeprecated {
        /// Deprecation info
        deprecation_info: DeprecationInfo,
        /// Dependent schemas
        dependents: Vec<SchemaReference>,
    },

    /// Compatibility check failed event
    CompatibilityCheckFailed {
        /// Previous version that was checked
        previous_version: SemanticVersion,
        /// Violations found
        violations: Vec<serde_json::Value>,
        /// Compatibility mode
        mode: String,
    },

    /// Validation failed event
    ValidationFailed {
        /// Validation errors
        errors: Vec<serde_json::Value>,
    },

    /// Consumer registered event
    ConsumerRegistered {
        /// Consumer ID
        consumer_id: Uuid,
        /// Consumer name
        consumer_name: String,
        /// Subscription type (producer, consumer, both)
        subscription_type: String,
    },

    /// Consumer unregistered event
    ConsumerUnregistered {
        /// Consumer ID
        consumer_id: Uuid,
    },

    /// Generic event payload
    Generic {
        /// Event data
        data: HashMap<String, serde_json::Value>,
    },
}

/// Event subscription for receiving schema events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    /// Subscription ID
    pub subscription_id: Uuid,
    /// Subscriber ID
    pub subscriber_id: String,
    /// Event types to receive
    pub event_types: Vec<EventType>,
    /// Filters to apply
    pub filters: HashMap<String, serde_json::Value>,
    /// Delivery method
    pub delivery_method: DeliveryMethod,
    /// Whether subscription is active
    pub active: bool,
}

/// Event delivery methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeliveryMethod {
    /// HTTP webhook
    Webhook {
        /// Webhook URL
        url: String,
        /// HTTP headers to include
        headers: HashMap<String, String>,
    },
    /// Email notification
    Email {
        /// Email address
        address: String,
    },
    /// Kafka consumer group
    Kafka {
        /// Consumer group ID
        consumer_group: String,
    },
    /// WebSocket connection
    WebSocket {
        /// Connection ID
        connection_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_event() {
        let schema_id = Uuid::new_v4();
        let version = SemanticVersion::new(1, 0, 0);
        let payload = EventPayload::SchemaRegistered {
            schema_name: "TestSchema".to_string(),
            namespace: "test".to_string(),
            validation_result: None,
            compatibility_result: None,
        };

        let event = SchemaEvent::new(
            EventType::SchemaRegistered,
            schema_id,
            version,
            "test@example.com".to_string(),
            payload,
        );

        assert_eq!(event.event_type, EventType::SchemaRegistered);
        assert_eq!(event.schema_id, schema_id);
        assert!(event.metadata.is_empty());
    }

    #[test]
    fn test_event_with_metadata() {
        let schema_id = Uuid::new_v4();
        let version = SemanticVersion::new(1, 0, 0);
        let payload = EventPayload::Generic {
            data: HashMap::new(),
        };

        let event = SchemaEvent::new(
            EventType::SchemaCreated,
            schema_id,
            version,
            "test@example.com".to_string(),
            payload,
        )
        .with_metadata("key".to_string(), serde_json::json!("value"));

        assert_eq!(event.metadata.len(), 1);
        assert_eq!(event.metadata.get("key").unwrap(), &serde_json::json!("value"));
    }
}
