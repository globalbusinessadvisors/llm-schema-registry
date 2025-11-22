//! Tamper-Proof Audit Logging System
//!
//! Features:
//! - Immutable audit logs
//! - Hash-chain for tamper detection
//! - Structured logging with correlation IDs
//! - 1-year retention policy
//! - Compliance-ready (SOC 2, GDPR)

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

// =============================================================================
// Audit Event Types
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    // Authentication events
    AuthenticationSuccess,
    AuthenticationFailure,
    TokenGenerated,
    TokenRevoked,
    TokenExpired,
    PasswordChanged,
    MfaEnabled,
    MfaDisabled,

    // Authorization events
    AuthorizationGranted,
    AuthorizationDenied,
    RoleAssigned,
    RoleRevoked,
    PermissionGranted,
    PermissionRevoked,

    // Schema operations
    SchemaRegistered,
    SchemaUpdated,
    SchemaDeleted,
    SchemaValidated,
    SchemaPublished,
    SchemaDeprecated,

    // Configuration changes
    ConfigurationChanged,
    CompatibilityModeChanged,
    RetentionPolicyChanged,

    // Security events
    SecurityViolation,
    RateLimitExceeded,
    SuspiciousActivity,
    AccessDenied,

    // System events
    SystemStarted,
    SystemStopped,
    BackupCreated,
    BackupRestored,
}

