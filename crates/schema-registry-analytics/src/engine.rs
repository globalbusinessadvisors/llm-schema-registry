//! Analytics engine - main orchestrator
//!
//! This module provides the main AnalyticsEngine that coordinates all components:
//! event bus, aggregator, storage, and provides the public API.

use crate::aggregator::DataAggregator;
use crate::error::{AnalyticsError, Result};
use crate::event_bus::{EventBus, EventConsumer, EventProcessor};
use crate::query::QueryExecutor;
use crate::reports::ReportGenerator;
use crate::storage::{AnalyticsStorage, StorageConfig};
use crate::types::{
    Operation, PerformanceMetrics, SchemaHealthScore, SchemaId, SchemaStats, SchemaUsageEvent,
    TimePeriod, TopSchemaEntry, UsageStats,
};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{debug, info};

/// Configuration for the analytics engine
#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    /// Storage configuration
    pub storage_config: StorageConfig,

    /// Event bus capacity
    pub event_bus_capacity: usize,

    /// Enable automatic cleanup
    pub auto_cleanup: bool,

    /// Cleanup interval in seconds
    pub cleanup_interval_seconds: u64,

    /// Time periods to aggregate
    pub aggregation_periods: Vec<TimePeriod>,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            storage_config: StorageConfig::default(),
            event_bus_capacity: 10_000,
            auto_cleanup: true,
            cleanup_interval_seconds: 3600, // 1 hour
            aggregation_periods: vec![
                TimePeriod::Minute1,
                TimePeriod::Minute5,
                TimePeriod::Hour1,
                TimePeriod::Day1,
            ],
        }
    }
}

/// Main analytics engine
pub struct AnalyticsEngine {
    /// Event bus for real-time event streaming
    event_bus: Arc<EventBus>,

    /// Data aggregator for time-series aggregation
    aggregator: Arc<DataAggregator>,

    /// Analytics storage
    storage: Arc<AnalyticsStorage>,

    /// Query executor
    query_executor: Arc<QueryExecutor>,

    /// Report generator
    report_generator: Arc<ReportGenerator>,

    /// Shutdown signal
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,

    /// Configuration
    config: AnalyticsConfig,
}

impl AnalyticsEngine {
    /// Create a new analytics engine with default configuration
    pub fn new() -> Self {
        Self::with_config(AnalyticsConfig::default())
    }

