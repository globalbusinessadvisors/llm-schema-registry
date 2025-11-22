# Production-Grade Observability for LLM Schema Registry

**Complete monitoring, logging, tracing, and alerting implementation**

## Overview

The LLM Schema Registry implements comprehensive production-grade observability covering:

- **Metrics**: 40+ Prometheus metrics (RED + USE + Business metrics)
- **Tracing**: OpenTelemetry distributed tracing with Jaeger
- **Logging**: Structured JSON logging with correlation IDs
- **Dashboards**: 10+ Grafana dashboards
- **Alerting**: 25+ alert rules with runbooks
- **SLI/SLO**: Service Level Indicators and Objectives with error budgets

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  Observability Stack                         │
│                                                               │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐ │
│  │   Metrics      │  │    Traces      │  │     Logs       │ │
│  │  (Prometheus)  │  │   (Jaeger)     │  │    (Loki)      │ │
│  └────────────────┘  └────────────────┘  └────────────────┘ │
│          │                   │                   │            │
│          └───────────────────┴───────────────────┘            │
│                             │                                 │
│                     ┌───────────────┐                         │
│                     │    Grafana    │                         │
│                     │  (Dashboards) │                         │
│                     └───────────────┘                         │
│                             │                                 │
│                     ┌───────────────┐                         │
│                     │  AlertManager │                         │
│                     │   (Alerts)    │                         │
│                     └───────────────┘                         │
└─────────────────────────────────────────────────────────────┘
```

## Table of Contents

1. [Metrics](#metrics)
2. [Distributed Tracing](#distributed-tracing)
3. [Structured Logging](#structured-logging)
4. [Dashboards](#dashboards)
5. [Alerting](#alerting)
6. [SLI/SLO Monitoring](#slislo-monitoring)
7. [Deployment](#deployment)
8. [Usage Examples](#usage-examples)
9. [Troubleshooting](#troubleshooting)

## Metrics

### Metrics Categories

The Schema Registry exposes 40+ metrics across 7 categories:

#### 1. Request Metrics (RED Metrics)

```promql
# Request rate
sum(rate(schema_registry_http_requests_total[5m])) by (path)

# Error rate
sum(rate(schema_registry_http_requests_total{status=~"5.."}[5m]))
/
sum(rate(schema_registry_http_requests_total[5m]))

# Request duration (p95)
histogram_quantile(0.95,
  sum(rate(schema_registry_http_request_duration_seconds_bucket[5m])) by (le)
)
```

**Metrics:**
- `schema_registry_http_requests_total` - Total HTTP requests (counter)
- `schema_registry_http_request_duration_seconds` - Request duration (histogram)
- `schema_registry_http_requests_in_flight` - Current requests (gauge)
- `schema_registry_http_request_size_bytes` - Request size (histogram)
- `schema_registry_http_response_size_bytes` - Response size (histogram)

#### 2. Business Metrics

```promql
# Schemas registered per second
sum(rate(schema_registry_schemas_registered_total[5m])) by (format)

# Active schemas by format
sum(schema_registry_schemas_active_total) by (format)

# Validation success rate
sum(rate(schema_registry_validations_total{result="success"}[5m]))
/
sum(rate(schema_registry_validations_total[5m]))
```

**Metrics:**
- `schema_registry_schemas_registered_total` - Total schemas registered
- `schema_registry_schemas_active_total` - Active schemas
- `schema_registry_validations_total` - Total validations
- `schema_registry_compatibility_checks_total` - Compatibility checks

#### 3. Storage Metrics - Cache

```promql
# Cache hit rate
schema_registry_cache_hit_rate{tier="L1"}

# Cache operations
sum(rate(schema_registry_cache_operations_total[5m])) by (operation, result)
```

**Metrics:**
- `schema_registry_cache_operations_total` - Cache operations
- `schema_registry_cache_hit_rate` - Cache hit rate (0.0-1.0)
- `schema_registry_cache_size_bytes` - Cache size in bytes
- `schema_registry_cache_items_total` - Items in cache
- `schema_registry_cache_evictions_total` - Cache evictions

#### 4. Storage Metrics - Database

```promql
# Connection pool utilization
schema_registry_db_connections_active{pool="postgres"}
/
schema_registry_db_connections_max{pool="postgres"}

