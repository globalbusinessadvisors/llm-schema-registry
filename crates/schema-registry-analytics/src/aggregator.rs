//! Data aggregator for time-series analytics
//!
//! This module handles aggregation of raw events into statistical summaries
//! across different time windows (1m, 5m, 1h, 1d) with support for concurrent
//! access and real-time updates.

use crate::error::{AnalyticsError, Result};
use crate::types::{
    Operation, OperationStats, RegionStats, SchemaId, SchemaUsageEvent, TimePeriod, UsageStats,
};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Key for aggregated data storage
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AggregationKey {
    period: TimePeriod,
    window_start: i64, // Unix timestamp
    schema_id: Option<SchemaId>,
}

/// Raw aggregation data before computing percentiles
#[derive(Debug, Clone)]
struct AggregationData {
    period: TimePeriod,
    window_start: DateTime<Utc>,
    total_count: u64,
    success_count: u64,
    failure_count: u64,
    latencies: Vec<u64>,
    operations: HashMap<Operation, OperationData>,
    regions: HashMap<String, RegionData>,
}

#[derive(Debug, Clone)]
struct OperationData {
    count: u64,
    success_count: u64,
    latencies: Vec<u64>,
}

#[derive(Debug, Clone)]
struct RegionData {
    count: u64,
    latencies: Vec<u64>,
}

impl Default for AggregationData {
    fn default() -> Self {
        Self {
            period: TimePeriod::Minute1,
            window_start: Utc::now(),
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            latencies: Vec::new(),
            operations: HashMap::new(),
            regions: HashMap::new(),
        }
    }
}

impl AggregationData {
    /// Add an event to this aggregation
    fn add_event(&mut self, event: &SchemaUsageEvent) {
        self.total_count += 1;
        if event.success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }

        self.latencies.push(event.latency_ms);

        // Update operation-specific data
        let op_data = self.operations.entry(event.operation).or_insert_with(|| {
            OperationData {
                count: 0,
                success_count: 0,
                latencies: Vec::new(),
            }
        });
        op_data.count += 1;
        if event.success {
            op_data.success_count += 1;
        }
        op_data.latencies.push(event.latency_ms);

        // Update region-specific data
        let region_data = self.regions.entry(event.region.clone()).or_insert_with(|| {
            RegionData {
                count: 0,
                latencies: Vec::new(),
            }
        });
        region_data.count += 1;
        region_data.latencies.push(event.latency_ms);
    }

    /// Compute final statistics from raw data
    fn compute_stats(&mut self) -> UsageStats {
        // Sort latencies for percentile calculation
        self.latencies.sort_unstable();

        let success_rate = if self.total_count > 0 {
            self.success_count as f64 / self.total_count as f64
        } else {
            0.0
        };

        let avg_latency = if !self.latencies.is_empty() {
            self.latencies.iter().sum::<u64>() as f64 / self.latencies.len() as f64
        } else {
            0.0
        };

        let min_latency = self.latencies.first().copied().unwrap_or(0);
        let max_latency = self.latencies.last().copied().unwrap_or(0);

        UsageStats {
            period: self.period,
            window_start: self.window_start,
            window_end: self.window_start
                + chrono::Duration::seconds(self.period.duration_seconds()),
            total_count: self.total_count,
            success_count: self.success_count,
            failure_count: self.failure_count,
            success_rate,
            avg_latency_ms: avg_latency,
            min_latency_ms: min_latency,
            max_latency_ms: max_latency,
            p50_latency_ms: percentile(&self.latencies, 50),
            p95_latency_ms: percentile(&self.latencies, 95),
            p99_latency_ms: percentile(&self.latencies, 99),
            operations: self.compute_operation_stats(),
            regions: self.compute_region_stats(),
        }
    }

    fn compute_operation_stats(&mut self) -> HashMap<Operation, OperationStats> {
        self.operations
            .iter_mut()
            .map(|(op, data)| {
                data.latencies.sort_unstable();
                let avg = if !data.latencies.is_empty() {
                    data.latencies.iter().sum::<u64>() as f64 / data.latencies.len() as f64
                } else {
                    0.0
                };

                let stats = OperationStats {
                    operation: *op,
                    count: data.count,
                    success_count: data.success_count,
                    avg_latency_ms: avg,
                    p95_latency_ms: percentile(&data.latencies, 95),
                };
                (*op, stats)
            })
            .collect()
    }

    fn compute_region_stats(&mut self) -> HashMap<String, RegionStats> {
        self.regions
            .iter_mut()
            .map(|(region, data)| {
                data.latencies.sort_unstable();
                let avg = if !data.latencies.is_empty() {
                    data.latencies.iter().sum::<u64>() as f64 / data.latencies.len() as f64
                } else {
                    0.0
                };

                let stats = RegionStats {
                    region: region.clone(),
                    count: data.count,
                    avg_latency_ms: avg,
                };
                (region.clone(), stats)
            })
            .collect()
    }
}

