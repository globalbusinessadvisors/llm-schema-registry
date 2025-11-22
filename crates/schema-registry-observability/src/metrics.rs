//! Comprehensive Prometheus metrics instrumentation
//!
//! This module implements 40+ production-grade metrics covering:
//! - Request metrics (rate, duration, errors)
//! - Business metrics (schemas, validations, compatibility checks)
//! - Storage metrics (cache hit rate, DB connections, query duration)
//! - System metrics (memory, CPU, goroutines)

use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, register_int_counter_vec,
    register_int_gauge_vec, CounterVec, GaugeVec, HistogramVec, IntCounterVec, IntGaugeVec,
    Registry, TextEncoder,
};
use std::sync::Arc;

/// Comprehensive metrics collector for the schema registry
pub struct MetricsCollector {
    pub registry: Registry,

    // Request metrics (RED metrics)
    pub http_requests_total: IntCounterVec,
    pub http_request_duration_seconds: HistogramVec,
    pub http_requests_in_flight: IntGaugeVec,
    pub http_request_size_bytes: HistogramVec,
    pub http_response_size_bytes: HistogramVec,

    // gRPC metrics
    pub grpc_requests_total: IntCounterVec,
    pub grpc_request_duration_seconds: HistogramVec,
    pub grpc_requests_in_flight: IntGaugeVec,

    // Business metrics - Schemas
    pub schemas_registered_total: IntCounterVec,
    pub schemas_active_total: IntGaugeVec,
    pub schemas_deprecated_total: IntGaugeVec,
    pub schemas_deleted_total: IntGaugeVec,
    pub schema_versions_total: IntGaugeVec,
    pub schema_size_bytes: HistogramVec,

    // Business metrics - Validation
    pub validations_total: IntCounterVec,
    pub validation_duration_seconds: HistogramVec,
    pub validation_errors_total: IntCounterVec,

    // Business metrics - Compatibility
    pub compatibility_checks_total: IntCounterVec,
    pub compatibility_check_duration_seconds: HistogramVec,
    pub compatibility_violations_total: IntCounterVec,

    // Storage metrics - Cache
    pub cache_operations_total: IntCounterVec,
    pub cache_hit_rate: GaugeVec,
    pub cache_size_bytes: GaugeVec,
    pub cache_items_total: IntGaugeVec,
    pub cache_evictions_total: IntCounterVec,

    // Storage metrics - Database
    pub db_connections_active: IntGaugeVec,
    pub db_connections_idle: IntGaugeVec,
    pub db_connections_max: IntGaugeVec,
    pub db_query_duration_seconds: HistogramVec,
    pub db_queries_total: IntCounterVec,
    pub db_errors_total: IntCounterVec,
    pub db_pool_wait_duration_seconds: HistogramVec,

    // Storage metrics - Redis
    pub redis_operations_total: IntCounterVec,
    pub redis_operation_duration_seconds: HistogramVec,
    pub redis_errors_total: IntCounterVec,
    pub redis_connections_active: IntGaugeVec,

    // Storage metrics - S3
    pub s3_operations_total: IntCounterVec,
    pub s3_operation_duration_seconds: HistogramVec,
    pub s3_errors_total: IntCounterVec,
    pub s3_bytes_transferred_total: IntCounterVec,

    // System metrics
    pub process_cpu_seconds_total: CounterVec,
    pub process_memory_bytes: GaugeVec,
    pub process_open_fds: IntGaugeVec,
    pub process_threads_total: IntGaugeVec,
    pub tokio_tasks_total: IntGaugeVec,
    pub tokio_tasks_active: IntGaugeVec,
}

impl MetricsCollector {
    /// Creates a new metrics collector with all metrics registered
    pub fn new() -> Result<Arc<Self>, prometheus::Error> {
        let registry = Registry::new();

        // Request metrics
        let http_requests_total = register_int_counter_vec!(
            "schema_registry_http_requests_total",
            "Total HTTP requests by method, path, and status",
            &["method", "path", "status"]
        )?;

        let http_request_duration_seconds = register_histogram_vec!(
            "schema_registry_http_request_duration_seconds",
            "HTTP request duration in seconds",
            &["method", "path"],
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        )?;

        let http_requests_in_flight = register_int_gauge_vec!(
            "schema_registry_http_requests_in_flight",
            "Current number of HTTP requests being processed",
            &["method", "path"]
        )?;

        let http_request_size_bytes = register_histogram_vec!(
            "schema_registry_http_request_size_bytes",
            "HTTP request size in bytes",
            &["method", "path"],
            vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0]
        )?;

