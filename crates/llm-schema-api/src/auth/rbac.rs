// Role-Based Access Control (RBAC) implementation

use super::{AuthError, AuthPrincipal, Permission};
use std::collections::HashMap;

pub struct RbacEnforcer {
    role_permissions: HashMap<String, Vec<Permission>>,
}

impl RbacEnforcer {
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();

        // Define role permissions
        role_permissions.insert(
            "admin".to_string(),
            vec![
                Permission::SchemaRead,
                Permission::SchemaWrite,
                Permission::SchemaDelete,
                Permission::SchemaValidate,
                Permission::SubjectRead,
                Permission::SubjectWrite,
                Permission::SubjectDelete,
                Permission::CompatibilityCheck,
                Permission::CompatibilityConfigRead,
                Permission::CompatibilityConfigWrite,
                Permission::AdminAccess,
                Permission::UserManagement,
                Permission::ConfigManagement,
                Permission::MetricsRead,
                Permission::HealthCheck,
            ],
        );

        role_permissions.insert(
            "developer".to_string(),
            vec![
                Permission::SchemaRead,
                Permission::SchemaWrite,
                Permission::SchemaValidate,
                Permission::SubjectRead,
                Permission::CompatibilityCheck,
                Permission::MetricsRead,
                Permission::HealthCheck,
            ],
        );

        role_permissions.insert(
            "reader".to_string(),
            vec![
                Permission::SchemaRead,
                Permission::SubjectRead,
                Permission::SchemaValidate,
                Permission::HealthCheck,
            ],
        );

        role_permissions.insert(
            "service".to_string(),
            vec![
                Permission::SchemaRead,
                Permission::SchemaValidate,
                Permission::CompatibilityCheck,
                Permission::HealthCheck,
            ],
        );

        Self { role_permissions }
    }

    pub fn check_permission(
        &self,
        principal: &AuthPrincipal,
        permission: &Permission,
    ) -> Result<(), AuthError> {
        if principal.has_permission(permission) {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions)
        }
    }

    pub fn require_any_permission(
        &self,
        principal: &AuthPrincipal,
        permissions: &[Permission],
    ) -> Result<(), AuthError> {
        if permissions.iter().any(|p| principal.has_permission(p)) {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions)
        }
    }

    pub fn require_all_permissions(
        &self,
        principal: &AuthPrincipal,
        permissions: &[Permission],
    ) -> Result<(), AuthError> {
        if permissions.iter().all(|p| principal.has_permission(p)) {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions)
        }
    }
}

impl Default for RbacEnforcer {
    fn default() -> Self {
        Self::new()
    }
}