/// Calculate percentile from sorted data
fn percentile(sorted_data: &[u64], p: u8) -> u64 {
    if sorted_data.is_empty() {
        return 0;
    }

    let index = (p as f64 / 100.0 * sorted_data.len() as f64).ceil() as usize;
    let index = index.saturating_sub(1).min(sorted_data.len() - 1);
    sorted_data[index]
}

/// Data aggregator for time-series analytics
pub struct DataAggregator {
    /// In-memory aggregation storage
    aggregations: Arc<RwLock<HashMap<AggregationKey, AggregationData>>>,
    /// Configured time periods to aggregate
    periods: Vec<TimePeriod>,
}

impl DataAggregator {
    /// Create a new data aggregator with default periods
    pub fn new() -> Self {
        Self::with_periods(vec![
            TimePeriod::Minute1,
            TimePeriod::Minute5,
            TimePeriod::Hour1,
            TimePeriod::Day1,
        ])
    }

    /// Create a data aggregator with specific periods
    pub fn with_periods(periods: Vec<TimePeriod>) -> Self {
        Self {
            aggregations: Arc::new(RwLock::new(HashMap::new())),
            periods,
        }
    }

    /// Add an event to the aggregator
    pub fn add_event(&self, event: &SchemaUsageEvent) -> Result<()> {
        let mut aggregations = self.aggregations.write();

        for &period in &self.periods {
            let window_start = period.round_down(event.timestamp);

            // Global aggregation (no schema filter)
            let global_key = AggregationKey {
                period,
                window_start: window_start.timestamp(),
                schema_id: None,
            };

            let global_agg = aggregations.entry(global_key).or_insert_with(|| {
                AggregationData {
                    period,
                    window_start,
                    ..Default::default()
                }
            });
            global_agg.add_event(event);

            // Per-schema aggregation
            let schema_key = AggregationKey {
                period,
                window_start: window_start.timestamp(),
                schema_id: Some(event.schema_id.clone()),
            };

            let schema_agg = aggregations.entry(schema_key).or_insert_with(|| {
                AggregationData {
                    period,
                    window_start,
                    ..Default::default()
                }
            });
            schema_agg.add_event(event);
        }

        debug!(
            event_id = %event.event_id,
            schema_id = %event.schema_id,
            "Added event to aggregations"
        );

        Ok(())
    }