# Query duration (p95)
histogram_quantile(0.95,
  sum(rate(schema_registry_db_query_duration_seconds_bucket[5m])) by (le, query)
)
```

**Metrics:**
- `schema_registry_db_connections_active` - Active DB connections
- `schema_registry_db_connections_idle` - Idle DB connections
- `schema_registry_db_query_duration_seconds` - Query duration
- `schema_registry_db_errors_total` - Database errors

#### 5. Storage Metrics - Redis & S3

**Redis Metrics:**
- `schema_registry_redis_operations_total`
- `schema_registry_redis_operation_duration_seconds`
- `schema_registry_redis_errors_total`

**S3 Metrics:**
- `schema_registry_s3_operations_total`
- `schema_registry_s3_operation_duration_seconds`
- `schema_registry_s3_errors_total`
- `schema_registry_s3_bytes_transferred_total`

#### 6. System Metrics

**Metrics:**
- `schema_registry_process_cpu_seconds_total` - CPU usage
- `schema_registry_process_memory_bytes` - Memory usage
- `schema_registry_process_open_fds` - Open file descriptors
- `schema_registry_tokio_tasks_active` - Active async tasks

#### 7. gRPC Metrics

**Metrics:**
- `schema_registry_grpc_requests_total`
- `schema_registry_grpc_request_duration_seconds`
- `schema_registry_grpc_requests_in_flight`

### Metrics Endpoint

Metrics are exposed at `/metrics` in Prometheus text format:

```bash
curl http://localhost:8080/metrics
```

## Distributed Tracing

### Overview

Distributed tracing is implemented using OpenTelemetry with Jaeger as the backend.

### Configuration

```rust
use schema_registry_observability::{init_tracing, TracingConfig};

let config = TracingConfig {
    service_name: "schema-registry".to_string(),
    service_version: env!("CARGO_PKG_VERSION").to_string(),
    environment: "production".to_string(),
    otlp_endpoint: "http://jaeger:4317".to_string(),
    sampling_rate: 0.1,  // 10% sampling
    json_logs: true,
    log_level: "info".to_string(),
};

init_tracing(config)?;
```

### Environment Variables

```bash
# Tracing configuration
OTLP_ENDPOINT=http://jaeger:4317
TRACE_SAMPLING_RATE=0.1
JSON_LOGS=true
LOG_LEVEL=info
ENVIRONMENT=production
```

### Instrumenting Code

```rust
use tracing::instrument;

#[instrument(
    name = "register_schema",
    skip(state, input),
    fields(
        schema.subject = %input.subject,
        schema.format = ?input.schema_type,
    )
)]
pub async fn register_schema(
    State(state): State<AppState>,
    Json(input): Json<SchemaInput>,
) -> Result<Json<SchemaResponse>, ApiError> {
    // Function automatically traced
    let result = state.schema_service.register(input).await?;

    tracing::info!(
        schema.id = %result.id,
        schema.version = %result.version,
        "Schema registered successfully"
    );

    Ok(Json(result))
}
```

### Trace Context Propagation

Trace context is automatically propagated via HTTP headers:
- `traceparent` - W3C Trace Context
- `tracestate` - Vendor-specific trace state

```rust
use schema_registry_observability::trace_context;

// Extract context from incoming request
let context = trace_context::extract_trace_context(headers);

// Inject context into outgoing request
trace_context::inject_trace_context(&context, &mut outgoing_headers);
```

### Jaeger UI

Access Jaeger UI at: `http://localhost:16686`

**Features:**
- Search traces by service, operation, tags
- View trace timeline and spans
- Analyze latency distribution
- Find critical path in distributed calls

## Structured Logging

