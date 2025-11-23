//! Analytics storage layer
//!
//! This module provides in-memory storage for analytics data with retention policies
//! and query capabilities. It's designed to be easily replaced with TimescaleDB or
//! other time-series databases in production.

use crate::error::{AnalyticsError, Result};
use crate::types::{
    Operation, SchemaId, SchemaStats, SchemaUsageEvent, TopSchemaEntry, TrendDirection,
    SchemaTrend,
};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tracing::{debug, info};

/// Raw event storage entry
#[derive(Debug, Clone)]
struct EventEntry {
    event: SchemaUsageEvent,
    #[allow(dead_code)]
    indexed_at: DateTime<Utc>,
}

/// Analytics storage
pub struct AnalyticsStorage {
    /// Raw events (limited retention)
    events: Arc<RwLock<BTreeMap<i64, Vec<EventEntry>>>>,

    /// Per-schema statistics
    schema_stats: Arc<RwLock<HashMap<SchemaId, SchemaStatsData>>>,

    /// Client tracking
    clients: Arc<RwLock<HashMap<String, ClientData>>>,

    /// Configuration
    config: StorageConfig,
}

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// How many days to retain detailed events
    pub detailed_retention_days: i64,

    /// Maximum number of events to store per day
    pub max_events_per_day: usize,

    /// Enable event storage (can disable for memory-constrained environments)
    pub store_raw_events: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            detailed_retention_days: 90,
            max_events_per_day: 1_000_000,
            store_raw_events: true,
        }
    }
}

/// Internal schema statistics data
#[derive(Debug, Clone)]
struct SchemaStatsData {
    schema_id: SchemaId,
    total_operations: u64,
    read_count: u64,
    write_count: u64,
    validation_count: u64,
    compatibility_check_count: u64,
    delete_count: u64,
    success_count: u64,
    failure_count: u64,
    first_accessed: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    unique_clients: HashMap<String, ()>, // Set of client IDs
    latencies: Vec<u64>,
}

impl Default for SchemaStatsData {
    fn default() -> Self {
        Self {
            schema_id: SchemaId::Name("unknown".to_string()),
            total_operations: 0,
            read_count: 0,
            write_count: 0,
            validation_count: 0,
            compatibility_check_count: 0,
            delete_count: 0,
            success_count: 0,
            failure_count: 0,
            first_accessed: Utc::now(),
            last_accessed: Utc::now(),
            unique_clients: HashMap::new(),
            latencies: Vec::new(),
        }
    }
}

impl SchemaStatsData {
    fn update(&mut self, event: &SchemaUsageEvent) {
        self.total_operations += 1;
        self.last_accessed = event.timestamp;

        match event.operation {
            Operation::Read => self.read_count += 1,
            Operation::Write => self.write_count += 1,
            Operation::Validate => self.validation_count += 1,
            Operation::CheckCompatibility => self.compatibility_check_count += 1,
            Operation::Delete => self.delete_count += 1,
            _ => {}
        }

        if event.success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }

        self.unique_clients.insert(event.client_id.clone(), ());
        self.latencies.push(event.latency_ms);

        // Keep latencies bounded for memory
        if self.latencies.len() > 10_000 {
            self.latencies.drain(0..5_000);
        }
    }

    fn to_schema_stats(&self) -> SchemaStats {
        let avg_latency = if !self.latencies.is_empty() {
            self.latencies.iter().sum::<u64>() as f64 / self.latencies.len() as f64
        } else {
            0.0
        };

        let success_rate = if self.total_operations > 0 {
            self.success_count as f64 / self.total_operations as f64
        } else {
            0.0
        };

        SchemaStats {
            schema_id: self.schema_id.clone(),
            total_operations: self.total_operations,
            read_count: self.read_count,
            write_count: self.write_count,
            validation_count: self.validation_count,
            compatibility_check_count: self.compatibility_check_count,
            last_accessed: self.last_accessed,
            first_accessed: self.first_accessed,
            unique_clients: self.unique_clients.len() as u64,
            avg_latency_ms: avg_latency,
            success_rate,
        }
    }
}