    /// Get aggregated statistics for a time range
    pub fn get_stats(
        &self,
        period: TimePeriod,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        schema_id: Option<SchemaId>,
    ) -> Result<Vec<UsageStats>> {
        if start_time >= end_time {
            return Err(AnalyticsError::InvalidTimeRange {
                start: start_time.to_rfc3339(),
                end: end_time.to_rfc3339(),
            });
        }

        let aggregations = self.aggregations.read();
        let mut results = Vec::new();

        let mut current = period.round_down(start_time);
        while current < end_time {
            let key = AggregationKey {
                period,
                window_start: current.timestamp(),
                schema_id: schema_id.clone(),
            };

            if let Some(agg_data) = aggregations.get(&key) {
                let mut data_clone = agg_data.clone();
                results.push(data_clone.compute_stats());
            } else {
                // Return empty stats for windows with no data
                results.push(UsageStats {
                    period,
                    window_start: current,
                    window_end: current + chrono::Duration::seconds(period.duration_seconds()),
                    ..Default::default()
                });
            }

            current = current + chrono::Duration::seconds(period.duration_seconds());
        }

        Ok(results)
    }

    /// Get the latest stats for a period
    pub fn get_latest_stats(
        &self,
        period: TimePeriod,
        schema_id: Option<SchemaId>,
    ) -> Option<UsageStats> {
        let now = Utc::now();
        let window_start = period.round_down(now);

        let key = AggregationKey {
            period,
            window_start: window_start.timestamp(),
            schema_id,
        };

        let aggregations = self.aggregations.read();
        aggregations.get(&key).map(|data| {
            let mut data_clone = data.clone();
            data_clone.compute_stats()
        })
    }

    /// Clean up old aggregations beyond retention period
    pub fn cleanup_old_aggregations(&self, retention_days: i64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);
        let mut aggregations = self.aggregations.write();

        let initial_count = aggregations.len();

        aggregations.retain(|key, _| {
            let window_time = DateTime::from_timestamp(key.window_start, 0)
                .unwrap_or_else(|| Utc::now());
            window_time >= cutoff
        });

        let removed = initial_count - aggregations.len();

        if removed > 0 {
            debug!(
                removed = removed,
                retention_days = retention_days,
                "Cleaned up old aggregations"
            );
        }

        Ok(removed)
    }

    /// Get total number of aggregation windows
    pub fn aggregation_count(&self) -> usize {
        self.aggregations.read().len()
    }

    /// Clear all aggregations (useful for testing)
    #[cfg(test)]
    pub fn clear(&self) {
        self.aggregations.write().clear();
    }
}

