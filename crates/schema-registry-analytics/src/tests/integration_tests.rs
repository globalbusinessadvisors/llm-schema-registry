//! Integration tests for the analytics engine
//!
//! These tests verify the end-to-end functionality of the analytics system.

use crate::{
    AnalyticsEngine, Operation, QueryBuilder, SchemaUsageEvent, TimePeriod,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

#[tokio::test]
async fn test_end_to_end_event_flow() {
    let engine = AnalyticsEngine::new();
    engine.start().await.unwrap();

    let schema_id = Uuid::new_v4();

    // Record a batch of events
    for i in 0..20 {
        let event = SchemaUsageEvent::new(
            schema_id,
            Operation::Read,
            format!("client-{}", i % 5),
            if i % 2 == 0 { "us-west-1" } else { "us-east-1" }.to_string(),
            100 + i * 10,
            i % 10 != 0, // 90% success rate
        );
        engine.record_event(event).unwrap();
    }

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Verify data is stored
    let stats = engine.get_schema_stats(&schema_id.into());
    assert!(stats.is_some());

    let stats = stats.unwrap();
    assert_eq!(stats.total_operations, 20);
    assert_eq!(stats.read_count, 20);
    assert!(stats.unique_clients > 0);

    // Verify aggregations
    let now = Utc::now();
    let start = now - Duration::hours(1);

    let agg_stats = engine.get_usage_stats(
        TimePeriod::Minute1,
        start,
        now,
        Some(schema_id.into()),
    ).unwrap();

    assert!(!agg_stats.is_empty());

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_multi_schema_tracking() {
    let engine = AnalyticsEngine::new();
    engine.start().await.unwrap();

    let schema1 = Uuid::new_v4();
    let schema2 = Uuid::new_v4();
    let schema3 = Uuid::new_v4();

    // Different usage patterns
    for _ in 0..10 {
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

    for _ in 0..5 {
        let event = SchemaUsageEvent::new(
            schema2,
            Operation::Write,
            "client-2".to_string(),
            "us-east-1".to_string(),
            200,
            true,
        );
        engine.record_event(event).unwrap();
    }

    for _ in 0..15 {
        let event = SchemaUsageEvent::new(
            schema3,
            Operation::Validate,
            "client-3".to_string(),
            "eu-west-1".to_string(),
            150,
            true,
        );
        engine.record_event(event).unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Check top schemas
    let top_reads = engine.get_top_schemas(Some(Operation::Read), 3);
    let top_overall = engine.get_top_schemas(None, 3);

    assert!(!top_overall.is_empty());
    assert_eq!(top_overall[0].rank, 1);

    // Get all stats
    let all_stats = engine.get_all_schema_stats();
    assert!(all_stats.len() >= 3);

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_query_builder_integration() {
    let engine = AnalyticsEngine::new();
    engine.start().await.unwrap();

    let schema_id = Uuid::new_v4();

    // Record events with different operations
    for _ in 0..5 {
        let event = SchemaUsageEvent::new(
            schema_id,
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );
        engine.record_event(event).unwrap();
    }

    for _ in 0..3 {
        let event = SchemaUsageEvent::new(
            schema_id,
            Operation::Write,
            "client-1".to_string(),
            "us-west-1".to_string(),
            150,
            false, // failures
        );
        engine.record_event(event).unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Query with filters
    let query = QueryBuilder::last_hours(1)
        .schema(schema_id)
        .operation(Operation::Read)
        .success_only();

    let events = query.execute_events(engine.query_executor().as_ref()).unwrap();
    assert!(events.iter().all(|e| e.success && e.operation == Operation::Read));

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_performance_metrics() {
    let engine = AnalyticsEngine::new();
    engine.start().await.unwrap();

    // Record various operation types
    for i in 0..10 {
        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Validate,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100 + i * 10,
            true,
        );
        engine.record_event(event).unwrap();

        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::CheckCompatibility,
            "client-1".to_string(),
            "us-west-1".to_string(),
            200 + i * 20,
            true,
        );
        engine.record_event(event).unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let metrics = engine.get_performance_metrics();
    assert!(metrics.is_ok());

    let metrics = metrics.unwrap();
    assert!(metrics.compatibility_check_performance.check_count > 0);

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_health_scoring() {
    let engine = AnalyticsEngine::new();
    engine.start().await.unwrap();

    let schema_id = Uuid::new_v4();

    // Record events with good health
    for i in 0..100 {
        let event = SchemaUsageEvent::new(
            schema_id,
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            50 + i % 50, // Low latency
            i < 95, // 95% success rate
        );
        engine.record_event(event).unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let health = engine.get_schema_health(&schema_id.into());
    assert!(health.is_some());

    let health = health.unwrap();
    assert!(health.overall_score > 80); // Should have good health
    assert!(health.success_rate_score >= 90);
    assert!(!health.is_zombie);

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_reporting() {
    let engine = AnalyticsEngine::new();
    engine.start().await.unwrap();

    // Record events
    for _ in 0..20 {
        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );
        engine.record_event(event).unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let report_gen = engine.report_generator();

    // Daily summary
    let daily = report_gen.generate_daily_summary(Utc::now());
    assert!(daily.is_ok());

    // Anomaly detection
    let anomalies = report_gen.detect_anomalies(24);
    assert!(anomalies.is_ok());

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_event_recording() {
    let engine = AnalyticsEngine::new();
    engine.start().await.unwrap();

    let schema_id = Uuid::new_v4();

    // Spawn multiple tasks recording events concurrently
    let mut handles = vec![];

    for task_id in 0..5 {
        let engine_clone = engine.clone();
        let handle = tokio::spawn(async move {
            for i in 0..10 {
                let event = SchemaUsageEvent::new(
                    schema_id,
                    Operation::Read,
                    format!("client-{}", task_id),
                    "us-west-1".to_string(),
                    100 + i * 10,
                    true,
                );
                let _ = engine_clone.record_event(event);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify all events were recorded
    let stats = engine.get_schema_stats(&schema_id.into());
    assert!(stats.is_some());

    let stats = stats.unwrap();
    assert_eq!(stats.total_operations, 50); // 5 tasks * 10 events each

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_time_series_aggregation() {
    let engine = AnalyticsEngine::new();
    engine.start().await.unwrap();

    let schema_id = Uuid::new_v4();

    // Record events over time
    for i in 0..30 {
        let event = SchemaUsageEvent::new(
            schema_id,
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100 + i * 5,
            true,
        );
        engine.record_event(event).unwrap();

        // Small delay to spread events over time
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Query different time periods
    let now = Utc::now();
    let start = now - Duration::hours(1);

    let minute_stats = engine.get_usage_stats(
        TimePeriod::Minute1,
        start,
        now,
        Some(schema_id.into()),
    ).unwrap();

    assert!(!minute_stats.is_empty());

    // Verify aggregation
    let total_count: u64 = minute_stats.iter().map(|s| s.total_count).sum();
    assert_eq!(total_count, 30);

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_engine_stats() {
    let engine = AnalyticsEngine::new();
    engine.start().await.unwrap();

    // Record some events
    for _ in 0..10 {
        let event = SchemaUsageEvent::new(
            Uuid::new_v4(),
            Operation::Read,
            "client-1".to_string(),
            "us-west-1".to_string(),
            100,
            true,
        );
        engine.record_event(event).unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let stats = engine.get_engine_stats();
    assert!(stats.active_subscribers >= 0);
    assert!(stats.total_events_stored > 0);
    assert!(stats.total_schemas_tracked > 0);

    engine.shutdown().await.unwrap();
}

// Helper to make engine cloneable for tests
impl Clone for AnalyticsEngine {
    fn clone(&self) -> Self {
        Self {
            event_bus: self.event_bus.clone(),
            aggregator: self.aggregator.clone(),
            storage: self.storage.clone(),
            query_executor: self.query_executor.clone(),
            report_generator: self.report_generator.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
            shutdown_rx: self.shutdown_rx.clone(),
            config: self.config.clone(),
        }
    }
}