### JSON Log Format

All logs are output in structured JSON format:

```json
{
  "timestamp": "2025-11-22T03:00:00.000Z",
  "level": "INFO",
  "message": "Schema registered successfully",
  "target": "schema_registry_api::handlers",
  "file": "handlers.rs",
  "line": 42,
  "correlation_id": "abc123-def456-ghi789",
  "request_id": "req-uuid-here",
  "schema_id": "schema-uuid",
  "user_id": "user-123",
  "fields": {
    "schema_version": "1.0.0",
    "format": "json"
  }
}
```

### Log Context

```rust
use schema_registry_observability::LogContext;

// Create context from HTTP headers
let ctx = LogContext::from_headers(headers)
    .with_correlation_id("abc-123")
    .with_schema_id("schema-456")
    .with_field("custom_field", "value");

// Use in structured logging
tracing::info!(
    correlation_id = ?ctx.correlation_id,
    schema_id = ?ctx.schema_id,
    "Processing schema validation"
);
```

### Correlation IDs

Correlation IDs are automatically:
1. Extracted from `x-correlation-id` header (if present)
2. Generated if not present
3. Propagated to all logs and traces
4. Returned in response headers

### Log Sampling

High-volume endpoints (health checks) are sampled at 1%:

```rust
let sampling_config = LogSamplingConfig {
    sampled_paths: hashmap! {
        "/health" => 0.01,      // 1% sampling
        "/ready" => 0.01,
        "/metrics" => 0.01,
    },
    default_sample_rate: 1.0,  // 100% for other paths
};
```

## Dashboards

### Available Dashboards

1. **RED Dashboard** (`red-dashboard.json`)
   - Request Rate by endpoint
   - Error Rate percentage
   - Request Duration (p50, p95, p99)
   - Requests by Status Code
   - Requests in Flight

2. **USE Dashboard** (`use-dashboard.json`)
   - CPU Utilization
   - Memory Utilization
   - Database Connection Pool Saturation
   - Redis Connection Pool
   - Tokio Task Saturation
   - Database Errors

3. **Business Metrics Dashboard** (`business-metrics.json`)
   - Schemas Registered Rate
   - Total Active Schemas
   - Schema Formats Distribution
   - Validation Rate and Success Rate
   - Compatibility Checks
   - Cache Hit Rate
   - Top Schemas by Access

4. **SLI/SLO Dashboard** (`sli-slo-dashboard.json`)
   - Availability SLI (99.9% target)
   - Latency SLI p95 (<10ms target)
   - Error Rate SLI (<1% target)
   - Error Budget Remaining
   - Error Budget Burn Rate
   - SLO Compliance Timeline
   - Uptime (Last 30 days)

### Importing Dashboards

```bash
# Copy dashboard JSONs to Grafana provisioning directory
cp deployments/monitoring/grafana/dashboards/*.json \
   /etc/grafana/provisioning/dashboards/

# Restart Grafana
systemctl restart grafana-server
```

### Dashboard Variables

Common dashboard variables:
- `$environment` - Filter by environment (dev, staging, prod)
- `$instance` - Filter by instance
- `$interval` - Time interval for aggregation

## Alerting

### Alert Rules

25+ production-ready alert rules covering:

#### 1. SLO Violations (Critical)
- `HighErrorRate` - Error rate >5% for 5 minutes
- `HighLatencyP95` - p95 latency >10ms for 10 minutes
- `LowAvailability` - Availability <99.9%
- `FastErrorBudgetBurn` - Error budget burning at 14.4x rate
- `SlowErrorBudgetBurn` - Error budget burning at 6x rate

#### 2. Resource Saturation (Warning/Critical)
- `DatabasePoolExhausted` - DB pool >90% utilized
- `DatabasePoolCritical` - DB pool >95% utilized
- `HighMemoryUsage` - Memory >4GB
- `LowCacheHitRate` - Cache hit rate <70%
- `CriticalCacheHitRate` - Cache hit rate <50%

