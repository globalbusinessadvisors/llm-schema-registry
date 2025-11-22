//! Schema lifecycle state machine
//!
//! Implements the 11-state lifecycle model from PSEUDOCODE.md

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{Error, Result};

/// Schema lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SchemaState {
    /// Initial creation, not validated
    Draft,
    /// In validation pipeline
    Validating,
    /// Failed structural validation
    ValidationFailed,
    /// Passed validation, checking compatibility
    CompatibilityCheck,
    /// Failed compatibility check
    IncompatibleRejected,
    /// Passed all checks, stored but not active
    Registered,
    /// Currently in use for production
    Active,
    /// Marked for future removal
    Deprecated,
    /// No longer usable, kept for historical record
    Archived,
    /// Discarded without registration
    Abandoned,
    /// In process of reverting to previous version
    RollingBack,
}

impl std::fmt::Display for SchemaState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaState::Draft => write!(f, "DRAFT"),
            SchemaState::Validating => write!(f, "VALIDATING"),
            SchemaState::ValidationFailed => write!(f, "VALIDATION_FAILED"),
            SchemaState::CompatibilityCheck => write!(f, "COMPATIBILITY_CHECK"),
            SchemaState::IncompatibleRejected => write!(f, "INCOMPATIBLE_REJECTED"),
            SchemaState::Registered => write!(f, "REGISTERED"),
            SchemaState::Active => write!(f, "ACTIVE"),
            SchemaState::Deprecated => write!(f, "DEPRECATED"),
            SchemaState::Archived => write!(f, "ARCHIVED"),
            SchemaState::Abandoned => write!(f, "ABANDONED"),
            SchemaState::RollingBack => write!(f, "ROLLING_BACK"),
        }
    }
}

impl SchemaState {
    /// Check if transition to another state is valid
    pub fn can_transition_to(&self, target: SchemaState) -> bool {
        match (self, target) {
            // From Draft
            (SchemaState::Draft, SchemaState::Validating) => true,

            // From Validating
            (SchemaState::Validating, SchemaState::ValidationFailed) => true,
            (SchemaState::Validating, SchemaState::CompatibilityCheck) => true,

            // From ValidationFailed
            (SchemaState::ValidationFailed, SchemaState::Draft) => true,
            (SchemaState::ValidationFailed, SchemaState::Abandoned) => true,

            // From CompatibilityCheck
            (SchemaState::CompatibilityCheck, SchemaState::IncompatibleRejected) => true,
            (SchemaState::CompatibilityCheck, SchemaState::Registered) => true,

            // From IncompatibleRejected
            (SchemaState::IncompatibleRejected, SchemaState::Draft) => true,
            (SchemaState::IncompatibleRejected, SchemaState::Abandoned) => true,

            // From Registered
            (SchemaState::Registered, SchemaState::Active) => true,
            (SchemaState::Registered, SchemaState::Abandoned) => true,

            // From Active
            (SchemaState::Active, SchemaState::Deprecated) => true,
            (SchemaState::Active, SchemaState::Active) => true, // Allow metadata updates
            (SchemaState::Active, SchemaState::RollingBack) => true,

            // From Deprecated
            (SchemaState::Deprecated, SchemaState::Archived) => true,
            (SchemaState::Deprecated, SchemaState::Active) => true, // Reactivation

            // From RollingBack
            (SchemaState::RollingBack, SchemaState::Active) => true,
            (SchemaState::RollingBack, SchemaState::Deprecated) => true,

            // Terminal states
            (SchemaState::Archived, _) => false,
            (SchemaState::Abandoned, _) => false,

            // All other transitions are invalid
            _ => false,
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, SchemaState::Archived | SchemaState::Abandoned)
    }

    /// Check if this is an active state (can be used in production)
    pub fn is_active(&self) -> bool {
        matches!(self, SchemaState::Active)
    }
}

/// State transition record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Source state
    pub from_state: SchemaState,
    /// Target state
    pub to_state: SchemaState,
    /// What triggered this transition
    pub trigger: String,
    /// When the transition occurred
    pub timestamp: DateTime<Utc>,
    /// Who or what triggered the transition
    pub actor: String,
    /// Optional reason for the transition
    pub reason: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl StateTransition {
    /// Create a new state transition
    pub fn new(
        from_state: SchemaState,
        to_state: SchemaState,
        trigger: String,
        actor: String,
    ) -> Self {
        Self {
            from_state,
            to_state,
            trigger,
            timestamp: Utc::now(),
            actor,
            reason: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the reason for this transition
    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }

    /// Add metadata to this transition
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Schema lifecycle tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaLifecycle {
    /// Schema ID
    pub schema_id: Uuid,
    /// Current state
    pub current_state: SchemaState,
    /// State transition history
    pub state_history: Vec<StateTransition>,
    /// When the lifecycle was created
    pub created_at: DateTime<Utc>,
    /// When the lifecycle was last updated
    pub updated_at: DateTime<Utc>,
}

impl SchemaLifecycle {
    /// Create a new lifecycle in Draft state
    pub fn new(schema_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            schema_id,
            current_state: SchemaState::Draft,
            state_history: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Transition to a new state
    pub fn transition(
        &mut self,
        to_state: SchemaState,
        trigger: String,
        actor: String,
    ) -> Result<()> {
        // Check if transition is valid
        if !self.current_state.can_transition_to(to_state) {
            return Err(Error::StateTransitionError(format!(
                "Cannot transition from {} to {}",
                self.current_state, to_state
            )));
        }

        // Record the transition
        let transition = StateTransition::new(
            self.current_state,
            to_state,
            trigger,
            actor,
        );

        self.state_history.push(transition);
        self.current_state = to_state;
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Get the previous state
    pub fn previous_state(&self) -> Option<SchemaState> {
        self.state_history.last().map(|t| t.from_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(SchemaState::Draft.can_transition_to(SchemaState::Validating));
        assert!(SchemaState::Validating.can_transition_to(SchemaState::CompatibilityCheck));
        assert!(SchemaState::CompatibilityCheck.can_transition_to(SchemaState::Registered));
        assert!(SchemaState::Registered.can_transition_to(SchemaState::Active));
        assert!(SchemaState::Active.can_transition_to(SchemaState::Deprecated));
        assert!(SchemaState::Deprecated.can_transition_to(SchemaState::Archived));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!SchemaState::Draft.can_transition_to(SchemaState::Active));
        assert!(!SchemaState::Archived.can_transition_to(SchemaState::Active));
        assert!(!SchemaState::Abandoned.can_transition_to(SchemaState::Draft));
    }

    #[test]
    fn test_terminal_states() {
        assert!(SchemaState::Archived.is_terminal());
        assert!(SchemaState::Abandoned.is_terminal());
        assert!(!SchemaState::Active.is_terminal());
    }

    #[test]
    fn test_lifecycle_transition() {
        let schema_id = Uuid::new_v4();
        let mut lifecycle = SchemaLifecycle::new(schema_id);

        assert_eq!(lifecycle.current_state, SchemaState::Draft);

        // Valid transition
        let result = lifecycle.transition(
            SchemaState::Validating,
            "submit_for_validation".to_string(),
            "user@example.com".to_string(),
        );
        assert!(result.is_ok());
        assert_eq!(lifecycle.current_state, SchemaState::Validating);
        assert_eq!(lifecycle.state_history.len(), 1);

        // Invalid transition
        let result = lifecycle.transition(
            SchemaState::Active,
            "invalid".to_string(),
            "user@example.com".to_string(),
        );
        assert!(result.is_err());
        assert_eq!(lifecycle.current_state, SchemaState::Validating); // State unchanged
    }
}
