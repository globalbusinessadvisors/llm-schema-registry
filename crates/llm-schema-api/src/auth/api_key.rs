use super::{AuthError, AuthPrincipal, Permission, Role};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// API Key
// ============================================================================

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub key_id: String,
    pub key_hash: String,
    pub name: String,
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: HashSet<Permission>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
}

impl ApiKey {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < chrono::Utc::now()
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        self.enabled && !self.is_expired()
    }

    pub fn to_principal(&self) -> AuthPrincipal {
        AuthPrincipal {
            user_id: self.user_id.clone(),
            email: None,
            roles: self.roles.clone(),
            permissions: self.permissions.clone(),
            metadata: HashMap::from([
                ("key_id".to_string(), self.key_id.clone()),
                ("key_name".to_string(), self.name.clone()),
            ]),
        }
    }
}

// ============================================================================
// API Key Manager
// ============================================================================

pub struct ApiKeyManager {
    keys: Arc<RwLock<HashMap<String, ApiKey>>>,
}

impl ApiKeyManager {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_key(
        &self,
        name: String,
        user_id: String,
        roles: Vec<String>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> (String, ApiKey) {
        // Generate random API key
        let key_bytes: [u8; 32] = rand::random();
        let api_key = format!(
            "llmsr_{}",
            base64::encode_config(key_bytes, base64::URL_SAFE_NO_PAD)
        );

        // Hash the key for storage
        let key_hash = Self::hash_key(&api_key);

        // Resolve permissions from roles
        let permissions = Self::resolve_permissions(&roles);

        let key_id = uuid::Uuid::new_v4().to_string();

        let key_info = ApiKey {
            key_id: key_id.clone(),
            key_hash,
            name,
            user_id,
            roles,
            permissions,
            created_at: chrono::Utc::now(),
            expires_at,
            last_used: None,
            enabled: true,
        };

        self.keys.write().await.insert(key_id, key_info.clone());

        (api_key, key_info)
    }

    pub async fn verify_key(&self, api_key: &str) -> Result<AuthPrincipal, AuthError> {
        let key_hash = Self::hash_key(api_key);

        let mut keys = self.keys.write().await;

        // Find key by hash
        let key_info = keys
            .values_mut()
            .find(|k| k.key_hash == key_hash)
            .ok_or(AuthError::InvalidApiKey)?;

        if !key_info.is_valid() {
            return Err(AuthError::InvalidApiKey);
        }

        // Update last used timestamp
        key_info.last_used = Some(chrono::Utc::now());

        Ok(key_info.to_principal())
    }

    pub async fn revoke_key(&self, key_id: &str) -> Result<(), AuthError> {
        let mut keys = self.keys.write().await;

        let key = keys
            .get_mut(key_id)
            .ok_or(AuthError::InvalidApiKey)?;

        key.enabled = false;
        Ok(())
    }

    pub async fn list_keys_for_user(&self, user_id: &str) -> Vec<ApiKey> {
        let keys = self.keys.read().await;
        keys.values()
            .filter(|k| k.user_id == user_id)
            .cloned()
            .collect()
    }

    fn hash_key(api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn resolve_permissions(roles: &[String]) -> HashSet<Permission> {
        let mut permissions = HashSet::new();

        for role_name in roles {
            let role = match role_name.as_str() {
                "admin" => Role::admin(),
                "developer" => Role::developer(),
                "reader" => Role::reader(),
                "service" => Role::service(),
                _ => continue,
            };

            permissions.extend(role.permissions);
        }

        permissions
    }
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// API Key Extractor
// ============================================================================

pub fn extract_api_key(value: &str) -> Option<&str> {
    if value.starts_with("llmsr_") {
        Some(value)
    } else {
        value
            .strip_prefix("ApiKey ")
            .or_else(|| value.strip_prefix("apikey "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_key_lifecycle() {
        let manager = ApiKeyManager::new();

        // Create key
        let (api_key, key_info) = manager
            .create_key(
                "Test Key".to_string(),
                "user123".to_string(),
                vec!["developer".to_string()],
                None,
            )
            .await;

        assert!(api_key.starts_with("llmsr_"));
        assert!(key_info.is_valid());

        // Verify key
        let principal = manager.verify_key(&api_key).await.unwrap();
        assert_eq!(principal.user_id, "user123");
        assert!(!principal.permissions.is_empty());

        // Revoke key
        manager.revoke_key(&key_info.key_id).await.unwrap();

        // Should fail after revocation
        assert!(manager.verify_key(&api_key).await.is_err());
    }
}
