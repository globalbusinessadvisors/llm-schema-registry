# SOC 2 Type II Compliance System - Usage Guide

## Overview

The LLM Schema Registry now includes a comprehensive SOC 2 Type II compliance system implementing all five Trust Service Principles:

1. **Security (CC)** - Common Criteria controls
2. **Availability** - System uptime and incident management
3. **Processing Integrity** - Data accuracy and completeness
4. **Confidentiality** - Data protection and encryption
5. **Privacy** - PII handling and consent management

## Quick Start

### Basic Usage

```rust
use schema_registry_security::SecurityManager;

// Create security manager with SOC 2 compliance
let security_manager = SecurityManager::default_with_compliance_for_testing();

// Access compliance components
let monitor = security_manager.compliance_monitor.unwrap();
let evidence_collector = security_manager.evidence_collector.unwrap();
let reporter = security_manager.compliance_reporter.unwrap();
```

### Production Setup

```rust
use schema_registry_security::{
    AuditLogger, JwtManager, SecretsManager, SecurityManager,
    TokenRevocationList,
};
use std::sync::Arc;

// Initialize security components
let audit_logger = Arc::new(AuditLogger::new());
let revocation_list = Arc::new(TokenRevocationList::new());
let jwt_manager = Arc::new(JwtManager::new_hs256(
    b"your-secret-key-minimum-32-bytes",
    revocation_list,
));
let secrets_manager = Arc::new(SecretsManager::new(
    secrets_backend,
    rotation_config,
));

// Create security manager with full compliance
let security_manager = SecurityManager::with_compliance(
    audit_logger,
    jwt_manager,
    secrets_manager,
);
```

## Features

### 1. Control Management

The system implements **108+ controls** across all Trust Service Principles:

- **52 Security Controls** (CC6.1 - CC7.5)
- **15 Availability Controls** (A1.1 - A1.4)
- **12 Processing Integrity Controls** (PI1.1 - PI1.4)
- **12 Confidentiality Controls** (C1.1 - C1.4)
- **17 Privacy Controls** (P1 - P8)

```rust
use schema_registry_security::AllControls;

let controls = AllControls::new();

// Check control status
println!("Security controls: {}", controls.security.count_implemented());
println!("Total controls: {}", controls.total_controls());
```

### 2. Evidence Collection

Automatically collect evidence for compliance audits:

```rust
use schema_registry_security::{EvidenceCollector, DateRange, ReportPeriod};

let collector = evidence_collector.as_ref().unwrap();

// Collect specific evidence
let login_evidence = collector
    .collect_evidence(
        EvidenceType::LoginAttempts,
        DateRange::last_30_days()
    )
    .await?;

// Generate comprehensive compliance package
let package = collector
    .generate_compliance_package(ReportPeriod::Quarterly)
    .await?;

// Export for auditors
collector
    .export_evidence_package(&package, Path::new("./compliance_package.json"))
    .await?;
```

### 3. Compliance Monitoring

Real-time monitoring of compliance status:

```rust
use schema_registry_security::ComplianceMonitor;

let monitor = compliance_monitor.as_ref().unwrap();

// Get current compliance metrics
let metrics = monitor.get_current_metrics().await;
println!("Compliance rate: {:.1}%", metrics.compliance_percentage());
println!("Audit ready: {}", metrics.is_audit_ready());

// Calculate compliance score
let score = monitor.calculate_compliance_score().await;
println!("Overall score: {:.1}", score.overall_score);
println!("Security score: {:.1}", score.security_score);

// Identify compliance gaps
let gaps = monitor.identify_gaps().await;
for gap in gaps {
    println!("Gap: {} - {}", gap.control_id, gap.description);
}

// Generate health report
let health = monitor.health_report().await;
println!("Health status: {:?}", health.overall_health);
```

### 4. Compliance Reporting

Generate auditor-ready SOC 2 reports:

```rust
use schema_registry_security::{ComplianceReporter, ReportPeriod};

let reporter = compliance_reporter.as_ref().unwrap();

// Generate comprehensive SOC 2 report
let report = reporter
    .generate_soc2_report(
        "Your Organization".to_string(),
        "Schema Registry Service".to_string(),
        ReportPeriod::Quarterly,
    )
    .await?;

// Export report
reporter
    .export_to_json(&report, Path::new("./soc2_report.json"))
    .await?;

// Generate summary for executives
let summary = reporter.generate_compliance_summary().await?;
println!("Compliance: {:.1}%", summary.overall_compliance_rate);
println!("Critical gaps: {}", summary.critical_gaps);
```

### 5. Control Testing

Automated testing of controls:

```rust
use schema_registry_security::{ControlTester, TestFrequency};

let controls = Arc::new(RwLock::new(AllControls::new()));
let tester = ControlTester::new(controls);

// Initialize standard test definitions
tester.initialize_standard_tests().await;

// Run specific control test
let result = tester.run_control_test("CC6.1-MFA").await?;
println!("Test passed: {}", result.passed);
println!("Score: {:.1}", result.score);

// Run all automated tests
let results = tester.automated_control_tests().await;
println!("Success rate: {:.1}%", tester.get_success_rate().await);

// Generate test schedule
let schedule = tester.schedule_control_tests().await;
println!("Due tests: {}", schedule.get_due_tests().len());
```