#### 3. Database Issues
- `SlowDatabaseQueries` - p95 query duration >50ms
- `DatabaseConnectionErrors` - DB connection errors detected
- `DatabasePoolWaitTime` - Pool wait time >100ms

#### 4. Redis/S3 Issues
- `RedisConnectionErrors`
- `SlowRedisOperations`
- `S3Errors`
- `SlowS3Operations`

#### 5. Business Metrics
- `ValidationFailureSpike` - Validation failures >10%
- `CompatibilityCheckFailures` - Compatibility failures >20%

#### 6. Security
- `HighAuthenticationFailureRate` - Possible brute force attack
- `RateLimitHit` - Rate limits being hit frequently

#### 7. Health
- `ServiceDown` - Service is down
- `HealthCheckFailing` - Health check endpoint failing

### Alert Configuration

```yaml
# File: deployments/monitoring/alerts.yaml

groups:
  - name: schema_registry_slo_violations
    interval: 30s
    rules:
      - alert: HighErrorRate
        expr: |
          (sum(rate(schema_registry_http_requests_total{status=~"5.."}[5m]))
          /
          sum(rate(schema_registry_http_requests_total[5m])))
          > 0.05
        for: 5m
        labels:
          severity: critical
          component: api
        annotations:
          summary: "Error rate exceeds 5% for 5 minutes"
          runbook: "https://runbooks.example.com/high-error-rate"
          dashboard: "https://grafana.example.com/d/slo-dashboard"
```

### AlertManager Configuration

```yaml
# File: deployments/monitoring/alertmanager.yml

route:
  group_by: ['alertname', 'severity']
  routes:
    - match:
        severity: critical
      receiver: pagerduty
      continue: true
    - match:
        severity: warning
      receiver: slack-warnings
```

### Runbooks

Each alert includes a runbook link with troubleshooting steps:

**Example Runbook Structure:**
1. **Symptoms** - What you're seeing
2. **Impact** - Business impact
3. **Diagnosis** - How to diagnose the issue
4. **Resolution** - Step-by-step fix
5. **Prevention** - How to prevent recurrence

## SLI/SLO Monitoring

### Service Level Objectives

| SLO | Target | Window | Error Budget |
|-----|--------|--------|--------------|
| **Availability** | 99.9% | 30 days | 0.1% (43.2 min) |
| **Latency (p95)** | <10ms | 30 days | N/A |
| **Latency (p99)** | <25ms | 30 days | N/A |
| **Error Rate** | <1% | 30 days | 1% |

### SLI Queries

```promql
# Availability SLI
sum(rate(schema_registry_http_requests_total{status=~"2.."}[30d]))
/
sum(rate(schema_registry_http_requests_total[30d]))

# Latency SLI (p95)
histogram_quantile(0.95,
  sum(rate(schema_registry_http_request_duration_seconds_bucket[30d])) by (le)
)

# Error Rate SLI
sum(rate(schema_registry_http_requests_total{status=~"5.."}[30d]))
/
sum(rate(schema_registry_http_requests_total[30d]))
```

### Error Budget Policy

| Remaining Budget | Action |
|------------------|--------|
| >50% | No restrictions. Full feature velocity. |
| 25%-50% | Caution. Review deployment frequency. |
| 10%-25% | Slow down. Reduce deployments. Focus on reliability. |
| <10% | Code freeze (except fixes). All hands on reliability. |
| 0% | Full stop. SLO violated. Incident declared. |

### Burn Rate Alerts

**Fast Burn (2 hour window):**
- Burn rate >14.4x triggers page
- Would exhaust budget in ~2 hours

**Slow Burn (5 hour window):**
- Burn rate >6x triggers ticket
- Would exhaust budget in ~5 hours

## Deployment

### Docker Compose (Development)

```bash
# Start monitoring stack
docker-compose -f deployments/monitoring/docker-compose.monitoring.yml up -d

# Verify services
docker-compose ps

# Access UIs
# - Prometheus: http://localhost:9090
# - Grafana: http://localhost:3000 (admin/admin)
# - Jaeger: http://localhost:16686
# - AlertManager: http://localhost:9093
```

