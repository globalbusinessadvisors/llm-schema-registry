use super::{AuthError, AuthPrincipal, Permission};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// JWT Claims
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,                    // Subject (user ID)
    pub email: Option<String>,          // Email address
    pub roles: Vec<String>,             // User roles
    pub permissions: Vec<String>,       // Permissions
    pub exp: u64,                       // Expiry timestamp
    pub iat: u64,                       // Issued at
    pub iss: String,                    // Issuer
    pub aud: String,                    // Audience
    pub jti: String,                    // JWT ID
}

impl Claims {
    pub fn new(
        user_id: String,
        email: Option<String>,
        roles: Vec<String>,
        permissions: Vec<String>,
        expiry_seconds: u64,
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
            exp: now + expiry_seconds,
            iat: now,
            iss: "llm-schema-registry".to_string(),
            aud: "llm-schema-registry-api".to_string(),
            jti: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.exp <= now
    }

    pub fn to_principal(&self) -> Result<AuthPrincipal, AuthError> {
        let permissions: HashSet<Permission> = self
            .permissions
            .iter()
            .filter_map(|p| Permission::from_str(p))
            .collect();

        Ok(AuthPrincipal {
            user_id: self.sub.clone(),
            email: self.email.clone(),
            roles: self.roles.clone(),
            permissions,
            metadata: std::collections::HashMap::new(),
        })
    }
}

// ============================================================================
// JWT Manager
// ============================================================================

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtManager {
    pub fn new(secret: &[u8]) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["llm-schema-registry"]);
        validation.set_audience(&["llm-schema-registry-api"]);

        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation,
        }
    }

    pub fn from_rsa_keys(private_pem: &[u8], public_pem: &[u8]) -> Result<Self, AuthError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&["llm-schema-registry"]);
        validation.set_audience(&["llm-schema-registry-api"]);

        Ok(Self {
            encoding_key: EncodingKey::from_rsa_pem(private_pem)
                .map_err(|e| AuthError::InternalError(e.to_string()))?,
            decoding_key: DecodingKey::from_rsa_pem(public_pem)
                .map_err(|e| AuthError::InternalError(e.to_string()))?,
            validation,
        })
    }

    pub fn generate_token(&self, claims: Claims) -> Result<String, AuthError> {
        if claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::InternalError(e.to_string()))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::InvalidToken(e.to_string()),
            })?;

        let claims = token_data.claims;

        if claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        Ok(claims)
    }

    pub fn verify_and_get_principal(&self, token: &str) -> Result<AuthPrincipal, AuthError> {
        let claims = self.verify_token(token)?;
        claims.to_principal()
    }
}

// ============================================================================
// Token Extractor
// ============================================================================

pub fn extract_bearer_token(authorization: &str) -> Option<&str> {
    authorization
        .strip_prefix("Bearer ")
        .or_else(|| authorization.strip_prefix("bearer "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_lifecycle() {
        let secret = b"test-secret-key-minimum-32-bytes-long";
        let manager = JwtManager::new(secret);

        // Create claims
        let claims = Claims::new(
            "user123".to_string(),
            Some("user@example.com".to_string()),
            vec!["developer".to_string()],
            vec!["schema:read".to_string(), "schema:write".to_string()],
            3600,
        );

        // Generate token
        let token = manager.generate_token(claims.clone()).unwrap();
        assert!(!token.is_empty());

        // Verify token
        let verified_claims = manager.verify_token(&token).unwrap();
        assert_eq!(verified_claims.sub, claims.sub);
        assert_eq!(verified_claims.email, claims.email);
    }

    #[test]
    fn test_expired_token() {
        let secret = b"test-secret-key-minimum-32-bytes-long";
        let manager = JwtManager::new(secret);

        // Create expired claims
        let mut claims = Claims::new(
            "user123".to_string(),
            None,
            vec![],
            vec![],
            0,
        );
        claims.exp = 1; // Set to past timestamp

        // Should fail to generate
        assert!(manager.generate_token(claims).is_err());
    }

    #[test]
    fn test_bearer_token_extraction() {
        assert_eq!(
            extract_bearer_token("Bearer abc123"),
            Some("abc123")
        );
        assert_eq!(
            extract_bearer_token("bearer abc123"),
            Some("abc123")
        );
        assert_eq!(extract_bearer_token("abc123"), None);
    }
}