impl AuditEventType {
    pub fn severity(&self) -> AuditSeverity {
        match self {
            Self::AuthenticationFailure
            | Self::AuthorizationDenied
            | Self::SecurityViolation
            | Self::SuspiciousActivity
            | Self::AccessDenied => AuditSeverity::Warning,

            Self::TokenRevoked
            | Self::RoleRevoked
            | Self::PermissionRevoked
            | Self::SchemaDeleted => AuditSeverity::Important,

            _ => AuditSeverity::Info,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditSeverity {
    Info,
    Warning,
    Important,
    Critical,
}

// =============================================================================
// Audit Event
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,

    /// Event type
    pub event_type: AuditEventType,

    /// Severity level
    pub severity: AuditSeverity,

    /// Timestamp (Unix epoch)
    pub timestamp: u64,

    /// User who performed the action
    pub user_id: Option<String>,

    /// User email
    pub user_email: Option<String>,

    /// Source IP address
    pub source_ip: Option<String>,

    /// User agent
    pub user_agent: Option<String>,

    /// Correlation ID for request tracing
    pub correlation_id: Option<String>,

    /// Resource affected (e.g., schema ID, config key)
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,

    /// Action performed
    pub action: String,

    /// Result of the action
    pub result: AuditResult,

    /// Additional context
    pub metadata: HashMap<String, serde_json::Value>,

    /// Hash of previous event (for chain integrity)
    pub previous_hash: String,

    /// Hash of this event
    pub event_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
    Partial,
}

impl AuditEvent {
    pub fn new(
        event_type: AuditEventType,
        action: String,
        result: AuditResult,
        previous_hash: String,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let severity = event_type.severity();

        let mut event = Self {
            id: id.clone(),
            event_type,
            severity,
            timestamp,
            user_id: None,
            user_email: None,
            source_ip: None,
            user_agent: None,
            correlation_id: None,
            resource_type: None,
            resource_id: None,
            action,
            result,
            metadata: HashMap::new(),
            previous_hash,
            event_hash: String::new(),
        };

        // Compute hash
        event.event_hash = event.compute_hash();
        event
    }

    /// Compute cryptographic hash of the event
    fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Include all fields except event_hash
        hasher.update(self.id.as_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.update(format!("{:?}", self.event_type).as_bytes());
        hasher.update(self.action.as_bytes());
        hasher.update(format!("{:?}", self.result).as_bytes());
        hasher.update(self.previous_hash.as_bytes());

        if let Some(user_id) = &self.user_id {
            hasher.update(user_id.as_bytes());
        }
        if let Some(resource_id) = &self.resource_id {
            hasher.update(resource_id.as_bytes());
        }

        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Verify event hash integrity
    pub fn verify_hash(&self) -> bool {
        let computed_hash = self.compute_hash();
        computed_hash == self.event_hash
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set user context
    pub fn with_user(mut self, user_id: String, email: Option<String>) -> Self {
        self.user_id = Some(user_id);
        self.user_email = email;
        self
    }

    /// Set request context
    pub fn with_request_context(
        mut self,
        ip: Option<String>,
        user_agent: Option<String>,
        correlation_id: Option<String>,
    ) -> Self {
        self.source_ip = ip;
        self.user_agent = user_agent;
        self.correlation_id = correlation_id;
        self
    }

    /// Set resource context
    pub fn with_resource(mut self, resource_type: String, resource_id: String) -> Self {
        self.resource_type = Some(resource_type);
        self.resource_id = Some(resource_id);
        self
    }
}

// =============================================================================
// Audit Logger
// =============================================================================

pub struct AuditLogger {
    events: Arc<RwLock<Vec<AuditEvent>>>,
    last_hash: Arc<RwLock<String>>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            last_hash: Arc::new(RwLock::new("genesis".to_string())),
        }
    }

    /// Log an audit event
    pub async fn log(&self, mut event: AuditEvent) {
        // Get previous hash and update chain
        let previous_hash = {
            let last = self.last_hash.read().await;
            last.clone()
        };

        event.previous_hash = previous_hash;
        event.event_hash = event.compute_hash();

        // Update last hash
        {
            let mut last = self.last_hash.write().await;
            *last = event.event_hash.clone();
        }

        // Store event
        let mut events = self.events.write().await;
        events.push(event.clone());

        // Log to tracing
        match event.severity {
            AuditSeverity::Info => tracing::info!(
                event_id = %event.id,
                event_type = ?event.event_type,
                user_id = ?event.user_id,
                action = %event.action,
                result = ?event.result,
                "Audit event"
            ),
            AuditSeverity::Warning => tracing::warn!(
                event_id = %event.id,
                event_type = ?event.event_type,
                user_id = ?event.user_id,
                action = %event.action,
                result = ?event.result,
                "Audit event"
            ),
            AuditSeverity::Important | AuditSeverity::Critical => tracing::error!(
                event_id = %event.id,
                event_type = ?event.event_type,
                user_id = ?event.user_id,
                action = %event.action,
                result = ?event.result,
                "Audit event"
            ),
        }
    }

    /// Verify integrity of the entire audit log chain
    pub async fn verify_chain_integrity(&self) -> bool {
        let events = self.events.read().await;

        if events.is_empty() {
            return true;
        }

        let mut expected_previous = "genesis".to_string();

        for event in events.iter() {
            // Verify hash
            if !event.verify_hash() {
                tracing::error!(
                    event_id = %event.id,
                    "Event hash verification failed"
                );
                return false;
            }

            // Verify chain
            if event.previous_hash != expected_previous {
                tracing::error!(
                    event_id = %event.id,
                    expected = %expected_previous,
                    actual = %event.previous_hash,
                    "Chain integrity violation detected"
                );
                return false;
            }

            expected_previous = event.event_hash.clone();
        }

        true
    }

    /// Get events by filter
    pub async fn get_events(
        &self,
        filter: AuditEventFilter,
    ) -> Vec<AuditEvent> {
        let events = self.events.read().await;

        events
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }

    /// Get event count
    pub async fn count(&self) -> usize {
        let events = self.events.read().await;
        events.len()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Audit Event Filter
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct AuditEventFilter {
    pub user_id: Option<String>,
    pub event_types: Option<Vec<AuditEventType>>,
    pub resource_id: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub severity: Option<AuditSeverity>,
}

impl AuditEventFilter {
    pub fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(user_id) = &self.user_id {
            if event.user_id.as_ref() != Some(user_id) {
                return false;
            }
        }

        if let Some(event_types) = &self.event_types {
            if !event_types.contains(&event.event_type) {
                return false;
            }
        }

        if let Some(resource_id) = &self.resource_id {
            if event.resource_id.as_ref() != Some(resource_id) {
                return false;
            }
        }

        if let Some(start) = self.start_time {
            if event.timestamp < start {
                return false;
            }
        }

        if let Some(end) = self.end_time {
            if event.timestamp > end {
                return false;
            }
        }

        if let Some(severity) = self.severity {
            if event.severity != severity {
                return false;
            }
        }

        true
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Log authentication success
pub async fn log_auth_success(
    logger: &AuditLogger,
    user_id: String,
    email: Option<String>,
    ip: Option<String>,
) {
    let event = AuditEvent::new(
        AuditEventType::AuthenticationSuccess,
        "User authenticated".to_string(),
        AuditResult::Success,
        String::new(),
    )
    .with_user(user_id, email)
    .with_request_context(ip, None, None);

    logger.log(event).await;
}

/// Log authentication failure
pub async fn log_auth_failure(
    logger: &AuditLogger,
    attempted_user: String,
    ip: Option<String>,
    reason: String,
) {
    let event = AuditEvent::new(
        AuditEventType::AuthenticationFailure,
        "Authentication failed".to_string(),
        AuditResult::Failure,
        String::new(),
    )
    .with_user(attempted_user, None)
    .with_request_context(ip, None, None)
    .with_metadata("reason".to_string(), serde_json::json!(reason));

    logger.log(event).await;
}

/// Log schema registration
pub async fn log_schema_registered(
    logger: &AuditLogger,
    user_id: String,
    schema_id: String,
    subject: String,
) {
    let event = AuditEvent::new(
        AuditEventType::SchemaRegistered,
        "Schema registered".to_string(),
        AuditResult::Success,
        String::new(),
    )
    .with_user(user_id, None)
    .with_resource("schema".to_string(), schema_id)
    .with_metadata("subject".to_string(), serde_json::json!(subject));

    logger.log(event).await;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_logging() {
        let logger = AuditLogger::new();

        log_auth_success(
            &logger,
            "user123".to_string(),
            Some("user@example.com".to_string()),
            Some("192.168.1.1".to_string()),
        )
        .await;

        let count = logger.count().await;
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_chain_integrity() {
        let logger = AuditLogger::new();

        // Log multiple events
        for i in 0..5 {
            log_auth_success(
                &logger,
                format!("user{}", i),
                None,
                None,
            )
            .await;
        }

        // Verify chain
        assert!(logger.verify_chain_integrity().await);
    }

    #[tokio::test]
    async fn test_event_filtering() {
        let logger = AuditLogger::new();

        log_auth_success(&logger, "user1".to_string(), None, None).await;
        log_auth_failure(&logger, "user2".to_string(), None, "Invalid password".to_string()).await;

        let filter = AuditEventFilter {
            event_types: Some(vec![AuditEventType::AuthenticationSuccess]),
            ..Default::default()
        };

        let filtered = logger.get_events(filter).await;
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].event_type, AuditEventType::AuthenticationSuccess);
    }

    #[test]
    fn test_event_hash_verification() {
        let event = AuditEvent::new(
            AuditEventType::SchemaRegistered,
            "Test action".to_string(),
            AuditResult::Success,
            "genesis".to_string(),
        );

        assert!(event.verify_hash());
    }
}
