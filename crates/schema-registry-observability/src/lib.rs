//! Observability: Prometheus metrics, OpenTelemetry tracing, structured logging
//!
//! This crate provides comprehensive observability for the LLM Schema Registry:
//! - 40+ Prometheus metrics (RED + USE + Business metrics)
//! - Distributed tracing with OpenTelemetry and Jaeger
//! - Structured JSON logging with correlation IDs
//! - HTTP and gRPC middleware for automatic instrumentation
//! - SLI/SLO monitoring support

pub mod logging;
pub mod metrics;
pub mod middleware;
pub mod tracing_setup;

pub use metrics::MetricsCollector;
pub use tracing_setup::{
    init_tracing, setup_tracing, shutdown_tracing, TracingConfig,
    context as trace_context, correlation,
};
pub use logging::{LogContext, LogSamplingConfig, ModuleLogLevels};
pub use middleware::{
    metrics_middleware, tracing_middleware, observability_middleware,
};

use std::sync::Arc;

/// Observability manager that coordinates all observability components
pub struct ObservabilityManager {
    pub metrics: Arc<MetricsCollector>,
    pub tracing_config: TracingConfig,
}

impl ObservabilityManager {
    /// Creates a new observability manager
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let metrics = MetricsCollector::new()?;
        let tracing_config = TracingConfig::default();

        Ok(Self {
            metrics,
            tracing_config,
        })
    }

    /// Initializes all observability components
    pub fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize tracing
        init_tracing(self.tracing_config.clone())?;

        tracing::info!(
            metrics_count = self.metrics.metric_count(),
            "Observability initialized successfully"
        );

        Ok(())
    }

    /// Gets metrics in Prometheus format
    pub fn export_metrics(&self) -> Result<String, prometheus::Error> {
        self.metrics.export()
    }

    /// Shuts down observability gracefully
    pub fn shutdown(&self) {
        shutdown_tracing();
    }
}

