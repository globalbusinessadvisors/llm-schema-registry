//! Evidence Collection for SOC 2 Audits
//!
//! This module provides automated evidence collection capabilities for
//! SOC 2 Type II audits. It integrates with the existing audit logger
//! to extract compliance-relevant evidence.

use crate::audit::{AuditEvent, AuditEventFilter, AuditEventType, AuditLogger};
use crate::soc2::{Result, Soc2Error};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

// =============================================================================
// Date Range and Periods
// =============================================================================

/// Date range for evidence collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl DateRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    pub fn last_30_days() -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::days(30);
        Self { start, end }
    }

    pub fn last_90_days() -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::days(90);
        Self { start, end }
    }

    pub fn last_year() -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::days(365);
        Self { start, end }
    }

    pub fn to_unix_range(&self) -> (u64, u64) {
        (
            self.start.timestamp() as u64,
            self.end.timestamp() as u64,
        )
    }
}

/// Reporting period for compliance reports
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReportPeriod {
    Monthly,
    Quarterly,
    Annual,
    Custom,
}

// =============================================================================
// Evidence Types
// =============================================================================

/// Types of evidence that can be collected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceType {
    // Access Control Evidence
    UserAccessList,
    AccessChangeLog,
    MfaEnrollmentReport,
    PasswordChangeHistory,
    SessionManagementLogs,

    // Authentication Evidence
    LoginAttempts,
    FailedLoginAttempts,
    SuccessfulLogins,
    TokenGenerationLog,
    TokenRevocationLog,

    // Authorization Evidence
    RoleAssignments,
    PermissionMatrix,
    AccessReviews,
    PrivilegedAccessLog,
    AuthorizationDenials,

    // System Operations Evidence
    ChangeRequests,
    DeploymentHistory,
    PatchHistory,
    ConfigurationChanges,
    SystemRestarts,

    // Monitoring Evidence
    SecurityAlerts,
    SecurityViolations,
    PerformanceMetrics,
    AvailabilityMetrics,
    ErrorRates,

    // Backup and Recovery Evidence
    BackupReports,
    BackupVerification,
    RestoreTests,
    DisasterRecoveryTests,

    // Data Protection Evidence
    EncryptionReport,
    DataRetentionReport,
    DataDeletionLog,
    DataAccessLog,
    DataExportLog,

    // Incident Management Evidence
    SecurityIncidents,
    IncidentResponses,
    PostIncidentReviews,
    IncidentEscalations,

    // Schema-Specific Evidence
    SchemaRegistrations,
    SchemaUpdates,
    SchemaValidations,
    SchemaDeletions,
    SchemaAccessLog,
}

impl EvidenceType {
    /// Get the audit event types that correspond to this evidence type
    pub fn audit_event_types(&self) -> Vec<AuditEventType> {
        match self {
            EvidenceType::LoginAttempts => vec![
                AuditEventType::AuthenticationSuccess,
                AuditEventType::AuthenticationFailure,
            ],
            EvidenceType::FailedLoginAttempts => vec![AuditEventType::AuthenticationFailure],
            EvidenceType::SuccessfulLogins => vec![AuditEventType::AuthenticationSuccess],
            EvidenceType::TokenGenerationLog => vec![AuditEventType::TokenGenerated],
            EvidenceType::TokenRevocationLog => vec![AuditEventType::TokenRevoked],
            EvidenceType::RoleAssignments => vec![
                AuditEventType::RoleAssigned,
                AuditEventType::RoleRevoked,
            ],
            EvidenceType::PermissionMatrix => vec![
                AuditEventType::PermissionGranted,
                AuditEventType::PermissionRevoked,
            ],
            EvidenceType::AuthorizationDenials => vec![AuditEventType::AuthorizationDenied],
            EvidenceType::ChangeRequests => vec![AuditEventType::ConfigurationChanged],
            EvidenceType::SecurityAlerts => vec![
                AuditEventType::SecurityViolation,
                AuditEventType::SuspiciousActivity,
            ],
            EvidenceType::SecurityViolations => vec![AuditEventType::SecurityViolation],
            EvidenceType::BackupReports => vec![AuditEventType::BackupCreated],
            EvidenceType::SchemaRegistrations => vec![AuditEventType::SchemaRegistered],
            EvidenceType::SchemaUpdates => vec![AuditEventType::SchemaUpdated],
            EvidenceType::SchemaDeletions => vec![AuditEventType::SchemaDeleted],
            EvidenceType::PasswordChangeHistory => vec![AuditEventType::PasswordChanged],
            _ => vec![],
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &str {
        match self {
            EvidenceType::UserAccessList => "List of all users and their access levels",
            EvidenceType::AccessChangeLog => "History of access permission changes",
            EvidenceType::MfaEnrollmentReport => "Multi-factor authentication enrollment status",
            EvidenceType::LoginAttempts => "All login attempts (successful and failed)",
            EvidenceType::FailedLoginAttempts => "Failed login attempts for security monitoring",
            EvidenceType::RoleAssignments => "RBAC role assignment history",
            EvidenceType::SecurityAlerts => "Security incidents and alerts",
            EvidenceType::BackupReports => "Backup execution and verification logs",
            EvidenceType::EncryptionReport => "Encryption status of data at rest and in transit",
            EvidenceType::DataDeletionLog => "PII and sensitive data deletion requests",
            _ => "Compliance evidence",
        }
    }
}

// =============================================================================
// Evidence Structure
// =============================================================================

/// A piece of evidence for compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub evidence_type: EvidenceType,
    pub collected_at: DateTime<Utc>,
    pub date_range: DateRange,
    pub summary: EvidenceSummary,
    pub events: Vec<AuditEvent>,
    pub metrics: HashMap<String, serde_json::Value>,
}

impl Evidence {
    pub fn new(
        evidence_type: EvidenceType,
        date_range: DateRange,
        events: Vec<AuditEvent>,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let summary = EvidenceSummary {
            total_events: events.len(),
            unique_users: Self::count_unique_users(&events),
            unique_resources: Self::count_unique_resources(&events),
        };

        Self {
            id,
            evidence_type,
            collected_at: Utc::now(),
            date_range,
            summary,
            events,
            metrics: HashMap::new(),
        }
    }

