//! Observability middleware for HTTP and gRPC
//!
//! This module provides middleware for:
//! - Automatic metrics collection
//! - Distributed tracing
//! - Structured logging with correlation IDs
//! - Request/response instrumentation

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, Instrument};

use crate::logging::LogContext;
use crate::metrics::MetricsCollector;
use crate::tracing_setup::correlation;

/// Metrics middleware for HTTP requests
pub async fn metrics_middleware(
    metrics: Arc<MetricsCollector>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().to_string();
    let path = normalize_path(req.uri().path());
    let start = Instant::now();

    // Increment in-flight counter
    metrics
        .http_requests_in_flight
        .with_label_values(&[&method, &path])
        .inc();

    // Process request
    let response = next.run(req).await;
    let duration = start.elapsed();
    let status = response.status().as_u16().to_string();

    // Decrement in-flight counter
    metrics
        .http_requests_in_flight
        .with_label_values(&[&method, &path])
        .dec();

    // Record metrics
    metrics
        .http_requests_total
        .with_label_values(&[&method, &path, &status])
        .inc();

    metrics
        .http_request_duration_seconds
        .with_label_values(&[&method, &path])
        .observe(duration.as_secs_f64());

    response
}

/// Tracing middleware for HTTP requests
pub async fn tracing_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = normalize_path(uri.path());
    let headers = req.headers().clone();

    // Extract or generate correlation ID
    let correlation_id = correlation::get_or_generate_correlation_id(&headers);

    // Create log context
    let log_ctx = LogContext::from_headers(&headers)
        .with_correlation_id(&correlation_id)
        .with_request_id(uuid::Uuid::new_v4().to_string());

    // Create span for request
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        correlation_id = %correlation_id,
        request_id = log_ctx.request_id.as_deref().unwrap_or("unknown"),
        user_id = log_ctx.user_id.as_deref(),
        tenant_id = log_ctx.tenant_id.as_deref(),
    );

    async move {
        let start = Instant::now();

        info!(
            method = %method,
            path = %path,
            "Processing request"
        );

        // Process request
        let mut response = next.run(req).await;

        let duration = start.elapsed();
        let status = response.status();

        // Add correlation ID to response headers
        if let Ok(value) = axum::http::HeaderValue::from_str(&correlation_id) {
            response
                .headers_mut()
                .insert("x-correlation-id", value);
        }

        // Log response
        let log_level = if status.is_server_error() {
            tracing::Level::ERROR
        } else if status.is_client_error() {
            tracing::Level::WARN
        } else {
            tracing::Level::INFO
        };

        match log_level {
            tracing::Level::ERROR => {
                tracing::error!(
                    status = %status,
                    duration_ms = duration.as_millis(),
                    "Request completed with error"
                );
            }
            tracing::Level::WARN => {
                tracing::warn!(
                    status = %status,
                    duration_ms = duration.as_millis(),
                    "Request completed with client error"
                );
            }
            _ => {
                tracing::info!(
                    status = %status,
                    duration_ms = duration.as_millis(),
                    "Request completed successfully"
                );
            }
        }

        response
    }
    .instrument(span)
    .await
}

/// Combined observability middleware
/// Note: This is a simplified version. In practice, you'd use tower::ServiceBuilder
/// to compose middleware layers properly.
pub async fn observability_middleware(
    metrics: Arc<MetricsCollector>,
    req: Request,
    next: Next,
) -> Response {
    // Apply both tracing and metrics
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = normalize_path(uri.path());
    let headers = req.headers().clone();
    let start = Instant::now();

    // Extract correlation ID
    let correlation_id = crate::correlation::get_or_generate_correlation_id(&headers);

    // Increment in-flight
    metrics
        .http_requests_in_flight
        .with_label_values(&[method.as_str(), &path])
        .inc();

    // Create span and process request
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        correlation_id = %correlation_id,
    );

    let response = async {
        info!("Processing request");
        next.run(req).await
    }
    .instrument(span)
    .await;

    let duration = start.elapsed();
    let status = response.status().as_u16().to_string();

    // Decrement in-flight
    metrics
        .http_requests_in_flight
        .with_label_values(&[method.as_str(), &path])
        .dec();

    // Record metrics
    metrics
        .http_requests_total
        .with_label_values(&[method.as_str(), &path, &status])
        .inc();

    metrics
        .http_request_duration_seconds
        .with_label_values(&[method.as_str(), &path])
        .observe(duration.as_secs_f64());

    response
}

