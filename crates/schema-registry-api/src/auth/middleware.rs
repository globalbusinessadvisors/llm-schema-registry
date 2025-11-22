use super::{
    api_key::{extract_api_key, ApiKeyManager},
    jwt::{extract_bearer_token, JwtManager},
    AuthError, AuthPrincipal,
};
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

// ============================================================================
// Authentication State
// ============================================================================

#[derive(Clone)]
pub struct AuthState {
    pub jwt_manager: Arc<JwtManager>,
    pub api_key_manager: Arc<ApiKeyManager>,
}

// ============================================================================
// Authentication Middleware
// ============================================================================

pub async fn authenticate(
    State(auth_state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthResponse> {
    // Extract authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let principal = if let Some(auth_value) = auth_header {
        // Try JWT first
        if let Some(token) = extract_bearer_token(auth_value) {
            auth_state
                .jwt_manager
                .verify_and_get_principal(token)
                .map_err(|_| AuthResponse::Unauthorized)?
        }
        // Try API key
        else if let Some(api_key) = extract_api_key(auth_value) {
            auth_state
                .api_key_manager
                .verify_key(api_key)
                .await
                .map_err(|_| AuthResponse::Unauthorized)?
        } else {
            return Err(AuthResponse::Unauthorized);
        }
    } else {
        return Err(AuthResponse::Unauthorized);
    };

    // Insert principal into request extensions
    request.extensions_mut().insert(principal);

    Ok(next.run(request).await)
}

// ============================================================================
// Optional Authentication Middleware
// ============================================================================

pub async fn optional_authenticate(
    State(auth_state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract authorization header
    if let Some(auth_header) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        // Try JWT first
        if let Some(token) = extract_bearer_token(auth_header) {
            if let Ok(principal) = auth_state.jwt_manager.verify_and_get_principal(token) {
                request.extensions_mut().insert(principal);
            }
        }
        // Try API key
        else if let Some(api_key) = extract_api_key(auth_header) {
            if let Ok(principal) = auth_state.api_key_manager.verify_key(api_key).await {
                request.extensions_mut().insert(principal);
            }
        }
    }

    next.run(request).await
}

// ============================================================================
// Authorization Response
// ============================================================================

pub enum AuthResponse {
    Unauthorized,
    Forbidden,
}

impl IntoResponse for AuthResponse {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthResponse::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AuthResponse::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
        };

        (status, message).into_response()
    }
}

// ============================================================================
// Principal Extractor
// ============================================================================

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for AuthPrincipal
where
    S: Send + Sync,
{
    type Rejection = AuthResponse;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthPrincipal>()
            .cloned()
            .ok_or(AuthResponse::Unauthorized)
    }
}

// ============================================================================
// Permission Guard Macro
// ============================================================================

#[macro_export]
macro_rules! require_permission {
    ($principal:expr, $permission:expr) => {
        if !$principal.has_permission(&$permission) {
            return Err($crate::models::ApiError::forbidden(
                "Insufficient permissions",
            ));
        }
    };
}
