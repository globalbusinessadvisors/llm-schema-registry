//! Distributed tracing setup with OpenTelemetry and Jaeger
//!
//! This module configures:
//! - OpenTelemetry trace exporter (OTLP to Jaeger)
//! - Tracing subscriber with JSON formatting
//! - Correlation IDs for request tracking
//! - 10% head-based sampling strategy
//! - Trace context propagation

use opentelemetry::{
    global,
    trace::TraceError,
    KeyValue,
};
use opentelemetry_sdk::{
    trace::{RandomIdGenerator, Sampler},
    Resource,
};
use opentelemetry_otlp::WithExportConfig;
use tracing::info;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    EnvFilter, Registry,
};

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Service name for tracing
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Environment (dev, staging, production)
    pub environment: String,
    /// OTLP endpoint (e.g., http://jaeger:4317)
    pub otlp_endpoint: String,
    /// Sampling rate (0.0 to 1.0, default 0.1 = 10%)
    pub sampling_rate: f64,
    /// Enable JSON logging
    pub json_logs: bool,
    /// Log level
    pub log_level: String,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "schema-registry".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()),
            otlp_endpoint: std::env::var("OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://jaeger:4317".to_string()),
            sampling_rate: std::env::var("TRACE_SAMPLING_RATE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.1),
            json_logs: std::env::var("JSON_LOGS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        }
    }
}

/// Initializes comprehensive tracing and logging
pub fn init_tracing(config: TracingConfig) -> Result<(), TraceError> {
    // Initialize OpenTelemetry tracer
    let tracer = init_tracer(&config)?;

    // Create tracing layers
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Create filter layer
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Create subscriber
    let subscriber = Registry::default()
        .with(env_filter)
        .with(telemetry_layer);

    // Add fmt layer based on configuration
    if config.json_logs {
        let fmt_layer = fmt::layer()
            .json()
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE);

        let subscriber = subscriber.with(fmt_layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    } else {
        let fmt_layer = fmt::layer()
            .with_thread_ids(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE);

        let subscriber = subscriber.with(fmt_layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    }

    info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = %config.environment,
        otlp_endpoint = %config.otlp_endpoint,
        sampling_rate = %config.sampling_rate,
        "Tracing initialized successfully"
    );

    Ok(())
}

/// Initializes OpenTelemetry tracer with OTLP exporter
fn init_tracer(config: &TracingConfig) -> Result<opentelemetry_sdk::trace::Tracer, TraceError> {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&config.otlp_endpoint);

    let trace_config = opentelemetry_sdk::trace::config()
        .with_sampler(Sampler::ParentBased(Box::new(
            Sampler::TraceIdRatioBased(config.sampling_rate),
        )))
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(128)
        .with_max_attributes_per_span(64)
        .with_max_links_per_span(64)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", config.service_name.clone()),
            KeyValue::new("service.version", config.service_version.clone()),
            KeyValue::new("deployment.environment", config.environment.clone()),
            KeyValue::new("telemetry.sdk.name", "opentelemetry"),
            KeyValue::new("telemetry.sdk.language", "rust"),
        ]));

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
}

/// Shuts down tracing gracefully, flushing remaining spans
pub fn shutdown_tracing() {
    info!("Shutting down tracing");
    global::shutdown_tracer_provider();
}

/// Helper function to setup basic tracing (for convenience)
pub fn setup_tracing() -> Result<(), TraceError> {
    init_tracing(TracingConfig::default())
}

/// Middleware helper for extracting/injecting trace context
pub mod context {
    use opentelemetry::propagation::{Extractor, Injector, TextMapPropagator};
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    /// Extracts trace context from HTTP headers
    pub struct HeaderExtractor<'a>(pub &'a axum::http::HeaderMap);

    impl<'a> Extractor for HeaderExtractor<'a> {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|v| v.to_str().ok())
        }

        fn keys(&self) -> Vec<&str> {
            self.0.keys().map(|k| k.as_str()).collect()
        }
    }

    /// Injects trace context into HTTP headers
    pub struct HeaderInjector<'a>(pub &'a mut axum::http::HeaderMap);

    impl<'a> Injector for HeaderInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            if let Ok(header_name) = axum::http::HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = axum::http::HeaderValue::from_str(&value) {
                    self.0.insert(header_name, header_value);
                }
            }
        }
    }

    /// Extracts trace context from gRPC metadata
    pub struct MetadataExtractor<'a>(pub &'a tonic::metadata::MetadataMap);

    impl<'a> Extractor for MetadataExtractor<'a> {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|v| v.to_str().ok())
        }

        fn keys(&self) -> Vec<&str> {
            self.0.keys().map(|k| match k {
                tonic::metadata::KeyRef::Ascii(key) => key.as_str(),
                tonic::metadata::KeyRef::Binary(key) => key.as_str(),
            }).collect()
        }
    }

    /// Injects trace context into gRPC metadata
    pub struct MetadataInjector<'a>(pub &'a mut tonic::metadata::MetadataMap);

    impl<'a> Injector for MetadataInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            if let Ok(key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
                if let Ok(value) = value.parse() {
                    self.0.insert(key, value);
                }
            }
        }
    }

    /// Propagates trace context from HTTP headers
    pub fn extract_trace_context(headers: &axum::http::HeaderMap) -> opentelemetry::Context {
        let propagator = TraceContextPropagator::new();
        propagator.extract(&HeaderExtractor(headers))
    }

    /// Injects trace context into HTTP headers
    pub fn inject_trace_context(
        context: &opentelemetry::Context,
        headers: &mut axum::http::HeaderMap,
    ) {
        let propagator = TraceContextPropagator::new();
        propagator.inject_context(context, &mut HeaderInjector(headers));
    }

    /// Propagates trace context from gRPC metadata
    pub fn extract_grpc_context(metadata: &tonic::metadata::MetadataMap) -> opentelemetry::Context {
        let propagator = TraceContextPropagator::new();
        propagator.extract(&MetadataExtractor(metadata))
    }

    /// Injects trace context into gRPC metadata
    pub fn inject_grpc_context(
        context: &opentelemetry::Context,
        metadata: &mut tonic::metadata::MetadataMap,
    ) {
        let propagator = TraceContextPropagator::new();
        propagator.inject_context(context, &mut MetadataInjector(metadata));
    }
}

/// Correlation ID helpers
pub mod correlation {
    use uuid::Uuid;

    /// Generates a new correlation ID
    pub fn generate_correlation_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Extracts correlation ID from headers or generates a new one
    pub fn get_or_generate_correlation_id(headers: &axum::http::HeaderMap) -> String {
        headers
            .get("x-correlation-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(generate_correlation_id)
    }

    /// Sets correlation ID in response headers
    pub fn set_correlation_id(headers: &mut axum::http::HeaderMap, correlation_id: &str) {
        if let Ok(value) = axum::http::HeaderValue::from_str(correlation_id) {
            headers.insert("x-correlation-id", value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "schema-registry");
        assert!(config.sampling_rate >= 0.0 && config.sampling_rate <= 1.0);
    }

    #[test]
    fn test_correlation_id_generation() {
        let id1 = correlation::generate_correlation_id();
        let id2 = correlation::generate_correlation_id();
        assert_ne!(id1, id2);
        assert!(uuid::Uuid::parse_str(&id1).is_ok());
    }
}
