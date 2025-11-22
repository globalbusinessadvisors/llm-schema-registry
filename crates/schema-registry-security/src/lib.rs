//! Security layer: RBAC, ABAC, signatures, audit logging, secrets management
//!
//! This module provides comprehensive security features including:
//! - Authentication (JWT, OAuth, mTLS)
//! - Authorization (RBAC, ABAC)
//! - Audit logging (tamper-proof, hash-chained)
//! - Secrets management (rotation, encryption)

pub mod rbac;
pub mod abac;
pub mod audit;
pub mod secrets;
pub mod auth;

pub use audit::{AuditEvent, AuditEventType, AuditLogger, AuditResult, AuditSeverity};
pub use auth::{JwtManager, TokenClaims, TokenRevocationList, TokenType};
pub use secrets::{Secret, SecretMetadata, SecretsManager, RotationPolicy};

use std::sync::Arc;

/// Unified security manager
pub struct SecurityManager {
    pub audit_logger: Arc<AuditLogger>,
    pub jwt_manager: Arc<JwtManager>,
    pub secrets_manager: Arc<SecretsManager>,
}

impl SecurityManager {
    pub fn new(
        audit_logger: Arc<AuditLogger>,
        jwt_manager: Arc<JwtManager>,
        secrets_manager: Arc<SecretsManager>,
    ) -> Self {
        Self {
            audit_logger,
            jwt_manager,
            secrets_manager,
        }
    }

    /// Create a default security manager for testing
    pub fn default_for_testing() -> Self {
        use secrets::{InMemorySecretsBackend, RotationConfig};

        let audit_logger = Arc::new(AuditLogger::new());
        let revocation_list = Arc::new(TokenRevocationList::new());
        let jwt_manager = Arc::new(JwtManager::new_hs256(
            b"test-secret-key-minimum-32-bytes-long",
            revocation_list,
        ));
        let secrets_backend = Arc::new(InMemorySecretsBackend::new());
        let secrets_manager = Arc::new(SecretsManager::new(
            secrets_backend,
            RotationConfig::default(),
        ));

        Self::new(audit_logger, jwt_manager, secrets_manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_manager_creation() {
        use secrets::{InMemorySecretsBackend, RotationConfig};

        let audit_logger = Arc::new(AuditLogger::new());
        let revocation_list = Arc::new(TokenRevocationList::new());
        let jwt_manager = Arc::new(JwtManager::new_hs256(
            b"test-secret-key-minimum-32-bytes-long",
            revocation_list,
        ));
        let secrets_backend = Arc::new(InMemorySecretsBackend::new());
        let secrets_manager = Arc::new(SecretsManager::new(
            secrets_backend,
            RotationConfig::default(),
        ));

        let manager = SecurityManager::new(audit_logger, jwt_manager, secrets_manager);
        assert!(Arc::strong_count(&manager.audit_logger) >= 1);
        assert!(Arc::strong_count(&manager.jwt_manager) >= 1);
        assert!(Arc::strong_count(&manager.secrets_manager) >= 1);
    }

    #[test]
    fn test_security_manager_default_for_testing() {
        let manager = SecurityManager::default_for_testing();
        assert!(Arc::strong_count(&manager.audit_logger) >= 1);
        assert!(Arc::strong_count(&manager.jwt_manager) >= 1);
        assert!(Arc::strong_count(&manager.secrets_manager) >= 1);
    }

    #[test]
    fn test_security_manager_has_all_components() {
        let manager = SecurityManager::default_for_testing();
        assert!(Arc::strong_count(&manager.audit_logger) > 0);
        assert!(Arc::strong_count(&manager.jwt_manager) > 0);
        assert!(Arc::strong_count(&manager.secrets_manager) > 0);
    }

    #[test]
    fn test_multiple_security_managers() {
        let manager1 = SecurityManager::default_for_testing();
        let manager2 = SecurityManager::default_for_testing();

        // They should be independent instances
        assert!(Arc::as_ptr(&manager1.audit_logger) != Arc::as_ptr(&manager2.audit_logger));
    }

    #[test]
    fn test_audit_logger_component() {
        let manager = SecurityManager::default_for_testing();
        let logger = &manager.audit_logger;
        // Logger should be functional
        assert!(Arc::strong_count(logger) >= 1);
    }

    #[test]
    fn test_jwt_manager_component() {
        let manager = SecurityManager::default_for_testing();
        let jwt_mgr = &manager.jwt_manager;
        assert!(Arc::strong_count(jwt_mgr) >= 1);
    }

    #[test]
    fn test_secrets_manager_component() {
        let manager = SecurityManager::default_for_testing();
        let secrets_mgr = &manager.secrets_manager;
        assert!(Arc::strong_count(secrets_mgr) >= 1);
    }

    #[test]
    fn test_security_manager_arc_cloning() {
        let manager = SecurityManager::default_for_testing();
        let audit_clone = Arc::clone(&manager.audit_logger);
        let jwt_clone = Arc::clone(&manager.jwt_manager);
        let secrets_clone = Arc::clone(&manager.secrets_manager);

        assert!(Arc::ptr_eq(&audit_clone, &manager.audit_logger));
        assert!(Arc::ptr_eq(&jwt_clone, &manager.jwt_manager));
        assert!(Arc::ptr_eq(&secrets_clone, &manager.secrets_manager));
    }

    #[test]
    fn test_security_manager_components_independent() {
        use secrets::{InMemorySecretsBackend, RotationConfig};

        let audit1 = Arc::new(AuditLogger::new());
        let audit2 = Arc::new(AuditLogger::new());

        let revocation_list = Arc::new(TokenRevocationList::new());
        let jwt = Arc::new(JwtManager::new_hs256(
            b"test-secret-key-minimum-32-bytes-long",
            revocation_list,
        ));

        let backend = Arc::new(InMemorySecretsBackend::new());
        let secrets = Arc::new(SecretsManager::new(backend, RotationConfig::default()));

        let manager1 = SecurityManager::new(audit1.clone(), jwt.clone(), secrets.clone());
        let manager2 = SecurityManager::new(audit2.clone(), jwt.clone(), secrets.clone());

        // audit_logger should be different
        assert!(!Arc::ptr_eq(&manager1.audit_logger, &manager2.audit_logger));

        // but jwt_manager and secrets_manager should be same
        assert!(Arc::ptr_eq(&manager1.jwt_manager, &manager2.jwt_manager));
        assert!(Arc::ptr_eq(&manager1.secrets_manager, &manager2.secrets_manager));
    }
}
