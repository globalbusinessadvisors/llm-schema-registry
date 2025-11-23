//! Reporting system for analytics summaries
//!
//! This module provides various report generators for daily, weekly, and monthly
//! summaries, health scorecards, and anomaly detection.

use crate::error::Result;
use crate::query::QueryExecutor;
use crate::storage::AnalyticsStorage;
use crate::types::{
    Operation, SchemaHealthScore, SchemaId, SchemaTrend, TimePeriod, TopSchemaEntry,
};
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Daily usage summary report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsageSummary {
    /// Report date
    pub date: DateTime<Utc>,
    /// Total operations across all schemas
    pub total_operations: u64,
    /// Total successful operations
    pub success_count: u64,
    /// Total failed operations
    pub failure_count: u64,
    /// Overall success rate
    pub success_rate: f64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Top 10 most used schemas
    pub top_schemas: Vec<TopSchemaEntry>,
    /// Unique clients
    pub unique_clients: u64,
    /// Unique schemas accessed
    pub unique_schemas: u64,
    /// Operations by type
    pub operations_breakdown: Vec<OperationBreakdown>,
}

/// Weekly trends report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyTrendsReport {
    /// Week start date
    pub week_start: DateTime<Utc>,
    /// Week end date
    pub week_end: DateTime<Utc>,
    /// Daily summaries for the week
    pub daily_summaries: Vec<DailyUsageSummary>,
    /// Week-over-week comparison
    pub wow_change: WowChange,
    /// Trending schemas (up and down)
    pub trending_up: Vec<SchemaTrend>,
    pub trending_down: Vec<SchemaTrend>,
    /// New schemas this week
    pub new_schemas: Vec<SchemaId>,
}

/// Month-over-month changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WowChange {
    /// Change in total operations (percentage)
    pub operations_change_pct: f64,
    /// Change in success rate (absolute percentage points)
    pub success_rate_change_pct: f64,
    /// Change in average latency (percentage)
    pub latency_change_pct: f64,
}

/// Monthly aggregate report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyAggregateReport {
    /// Month and year
    pub month: String,
    /// Total operations
    pub total_operations: u64,
    /// Average daily operations
    pub avg_daily_operations: f64,
    /// Peak day operations
    pub peak_operations: u64,
    /// Peak day date
    pub peak_date: DateTime<Utc>,
    /// Overall success rate
    pub success_rate: f64,
    /// Schema growth (new schemas)
    pub new_schemas_count: usize,
    /// Top performers
    pub top_schemas: Vec<TopSchemaEntry>,
}

/// Operation breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationBreakdown {
    /// Operation type
    pub operation: Operation,
    /// Count
    pub count: u64,
    /// Percentage of total
    pub percentage: f64,
}

/// Anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// When the anomaly was detected
    pub detected_at: DateTime<Utc>,
    /// Type of anomaly
    pub anomaly_type: AnomalyType,
    /// Severity
    pub severity: AnomalySeverity,
    /// Description
    pub description: String,
    /// Related schema (if applicable)
    pub schema_id: Option<SchemaId>,
    /// Metric value
    pub value: f64,
    /// Expected value or threshold
    pub threshold: f64,
}

/// Type of anomaly
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnomalyType {
    /// Spike in error rate
    ErrorRateSpike,
    /// Spike in latency
    LatencySpike,
    /// Drop in traffic
    TrafficDrop,
    /// Unusual operation count
    UnusualOperationCount,
}

/// Anomaly severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnomalySeverity {
    /// Critical - immediate attention required
    Critical,
    /// Warning - should be investigated
    Warning,
    /// Info - for awareness
    Info,
}

/// Report generator
pub struct ReportGenerator {
    query_executor: Arc<QueryExecutor>,
    storage: Arc<AnalyticsStorage>,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new(query_executor: Arc<QueryExecutor>, storage: Arc<AnalyticsStorage>) -> Self {
        Self {
            query_executor,
            storage,
        }
    }

    /// Generate daily usage summary
    pub fn generate_daily_summary(&self, date: DateTime<Utc>) -> Result<DailyUsageSummary> {
        let start = date.date_naive().and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let _end = start + Duration::days(1);

        // Get hourly stats for the day
        let stats = self.query_executor.query_recent(
            Duration::days(1),
            TimePeriod::Hour1,
        )?;

        // Aggregate stats
        let total_operations: u64 = stats.iter().map(|s| s.total_count).sum();
        let success_count: u64 = stats.iter().map(|s| s.success_count).sum();
        let failure_count: u64 = stats.iter().map(|s| s.failure_count).sum();

        let success_rate = if total_operations > 0 {
            success_count as f64 / total_operations as f64
        } else {
            0.0
        };

        let avg_latency = if !stats.is_empty() {
            stats.iter().map(|s| s.avg_latency_ms).sum::<f64>() / stats.len() as f64
        } else {
            0.0
        };

        // Get top schemas
        let top_schemas = self.query_executor.query_top_schemas(None, 10);

        // Get unique counts from storage
        let storage_stats = self.storage.get_storage_stats();

        // Operation breakdown
        let mut operations_breakdown = Vec::new();
        if let Some(latest) = stats.last() {
            for (op, op_stats) in &latest.operations {
                operations_breakdown.push(OperationBreakdown {
                    operation: *op,
                    count: op_stats.count,
                    percentage: if total_operations > 0 {
                        (op_stats.count as f64 / total_operations as f64) * 100.0
                    } else {
                        0.0
                    },
                });
            }
        }

        Ok(DailyUsageSummary {
            date,
            total_operations,
            success_count,
            failure_count,
            success_rate,
            avg_latency_ms: avg_latency,
            top_schemas,
            unique_clients: storage_stats.total_clients as u64,
            unique_schemas: storage_stats.total_schemas as u64,
            operations_breakdown,
        })
    }