    /// Create a new analytics engine with custom configuration
    pub fn with_config(config: AnalyticsConfig) -> Self {
        let event_bus = Arc::new(EventBus::with_capacity(config.event_bus_capacity));
        let aggregator = Arc::new(DataAggregator::with_periods(
            config.aggregation_periods.clone(),
        ));
        let storage = Arc::new(AnalyticsStorage::with_config(
            config.storage_config.clone(),
        ));

        let query_executor = Arc::new(QueryExecutor::new(
            storage.clone(),
            aggregator.clone(),
        ));

        let report_generator = Arc::new(ReportGenerator::new(
            query_executor.clone(),
            storage.clone(),
        ));

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            event_bus,
            aggregator,
            storage,
            query_executor,
            report_generator,
            shutdown_tx,
            shutdown_rx,
            config,
        }
    }

    /// Start the analytics engine background tasks
    pub async fn start(&self) -> Result<()> {
        info!("Starting analytics engine");

        // Start event consumer
        let processor = Arc::new(AnalyticsProcessor {
            storage: self.storage.clone(),
            aggregator: self.aggregator.clone(),
        });

        let consumer = EventConsumer::new(
            self.event_bus.subscribe(),
            processor,
            self.shutdown_rx.clone(),
        );

        tokio::spawn(async move {
            consumer.run().await;
        });

        // Start cleanup task if enabled
        if self.config.auto_cleanup {
            let storage = self.storage.clone();
            let aggregator = self.aggregator.clone();
            let interval = self.config.cleanup_interval_seconds;
            let mut shutdown = self.shutdown_rx.clone();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval)) => {
                            debug!("Running scheduled cleanup");
                            if let Err(e) = storage.cleanup() {
                                tracing::error!(error = %e, "Storage cleanup failed");
                            }
                            if let Err(e) = aggregator.cleanup_old_aggregations(90) {
                                tracing::error!(error = %e, "Aggregator cleanup failed");
                            }
                        }
                        _ = shutdown.changed() => {
                            if *shutdown.borrow() {
                                debug!("Cleanup task shutting down");
                                break;
                            }
                        }
                    }
                }
            });
        }

        info!("Analytics engine started successfully");
        Ok(())
    }

    /// Record a schema usage event
    ///
    /// This is the main entry point for tracking schema operations.
    pub fn record_event(&self, event: SchemaUsageEvent) -> Result<()> {
        self.event_bus.publish(event)?;
        Ok(())
    }

    /// Record an event asynchronously
    pub async fn record_event_async(&self, event: SchemaUsageEvent) -> Result<()> {
        self.event_bus.publish_async(event).await?;
        Ok(())
    }

    /// Try to record an event (best-effort, doesn't fail)
    pub fn try_record_event(&self, event: SchemaUsageEvent) {
        self.event_bus.try_publish(event);
    }

    /// Get usage statistics for a time range
    pub fn get_usage_stats(
        &self,
        period: TimePeriod,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        schema_id: Option<SchemaId>,
    ) -> Result<Vec<UsageStats>> {
        self.aggregator.get_stats(period, start_time, end_time, schema_id)
    }

    /// Get latest usage statistics
    pub fn get_latest_stats(&self, period: TimePeriod) -> Option<UsageStats> {
        self.aggregator.get_latest_stats(period, None)
    }

    /// Get top schemas by operation count
    pub fn get_top_schemas(&self, operation: Option<Operation>, limit: usize) -> Vec<TopSchemaEntry> {
        self.storage.get_top_schemas(operation, limit)
    }

    /// Get schema statistics
    pub fn get_schema_stats(&self, schema_id: &SchemaId) -> Option<SchemaStats> {
        self.storage.get_schema_stats(schema_id)
    }

    /// Get all schema statistics
    pub fn get_all_schema_stats(&self) -> Vec<SchemaStats> {
        self.storage.get_all_schema_stats()
    }

    /// Get schema health scorecard
    pub fn get_schema_health(&self, schema_id: &SchemaId) -> Option<SchemaHealthScore> {
        self.report_generator.generate_health_scorecard(schema_id)
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> Result<PerformanceMetrics> {
        // Get recent stats to compute performance metrics
        let stats = self.query_executor.query_recent(
            Duration::hours(1),
            TimePeriod::Minute5,
        )?;

        let mut validation_by_format = std::collections::HashMap::new();
        let mut compatibility_check_count = 0;
        let mut compatibility_latencies = Vec::new();

        // Aggregate performance data
        for stat in &stats {
            for (op, op_stats) in &stat.operations {
                match op {
                    Operation::Validate => {
                        // Simplified: in production would track by format
                        validation_by_format.insert(
                            "all".to_string(),
                            crate::types::FormatPerformance {
                                format: "all".to_string(),
                                avg_validation_ms: op_stats.avg_latency_ms,
                                p95_validation_ms: op_stats.p95_latency_ms,
                                validation_count: op_stats.count,
                            },
                        );
                    }
                    Operation::CheckCompatibility => {
                        compatibility_check_count += op_stats.count;
                        compatibility_latencies.push(op_stats.avg_latency_ms);
                    }
                    _ => {}
                }
            }
        }

        let avg_compatibility_ms = if !compatibility_latencies.is_empty() {
            compatibility_latencies.iter().sum::<f64>() / compatibility_latencies.len() as f64
        } else {
            0.0
        };

        Ok(PerformanceMetrics {
            timestamp: Utc::now(),
            validation_by_format,
            compatibility_check_performance: crate::types::CompatibilityPerformance {
                avg_check_ms: avg_compatibility_ms,
                p95_check_ms: 0, // Would calculate from raw data
                check_count: compatibility_check_count,
            },
            system_latency: crate::types::LatencyDistribution {
                p50_ms: stats.last().map(|s| s.p50_latency_ms).unwrap_or(0),
                p75_ms: 0, // Would calculate
                p90_ms: 0,
                p95_ms: stats.last().map(|s| s.p95_latency_ms).unwrap_or(0),
                p99_ms: stats.last().map(|s| s.p99_latency_ms).unwrap_or(0),
                p999_ms: 0,
            },
        })
    }

    /// Get query executor for advanced queries
    pub fn query_executor(&self) -> Arc<QueryExecutor> {
        self.query_executor.clone()
    }

    /// Get report generator for generating reports
    pub fn report_generator(&self) -> Arc<ReportGenerator> {
        self.report_generator.clone()
    }

    /// Shutdown the analytics engine gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down analytics engine");

        // Signal shutdown to all background tasks
        self.shutdown_tx.send(true)
            .map_err(|e| AnalyticsError::internal(format!("Failed to send shutdown signal: {}", e)))?;

        // Give tasks time to finish
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        info!("Analytics engine shut down");
        Ok(())
    }

    /// Get engine statistics
    pub fn get_engine_stats(&self) -> EngineStats {
        let storage_stats = self.storage.get_storage_stats();

        EngineStats {
            active_subscribers: self.event_bus.subscriber_count(),
            total_events_stored: storage_stats.total_events,
            total_schemas_tracked: storage_stats.total_schemas,
            total_clients: storage_stats.total_clients,
            aggregation_windows: self.aggregator.aggregation_count(),
        }
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Event processor implementation
struct AnalyticsProcessor {
    storage: Arc<AnalyticsStorage>,
    aggregator: Arc<DataAggregator>,
}

#[async_trait::async_trait]
impl EventProcessor for AnalyticsProcessor {
    async fn process(&self, event: SchemaUsageEvent) -> Result<()> {
        // Store in storage
        self.storage.store_event(event.clone())?;

        // Add to aggregator
        self.aggregator.add_event(&event)?;

        Ok(())
    }
}

/// Engine statistics
#[derive(Debug, Clone)]
pub struct EngineStats {
    pub active_subscribers: usize,
    pub total_events_stored: usize,
    pub total_schemas_tracked: usize,
    pub total_clients: usize,
    pub aggregation_windows: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = AnalyticsEngine::new();
        // Just verify we can get stats without panicking
        let _stats = engine.get_engine_stats();
    }

    #[tokio::test]
    async fn test_record_event() {
        let engine = AnalyticsEngine::new();
        engine.start().await.unwrap();

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "test-client".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );

        let result = engine.record_event(event);
        assert!(result.is_ok());

        // Give time for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        engine.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_usage_stats() {
        let engine = AnalyticsEngine::new();
        engine.start().await.unwrap();

        let schema_id = Uuid::new_v4();

        // Record some events
        for _ in 0..5 {
            let event = SchemaUsageEvent::new(
                schema_id,
                Operation::Read,
                "test-client".to_string(),
                "us-west-1".to_string(),
                100,
                true,
            );
            engine.record_event(event).unwrap();
        }

        // Give time for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let now = Utc::now();
        let start = now - Duration::hours(1);

        let stats = engine.get_usage_stats(
            TimePeriod::Minute1,
            start,
            now,
            Some(schema_id.into()),
        );

        assert!(stats.is_ok());

        engine.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_top_schemas() {
        let engine = AnalyticsEngine::new();
        engine.start().await.unwrap();

        let schema1 = Uuid::new_v4();
        let schema2 = Uuid::new_v4();

        // Schema 1: 3 events
        for _ in 0..3 {
            let event = SchemaUsageEvent::new(
                schema1,
                Operation::Read,
                "client-1".to_string(),
                "us-west-1".to_string(),
                100,
                true,
            );
            engine.record_event(event).unwrap();
        }

        // Schema 2: 5 events
        for _ in 0..5 {
            let event = SchemaUsageEvent::new(
                schema2,
                Operation::Read,
                "client-1".to_string(),
                "us-west-1".to_string(),
                100,
                true,
            );
            engine.record_event(event).unwrap();
        }

        // Give time for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let top = engine.get_top_schemas(Some(Operation::Read), 2);
        assert!(top.len() <= 2);

        engine.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_engine_stats() {
        let engine = AnalyticsEngine::new();
        engine.start().await.unwrap();

        let stats = engine.get_engine_stats();
        // Just verify we can get stats
        let _count = stats.active_subscribers;

        engine.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let engine = AnalyticsEngine::new();
        engine.start().await.unwrap();

        // Record some events
        for _ in 0..10 {
            let event = SchemaUsageEvent::new(
                Uuid::new_v4(),
                Operation::Validate,
                "client-1".to_string(),
                "us-west-1".to_string(),
                150,
                true,
            );
            engine.record_event(event).unwrap();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let metrics = engine.get_performance_metrics();
        assert!(metrics.is_ok());

        engine.shutdown().await.unwrap();
    }
}