/// Normalizes HTTP path for metrics (removes IDs)
fn normalize_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = segments
        .iter()
        .map(|seg| {
            // Replace UUID-like segments with placeholder
            if is_uuid_like(seg) {
                "{id}".to_string()
            }
            // Replace version-like segments
            else if seg.starts_with('v') && seg[1..].chars().all(|c| c.is_ascii_digit() || c == '.') {
                "{version}".to_string()
            } else {
                seg.to_string()
            }
        })
        .collect();

    normalized.join("/")
}

/// Checks if a string looks like a UUID
fn is_uuid_like(s: &str) -> bool {
    s.len() == 36 && s.chars().filter(|&c| c == '-').count() == 4
}

/// gRPC interceptor for metrics and tracing
pub mod grpc {
    use super::*;
    use std::task::{Context, Poll};
    use tonic::body::BoxBody;
    use tower::Service;

    /// Metrics interceptor for gRPC
    #[derive(Clone)]
    pub struct MetricsInterceptor {
        metrics: Arc<MetricsCollector>,
    }

    impl MetricsInterceptor {
        pub fn new(metrics: Arc<MetricsCollector>) -> Self {
            Self { metrics }
        }
    }

    impl<S> tower::Layer<S> for MetricsInterceptor {
        type Service = MetricsService<S>;

        fn layer(&self, service: S) -> Self::Service {
            MetricsService {
                inner: service,
                metrics: self.metrics.clone(),
            }
        }
    }

    #[derive(Clone)]
    pub struct MetricsService<S> {
        inner: S,
        metrics: Arc<MetricsCollector>,
    }

    impl<S> Service<tonic::codegen::http::Request<BoxBody>> for MetricsService<S>
    where
        S: Service<tonic::codegen::http::Request<BoxBody>, Response = tonic::codegen::http::Response<BoxBody>>
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
        >;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, req: tonic::codegen::http::Request<BoxBody>) -> Self::Future {
            let mut inner = self.inner.clone();
            let metrics = self.metrics.clone();

            Box::pin(async move {
                let method = req.uri().path().to_string();
                let service = extract_service_name(&method);
                let start = Instant::now();

                // Increment in-flight
                metrics
                    .grpc_requests_in_flight
                    .with_label_values(&[&service, &method])
                    .inc();

                // Call service
                let response = inner.call(req).await;
                let duration = start.elapsed();

                // Decrement in-flight
                metrics
                    .grpc_requests_in_flight
                    .with_label_values(&[&service, &method])
                    .dec();

                // Record metrics
                let status = if response.is_ok() { "ok" } else { "error" };
                metrics
                    .grpc_requests_total
                    .with_label_values(&[&service, &method, status])
                    .inc();

                metrics
                    .grpc_request_duration_seconds
                    .with_label_values(&[&service, &method])
                    .observe(duration.as_secs_f64());

                response
            })
        }
    }

    fn extract_service_name(path: &str) -> String {
        path.split('/')
            .nth(1)
            .unwrap_or("unknown")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            normalize_path("/api/v1/schemas/123e4567-e89b-12d3-a456-426614174000"),
            "/api/v1/schemas/{id}"
        );
        assert_eq!(
            normalize_path("/api/v1/schemas"),
            "/api/v1/schemas"
        );
        assert_eq!(
            normalize_path("/api/v2/users/123"),
            "/api/{version}/users/123"
        );
    }

    #[test]
    fn test_is_uuid_like() {
        assert!(is_uuid_like("123e4567-e89b-12d3-a456-426614174000"));
        assert!(!is_uuid_like("not-a-uuid"));
        assert!(!is_uuid_like("123"));
    }
}