    /// Generate weekly trends report
    pub fn generate_weekly_report(&self, week_start: DateTime<Utc>) -> Result<WeeklyTrendsReport> {
        let week_end = week_start + Duration::days(7);

        // Generate daily summaries for each day
        let mut daily_summaries = Vec::new();
        for i in 0..7 {
            let day = week_start + Duration::days(i);
            if let Ok(summary) = self.generate_daily_summary(day) {
                daily_summaries.push(summary);
            }
        }

        // Calculate week-over-week changes (simplified)
        let wow_change = WowChange {
            operations_change_pct: 0.0, // Would compare with previous week
            success_rate_change_pct: 0.0,
            latency_change_pct: 0.0,
        };

        // Get trending schemas
        let trending = self.storage.get_trending_schemas(
            (week_start, week_end),
            (week_start - Duration::days(7), week_start),
            10,
        )?;

        let trending_up: Vec<_> = trending
            .iter()
            .filter(|t| t.direction == crate::types::TrendDirection::Up)
            .cloned()
            .collect();

        let trending_down: Vec<_> = trending
            .iter()
            .filter(|t| t.direction == crate::types::TrendDirection::Down)
            .cloned()
            .collect();

        Ok(WeeklyTrendsReport {
            week_start,
            week_end,
            daily_summaries,
            wow_change,
            trending_up,
            trending_down,
            new_schemas: Vec::new(), // Would track new schemas
        })
    }

