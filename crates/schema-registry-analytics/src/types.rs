//! Core data types for analytics
//!
//! This module defines all the data structures used throughout the analytics engine,
//! including events, metrics, statistics, and query types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Schema identifier - can be either UUID or fully qualified name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaId {
    /// UUID-based identifier
    Uuid(Uuid),
    /// Fully qualified name (namespace.name)
    Name(String),
}

impl From<Uuid> for SchemaId {
    fn from(id: Uuid) -> Self {
        SchemaId::Uuid(id)
    }
}

impl From<String> for SchemaId {
    fn from(name: String) -> Self {
        SchemaId::Name(name)
    }
}

impl From<&str> for SchemaId {
    fn from(name: &str) -> Self {
        SchemaId::Name(name.to_string())
    }
}

impl std::fmt::Display for SchemaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaId::Uuid(id) => write!(f, "{}", id),
            SchemaId::Name(name) => write!(f, "{}", name),
        }
    }
}

/// Type of schema operation being tracked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Operation {
    /// Schema read operation
    Read,
    /// Schema write/register operation
    Write,
    /// Schema validation operation
    Validate,
    /// Compatibility check operation
    CheckCompatibility,
    /// Schema deletion operation
    Delete,
    /// Schema state transition operation
    StateTransition,
    /// Schema search operation
    Search,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Read => write!(f, "READ"),
            Operation::Write => write!(f, "WRITE"),
            Operation::Validate => write!(f, "VALIDATE"),
            Operation::CheckCompatibility => write!(f, "CHECK_COMPATIBILITY"),
            Operation::Delete => write!(f, "DELETE"),
            Operation::StateTransition => write!(f, "STATE_TRANSITION"),
            Operation::Search => write!(f, "SEARCH"),
        }
    }
}

/// Time period for aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimePeriod {
    /// 1-minute interval
    Minute1,
    /// 5-minute interval
    Minute5,
    /// 1-hour interval
    Hour1,
    /// 1-day interval
    Day1,
}

impl TimePeriod {
    /// Get duration in seconds
    pub fn duration_seconds(&self) -> i64 {
        match self {
            TimePeriod::Minute1 => 60,
            TimePeriod::Minute5 => 300,
            TimePeriod::Hour1 => 3600,
            TimePeriod::Day1 => 86400,
        }
    }

    /// Round a timestamp down to the start of the period
    pub fn round_down(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        let secs = timestamp.timestamp();
        let period_secs = self.duration_seconds();
        let rounded_secs = (secs / period_secs) * period_secs;
        DateTime::from_timestamp(rounded_secs, 0).unwrap_or(timestamp)
    }
}

/// Schema usage event - the fundamental tracking unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaUsageEvent {
    /// Unique event ID
    pub event_id: Uuid,
    /// Schema identifier
    pub schema_id: SchemaId,
    /// Operation performed
    pub operation: Operation,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Client/application identifier
    pub client_id: String,
    /// Region/datacenter identifier
    pub region: String,
    /// Operation latency in milliseconds
    pub latency_ms: u64,
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if operation failed
    pub error_message: Option<String>,
    /// Additional context/metadata
    pub metadata: HashMap<String, String>,
}

impl SchemaUsageEvent {
    /// Create a new schema usage event
    pub fn new(
        schema_id: impl Into<SchemaId>,
        operation: Operation,
        client_id: String,
        region: String,
        latency_ms: u64,
        success: bool,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            schema_id: schema_id.into(),
            operation,
            timestamp: Utc::now(),
            client_id,
            region,
            latency_ms,
            success,
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed event with error message
    pub fn failed(
        schema_id: impl Into<SchemaId>,
        operation: Operation,
        client_id: String,
        region: String,
        latency_ms: u64,
        error: String,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            schema_id: schema_id.into(),
            operation,
            timestamp: Utc::now(),
            client_id,
            region,
            latency_ms,
            success: false,
            error_message: Some(error),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the event
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Aggregated usage statistics for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Time period this stat represents
    pub period: TimePeriod,
    /// Start of the time window
    pub window_start: DateTime<Utc>,
    /// End of the time window
    pub window_end: DateTime<Utc>,
    /// Total number of operations
    pub total_count: u64,
    /// Number of successful operations
    pub success_count: u64,
    /// Number of failed operations
    pub failure_count: u64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Minimum latency
    pub min_latency_ms: u64,
    /// Maximum latency
    pub max_latency_ms: u64,
    /// 50th percentile latency
    pub p50_latency_ms: u64,
    /// 95th percentile latency
    pub p95_latency_ms: u64,
    /// 99th percentile latency
    pub p99_latency_ms: u64,
    /// Breakdown by operation type
    pub operations: HashMap<Operation, OperationStats>,
    /// Breakdown by region
    pub regions: HashMap<String, RegionStats>,
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            period: TimePeriod::Minute1,
            window_start: Utc::now(),
            window_end: Utc::now(),
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            success_rate: 0.0,
            avg_latency_ms: 0.0,
            min_latency_ms: 0,
            max_latency_ms: 0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            operations: HashMap::new(),
            regions: HashMap::new(),
        }
    }
}

