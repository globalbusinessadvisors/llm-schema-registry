//! Enhanced Authentication Module
//!
//! Features:
//! - JWT with RS256/HS256 support
//! - Token revocation list
//! - Refresh token support
//! - mTLS client certificate validation

use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

// =============================================================================
// Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Token revoked")]
    TokenRevoked,

    #[error("Missing authentication")]
    MissingAuth,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type Result<T> = std::result::Result<T, AuthError>;

// =============================================================================
// Token Claims
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Email
    pub email: Option<String>,
    /// Roles
    pub roles: Vec<String>,
    /// Permissions
    pub permissions: Vec<String>,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// JWT ID (for revocation)
    pub jti: String,
    /// Token type (access or refresh)
    pub token_type: TokenType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenType {
    Access,
    Refresh,
}

impl TokenClaims {
    pub fn new_access_token(
        user_id: String,
        email: Option<String>,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            sub: user_id,
            email,
            roles,
            permissions,
            iat: now,
            exp: now + 3600, // 1 hour for access tokens
            iss: "llm-schema-registry".to_string(),
            aud: "llm-schema-registry-api".to_string(),
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Access,
        }
    }

    pub fn new_refresh_token(user_id: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            sub: user_id,
            email: None,
            roles: vec![],
            permissions: vec![],
            iat: now,
            exp: now + (7 * 86400), // 7 days for refresh tokens
            iss: "llm-schema-registry".to_string(),
            aud: "llm-schema-registry-api".to_string(),
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Refresh,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.exp <= now
    }
}

// =============================================================================
// Token Revocation List
// =============================================================================

pub struct TokenRevocationList {
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}

impl TokenRevocationList {
    pub fn new() -> Self {
        Self {
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Revoke a token by JWT ID
    pub async fn revoke(&self, jti: String) {
        let mut tokens = self.revoked_tokens.write().await;
        tracing::info!(jti = %jti, "Token revoked");
        tokens.insert(jti);
    }

    /// Check if a token is revoked
    pub async fn is_revoked(&self, jti: &str) -> bool {
        let tokens = self.revoked_tokens.read().await;
        tokens.contains(jti)
    }

    /// Clear expired tokens from the revocation list
    pub async fn cleanup_expired(&self, max_age_secs: u64) {
        // In production, this would check expiration times
        // For now, this is a placeholder
        tracing::debug!("Cleaning up expired revoked tokens");
    }
}

impl Default for TokenRevocationList {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// JWT Manager with RS256/HS256 support
// =============================================================================

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    validation: Validation,
    revocation_list: Arc<TokenRevocationList>,
}

impl JwtManager {
    /// Create JWT manager with HS256 (symmetric key)
    pub fn new_hs256(secret: &[u8], revocation_list: Arc<TokenRevocationList>) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["llm-schema-registry"]);
        validation.set_audience(&["llm-schema-registry-api"]);
        validation.validate_exp = true;

        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            algorithm: Algorithm::HS256,
            validation,
            revocation_list,
        }
    }

    /// Create JWT manager with RS256 (asymmetric keys)
    pub fn new_rs256(
        private_key_pem: &[u8],
        public_key_pem: &[u8],
        revocation_list: Arc<TokenRevocationList>,
    ) -> Result<Self> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&["llm-schema-registry"]);
        validation.set_audience(&["llm-schema-registry-api"]);
        validation.validate_exp = true;

        Ok(Self {
            encoding_key: EncodingKey::from_rsa_pem(private_key_pem)
                .map_err(|e| AuthError::InternalError(format!("Invalid private key: {}", e)))?,
            decoding_key: DecodingKey::from_rsa_pem(public_key_pem)
                .map_err(|e| AuthError::InternalError(format!("Invalid public key: {}", e)))?,
            algorithm: Algorithm::RS256,
            validation,
            revocation_list,
        })
    }

    /// Generate a new token
    pub fn generate_token(&self, claims: &TokenClaims) -> Result<String> {
        if claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        let header = Header::new(self.algorithm);

        encode(&header, claims, &self.encoding_key)
            .map_err(|e| AuthError::InternalError(format!("Token generation failed: {}", e)))
    }

    /// Verify and decode a token
    pub async fn verify_token(&self, token: &str) -> Result<TokenClaims> {
        // Decode token
        let token_data = decode::<TokenClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::InvalidToken(e.to_string()),
            })?;

        let claims = token_data.claims;

        // Check if token is revoked
        if self.revocation_list.is_revoked(&claims.jti).await {
            return Err(AuthError::TokenRevoked);
        }

        // Double-check expiration
        if claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        Ok(claims)
    }

    /// Revoke a token
    pub async fn revoke_token(&self, token: &str) -> Result<()> {
        let claims = self.verify_token(token).await?;
        self.revocation_list.revoke(claims.jti).await;
        Ok(())
    }

    /// Generate token pair (access + refresh)
    pub fn generate_token_pair(
        &self,
        user_id: String,
        email: Option<String>,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Result<TokenPair> {
        let access_claims = TokenClaims::new_access_token(
            user_id.clone(),
            email,
            roles,
            permissions,
        );
        let refresh_claims = TokenClaims::new_refresh_token(user_id);

        Ok(TokenPair {
            access_token: self.generate_token(&access_claims)?,
            refresh_token: self.generate_token(&refresh_claims)?,
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        })
    }

    /// Refresh an access token using a refresh token
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Result<String> {
        let claims = self.verify_token(refresh_token).await?;

        if claims.token_type != TokenType::Refresh {
            return Err(AuthError::InvalidToken(
                "Not a refresh token".to_string(),
            ));
        }

        let new_claims = TokenClaims::new_access_token(
            claims.sub,
            claims.email,
            roles,
            permissions,
        );

        self.generate_token(&new_claims)
    }
}

