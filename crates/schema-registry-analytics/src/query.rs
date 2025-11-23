//! Query API for analytics data
//!
//! This module provides a high-level query interface for retrieving
//! and filtering analytics data with support for complex queries.

use crate::aggregator::DataAggregator;
use crate::error::{AnalyticsError, Result};
use crate::storage::AnalyticsStorage;
use crate::types::{
    AnalyticsQuery, Operation, SchemaId, SchemaStats, SchemaUsageEvent, TimePeriod,
    TopSchemaEntry, UsageStats,
};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

/// Query executor for analytics data
pub struct QueryExecutor {
    storage: Arc<AnalyticsStorage>,
    aggregator: Arc<DataAggregator>,
}

impl QueryExecutor {
    /// Create a new query executor
    pub fn new(storage: Arc<AnalyticsStorage>, aggregator: Arc<DataAggregator>) -> Self {
        Self {
            storage,
            aggregator,
        }
    }

    /// Execute a usage statistics query
    pub fn query_usage_stats(&self, query: &AnalyticsQuery) -> Result<Vec<UsageStats>> {
        self.validate_query(query)?;

        let period = query.aggregation_period.unwrap_or(TimePeriod::Hour1);

        // If schema IDs specified, query each one
        if let Some(schema_ids) = &query.schema_ids {
            let mut all_stats = Vec::new();

            for schema_id in schema_ids {
                let stats = self.aggregator.get_stats(
                    period,
                    query.start_time,
                    query.end_time,
                    Some(schema_id.clone()),
                )?;

                all_stats.extend(stats);
            }

            Ok(self.apply_limit_offset(all_stats, query))
        } else {
            // Global query
            let stats = self.aggregator.get_stats(
                period,
                query.start_time,
                query.end_time,
                None,
            )?;

            Ok(self.apply_limit_offset(stats, query))
        }
    }

    /// Query raw events
    pub fn query_events(&self, query: &AnalyticsQuery) -> Result<Vec<SchemaUsageEvent>> {
        self.validate_query(query)?;

        let mut events = self.storage.get_events(
            query.start_time,
            query.end_time,
            query.limit,
        )?;

        // Apply filters
        events.retain(|event| self.event_matches_query(event, query));

        // Apply offset if specified
        if let Some(offset) = query.offset {
            if offset < events.len() {
                events = events.into_iter().skip(offset).collect();
            } else {
                return Ok(Vec::new());
            }
        }

        Ok(events)
    }

    /// Query schema statistics
    pub fn query_schema_stats(&self, schema_ids: Option<Vec<SchemaId>>) -> Vec<SchemaStats> {
        match schema_ids {
            Some(ids) => ids
                .iter()
                .filter_map(|id| self.storage.get_schema_stats(id))
                .collect(),
            None => self.storage.get_all_schema_stats(),
        }
    }

    /// Query top schemas
    pub fn query_top_schemas(
        &self,
        operation: Option<Operation>,
        limit: usize,
    ) -> Vec<TopSchemaEntry> {
        self.storage.get_top_schemas(operation, limit)
    }

    /// Query for a specific time period (e.g., last 24 hours)
    pub fn query_recent(
        &self,
        duration: Duration,
        period: TimePeriod,
    ) -> Result<Vec<UsageStats>> {
        let end_time = Utc::now();
        let start_time = end_time - duration;

        self.aggregator.get_stats(period, start_time, end_time, None)
    }

    /// Query latest statistics
    pub fn query_latest(&self, period: TimePeriod) -> Option<UsageStats> {
        self.aggregator.get_latest_stats(period, None)
    }

    /// Query schema-specific latest statistics
    pub fn query_latest_for_schema(
        &self,
        schema_id: SchemaId,
        period: TimePeriod,
    ) -> Option<UsageStats> {
        self.aggregator.get_latest_stats(period, Some(schema_id))
    }

    /// Validate query parameters
    fn validate_query(&self, query: &AnalyticsQuery) -> Result<()> {
        if query.start_time >= query.end_time {
            return Err(AnalyticsError::InvalidTimeRange {
                start: query.start_time.to_rfc3339(),
                end: query.end_time.to_rfc3339(),
            });
        }

        // Validate reasonable time range (max 1 year)
        let max_range = Duration::days(365);
        if query.end_time - query.start_time > max_range {
            return Err(AnalyticsError::invalid_parameter(
                "Time range exceeds maximum of 365 days",
            ));
        }

        Ok(())
    }

    /// Check if an event matches query filters
    fn event_matches_query(&self, event: &SchemaUsageEvent, query: &AnalyticsQuery) -> bool {
        // Filter by schema IDs
        if let Some(schema_ids) = &query.schema_ids {
            if !schema_ids.contains(&event.schema_id) {
                return false;
            }
        }

        // Filter by operations
        if let Some(operations) = &query.operations {
            if !operations.contains(&event.operation) {
                return false;
            }
        }

        // Filter by regions
        if let Some(regions) = &query.regions {
            if !regions.contains(&event.region) {
                return false;
            }
        }

        // Filter by client IDs
        if let Some(client_ids) = &query.client_ids {
            if !client_ids.contains(&event.client_id) {
                return false;
            }
        }

        // Filter by success
        if let Some(success_only) = query.success_only {
            if success_only && !event.success {
                return false;
            }
        }

        true
    }