/// Statistics for a specific operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
    /// Operation type
    pub operation: Operation,
    /// Total count
    pub count: u64,
    /// Success count
    pub success_count: u64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// 95th percentile latency
    pub p95_latency_ms: u64,
}

/// Statistics for a specific region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionStats {
    /// Region identifier
    pub region: String,
    /// Total count
    pub count: u64,
    /// Average latency
    pub avg_latency_ms: f64,
}

/// Per-schema statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaStats {
    /// Schema identifier
    pub schema_id: SchemaId,
    /// Total number of operations on this schema
    pub total_operations: u64,
    /// Number of reads
    pub read_count: u64,
    /// Number of writes
    pub write_count: u64,
    /// Number of validations
    pub validation_count: u64,
    /// Number of compatibility checks
    pub compatibility_check_count: u64,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// First accessed timestamp
    pub first_accessed: DateTime<Utc>,
    /// Unique clients that accessed this schema
    pub unique_clients: u64,
    /// Average latency for operations on this schema
    pub avg_latency_ms: f64,
    /// Success rate
    pub success_rate: f64,
}

/// Performance metrics for different aspects of the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Timestamp when metrics were collected
    pub timestamp: DateTime<Utc>,
    /// Validation performance by format
    pub validation_by_format: HashMap<String, FormatPerformance>,
    /// Compatibility check performance
    pub compatibility_check_performance: CompatibilityPerformance,
    /// Overall system latency percentiles
    pub system_latency: LatencyDistribution,
}

/// Performance metrics for a specific serialization format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatPerformance {
    /// Serialization format
    pub format: String,
    /// Average validation time
    pub avg_validation_ms: f64,
    /// 95th percentile validation time
    pub p95_validation_ms: u64,
    /// Total validations performed
    pub validation_count: u64,
}

/// Compatibility check performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityPerformance {
    /// Average compatibility check time
    pub avg_check_ms: f64,
    /// 95th percentile check time
    pub p95_check_ms: u64,
    /// Total checks performed
    pub check_count: u64,
}

/// Latency distribution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDistribution {
    /// 50th percentile (median)
    pub p50_ms: u64,
    /// 75th percentile
    pub p75_ms: u64,
    /// 90th percentile
    pub p90_ms: u64,
    /// 95th percentile
    pub p95_ms: u64,
    /// 99th percentile
    pub p99_ms: u64,
    /// 99.9th percentile
    pub p999_ms: u64,
}