#[derive(Debug, Clone)]
struct ClientData {
    #[allow(dead_code)]
    client_id: String,
    #[allow(dead_code)]
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    request_count: u64,
}

impl AnalyticsStorage {
    /// Create a new analytics storage with default configuration
    pub fn new() -> Self {
        Self::with_config(StorageConfig::default())
    }

    /// Create a new analytics storage with custom configuration
    pub fn with_config(config: StorageConfig) -> Self {
        Self {
            events: Arc::new(RwLock::new(BTreeMap::new())),
            schema_stats: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Store a raw event
    pub fn store_event(&self, event: SchemaUsageEvent) -> Result<()> {
        // Update schema statistics
        let mut schema_stats = self.schema_stats.write();
        let stats = schema_stats
            .entry(event.schema_id.clone())
            .or_insert_with(|| SchemaStatsData {
                schema_id: event.schema_id.clone(),
                first_accessed: event.timestamp,
                ..Default::default()
            });
        stats.update(&event);
        drop(schema_stats);

        // Update client tracking
        let mut clients = self.clients.write();
        clients
            .entry(event.client_id.clone())
            .and_modify(|c| {
                c.last_seen = event.timestamp;
                c.request_count += 1;
            })
            .or_insert(ClientData {
                client_id: event.client_id.clone(),
                first_seen: event.timestamp,
                last_seen: event.timestamp,
                request_count: 1,
            });
        drop(clients);

        // Store raw event if enabled
        if self.config.store_raw_events {
            let day_key = event.timestamp.date_naive().and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp();

            let mut events = self.events.write();
            let day_events = events.entry(day_key).or_insert_with(Vec::new);

            if day_events.len() < self.config.max_events_per_day {
                day_events.push(EventEntry {
                    event,
                    indexed_at: Utc::now(),
                });
            } else {
                debug!("Event storage limit reached for day {}", day_key);
            }
        }

        Ok(())
    }

    /// Get events for a time range
    pub fn get_events(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<usize>,
    ) -> Result<Vec<SchemaUsageEvent>> {
        if start_time >= end_time {
            return Err(AnalyticsError::InvalidTimeRange {
                start: start_time.to_rfc3339(),
                end: end_time.to_rfc3339(),
            });
        }

        let events = self.events.read();
        let mut results = Vec::new();

        // Calculate day keys for the range
        let start_day_key = start_time.date_naive().and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let end_day_key = end_time.date_naive().and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp() + 86400; // Add one day to include the end day

        for (_, day_events) in events.range(start_day_key..end_day_key) {
            for entry in day_events {
                if entry.event.timestamp >= start_time && entry.event.timestamp < end_time {
                    results.push(entry.event.clone());

                    if let Some(limit) = limit {
                        if results.len() >= limit {
                            return Ok(results);
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get schema statistics
    pub fn get_schema_stats(&self, schema_id: &SchemaId) -> Option<SchemaStats> {
        self.schema_stats
            .read()
            .get(schema_id)
            .map(|data| data.to_schema_stats())
    }

    /// Get all schema statistics
    pub fn get_all_schema_stats(&self) -> Vec<SchemaStats> {
        self.schema_stats
            .read()
            .values()
            .map(|data| data.to_schema_stats())
            .collect()
    }

    /// Get top schemas by operation count
    pub fn get_top_schemas(
        &self,
        operation: Option<Operation>,
        limit: usize,
    ) -> Vec<TopSchemaEntry> {
        let schema_stats = self.schema_stats.read();

        let mut schemas: Vec<_> = schema_stats
            .values()
            .map(|data| {
                let value = match operation {
                    Some(Operation::Read) => data.read_count,
                    Some(Operation::Write) => data.write_count,
                    Some(Operation::Validate) => data.validation_count,
                    Some(Operation::CheckCompatibility) => data.compatibility_check_count,
                    Some(Operation::Delete) => data.delete_count,
                    _ => data.total_operations,
                };
                (data.schema_id.clone(), value)
            })
            .collect();

        // Sort by value descending
        schemas.sort_by(|a, b| b.1.cmp(&a.1));

        schemas
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(idx, (schema_id, value))| TopSchemaEntry {
                schema_id,
                value,
                rank: idx + 1,
                trend: None, // Trend calculation requires historical data
            })
            .collect()
    }

    /// Get zombie schemas (unused for a long time)
    pub fn get_zombie_schemas(&self, inactive_days: i64) -> Vec<SchemaId> {
        let cutoff = Utc::now() - chrono::Duration::days(inactive_days);

        self.schema_stats
            .read()
            .values()
            .filter(|data| data.last_accessed < cutoff)
            .map(|data| data.schema_id.clone())
            .collect()
    }

    /// Get trending schemas
    pub fn get_trending_schemas(
        &self,
        _current_period: (DateTime<Utc>, DateTime<Utc>),
        _previous_period: (DateTime<Utc>, DateTime<Utc>),
        limit: usize,
    ) -> Result<Vec<SchemaTrend>> {
        // This is a simplified version that uses current stats
        // In production, would query aggregated data for both periods

        let schema_stats = self.schema_stats.read();

        let mut trends: Vec<_> = schema_stats
            .values()
            .map(|data| {
                // Simplified: use total operations as proxy
                // In production: would compare actual period data
                let current_value = data.total_operations;
                let previous_value = current_value.saturating_sub(current_value / 10); // Simulate 10% less

                let change = current_value as i64 - previous_value as i64;
                let change_percent = if previous_value > 0 {
                    (change as f64 / previous_value as f64) * 100.0
                } else {
                    0.0
                };

                let direction = if change_percent > 10.0 {
                    TrendDirection::Up
                } else if change_percent < -10.0 {
                    TrendDirection::Down
                } else {
                    TrendDirection::Stable
                };

                SchemaTrend {
                    schema_id: data.schema_id.clone(),
                    current_value,
                    previous_value,
                    change_percent,
                    direction,
                }
            })
            .collect();

        // Sort by absolute change percent descending
        trends.sort_by(|a, b| {
            b.change_percent.abs().partial_cmp(&a.change_percent.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(trends.into_iter().take(limit).collect())
    }

    /// Clean up old data based on retention policy
    pub fn cleanup(&self) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(self.config.detailed_retention_days);
        let cutoff_timestamp = cutoff.timestamp();

        let mut events = self.events.write();
        let initial_count: usize = events.values().map(|v| v.len()).sum();

        events.retain(|&day_key, _| day_key >= cutoff_timestamp);

        let final_count: usize = events.values().map(|v| v.len()).sum();
        let removed = initial_count.saturating_sub(final_count);

        if removed > 0 {
            info!(
                removed = removed,
                retention_days = self.config.detailed_retention_days,
                "Cleaned up old events"
            );
        }

        Ok(removed)
    }

    /// Get storage statistics
    pub fn get_storage_stats(&self) -> StorageStats {
        let events = self.events.read();
        let total_events: usize = events.values().map(|v| v.len()).sum();

        StorageStats {
            total_events,
            total_schemas: self.schema_stats.read().len(),
            total_clients: self.clients.read().len(),
            oldest_event: events.keys().next().and_then(|&ts| {
                DateTime::from_timestamp(ts, 0)
            }),
            newest_event: events.values()
                .last()
                .and_then(|v| v.last())
                .map(|e| e.event.timestamp),
        }
    }

    /// Clear all data (useful for testing)
    #[cfg(test)]
    pub fn clear(&self) {
        self.events.write().clear();
        self.schema_stats.write().clear();
        self.clients.write().clear();
    }
}

impl Default for AnalyticsStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_events: usize,
    pub total_schemas: usize,
    pub total_clients: usize,
    pub oldest_event: Option<DateTime<Utc>>,
    pub newest_event: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_store_and_retrieve_event() {
        let storage = AnalyticsStorage::new();

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );

        let event_timestamp = event.timestamp;
        storage.store_event(event.clone()).unwrap();

        let start = event_timestamp - chrono::Duration::hours(1);
        let end = event_timestamp + chrono::Duration::hours(1);

        let events = storage.get_events(start, end, None).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, event.event_id);
    }

    #[test]
    fn test_schema_stats_update() {
        let storage = AnalyticsStorage::new();
        let schema_id = Uuid::new_v4();

        for i in 0..10 {
            let event = SchemaUsageEvent::new(
                schema_id,
                Operation::Read,
                format!("client-{}", i),
                "us-west-1".to_string(),
                100,
                true,
            );
            storage.store_event(event).unwrap();
        }

        let stats = storage.get_schema_stats(&schema_id.into()).unwrap();
        assert_eq!(stats.total_operations, 10);
        assert_eq!(stats.read_count, 10);
        assert_eq!(stats.unique_clients, 10);
    }

    #[test]
    fn test_top_schemas() {
        let storage = AnalyticsStorage::new();
        let schema1 = Uuid::new_v4();
        let schema2 = Uuid::new_v4();

        // Schema1: 5 reads
        for _ in 0..5 {
            let event = SchemaUsageEvent::new(
                schema1,
                Operation::Read,
                "client-1".to_string(),
                "us-west-1".to_string(),
                100,
                true,
            );
            storage.store_event(event).unwrap();
        }

        // Schema2: 10 reads
        for _ in 0..10 {
            let event = SchemaUsageEvent::new(
                schema2,
                Operation::Read,
                "client-1".to_string(),
                "us-west-1".to_string(),
                100,
                true,
            );
            storage.store_event(event).unwrap();
        }

        let top = storage.get_top_schemas(Some(Operation::Read), 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].rank, 1);
        assert_eq!(top[0].value, 10); // schema2
        assert_eq!(top[1].rank, 2);
        assert_eq!(top[1].value, 5); // schema1
    }

    #[test]
    fn test_zombie_schemas() {
        let storage = AnalyticsStorage::new();
        let schema_id = Uuid::new_v4();

        let old_event = SchemaUsageEvent {
            event_id: Uuid::new_v4(),
            schema_id: schema_id.into(),
            operation: Operation::Read,
            timestamp: Utc::now() - chrono::Duration::days(100),
            client_id: "client-1".to_string(),
            region: "us-west-1".to_string(),
            latency_ms: 100,
            success: true,
            error_message: None,
            metadata: HashMap::new(),
        };

        storage.store_event(old_event).unwrap();

        let zombies = storage.get_zombie_schemas(90);
        assert_eq!(zombies.len(), 1);
    }

    #[test]
    fn test_cleanup() {
        let mut config = StorageConfig::default();
        config.detailed_retention_days = 1; // Very short retention

        let storage = AnalyticsStorage::with_config(config);

        let old_event = SchemaUsageEvent {
            event_id: Uuid::new_v4(),
            schema_id: Uuid::new_v4().into(),
            operation: Operation::Read,
            timestamp: Utc::now() - chrono::Duration::days(2),
            client_id: "client-1".to_string(),
            region: "us-west-1".to_string(),
            latency_ms: 100,
            success: true,
            error_message: None,
            metadata: HashMap::new(),
        };

        storage.store_event(old_event).unwrap();

        let stats_before = storage.get_storage_stats();
        assert_eq!(stats_before.total_events, 1);

        let removed = storage.cleanup().unwrap();
        assert_eq!(removed, 1);

        let stats_after = storage.get_storage_stats();
        assert_eq!(stats_after.total_events, 0);
    }

    #[test]
    fn test_storage_stats() {
        let storage = AnalyticsStorage::new();

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );

        storage.store_event(event).unwrap();

        let stats = storage.get_storage_stats();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.total_schemas, 1);
        assert_eq!(stats.total_clients, 1);
        assert!(stats.newest_event.is_some());
    }
}
