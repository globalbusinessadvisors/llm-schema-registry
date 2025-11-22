pub mod jwt;
pub mod oauth;
pub mod api_key;
pub mod rbac;
pub mod middleware;

pub use jwt::*;
pub use oauth::*;
pub use api_key::*;
pub use rbac::*;
pub use middleware::*;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

// ============================================================================
// Authentication Principal
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthPrincipal {
    pub user_id: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub permissions: HashSet<Permission>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl AuthPrincipal {
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        self.roles.iter().any(|r| roles.contains(&r.as_str()))
    }
}

// ============================================================================
// Permissions
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // Schema operations
    SchemaRead,
    SchemaWrite,
    SchemaDelete,
    SchemaValidate,

    // Subject operations
    SubjectRead,
    SubjectWrite,
    SubjectDelete,

    // Compatibility operations
    CompatibilityCheck,
    CompatibilityConfigRead,
    CompatibilityConfigWrite,

    // Administration
    AdminAccess,
    UserManagement,
    ConfigManagement,

    // Metrics and monitoring
    MetricsRead,
    HealthCheck,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::SchemaRead => "schema:read",
            Permission::SchemaWrite => "schema:write",
            Permission::SchemaDelete => "schema:delete",
            Permission::SchemaValidate => "schema:validate",
            Permission::SubjectRead => "subject:read",
            Permission::SubjectWrite => "subject:write",
            Permission::SubjectDelete => "subject:delete",
            Permission::CompatibilityCheck => "compatibility:check",
            Permission::CompatibilityConfigRead => "compatibility:config:read",
            Permission::CompatibilityConfigWrite => "compatibility:config:write",
            Permission::AdminAccess => "admin:access",
            Permission::UserManagement => "admin:users",
            Permission::ConfigManagement => "admin:config",
            Permission::MetricsRead => "metrics:read",
            Permission::HealthCheck => "health:check",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "schema:read" => Some(Permission::SchemaRead),
            "schema:write" => Some(Permission::SchemaWrite),
            "schema:delete" => Some(Permission::SchemaDelete),
            "schema:validate" => Some(Permission::SchemaValidate),
            "subject:read" => Some(Permission::SubjectRead),
            "subject:write" => Some(Permission::SubjectWrite),
            "subject:delete" => Some(Permission::SubjectDelete),
            "compatibility:check" => Some(Permission::CompatibilityCheck),
            "compatibility:config:read" => Some(Permission::CompatibilityConfigRead),
            "compatibility:config:write" => Some(Permission::CompatibilityConfigWrite),
            "admin:access" => Some(Permission::AdminAccess),
            "admin:users" => Some(Permission::UserManagement),
            "admin:config" => Some(Permission::ConfigManagement),
            "metrics:read" => Some(Permission::MetricsRead),
            "health:check" => Some(Permission::HealthCheck),
            _ => None,
        }
    }
}

// ============================================================================
// Role Definitions
// ============================================================================

pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
}

impl Role {
    pub fn admin() -> Self {
        Self {
            name: "admin".to_string(),
            permissions: HashSet::from([
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
            ]),
        }
    }

    pub fn developer() -> Self {
        Self {
            name: "developer".to_string(),
            permissions: HashSet::from([
                Permission::SchemaRead,
                Permission::SchemaWrite,
                Permission::SchemaValidate,
                Permission::SubjectRead,
                Permission::CompatibilityCheck,
                Permission::MetricsRead,
                Permission::HealthCheck,
            ]),
        }
    }

    pub fn reader() -> Self {
        Self {
            name: "reader".to_string(),
            permissions: HashSet::from([
                Permission::SchemaRead,
                Permission::SubjectRead,
                Permission::SchemaValidate,
                Permission::HealthCheck,
            ]),
        }
    }

    pub fn service() -> Self {
        Self {
            name: "service".to_string(),
            permissions: HashSet::from([
                Permission::SchemaRead,
                Permission::SchemaValidate,
                Permission::CompatibilityCheck,
                Permission::HealthCheck,
            ]),
        }
    }
}

// ============================================================================
// Authentication Error
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Missing authentication")]
    MissingAuth,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("OAuth error: {0}")]
    OAuthError(String),

    #[error("Internal authentication error: {0}")]
    InternalError(String),
}