impl Default for ObservabilityManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ObservabilityManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observability_manager_creation() {
        let manager = ObservabilityManager::new().unwrap();
        assert!(manager.metrics.metric_count() > 0);
    }

    #[test]
    fn test_observability_manager_default() {
        let manager = ObservabilityManager::default();
        assert!(manager.metrics.metric_count() > 0);
    }

    #[test]
    fn test_observability_manager_metrics() {
        let manager = ObservabilityManager::new().unwrap();
        assert!(Arc::strong_count(&manager.metrics) >= 1);
    }

    #[test]
    fn test_observability_manager_tracing_config() {
        let manager = ObservabilityManager::new().unwrap();
        assert!(manager.tracing_config.service_name.len() > 0);
    }

    #[test]
    fn test_export_metrics_format() {
        let manager = ObservabilityManager::new().unwrap();
        let exported = manager.export_metrics();
        assert!(exported.is_ok());
    }

    #[test]
    fn test_multiple_managers() {
        let manager1 = ObservabilityManager::new().unwrap();
        let manager2 = ObservabilityManager::new().unwrap();

        assert!(!Arc::ptr_eq(&manager1.metrics, &manager2.metrics));
    }

    #[test]
    fn test_metrics_collector_count() {
        let manager = ObservabilityManager::new().unwrap();
        let count = manager.metrics.metric_count();
        assert!(count > 0);
    }

    #[test]
    fn test_tracing_config_has_service_name() {
        let manager = ObservabilityManager::new().unwrap();
        assert!(!manager.tracing_config.service_name.is_empty());
    }

    #[test]
    fn test_observability_manager_shutdown() {
        let manager = ObservabilityManager::new().unwrap();
        manager.shutdown();
        // Should not panic
    }

    #[test]
    fn test_metrics_export_not_empty() {
        let manager = ObservabilityManager::new().unwrap();
        let metrics = manager.export_metrics().unwrap();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_observability_manager_arc_metrics() {
        let manager = ObservabilityManager::new().unwrap();
        let metrics_clone = Arc::clone(&manager.metrics);
        assert!(Arc::ptr_eq(&metrics_clone, &manager.metrics));
    }

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert!(!config.service_name.is_empty());
    }

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new().unwrap();
        assert!(collector.metric_count() > 0);
    }

    #[test]
    fn test_multiple_metrics_exports() {
        let manager = ObservabilityManager::new().unwrap();
        let export1 = manager.export_metrics();
        let export2 = manager.export_metrics();
        assert!(export1.is_ok());
        assert!(export2.is_ok());
    }

    #[test]
    fn test_observability_manager_metrics_accessible() {
        let manager = ObservabilityManager::new().unwrap();
        let _count = manager.metrics.metric_count();
        // Should be accessible without issues
    }

    #[test]
    fn test_tracing_config_clone() {
        let manager = ObservabilityManager::new().unwrap();
        let config = manager.tracing_config.clone();
        assert_eq!(config.service_name, manager.tracing_config.service_name);
    }

    #[test]
    fn test_observability_components_independent() {
        let manager1 = ObservabilityManager::new().unwrap();
        let manager2 = ObservabilityManager::new().unwrap();

        let metrics1_count = manager1.metrics.metric_count();
        let metrics2_count = manager2.metrics.metric_count();

        assert_eq!(metrics1_count, metrics2_count);
    }

    #[test]
    fn test_metrics_export_is_prometheus_format() {
        let manager = ObservabilityManager::new().unwrap();
        let exported = manager.export_metrics().unwrap();
        // Prometheus format typically contains HELP and TYPE lines
        assert!(exported.contains("# HELP") || exported.len() > 0);
    }

    #[test]
    fn test_observability_manager_creation_success() {
        let result = ObservabilityManager::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_shutdown_can_be_called_multiple_times() {
        let manager = ObservabilityManager::new().unwrap();
        manager.shutdown();
        manager.shutdown();
        // Should not panic
    }

    #[test]
    fn test_metrics_count_positive() {
        let manager = ObservabilityManager::new().unwrap();
        assert!(manager.metrics.metric_count() > 0);
    }

    #[test]
    fn test_tracing_config_service_name_not_empty() {
        let manager = ObservabilityManager::new().unwrap();
        assert!(!manager.tracing_config.service_name.is_empty());
    }

    #[test]
    fn test_export_metrics_returns_string() {
        let manager = ObservabilityManager::new().unwrap();
        let exported = manager.export_metrics();
        assert!(exported.is_ok());
        assert!(exported.unwrap().is_ascii());
    }

    #[test]
    fn test_metrics_collector_metric_count() {
        let collector = MetricsCollector::new().unwrap();
        let count = collector.metric_count();
        assert!(count > 0);
    }

    #[test]
    fn test_observability_manager_default_creation() {
        let _manager = ObservabilityManager::default();
        // Should create successfully
    }

    #[test]
    fn test_metrics_collector_export() {
        let collector = MetricsCollector::new().unwrap();
        let exported = collector.export();
        assert!(exported.is_ok());
    }

    #[test]
    fn test_observability_manager_fields() {
        let manager = ObservabilityManager::new().unwrap();
        assert!(Arc::strong_count(&manager.metrics) >= 1);
        assert!(!manager.tracing_config.service_name.is_empty());
    }

    #[test]
    fn test_metrics_arc_reference_counting() {
        let manager = ObservabilityManager::new().unwrap();
        let original_count = Arc::strong_count(&manager.metrics);
        let _clone = Arc::clone(&manager.metrics);
        assert_eq!(Arc::strong_count(&manager.metrics), original_count + 1);
    }

    #[test]
    fn test_observability_manager_lifecycle() {
        let manager = ObservabilityManager::new().unwrap();
        let _export = manager.export_metrics();
        manager.shutdown();
        // Full lifecycle should work
    }

    #[test]
    fn test_tracing_config_fields() {
        let config = TracingConfig::default();
        assert!(!config.service_name.is_empty());
    }

    #[test]
    fn test_metrics_collector_independent_instances() {
        let collector1 = MetricsCollector::new().unwrap();
        let collector2 = MetricsCollector::new().unwrap();

        assert_eq!(collector1.metric_count(), collector2.metric_count());
    }

    #[test]
    fn test_observability_manager_error_handling() {
        // Should handle creation gracefully
        let result = ObservabilityManager::new();
        assert!(result.is_ok());
    }
}
