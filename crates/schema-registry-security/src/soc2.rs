//! SOC 2 Type II Compliance Controls
//!
//! This module implements comprehensive SOC 2 Type II compliance capabilities
//! for the LLM Schema Registry, covering all five Trust Service Principles:
//!
//! - **Security (CC)**: Common Criteria controls for access control, system security,
//!   change management, and risk mitigation
//! - **Availability**: System monitoring, incident management, and performance tracking
//! - **Processing Integrity**: Data validation, error handling, and transaction processing
//! - **Confidentiality**: Data encryption, classification, and secure disposal
//! - **Privacy**: PII handling, consent management, and data retention
//!
//! The system provides:
//! - Automated evidence collection for auditors
//! - Continuous compliance monitoring
//! - Real-time control effectiveness tracking
//! - Quarterly compliance reporting
//! - Integration with existing security infrastructure (AuditLogger, JwtManager, SecretsManager)

pub mod controls;
pub mod evidence;
pub mod monitoring;
pub mod reporting;
pub mod testing;

pub use controls::{
    AllControls, AvailabilityControls, ConfidentialityControls, ControlStatus,
    ProcessingIntegrityControls, PrivacyControls, SecurityControls,
};
pub use evidence::{
    DateRange, Evidence, EvidenceCollector, EvidenceType, ReportPeriod,
};
pub use monitoring::{
    ComplianceGap, ComplianceMetrics, ComplianceMonitor, ComplianceScore, MetricsCollector,
};
pub use reporting::{
    AvailabilityMetrics, ComplianceReporter, ControlAssertions, EvidenceSummary, Exception,
    RemediationItem, SOC2Report, SecurityMetrics,
};
pub use testing::{ControlTest, ControlTester, TestFrequency, TestResult, TestSchedule};

use thiserror::Error;

/// SOC 2 compliance errors
#[derive(Debug, Error)]
pub enum Soc2Error {
    #[error("Evidence collection failed: {0}")]
    EvidenceCollectionFailed(String),

    #[error("Control test failed: {0}")]
    ControlTestFailed(String),

    #[error("Report generation failed: {0}")]
    ReportGenerationFailed(String),

    #[error("Invalid control ID: {0}")]
    InvalidControlId(String),

    #[error("Missing required evidence: {0}")]
    MissingEvidence(String),

    #[error("Compliance gap detected: {0}")]
    ComplianceGap(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type Result<T> = std::result::Result<T, Soc2Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soc2_module_exports() {
        // Verify all key types are exported
        let _controls: Option<AllControls> = None;
        let _evidence: Option<EvidenceCollector> = None;
        let _monitor: Option<ComplianceMonitor> = None;
        let _reporter: Option<ComplianceReporter> = None;
        let _tester: Option<ControlTester> = None;
    }
}