    /// Generate monthly aggregate report
    pub fn generate_monthly_report(&self, month: DateTime<Utc>) -> Result<MonthlyAggregateReport> {
        let month_start = month.date_naive()
            .with_day(1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        let next_month = if month_start.month() == 12 {
            month_start
                .with_year(month_start.year() + 1)
                .unwrap()
                .with_month(1)
                .unwrap()
        } else {
            month_start.with_month(month_start.month() + 1).unwrap()
        };

        // Get daily stats
        let stats = self.query_executor.query_recent(
            next_month - month_start,
            TimePeriod::Day1,
        )?;

        let total_operations: u64 = stats.iter().map(|s| s.total_count).sum();
        let avg_daily = total_operations as f64 / stats.len().max(1) as f64;

        let (peak_operations, peak_date) = stats
            .iter()
            .map(|s| (s.total_count, s.window_start))
            .max_by_key(|(count, _)| *count)
            .unwrap_or((0, month_start));

        let success_count: u64 = stats.iter().map(|s| s.success_count).sum();
        let success_rate = if total_operations > 0 {
            success_count as f64 / total_operations as f64
        } else {
            0.0
        };

        let top_schemas = self.query_executor.query_top_schemas(None, 10);

        Ok(MonthlyAggregateReport {
            month: month_start.format("%Y-%m").to_string(),
            total_operations,
            avg_daily_operations: avg_daily,
            peak_operations,
            peak_date,
            success_rate,
            new_schemas_count: 0, // Would track new schemas
            top_schemas,
        })
    }

    /// Generate schema health scorecard
    pub fn generate_health_scorecard(&self, schema_id: &SchemaId) -> Option<SchemaHealthScore> {
        let stats = self.storage.get_schema_stats(schema_id)?;

        // Calculate scores (0-100)
        let success_rate_score = (stats.success_rate * 100.0) as u8;

        // Performance score based on latency (lower is better)
        // Assuming 100ms is excellent, 500ms is acceptable
        let performance_score = if stats.avg_latency_ms <= 100.0 {
            100
        } else if stats.avg_latency_ms >= 500.0 {
            50
        } else {
            (100.0 - ((stats.avg_latency_ms - 100.0) / 400.0 * 50.0)) as u8
        };

        // Activity score based on recent usage
        let days_since_last_access = (Utc::now() - stats.last_accessed).num_days();
        let activity_score = if days_since_last_access == 0 {
            100
        } else if days_since_last_access >= 30 {
            0
        } else {
            (100 - (days_since_last_access * 100 / 30)) as u8
        };

        // Overall score (weighted average)
        let overall_score = ((success_rate_score as u16 * 40
            + performance_score as u16 * 30
            + activity_score as u16 * 30)
            / 100) as u8;

        let is_zombie = days_since_last_access > 90;

        let mut recommendations = Vec::new();
        if success_rate_score < 95 {
            recommendations.push("Investigate validation failures".to_string());
        }
        if performance_score < 70 {
            recommendations.push("Review schema complexity and validation logic".to_string());
        }
        if is_zombie {
            recommendations.push("Consider deprecating or archiving this schema".to_string());
        }

        Some(SchemaHealthScore {
            schema_id: schema_id.clone(),
            overall_score,
            success_rate_score,
            performance_score,
            activity_score,
            is_zombie,
            days_since_last_access,
            recommendations,
        })
    }

    /// Detect anomalies in recent data
    pub fn detect_anomalies(&self, lookback_hours: i64) -> Result<Vec<Anomaly>> {
        let stats = self.query_executor.query_recent(
            Duration::hours(lookback_hours),
            TimePeriod::Hour1,
        )?;

        let mut anomalies = Vec::new();

        for stat in &stats {
            // Error rate spike detection
            if stat.total_count > 0 && stat.success_rate < 0.90 {
                let error_rate = 1.0 - stat.success_rate;
                if error_rate > 0.10 {
                    // More than 10% errors
                    let severity = if error_rate > 0.50 {
                        AnomalySeverity::Critical
                    } else if error_rate > 0.25 {
                        AnomalySeverity::Warning
                    } else {
                        AnomalySeverity::Info
                    };

                    anomalies.push(Anomaly {
                        detected_at: stat.window_start,
                        anomaly_type: AnomalyType::ErrorRateSpike,
                        severity,
                        description: format!(
                            "Error rate of {:.1}% detected",
                            error_rate * 100.0
                        ),
                        schema_id: None,
                        value: error_rate,
                        threshold: 0.10,
                    });
                }
            }

            // Latency spike detection
            if stat.p95_latency_ms > 1000 {
                // P95 > 1 second
                let severity = if stat.p95_latency_ms > 5000 {
                    AnomalySeverity::Critical
                } else if stat.p95_latency_ms > 2000 {
                    AnomalySeverity::Warning
                } else {
                    AnomalySeverity::Info
                };

                anomalies.push(Anomaly {
                    detected_at: stat.window_start,
                    anomaly_type: AnomalyType::LatencySpike,
                    severity,
                    description: format!(
                        "P95 latency of {}ms detected",
                        stat.p95_latency_ms
                    ),
                    schema_id: None,
                    value: stat.p95_latency_ms as f64,
                    threshold: 1000.0,
                });
            }
        }

        Ok(anomalies)
    }

    /// Export report to JSON
    pub fn export_to_json<T: Serialize>(&self, report: &T) -> Result<String> {
        serde_json::to_string_pretty(report)
            .map_err(|e| crate::error::AnalyticsError::Serialization(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregator::DataAggregator;
    use crate::storage::AnalyticsStorage;
    use crate::types::SchemaUsageEvent;
    use uuid::Uuid;

    fn setup() -> ReportGenerator {
        let storage = Arc::new(AnalyticsStorage::new());
        let aggregator = Arc::new(DataAggregator::new());
        let query_executor = Arc::new(QueryExecutor::new(
            storage.clone(),
            aggregator.clone(),
        ));

        ReportGenerator::new(query_executor, storage)
    }

    #[test]
    fn test_daily_summary_generation() {
        let generator = setup();
        let today = Utc::now();

        let result = generator.generate_daily_summary(today);
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert_eq!(summary.date.date_naive(), today.date_naive());
    }

    #[test]
    fn test_health_scorecard() {
        let generator = setup();
        let schema_id = Uuid::new_v4();

        // Add some events
        for i in 0..10 {
            let event = SchemaUsageEvent::new(
                schema_id,
                Operation::Read,
                "client-1".to_string(),
                "us-west-1".to_string(),
                50 + i * 10,
                i < 9, // 90% success rate
            );
            generator.storage.store_event(event).unwrap();
        }

        let scorecard = generator.generate_health_scorecard(&schema_id.into());
        assert!(scorecard.is_some());

        let scorecard = scorecard.unwrap();
        assert!(scorecard.overall_score > 0);
        assert!(scorecard.success_rate_score >= 80); // ~90% success
    }

    #[test]
    fn test_anomaly_detection() {
        let generator = setup();

        let result = generator.detect_anomalies(24);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_to_json() {
        let generator = setup();
        let summary = DailyUsageSummary {
            date: Utc::now(),
            total_operations: 1000,
            success_count: 950,
            failure_count: 50,
            success_rate: 0.95,
            avg_latency_ms: 125.5,
            top_schemas: Vec::new(),
            unique_clients: 10,
            unique_schemas: 25,
            operations_breakdown: Vec::new(),
        };

        let json = generator.export_to_json(&summary);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("total_operations"));
    }
}