    fn count_unique_users(events: &[AuditEvent]) -> usize {
        let mut users = std::collections::HashSet::new();
        for event in events {
            if let Some(user_id) = &event.user_id {
                users.insert(user_id.clone());
            }
        }
        users.len()
    }

    fn count_unique_resources(events: &[AuditEvent]) -> usize {
        let mut resources = std::collections::HashSet::new();
        for event in events {
            if let Some(resource_id) = &event.resource_id {
                resources.insert(resource_id.clone());
            }
        }
        resources.len()
    }

    pub fn with_metric(mut self, key: String, value: serde_json::Value) -> Self {
        self.metrics.insert(key, value);
        self
    }
}

/// Summary statistics for evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub total_events: usize,
    pub unique_users: usize,
    pub unique_resources: usize,
}

// =============================================================================
// Evidence Collector
// =============================================================================

/// Collects evidence from audit logs for compliance reporting
pub struct EvidenceCollector {
    audit_logger: Arc<AuditLogger>,
}

impl EvidenceCollector {
    pub fn new(audit_logger: Arc<AuditLogger>) -> Self {
        Self { audit_logger }
    }

    /// Collect evidence for a specific type and date range
    pub async fn collect_evidence(
        &self,
        evidence_type: EvidenceType,
        date_range: DateRange,
    ) -> Result<Evidence> {
        let (start, end) = date_range.to_unix_range();

        // Build filter for audit events
        let mut filter = AuditEventFilter {
            start_time: Some(start),
            end_time: Some(end),
            ..Default::default()
        };

        // Add event type filters if applicable
        let event_types = evidence_type.audit_event_types();
        if !event_types.is_empty() {
            filter.event_types = Some(event_types);
        }

        // Fetch events from audit logger
        let events = self.audit_logger.get_events(filter).await;

        Ok(Evidence::new(evidence_type, date_range, events))
    }

    /// Collect all access control evidence
    pub async fn collect_access_control_evidence(
        &self,
        date_range: DateRange,
    ) -> Result<Vec<Evidence>> {
        let mut evidence = Vec::new();

        evidence.push(
            self.collect_evidence(EvidenceType::LoginAttempts, date_range.clone())
                .await?,
        );
        evidence.push(
            self.collect_evidence(EvidenceType::FailedLoginAttempts, date_range.clone())
                .await?,
        );
        evidence.push(
            self.collect_evidence(EvidenceType::RoleAssignments, date_range.clone())
                .await?,
        );
        evidence.push(
            self.collect_evidence(EvidenceType::PermissionMatrix, date_range.clone())
                .await?,
        );

        Ok(evidence)
    }

    /// Collect all security monitoring evidence
    pub async fn collect_security_evidence(
        &self,
        date_range: DateRange,
    ) -> Result<Vec<Evidence>> {
        let mut evidence = Vec::new();

        evidence.push(
            self.collect_evidence(EvidenceType::SecurityAlerts, date_range.clone())
                .await?,
        );
        evidence.push(
            self.collect_evidence(EvidenceType::SecurityViolations, date_range.clone())
                .await?,
        );
        evidence.push(
            self.collect_evidence(EvidenceType::AuthorizationDenials, date_range.clone())
                .await?,
        );

        Ok(evidence)
    }