// =============================================================================
// Token Pair
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

// =============================================================================
// mTLS Client Certificate Validation
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCertificate {
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub not_before: u64,
    pub not_after: u64,
    pub fingerprint: String,
}

impl ClientCertificate {
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now >= self.not_before && now <= self.not_after
    }
}

pub struct MtlsValidator {
    trusted_ca_fingerprints: HashSet<String>,
}

impl MtlsValidator {
    pub fn new(trusted_cas: Vec<String>) -> Self {
        Self {
            trusted_ca_fingerprints: trusted_cas.into_iter().collect(),
        }
    }

    pub fn validate_certificate(&self, cert: &ClientCertificate) -> Result<()> {
        // Check validity period
        if !cert.is_valid() {
            return Err(AuthError::InvalidCredentials);
        }

        // Check if issued by trusted CA (simplified check)
        // In production, perform full certificate chain validation
        if !self.trusted_ca_fingerprints.contains(&cert.fingerprint) {
            return Err(AuthError::InvalidCredentials);
        }

        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_claims_creation() {
        let claims = TokenClaims::new_access_token(
            "user123".to_string(),
            Some("user@example.com".to_string()),
            vec!["developer".to_string()],
            vec!["schema:read".to_string()],
        );

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.token_type, TokenType::Access);
        assert!(!claims.is_expired());
    }

    #[tokio::test]
    async fn test_jwt_lifecycle_hs256() {
        let revocation_list = Arc::new(TokenRevocationList::new());
        let secret = b"test-secret-key-minimum-32-bytes-long";
        let manager = JwtManager::new_hs256(secret, revocation_list);

        let claims = TokenClaims::new_access_token(
            "user123".to_string(),
            Some("user@example.com".to_string()),
            vec!["developer".to_string()],
            vec!["schema:read".to_string()],
        );

        // Generate token
        let token = manager.generate_token(&claims).unwrap();

        // Verify token
        let verified = manager.verify_token(&token).await.unwrap();
        assert_eq!(verified.sub, "user123");
    }

    #[tokio::test]
    async fn test_token_revocation() {
        let revocation_list = Arc::new(TokenRevocationList::new());
        let secret = b"test-secret-key-minimum-32-bytes-long";
        let manager = JwtManager::new_hs256(secret, revocation_list);

        let claims = TokenClaims::new_access_token(
            "user123".to_string(),
            None,
            vec![],
            vec![],
        );

        let token = manager.generate_token(&claims).unwrap();

        // Revoke token
        manager.revoke_token(&token).await.unwrap();

        // Try to verify revoked token
        let result = manager.verify_token(&token).await;
        assert!(matches!(result, Err(AuthError::TokenRevoked)));
    }

    #[test]
    fn test_client_certificate_validation() {
        let mut cert = ClientCertificate {
            subject: "CN=client".to_string(),
            issuer: "CN=CA".to_string(),
            serial_number: "12345".to_string(),
            not_before: 0,
            not_after: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 86400,
            fingerprint: "abc123".to_string(),
        };

        assert!(cert.is_valid());

        // Make certificate expired
        cert.not_after = 100;
        assert!(!cert.is_valid());
    }
}
