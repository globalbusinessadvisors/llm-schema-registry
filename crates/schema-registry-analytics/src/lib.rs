//! # Schema Registry Analytics Engine
//!
//! Production-ready analytics engine for the LLM Schema Registry, providing:
//!
//! - Real-time event streaming and processing
//! - Time-series data aggregation (1m, 5m, 1h, 1d intervals)
//! - Schema usage tracking and metrics
//! - Performance analytics
//! - Popular schema identification
//! - Health scorecards and anomaly detection
//! - Comprehensive reporting system
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use schema_registry_analytics::{AnalyticsEngine, SchemaUsageEvent, Operation};
//! use uuid::Uuid;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create and start the analytics engine
//!     let engine = AnalyticsEngine::new();
//!     engine.start().await?;
//!
//!     // Record schema usage events
//!     let event = SchemaUsageEvent::new(
//!         Uuid::new_v4(),
//!         Operation::Read,
//!         "my-service".to_string(),
//!         "us-west-1".to_string(),
//!         125, // latency in ms
//!         true, // success
//!     );
//!     engine.record_event(event)?;
//!
//!     // Query analytics
//!     let top_schemas = engine.get_top_schemas(Some(Operation::Read), 10);
//!     println!("Top 10 schemas: {:?}", top_schemas);
//!
//!     // Get schema health
//!     let schema_id = Uuid::new_v4();
//!     if let Some(health) = engine.get_schema_health(&schema_id.into()) {
//!         println!("Health score: {}", health.overall_score);
//!     }
//!
//!     // Graceful shutdown
//!     engine.shutdown().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! ### Event Tracking
//!
//! Track all schema operations including reads, writes, validations, and compatibility checks:
//!
//! ```rust,no_run
//! # use schema_registry_analytics::{AnalyticsEngine, SchemaUsageEvent, Operation};
//! # use uuid::Uuid;
//! # let engine = AnalyticsEngine::new();
//! let event = SchemaUsageEvent::new(
//!     Uuid::new_v4(),
//!     Operation::Validate,
//!     "my-service".to_string(),
//!     "us-east-1".to_string(),
//!     50,
//!     true,
//! );
//! engine.record_event(event)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Usage Analytics
//!
//! Get aggregated usage statistics for any time period:
//!
//! ```rust,no_run
//! # use schema_registry_analytics::{AnalyticsEngine, TimePeriod};
//! # use chrono::{Utc, Duration};
//! # let engine = AnalyticsEngine::new();
//! let now = Utc::now();
//! let start = now - Duration::hours(24);
//!
//! let stats = engine.get_usage_stats(
//!     TimePeriod::Hour1,
//!     start,
//!     now,
//!     None, // Global stats
//! )?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Query API
//!
//! Use the fluent query builder for complex queries:
//!
//! ```rust,no_run
//! # use schema_registry_analytics::{AnalyticsEngine, QueryBuilder, Operation};
//! # use tokio;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let engine = AnalyticsEngine::new();
//! # engine.start().await?;
//! let query = QueryBuilder::last_days(7)
//!     .operation(Operation::Read)
//!     .region("us-west-1")
//!     .success_only()
//!     .limit(100);
//!
//! let results = query.execute(engine.query_executor().as_ref())?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Reporting
//!
//! Generate comprehensive reports:
//!
//! ```rust,no_run
//! # use schema_registry_analytics::AnalyticsEngine;
//! # use chrono::Utc;
//! # use tokio;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let engine = AnalyticsEngine::new();
//! let report_gen = engine.report_generator();
//!
//! // Daily summary
//! let daily = report_gen.generate_daily_summary(Utc::now())?;
//!
//! // Weekly trends
//! let weekly = report_gen.generate_weekly_report(Utc::now())?;
//!
//! // Anomaly detection
//! let anomalies = report_gen.detect_anomalies(24)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! The analytics engine consists of several components:
//!
//! - **Event Bus**: Real-time event streaming using tokio broadcast channels
//! - **Aggregator**: Time-series data aggregation with configurable windows
//! - **Storage**: In-memory storage with retention policies (prepared for TimescaleDB)
//! - **Query Executor**: High-level query interface with filtering and pagination
//! - **Report Generator**: Automated reporting and anomaly detection
//!
//! ## Configuration
//!
//! Customize the engine with `AnalyticsConfig`:
//!
//! ```rust,no_run
//! use schema_registry_analytics::{AnalyticsEngine, AnalyticsConfig, TimePeriod};
//!
//! let config = AnalyticsConfig {
//!     event_bus_capacity: 50_000,
//!     auto_cleanup: true,
//!     cleanup_interval_seconds: 1800,
//!     aggregation_periods: vec![
//!         TimePeriod::Minute1,
//!         TimePeriod::Hour1,
//!         TimePeriod::Day1,
//!     ],
//!     ..Default::default()
//! };
//!
//! let engine = AnalyticsEngine::with_config(config);
//! ```
//!
//! ## Performance
//!
//! - Event processing: <1ms latency
//! - Query response: <100ms for typical queries
//! - Thread-safe concurrent access
//! - Automatic memory management with retention policies
//!
//! ## Future Extensions
//!
//! The engine is designed to be easily extended with:
//!
//! - Kafka integration for event streaming
//! - TimescaleDB for persistent time-series storage
//! - Prometheus metrics export
//! - Advanced anomaly detection with ML models

pub mod aggregator;
pub mod engine;
pub mod error;
pub mod event_bus;
pub mod query;
pub mod reports;
pub mod storage;
pub mod types;

// Re-export main types for convenience
pub use aggregator::DataAggregator;
pub use engine::{AnalyticsConfig, AnalyticsEngine, EngineStats};
pub use error::{AnalyticsError, Result};
pub use event_bus::{EventBus, EventConsumer, EventProcessor, EventReceiver};
pub use query::{QueryBuilder, QueryExecutor};
pub use reports::{
    Anomaly, AnomalySeverity, AnomalyType, DailyUsageSummary, MonthlyAggregateReport,
    ReportGenerator, WeeklyTrendsReport,
};
pub use storage::{AnalyticsStorage, StorageConfig, StorageStats};
pub use types::{
    AnalyticsQuery, CompatibilityPerformance, FormatPerformance, LatencyDistribution, Operation,
    OperationStats, PerformanceMetrics, RegionStats, SchemaHealthScore, SchemaId, SchemaStats,
    SchemaTrend, SchemaUsageEvent, TimePeriod, TopSchemaEntry, TrendDirection, UsageStats,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exports() {
        // Verify all main types are accessible
        let _engine = AnalyticsEngine::new();
        let _config = AnalyticsConfig::default();
        let _bus = EventBus::new();
        let _aggregator = DataAggregator::new();
        let _storage = AnalyticsStorage::new();
    }
}