    /// Collect schema-specific evidence
    pub async fn collect_schema_evidence(
        &self,
        date_range: DateRange,
    ) -> Result<Vec<Evidence>> {
        let mut evidence = Vec::new();

        evidence.push(
            self.collect_evidence(EvidenceType::SchemaRegistrations, date_range.clone())
                .await?,
        );
        evidence.push(
            self.collect_evidence(EvidenceType::SchemaUpdates, date_range.clone())
                .await?,
        );
        evidence.push(
            self.collect_evidence(EvidenceType::SchemaDeletions, date_range.clone())
                .await?,
        );

        Ok(evidence)
    }

    /// Generate a comprehensive compliance report for a period
    pub async fn generate_compliance_package(
        &self,
        period: ReportPeriod,
    ) -> Result<CompliancePackage> {
        let date_range = match period {
            ReportPeriod::Monthly => DateRange::last_30_days(),
            ReportPeriod::Quarterly => DateRange::last_90_days(),
            ReportPeriod::Annual => DateRange::last_year(),
            ReportPeriod::Custom => DateRange::last_90_days(),
        };

        let mut all_evidence = Vec::new();

        // Collect all types of evidence
        all_evidence.extend(self.collect_access_control_evidence(date_range.clone()).await?);
        all_evidence.extend(self.collect_security_evidence(date_range.clone()).await?);
        all_evidence.extend(self.collect_schema_evidence(date_range.clone()).await?);

        Ok(CompliancePackage {
            id: uuid::Uuid::new_v4().to_string(),
            period,
            date_range,
            generated_at: Utc::now(),
            evidence: all_evidence,
        })
    }

    /// Export evidence package to JSON file
    pub async fn export_evidence_package(
        &self,
        package: &CompliancePackage,
        output_path: &Path,
    ) -> Result<()> {
        let json = serde_json::to_string_pretty(package)
            .map_err(|e| Soc2Error::SerializationError(e))?;

        tokio::fs::write(output_path, json)
            .await
            .map_err(|e| Soc2Error::IoError(e))?;

        Ok(())
    }

    /// Calculate compliance metrics from evidence
    pub fn calculate_compliance_metrics(
        &self,
        evidence: &[Evidence],
    ) -> ComplianceMetrics {
        let total_events: usize = evidence.iter().map(|e| e.events.len()).sum();
        let total_users: usize = evidence.iter().map(|e| e.summary.unique_users).max().unwrap_or(0);

        let security_violations = evidence
            .iter()
            .filter(|e| matches!(e.evidence_type, EvidenceType::SecurityViolations))
            .map(|e| e.events.len())
            .sum();

        let failed_logins = evidence
            .iter()
            .filter(|e| matches!(e.evidence_type, EvidenceType::FailedLoginAttempts))
            .map(|e| e.events.len())
            .sum();

        ComplianceMetrics {
            total_events,
            total_users,
            security_violations,
            failed_logins,
            evidence_items: evidence.len(),
        }
    }
}

/// Complete compliance evidence package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePackage {
    pub id: String,
    pub period: ReportPeriod,
    pub date_range: DateRange,
    pub generated_at: DateTime<Utc>,
    pub evidence: Vec<Evidence>,
}

/// Compliance metrics calculated from evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceMetrics {
    pub total_events: usize,
    pub total_users: usize,
    pub security_violations: usize,
    pub failed_logins: usize,
    pub evidence_items: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_range_last_30_days() {
        let range = DateRange::last_30_days();
        let duration = range.end.signed_duration_since(range.start);
        assert_eq!(duration.num_days(), 30);
    }

    #[test]
    fn test_evidence_type_description() {
        let evidence_type = EvidenceType::LoginAttempts;
        assert!(!evidence_type.description().is_empty());
    }

    #[test]
    fn test_evidence_creation() {
        let date_range = DateRange::last_30_days();
        let events = vec![];
        let evidence = Evidence::new(EvidenceType::LoginAttempts, date_range, events);
        assert_eq!(evidence.summary.total_events, 0);
    }

    #[tokio::test]
    async fn test_evidence_collector_creation() {
        let audit_logger = Arc::new(AuditLogger::new());
        let collector = EvidenceCollector::new(audit_logger);

        let date_range = DateRange::last_30_days();
        let evidence = collector
            .collect_evidence(EvidenceType::LoginAttempts, date_range)
            .await
            .unwrap();

        assert_eq!(evidence.events.len(), 0);
    }

    #[tokio::test]
    async fn test_compliance_package_generation() {
        let audit_logger = Arc::new(AuditLogger::new());
        let collector = EvidenceCollector::new(audit_logger);

        let package = collector
            .generate_compliance_package(ReportPeriod::Monthly)
            .await
            .unwrap();

        assert!(!package.id.is_empty());
    }
}