    /// Apply limit and offset to results
    fn apply_limit_offset<T>(&self, mut items: Vec<T>, query: &AnalyticsQuery) -> Vec<T> {
        // Apply offset
        if let Some(offset) = query.offset {
            if offset < items.len() {
                items = items.into_iter().skip(offset).collect();
            } else {
                return Vec::new();
            }
        }

        // Apply limit
        if let Some(limit) = query.limit {
            items.truncate(limit);
        }

        items
    }
}

/// Query builder for fluent query construction
pub struct QueryBuilder {
    query: AnalyticsQuery,
}

impl QueryBuilder {
    /// Create a new query builder for a time range
    pub fn new(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            query: AnalyticsQuery::new(start_time, end_time),
        }
    }

    /// Create a query for the last N hours
    pub fn last_hours(hours: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::hours(hours);
        Self::new(start, end)
    }

    /// Create a query for the last N days
    pub fn last_days(days: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::days(days);
        Self::new(start, end)
    }

    /// Create a query for today
    pub fn today() -> Self {
        let now = Utc::now();
        let start = now.date_naive().and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        Self::new(start, now)
    }

    /// Filter by schema IDs
    pub fn schemas(mut self, schema_ids: Vec<SchemaId>) -> Self {
        self.query.schema_ids = Some(schema_ids);
        self
    }

    /// Filter by a single schema
    pub fn schema(mut self, schema_id: impl Into<SchemaId>) -> Self {
        self.query.schema_ids = Some(vec![schema_id.into()]);
        self
    }

    /// Filter by operations
    pub fn operations(mut self, operations: Vec<Operation>) -> Self {
        self.query.operations = Some(operations);
        self
    }

    /// Filter by a single operation
    pub fn operation(mut self, operation: Operation) -> Self {
        self.query.operations = Some(vec![operation]);
        self
    }

    /// Filter by regions
    pub fn regions(mut self, regions: Vec<String>) -> Self {
        self.query.regions = Some(regions);
        self
    }

    /// Filter by a single region
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.query.regions = Some(vec![region.into()]);
        self
    }

    /// Filter by client IDs
    pub fn clients(mut self, client_ids: Vec<String>) -> Self {
        self.query.client_ids = Some(client_ids);
        self
    }

    /// Filter by successful operations only
    pub fn success_only(mut self) -> Self {
        self.query.success_only = Some(true);
        self
    }

    /// Set aggregation period
    pub fn aggregate_by(mut self, period: TimePeriod) -> Self {
        self.query.aggregation_period = Some(period);
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.query.limit = Some(limit);
        self
    }

    /// Set offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.query.offset = Some(offset);
        self
    }

    /// Build the query
    pub fn build(self) -> AnalyticsQuery {
        self.query
    }

    /// Execute the query with the given executor
    pub fn execute(self, executor: &QueryExecutor) -> Result<Vec<UsageStats>> {
        executor.query_usage_stats(&self.query)
    }

    /// Execute as event query
    pub fn execute_events(self, executor: &QueryExecutor) -> Result<Vec<SchemaUsageEvent>> {
        executor.query_events(&self.query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregator::DataAggregator;
    use crate::storage::AnalyticsStorage;
    use uuid::Uuid;

    fn setup() -> QueryExecutor {
        let storage = Arc::new(AnalyticsStorage::new());
        let aggregator = Arc::new(DataAggregator::new());
        QueryExecutor::new(storage, aggregator)
    }

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::last_hours(24)
            .operation(Operation::Read)
            .region("us-west-1")
            .success_only()
            .limit(100)
            .build();

        assert_eq!(query.operations, Some(vec![Operation::Read]));
        assert_eq!(query.regions, Some(vec!["us-west-1".to_string()]));
        assert_eq!(query.success_only, Some(true));
        assert_eq!(query.limit, Some(100));
    }

    #[test]
    fn test_query_builder_today() {
        let query = QueryBuilder::today().build();

        let now = Utc::now();
        assert!(query.start_time.date_naive() == now.date_naive());
        assert!(query.end_time <= now);
    }

    #[test]
    fn test_query_validation() {
        let executor = setup();

        // Invalid time range
        let query = AnalyticsQuery::new(Utc::now(), Utc::now() - Duration::hours(1));
        assert!(executor.query_usage_stats(&query).is_err());

        // Too large time range
        let query = AnalyticsQuery::new(
            Utc::now() - Duration::days(400),
            Utc::now(),
        );
        assert!(executor.query_usage_stats(&query).is_err());
    }

    #[test]
    fn test_event_matches_query() {
        let executor = setup();
        let schema_id = Uuid::new_v4();

        let event = SchemaUsageEvent::new(
            schema_id,
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );

        let query = QueryBuilder::last_hours(1)
            .operation(Operation::Read)
            .success_only()
            .build();

        assert!(executor.event_matches_query(&event, &query));

        // Should not match write operation
        let query = QueryBuilder::last_hours(1)
            .operation(Operation::Write)
            .build();

        assert!(!executor.event_matches_query(&event, &query));
    }

    #[test]
    fn test_query_recent() {
        let executor = setup();

        let result = executor.query_recent(Duration::hours(1), TimePeriod::Minute1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_latest() {
        let executor = setup();

        // Should return None if no data
        let result = executor.query_latest(TimePeriod::Minute1);
        assert!(result.is_none());
    }
}