/// Query parameters for analytics queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsQuery {
    /// Start of time range (inclusive)
    pub start_time: DateTime<Utc>,
    /// End of time range (exclusive)
    pub end_time: DateTime<Utc>,
    /// Filter by schema IDs
    pub schema_ids: Option<Vec<SchemaId>>,
    /// Filter by operations
    pub operations: Option<Vec<Operation>>,
    /// Filter by regions
    pub regions: Option<Vec<String>>,
    /// Filter by clients
    pub client_ids: Option<Vec<String>>,
    /// Filter by success/failure
    pub success_only: Option<bool>,
    /// Time period for aggregation
    pub aggregation_period: Option<TimePeriod>,
    /// Limit number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl AnalyticsQuery {
    /// Create a new query for a time range
    pub fn new(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            start_time,
            end_time,
            schema_ids: None,
            operations: None,
            regions: None,
            client_ids: None,
            success_only: None,
            aggregation_period: None,
            limit: None,
            offset: None,
        }
    }

    /// Filter by schema IDs
    pub fn with_schemas(mut self, schema_ids: Vec<SchemaId>) -> Self {
        self.schema_ids = Some(schema_ids);
        self
    }

    /// Filter by operations
    pub fn with_operations(mut self, operations: Vec<Operation>) -> Self {
        self.operations = Some(operations);
        self
    }

    /// Filter by regions
    pub fn with_regions(mut self, regions: Vec<String>) -> Self {
        self.regions = Some(regions);
        self
    }

    /// Filter by success only
    pub fn success_only(mut self) -> Self {
        self.success_only = Some(true);
        self
    }

    /// Set aggregation period
    pub fn aggregate_by(mut self, period: TimePeriod) -> Self {
        self.aggregation_period = Some(period);
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Top schema entry for popularity rankings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopSchemaEntry {
    /// Schema identifier
    pub schema_id: SchemaId,
    /// Metric value (e.g., count, frequency)
    pub value: u64,
    /// Rank (1-based)
    pub rank: usize,
    /// Change from previous period (positive = trending up)
    pub trend: Option<i64>,
}

/// Schema health scorecard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaHealthScore {
    /// Schema identifier
    pub schema_id: SchemaId,
    /// Overall health score (0-100)
    pub overall_score: u8,
    /// Success rate score (0-100)
    pub success_rate_score: u8,
    /// Performance score (0-100)
    pub performance_score: u8,
    /// Usage activity score (0-100)
    pub activity_score: u8,
    /// Whether schema appears to be zombie (unused)
    pub is_zombie: bool,
    /// Days since last access
    pub days_since_last_access: i64,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrendDirection {
    /// Usage increasing
    Up,
    /// Usage decreasing
    Down,
    /// Usage stable
    Stable,
}

/// Schema trend information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaTrend {
    /// Schema identifier
    pub schema_id: SchemaId,
    /// Current period value
    pub current_value: u64,
    /// Previous period value
    pub previous_value: u64,
    /// Percentage change
    pub change_percent: f64,
    /// Trend direction
    pub direction: TrendDirection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let schema_id: SchemaId = uuid.into();
        assert_eq!(schema_id, SchemaId::Uuid(uuid));
    }

    #[test]
    fn test_schema_id_from_string() {
        let name = "com.example.Schema";
        let schema_id: SchemaId = name.into();
        assert_eq!(schema_id, SchemaId::Name(name.to_string()));
    }

    #[test]
    fn test_time_period_duration() {
        assert_eq!(TimePeriod::Minute1.duration_seconds(), 60);
        assert_eq!(TimePeriod::Minute5.duration_seconds(), 300);
        assert_eq!(TimePeriod::Hour1.duration_seconds(), 3600);
        assert_eq!(TimePeriod::Day1.duration_seconds(), 86400);
    }

    #[test]
    fn test_time_period_round_down() {
        let timestamp = DateTime::from_timestamp(1700000000, 0).unwrap();

        let rounded_min = TimePeriod::Minute1.round_down(timestamp);
        assert_eq!(rounded_min.timestamp() % 60, 0);

        let rounded_hour = TimePeriod::Hour1.round_down(timestamp);
        assert_eq!(rounded_hour.timestamp() % 3600, 0);
    }

    #[test]
    fn test_usage_event_creation() {
        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            150,
            true,
        );

        assert_eq!(event.operation, Operation::Read);
        assert_eq!(event.client_id, "client-1");
        assert_eq!(event.region, "us-west-1");
        assert_eq!(event.latency_ms, 150);
        assert!(event.success);
    }

    #[test]
    fn test_failed_event() {
        let event = SchemaUsageEvent::failed(
            Uuid::new_v4(),
            Operation::Validate,
            "client-1".to_string(),
            "us-west-1".to_string(),
            50,
            "Validation failed".to_string(),
        );

        assert!(!event.success);
        assert_eq!(event.error_message, Some("Validation failed".to_string()));
    }

    #[test]
    fn test_analytics_query_builder() {
        let start = Utc::now();
        let end = Utc::now();

        let query = AnalyticsQuery::new(start, end)
            .with_operations(vec![Operation::Read, Operation::Write])
            .with_regions(vec!["us-west-1".to_string()])
            .success_only()
            .limit(100);

        assert_eq!(query.operations, Some(vec![Operation::Read, Operation::Write]));
        assert_eq!(query.success_only, Some(true));
        assert_eq!(query.limit, Some(100));
    }
}