        let http_response_size_bytes = register_histogram_vec!(
            "schema_registry_http_response_size_bytes",
            "HTTP response size in bytes",
            &["method", "path"],
            vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0]
        )?;

        // gRPC metrics
        let grpc_requests_total = register_int_counter_vec!(
            "schema_registry_grpc_requests_total",
            "Total gRPC requests by service, method, and status",
            &["service", "method", "status"]
        )?;

        let grpc_request_duration_seconds = register_histogram_vec!(
            "schema_registry_grpc_request_duration_seconds",
            "gRPC request duration in seconds",
            &["service", "method"],
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        )?;

        let grpc_requests_in_flight = register_int_gauge_vec!(
            "schema_registry_grpc_requests_in_flight",
            "Current number of gRPC requests being processed",
            &["service", "method"]
        )?;

        // Business metrics - Schemas
        let schemas_registered_total = register_int_counter_vec!(
            "schema_registry_schemas_registered_total",
            "Total schemas registered by format and state",
            &["format", "state"]
        )?;

        let schemas_active_total = register_int_gauge_vec!(
            "schema_registry_schemas_active_total",
            "Total active schemas by format",
            &["format"]
        )?;

        let schemas_deprecated_total = register_int_gauge_vec!(
            "schema_registry_schemas_deprecated_total",
            "Total deprecated schemas by format",
            &["format"]
        )?;

        let schemas_deleted_total = register_int_gauge_vec!(
            "schema_registry_schemas_deleted_total",
            "Total deleted schemas by format",
            &["format"]
        )?;

        let schema_versions_total = register_int_gauge_vec!(
            "schema_registry_schema_versions_total",
            "Total schema versions by subject",
            &["subject"]
        )?;

        let schema_size_bytes = register_histogram_vec!(
            "schema_registry_schema_size_bytes",
            "Schema size in bytes",
            &["format"],
            vec![100.0, 500.0, 1000.0, 5000.0, 10000.0, 50000.0, 100000.0]
        )?;

        // Business metrics - Validation
        let validations_total = register_int_counter_vec!(
            "schema_registry_validations_total",
            "Total validations by format and result",
            &["format", "result"]
        )?;

        let validation_duration_seconds = register_histogram_vec!(
            "schema_registry_validation_duration_seconds",
            "Validation duration in seconds",
            &["format"],
            vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5]
        )?;

        let validation_errors_total = register_int_counter_vec!(
            "schema_registry_validation_errors_total",
            "Total validation errors by format and error type",
            &["format", "error_type"]
        )?;

        // Business metrics - Compatibility
        let compatibility_checks_total = register_int_counter_vec!(
            "schema_registry_compatibility_checks_total",
            "Total compatibility checks by mode and result",
            &["mode", "result"]
        )?;

        let compatibility_check_duration_seconds = register_histogram_vec!(
            "schema_registry_compatibility_check_duration_seconds",
            "Compatibility check duration in seconds",
            &["mode"],
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
        )?;

        let compatibility_violations_total = register_int_counter_vec!(
            "schema_registry_compatibility_violations_total",
            "Total compatibility violations by mode and type",
            &["mode", "violation_type"]
        )?;

        // Storage metrics - Cache
        let cache_operations_total = register_int_counter_vec!(
            "schema_registry_cache_operations_total",
            "Total cache operations by operation, tier, and result",
            &["operation", "tier", "result"]
        )?;

        let cache_hit_rate = register_gauge_vec!(
            "schema_registry_cache_hit_rate",
            "Cache hit rate by tier (0.0 to 1.0)",
            &["tier"]
        )?;

        let cache_size_bytes = register_gauge_vec!(
            "schema_registry_cache_size_bytes",
            "Cache size in bytes by tier",
            &["tier"]
        )?;

        let cache_items_total = register_int_gauge_vec!(
            "schema_registry_cache_items_total",
            "Total items in cache by tier",
            &["tier"]
        )?;

        let cache_evictions_total = register_int_counter_vec!(
            "schema_registry_cache_evictions_total",
            "Total cache evictions by tier and reason",
            &["tier", "reason"]
        )?;

        // Storage metrics - Database
        let db_connections_active = register_int_gauge_vec!(
            "schema_registry_db_connections_active",
            "Active database connections by pool",
            &["pool"]
        )?;

        let db_connections_idle = register_int_gauge_vec!(
            "schema_registry_db_connections_idle",
            "Idle database connections by pool",
            &["pool"]
        )?;

        let db_connections_max = register_int_gauge_vec!(
            "schema_registry_db_connections_max",
            "Maximum database connections by pool",
            &["pool"]
        )?;

        let db_query_duration_seconds = register_histogram_vec!(
            "schema_registry_db_query_duration_seconds",
            "Database query duration in seconds",
            &["query", "operation"],
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
        )?;

        let db_queries_total = register_int_counter_vec!(
            "schema_registry_db_queries_total",
            "Total database queries by query and result",
            &["query", "operation", "result"]
        )?;

        let db_errors_total = register_int_counter_vec!(
            "schema_registry_db_errors_total",
            "Total database errors by type",
            &["error_type"]
        )?;

        let db_pool_wait_duration_seconds = register_histogram_vec!(
            "schema_registry_db_pool_wait_duration_seconds",
            "Time waiting for database connection from pool",
            &["pool"],
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
        )?;

        // Storage metrics - Redis
        let redis_operations_total = register_int_counter_vec!(
            "schema_registry_redis_operations_total",
            "Total Redis operations by command and result",
            &["command", "result"]
        )?;

        let redis_operation_duration_seconds = register_histogram_vec!(
            "schema_registry_redis_operation_duration_seconds",
            "Redis operation duration in seconds",
            &["command"],
            vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1]
        )?;

        let redis_errors_total = register_int_counter_vec!(
            "schema_registry_redis_errors_total",
            "Total Redis errors by type",
            &["error_type"]
        )?;

        let redis_connections_active = register_int_gauge_vec!(
            "schema_registry_redis_connections_active",
            "Active Redis connections",
            &["pool"]
        )?;

        // Storage metrics - S3
        let s3_operations_total = register_int_counter_vec!(
            "schema_registry_s3_operations_total",
            "Total S3 operations by operation and result",
            &["operation", "result"]
        )?;

        let s3_operation_duration_seconds = register_histogram_vec!(
            "schema_registry_s3_operation_duration_seconds",
            "S3 operation duration in seconds",
            &["operation"],
            vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        )?;

        let s3_errors_total = register_int_counter_vec!(
            "schema_registry_s3_errors_total",
            "Total S3 errors by type",
            &["error_type"]
        )?;

        let s3_bytes_transferred_total = register_int_counter_vec!(
            "schema_registry_s3_bytes_transferred_total",
            "Total bytes transferred to/from S3",
            &["direction"]
        )?;

        // System metrics
        let process_cpu_seconds_total = register_counter_vec!(
            "schema_registry_process_cpu_seconds_total",
            "Total CPU time consumed by the process",
            &["mode"]
        )?;

        let process_memory_bytes = register_gauge_vec!(
            "schema_registry_process_memory_bytes",
            "Process memory usage in bytes",
            &["type"]
        )?;

        let process_open_fds = register_int_gauge_vec!(
            "schema_registry_process_open_fds",
            "Number of open file descriptors",
            &[]
        )?;

        let process_threads_total = register_int_gauge_vec!(
            "schema_registry_process_threads_total",
            "Total number of threads",
            &[]
        )?;

        let tokio_tasks_total = register_int_gauge_vec!(
            "schema_registry_tokio_tasks_total",
            "Total number of Tokio tasks",
            &["state"]
        )?;

        let tokio_tasks_active = register_int_gauge_vec!(
            "schema_registry_tokio_tasks_active",
            "Number of active Tokio tasks",
            &[]
        )?;

        // Register all metrics with the registry
        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(http_request_duration_seconds.clone()))?;
        registry.register(Box::new(http_requests_in_flight.clone()))?;
        registry.register(Box::new(http_request_size_bytes.clone()))?;
        registry.register(Box::new(http_response_size_bytes.clone()))?;

        registry.register(Box::new(grpc_requests_total.clone()))?;
        registry.register(Box::new(grpc_request_duration_seconds.clone()))?;
        registry.register(Box::new(grpc_requests_in_flight.clone()))?;

        registry.register(Box::new(schemas_registered_total.clone()))?;
        registry.register(Box::new(schemas_active_total.clone()))?;
        registry.register(Box::new(schemas_deprecated_total.clone()))?;
        registry.register(Box::new(schemas_deleted_total.clone()))?;
        registry.register(Box::new(schema_versions_total.clone()))?;
        registry.register(Box::new(schema_size_bytes.clone()))?;

        registry.register(Box::new(validations_total.clone()))?;
        registry.register(Box::new(validation_duration_seconds.clone()))?;
        registry.register(Box::new(validation_errors_total.clone()))?;

        registry.register(Box::new(compatibility_checks_total.clone()))?;
        registry.register(Box::new(compatibility_check_duration_seconds.clone()))?;
        registry.register(Box::new(compatibility_violations_total.clone()))?;

        registry.register(Box::new(cache_operations_total.clone()))?;
        registry.register(Box::new(cache_hit_rate.clone()))?;
        registry.register(Box::new(cache_size_bytes.clone()))?;
        registry.register(Box::new(cache_items_total.clone()))?;
        registry.register(Box::new(cache_evictions_total.clone()))?;

        registry.register(Box::new(db_connections_active.clone()))?;
        registry.register(Box::new(db_connections_idle.clone()))?;
        registry.register(Box::new(db_connections_max.clone()))?;
        registry.register(Box::new(db_query_duration_seconds.clone()))?;
        registry.register(Box::new(db_queries_total.clone()))?;
        registry.register(Box::new(db_errors_total.clone()))?;
        registry.register(Box::new(db_pool_wait_duration_seconds.clone()))?;

        registry.register(Box::new(redis_operations_total.clone()))?;
        registry.register(Box::new(redis_operation_duration_seconds.clone()))?;
        registry.register(Box::new(redis_errors_total.clone()))?;
        registry.register(Box::new(redis_connections_active.clone()))?;

        registry.register(Box::new(s3_operations_total.clone()))?;
        registry.register(Box::new(s3_operation_duration_seconds.clone()))?;
        registry.register(Box::new(s3_errors_total.clone()))?;
        registry.register(Box::new(s3_bytes_transferred_total.clone()))?;

        registry.register(Box::new(process_cpu_seconds_total.clone()))?;
        registry.register(Box::new(process_memory_bytes.clone()))?;
        registry.register(Box::new(process_open_fds.clone()))?;
        registry.register(Box::new(process_threads_total.clone()))?;
        registry.register(Box::new(tokio_tasks_total.clone()))?;
        registry.register(Box::new(tokio_tasks_active.clone()))?;

        Ok(Arc::new(Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            http_requests_in_flight,
            http_request_size_bytes,
            http_response_size_bytes,
            grpc_requests_total,
            grpc_request_duration_seconds,
            grpc_requests_in_flight,
            schemas_registered_total,
            schemas_active_total,
            schemas_deprecated_total,
            schemas_deleted_total,
            schema_versions_total,
            schema_size_bytes,
            validations_total,
            validation_duration_seconds,
            validation_errors_total,
            compatibility_checks_total,
            compatibility_check_duration_seconds,
            compatibility_violations_total,
            cache_operations_total,
            cache_hit_rate,
            cache_size_bytes,
            cache_items_total,
            cache_evictions_total,
            db_connections_active,
            db_connections_idle,
            db_connections_max,
            db_query_duration_seconds,
            db_queries_total,
            db_errors_total,
            db_pool_wait_duration_seconds,
            redis_operations_total,
            redis_operation_duration_seconds,
            redis_errors_total,
            redis_connections_active,
            s3_operations_total,
            s3_operation_duration_seconds,
            s3_errors_total,
            s3_bytes_transferred_total,
            process_cpu_seconds_total,
            process_memory_bytes,
            process_open_fds,
            process_threads_total,
            tokio_tasks_total,
            tokio_tasks_active,
        }))
    }

    /// Exports metrics in Prometheus text format
    pub fn export(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families)
    }

    /// Gets metric count for reporting
    pub fn metric_count(&self) -> usize {
        self.registry.gather().len()
    }
}

// Note: Default trait removed as MetricsCollector returns Arc<Self>

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new().unwrap();
        assert!(collector.metric_count() > 0);
    }

    #[test]
    fn test_metrics_export() {
        let collector = MetricsCollector::new().unwrap();
        let export = collector.export().unwrap();
        assert!(export.contains("schema_registry_"));
    }

    #[test]
    fn test_http_metrics() {
        let collector = MetricsCollector::new().unwrap();
        collector
            .http_requests_total
            .with_label_values(&["GET", "/api/v1/schemas", "200"])
            .inc();

        let export = collector.export().unwrap();
        assert!(export.contains("schema_registry_http_requests_total"));
    }
}