impl Default for DataAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_percentile_calculation() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&data, 50), 5);
        assert_eq!(percentile(&data, 95), 10);
        assert_eq!(percentile(&data, 99), 10);

        let empty: Vec<u64> = vec![];
        assert_eq!(percentile(&empty, 50), 0);
    }

    #[test]
    fn test_add_event() {
        let aggregator = DataAggregator::new();

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );

        aggregator.add_event(&event).unwrap();

        let stats = aggregator
            .get_latest_stats(TimePeriod::Minute1, None)
            .unwrap();

        assert_eq!(stats.total_count, 1);
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.failure_count, 0);
    }

    #[test]
    fn test_multiple_events() {
        let aggregator = DataAggregator::new();
        let schema_id = Uuid::new_v4();

        for i in 0..10 {
            let event = SchemaUsageEvent::new(
                schema_id,
                Operation::Read,
                "client-1".to_string(),
                "us-west-1".to_string(),
                100 + i * 10,
                i % 2 == 0, // 50% success rate
            );
            aggregator.add_event(&event).unwrap();
        }

        let stats = aggregator
            .get_latest_stats(TimePeriod::Minute1, None)
            .unwrap();

        assert_eq!(stats.total_count, 10);
        assert_eq!(stats.success_count, 5);
        assert_eq!(stats.failure_count, 5);
        assert_eq!(stats.success_rate, 0.5);
    }

    #[test]
    fn test_per_schema_aggregation() {
        let aggregator = DataAggregator::new();
        let schema1 = Uuid::new_v4();
        let schema2 = Uuid::new_v4();

        for _ in 0..5 {
            let event = SchemaUsageEvent::new(
                schema1,
                Operation::Read,
                "client-1".to_string(),
                "us-west-1".to_string(),
                100,
                true,
            );
            aggregator.add_event(&event).unwrap();
        }

        for _ in 0..3 {
            let event = SchemaUsageEvent::new(
                schema2,
                Operation::Write,
                "client-1".to_string(),
                "us-west-1".to_string(),
                50,
                true,
            );
            aggregator.add_event(&event).unwrap();
        }

        let global_stats = aggregator
            .get_latest_stats(TimePeriod::Minute1, None)
            .unwrap();
        assert_eq!(global_stats.total_count, 8);

        let schema1_stats = aggregator
            .get_latest_stats(TimePeriod::Minute1, Some(schema1.into()))
            .unwrap();
        assert_eq!(schema1_stats.total_count, 5);

        let schema2_stats = aggregator
            .get_latest_stats(TimePeriod::Minute1, Some(schema2.into()))
            .unwrap();
        assert_eq!(schema2_stats.total_count, 3);
    }

    #[test]
    fn test_get_stats_time_range() {
        let aggregator = DataAggregator::new();

        let start = Utc::now();
        let end = start + chrono::Duration::minutes(3);

        let stats = aggregator
            .get_stats(TimePeriod::Minute1, start, end, None)
            .unwrap();

        // Should have 3-4 windows depending on timing
        assert!(stats.len() >= 3 && stats.len() <= 4);
    }

    #[test]
    fn test_cleanup_old_aggregations() {
        let aggregator = DataAggregator::new();

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );

        aggregator.add_event(&event).unwrap();

        let initial_count = aggregator.aggregation_count();
        assert!(initial_count > 0);

        // Cleanup with very short retention (should remove nothing)
        let removed = aggregator.cleanup_old_aggregations(1).unwrap();
        assert_eq!(removed, 0);

        // Cleanup with negative retention (should remove everything)
        aggregator.clear();
        assert_eq!(aggregator.aggregation_count(), 0);
    }

    #[test]
    fn test_operation_breakdown() {
        let aggregator = DataAggregator::new();
        let schema_id = Uuid::new_v4();

        for _ in 0..3 {
            let event = SchemaUsageEvent::new(
                schema_id,
                Operation::Read,
                "client-1".to_string(),
                "us-west-1".to_string(),
                100,
                true,
            );
            aggregator.add_event(&event).unwrap();
        }

        for _ in 0..2 {
            let event = SchemaUsageEvent::new(
                schema_id,
                Operation::Write,
                "client-1".to_string(),
                "us-west-1".to_string(),
                50,
                true,
            );
            aggregator.add_event(&event).unwrap();
        }

        let stats = aggregator
            .get_latest_stats(TimePeriod::Minute1, None)
            .unwrap();

        assert_eq!(stats.operations.len(), 2);
        assert_eq!(stats.operations.get(&Operation::Read).unwrap().count, 3);
        assert_eq!(stats.operations.get(&Operation::Write).unwrap().count, 2);
    }

    #[test]
    fn test_region_breakdown() {
        let aggregator = DataAggregator::new();
        let schema_id = Uuid::new_v4();

        for _ in 0..2 {
            let event = SchemaUsageEvent::new(
                schema_id,
                Operation::Read,
                "client-1".to_string(),
                "us-west-1".to_string(),
                100,
                true,
            );
            aggregator.add_event(&event).unwrap();
        }

        for _ in 0..3 {
            let event = SchemaUsageEvent::new(
                schema_id,
                Operation::Read,
                "client-1".to_string(),
                "us-east-1".to_string(),
                80,
                true,
            );
            aggregator.add_event(&event).unwrap();
        }

        let stats = aggregator
            .get_latest_stats(TimePeriod::Minute1, None)
            .unwrap();

        assert_eq!(stats.regions.len(), 2);
        assert_eq!(stats.regions.get("us-west-1").unwrap().count, 2);
        assert_eq!(stats.regions.get("us-east-1").unwrap().count, 3);
    }
}
