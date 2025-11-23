use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Instant;
use uuid::Uuid;

// ============================================================================
// Correlation ID Middleware
// ============================================================================

pub const X_CORRELATION_ID: &str = "X-Correlation-ID";
pub const X_REQUEST_ID: &str = "X-Request-ID";

pub async fn correlation_id_middleware(mut request: Request, next: Next) -> Response {
    // Generate or extract correlation ID
    let correlation_id = request
        .headers()
        .get(X_CORRELATION_ID)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Generate request ID
    let request_id = Uuid::new_v4().to_string();

    // Store IDs in request extensions
    request.extensions_mut().insert(correlation_id.clone());
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    // Process request
    let mut response = next.run(request).await;

    // Add IDs to response headers
    response.headers_mut().insert(
        X_CORRELATION_ID,
        HeaderValue::from_str(&correlation_id).unwrap(),
    );
    response.headers_mut().insert(
        X_REQUEST_ID,
        HeaderValue::from_str(&request_id).unwrap(),
    );

    response
}

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

// ============================================================================
// Request Logging Middleware
// ============================================================================

pub async fn request_logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    // Extract correlation ID if present
    let correlation_id = request
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    tracing::info!(
        correlation_id = %correlation_id,
        method = %method,
        uri = %uri,
        "Request started"
    );

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    tracing::info!(
        correlation_id = %correlation_id,
        method = %method,
        uri = %uri,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        "Request completed"
    );

    response
}

// ============================================================================
// Rate Limiting Middleware
// ============================================================================

use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap());
        let limiter = Arc::new(GovernorRateLimiter::direct(quota));
        Self { limiter }
    }

    pub fn check(&self) -> bool {
        self.limiter.check().is_ok()
    }
}

pub async fn rate_limit_middleware(
    axum::extract::State(rate_limiter): axum::extract::State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !rate_limiter.check() {
        tracing::warn!("Rate limit exceeded");
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

// ============================================================================
// Error Handler Middleware
// ============================================================================

pub async fn error_handler_middleware(request: Request, next: Next) -> Response {
    let response = next.run(request).await;

    // Log errors
    if response.status().is_server_error() {
        tracing::error!(
            status = %response.status(),
            "Server error occurred"
        );
    }

    response
}

// ============================================================================
// Security Headers Middleware
// ============================================================================

pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Add security headers
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    response
}

// ============================================================================
// Timeout Middleware
// ============================================================================

use tokio::time::{timeout, Duration};

pub async fn timeout_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    match timeout(Duration::from_secs(30), next.run(request)).await {
        Ok(response) => Ok(response),
        Err(_) => {
            tracing::error!("Request timeout");
            Err(StatusCode::REQUEST_TIMEOUT)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    async fn slow_handler() -> &'static str {
        tokio::time::sleep(Duration::from_secs(2)).await;
        "Slow response"
    }

    #[tokio::test]
    async fn test_correlation_id_middleware_generates_new_id() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(correlation_id_middleware));

        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert!(response.headers().contains_key(X_CORRELATION_ID));
        assert!(response.headers().contains_key(X_REQUEST_ID));
    }

    #[tokio::test]
    async fn test_correlation_id_middleware_preserves_existing_id() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(correlation_id_middleware));

        let correlation_id = "existing-correlation-id";
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header(X_CORRELATION_ID, correlation_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(X_CORRELATION_ID).unwrap(),
            correlation_id
        );
    }

    #[tokio::test]
    async fn test_correlation_id_middleware_different_request_ids() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(correlation_id_middleware));

        let request1 = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response1 = app.clone().oneshot(request1).await.unwrap();
        let request_id1 = response1.headers().get(X_REQUEST_ID).unwrap();

        let request2 = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response2 = app.oneshot(request2).await.unwrap();
        let request_id2 = response2.headers().get(X_REQUEST_ID).unwrap();

        assert_ne!(request_id1, request_id2);
    }

    #[tokio::test]
    async fn test_security_headers_middleware_adds_all_headers() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(security_headers_middleware));

        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
        assert_eq!(
            response.headers().get(header::X_FRAME_OPTIONS).unwrap(),
            "DENY"
        );
        assert_eq!(
            response.headers().get("X-XSS-Protection").unwrap(),
            "1; mode=block"
        );
        assert!(response
            .headers()
            .get(header::STRICT_TRANSPORT_SECURITY)
            .is_some());
    }

    #[tokio::test]
    async fn test_security_headers_middleware_hsts_value() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(security_headers_middleware));

        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(
            response
                .headers()
                .get(header::STRICT_TRANSPORT_SECURITY)
                .unwrap(),
            "max-age=31536000; includeSubDomains"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_new() {
        let limiter = RateLimiter::new(10);
        assert!(limiter.check());
    }

    #[tokio::test]
    async fn test_rate_limiter_enforces_limit() {
        let limiter = RateLimiter::new(2);

        // First two requests should succeed
        assert!(limiter.check());
        assert!(limiter.check());

        // Third request should fail
        assert!(!limiter.check());
    }

    #[tokio::test]
    async fn test_rate_limiter_resets_over_time() {
        let limiter = RateLimiter::new(1);

        // Use up the quota
        assert!(limiter.check());
        assert!(!limiter.check());

        // Wait for quota to reset
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should work again
        assert!(limiter.check());
    }

    #[tokio::test]
    async fn test_request_id_struct_creation() {
        let id = RequestId("test-id".to_string());
        assert_eq!(id.0, "test-id");
    }

    #[tokio::test]
    async fn test_request_id_clone() {
        let id1 = RequestId("test-id".to_string());
        let id2 = id1.clone();
        assert_eq!(id1.0, id2.0);
    }
}