## Control Coverage

### Security Controls (CC6.x - CC7.x)

- **CC6.1**: Access control policies, MFA, password policies
- **CC6.2**: User registration and identity verification
- **CC6.3**: Access removal and reviews
- **CC6.5**: RBAC/ABAC, least privilege, segregation of duties
- **CC6.6**: Authentication mechanisms and credential management
- **CC6.7**: Change management, patching, backups
- **CC6.8**: Change approval and testing
- **CC7.1**: Security monitoring and intrusion detection
- **CC7.2**: Incident response and documentation
- **CC7.3**: Malware protection and vulnerability scanning
- **CC7.4**: Network security and encryption in transit
- **CC7.5**: Encryption at rest and key management

### Availability Controls (A1.x)

- **A1.1**: SLA definitions and uptime monitoring
- **A1.2**: Performance and capacity monitoring
- **A1.3**: Incident management and escalation
- **A1.4**: Backup, recovery, and failover procedures

### Processing Integrity Controls (PI1.x)

- **PI1.1**: Input validation and error handling
- **PI1.2**: Data completeness and reconciliation
- **PI1.3**: Data validation and audit trails
- **PI1.4**: Approval workflows and authorization

### Confidentiality Controls (C1.x)

- **C1.1**: Data encryption at rest and in transit
- **C1.2**: Data classification and labeling
- **C1.3**: Access restrictions and data masking
- **C1.4**: Secure disposal and deletion verification

### Privacy Controls (P1 - P8)

- **P1**: Privacy notices and data collection disclosure
- **P2**: Consent management and opt-out mechanisms
- **P3**: Collection limitation and purpose specification
- **P4**: Data retention policies and secure disposal
- **P5**: Data subject access and correction
- **P6**: Third-party agreements and disclosure logging
- **P7**: PII encryption and access controls
- **P8**: Data accuracy and completeness

## Evidence Types

The system automatically collects the following evidence:

### Access Control Evidence
- User access lists
- Access change logs
- MFA enrollment reports
- Password change history
- Session management logs

### Authentication Evidence
- Login attempts (successful and failed)
- Token generation and revocation logs

### Authorization Evidence
- Role assignments
- Permission matrices
- Access reviews
- Authorization denials

### System Operations Evidence
- Change requests and deployment history
- Patch history and configuration changes
- System restarts

### Monitoring Evidence
- Security alerts and violations
- Performance and availability metrics
- Error rates

### Data Protection Evidence
- Encryption reports
- Data retention and deletion logs
- Data access and export logs

### Schema-Specific Evidence
- Schema registrations, updates, validations
- Schema deletions and access logs

## Integration with Existing Security

The SOC 2 system integrates seamlessly with existing security infrastructure:

- **AuditLogger**: Used for evidence collection and tamper-proof logging
- **JwtManager**: Provides authentication evidence
- **SecretsManager**: Demonstrates key management controls
- **RBAC/ABAC**: Implements access control requirements

## Audit Readiness

The system calculates an audit readiness score based on:

1. Control implementation percentage
2. Evidence collection completeness
3. Control test results
4. Compliance gap severity
5. Policy violation trends

**Audit Ready Criteria:**
- Audit readiness score ≥ 90%
- No missing critical controls
- No expired evidence
- All high-severity gaps remediated

## Reporting Periods

Generate reports for different compliance periods:

- **Monthly**: Last 30 days
- **Quarterly**: Last 90 days (standard for SOC 2 Type II)
- **Annual**: Last 365 days
- **Custom**: Define your own date range

## Best Practices

1. **Regular Evidence Collection**: Run evidence collection daily
2. **Control Testing**: Test critical controls monthly, others quarterly
3. **Gap Remediation**: Address high/critical gaps within 30 days
4. **Policy Reviews**: Review and update policies annually
5. **Incident Documentation**: Log all security incidents immediately
6. **Access Reviews**: Conduct quarterly access reviews
7. **Backup Testing**: Test backups and recovery monthly

## Compliance Metrics

Key metrics tracked by the system:

- **Control Effectiveness Rate**: Percentage of controls operating effectively
- **Evidence Collection Rate**: Completeness of evidence
- **Policy Violations**: Trend over time
- **Security Incidents**: Count and severity
- **Availability Metrics**: Uptime percentage, MTTR
- **Audit Readiness Score**: Overall preparedness for audit

## File Structure

```
crates/schema-registry-security/src/soc2/
├── controls.rs      - All 108+ control definitions
├── evidence.rs      - Evidence collection system
├── monitoring.rs    - Real-time compliance monitoring
├── reporting.rs     - SOC 2 report generation
└── testing.rs       - Control testing framework
```

## API Documentation

Full API documentation available via:

```bash
cargo doc --package schema-registry-security --open
```

## Support

For questions or issues related to SOC 2 compliance:

1. Review this guide
2. Check the inline code documentation
3. Run the comprehensive test suite
4. Generate a health report to identify specific issues

## License

Apache 2.0 - See LICENSE file for details