### Kubernetes (Production)

```bash
# Apply monitoring namespace
kubectl apply -f deployments/kubernetes/monitoring/namespace.yaml

# Deploy Prometheus
kubectl apply -f deployments/kubernetes/monitoring/prometheus/

# Deploy Grafana
kubectl apply -f deployments/kubernetes/monitoring/grafana/

# Deploy Jaeger
kubectl apply -f deployments/kubernetes/monitoring/jaeger/

# Verify deployments
kubectl get pods -n monitoring
```

## Usage Examples

### 1. Initialize Observability

```rust
use schema_registry_observability::ObservabilityManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize observability
    let obs_manager = ObservabilityManager::new()?;
    obs_manager.initialize()?;

    // Your application code
    run_application().await?;

    // Shutdown observability gracefully
    obs_manager.shutdown();

    Ok(())
}
```

### 2. Add Metrics to Custom Code

```rust
use schema_registry_observability::MetricsCollector;
use std::sync::Arc;

async fn process_schema(metrics: Arc<MetricsCollector>, schema: Schema) {
    let start = Instant::now();

    // Process schema
    let result = validate_schema(&schema).await;

    // Record metrics
    metrics
        .validations_total
        .with_label_values(&["json", if result.is_ok() { "success" } else { "failure" }])
        .inc();

    metrics
        .validation_duration_seconds
        .with_label_values(&["json"])
        .observe(start.elapsed().as_secs_f64());
}
```

### 3. Add Middleware to Axum

```rust
use axum::{Router, middleware};
use schema_registry_observability::{observability_middleware, MetricsCollector};

let metrics = MetricsCollector::new()?;

let app = Router::new()
    .route("/api/v1/schemas", post(register_schema))
    .layer(middleware::from_fn(move |req, next| {
        observability_middleware(metrics.clone(), req, next)
    }));
```

## Troubleshooting

### High Memory Usage

**Symptoms:**
- `HighMemoryUsage` alert firing
- Memory >4GB

**Diagnosis:**
1. Check memory metrics: `schema_registry_process_memory_bytes`
2. Check cache size: `schema_registry_cache_size_bytes`
3. Review heap profile

**Resolution:**
1. Reduce cache size limits
2. Increase memory limits if justified
3. Check for memory leaks

### Low Cache Hit Rate

**Symptoms:**
- `LowCacheHitRate` alert firing
- Performance degradation

**Diagnosis:**
1. Check hit rate: `schema_registry_cache_hit_rate`
2. Check eviction rate: `schema_registry_cache_evictions_total`
3. Review access patterns

**Resolution:**
1. Increase cache size
2. Adjust TTL settings
3. Implement cache warming

### Database Connection Pool Exhausted

**Symptoms:**
- `DatabasePoolExhausted` alert
- High query latency
- Timeouts

**Diagnosis:**
1. Check pool utilization
2. Check wait times
3. Review active queries

**Resolution:**
1. Increase max connections
2. Optimize slow queries
3. Add connection timeout
4. Scale database

## Metrics Reference

See full metrics reference in [METRICS.md](./METRICS.md)

## Best Practices

1. **Always use correlation IDs** - Track requests across services
2. **Instrument all async functions** - Use `#[instrument]` macro
3. **Set appropriate sampling rates** - Balance observability vs overhead
4. **Monitor error budgets** - Take action before exhaustion
5. **Review dashboards regularly** - Weekly SLO reviews
6. **Update runbooks** - Keep troubleshooting guides current
7. **Test alerts** - Validate alert rules fire correctly
8. **Tune thresholds** - Adjust based on actual traffic patterns

## Support

For questions or issues:
- Documentation: `/docs/`
- Runbooks: `https://runbooks.example.com/schema-registry/`
- Slack: `#schema-registry-sre`
- On-call: PagerDuty integration
