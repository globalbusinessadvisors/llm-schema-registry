# LLM-Schema-Registry: Integration Architecture

## System Context Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                      LLM DevOps Platform                            │
│                                                                     │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐        │
│  │LLM-Observatory│    │ LLM-Sentinel │    │ LLM-CostOps  │        │
│  │   (Monitor)   │    │  (Security)  │    │   (Finance)  │        │
│  └───────┬───────┘    └──────┬───────┘    └──────┬───────┘        │
│          │                   │                   │                 │
│          │ validate          │ validate          │ validate        │
│          │ telemetry         │ security          │ cost            │
│          │ schemas           │ schemas           │ schemas         │
│          ▼                   ▼                   ▼                 │
│  ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓       │
│  ┃           LLM-Schema-Registry (Core Service)            ┃       │
│  ┃  • Schema Registration & Retrieval                       ┃       │
│  ┃  • Compatibility Validation (8 modes)                    ┃       │
│  ┃  • Lifecycle Management (Draft→Active→Deprecated→Archive)┃       │
│  ┃  • Data Contract Enforcement (CEL rules)                 ┃       │
│  ┗━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛       │
│                         │                                           │
│          ┌──────────────┴──────────────┐                           │
│          │ expose schemas               │ expose schemas           │
│          ▼                              ▼                           │
│  ┌──────────────┐              ┌──────────────────┐               │
│  │LLM-Analytics │              │LLM-Governance    │               │
│  │     Hub      │              │   Dashboard      │               │
│  │  (Analytics) │              │   (Mgmt UI)      │               │
│  └──────────────┘              └──────────────────┘               │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                     Infrastructure Layer                            │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐              │
│  │PostgreSQL│  │  Redis  │  │   S3    │  │Prometheus│              │
│  │(Metadata)│  │ (Cache) │  │(Schemas)│  │ (Metrics)│              │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow: Telemetry Event Validation (LLM-Observatory)

```
┌─────────────┐
│   LLM App   │
│             │
└──────┬──────┘
       │
       │ 1. Emit inference event
       ▼
┌──────────────────┐
│ LLM-Observatory  │
│  Event Ingestion │
└────────┬─────────┘
         │
         │ 2. Extract schema_id from event header
         │    (e.g., schema_id: 42)
         ▼
    ┌────────┐
    │ Cache? │───No──┐
    └───┬────┘       │
        │            │ 3. Fetch schema by ID
       Yes           ▼
        │     ┌──────────────────┐
        │     │ Schema Registry  │
        │     │  GET /schemas/42 │
        │     └─────────┬────────┘
        │               │
        └───────────────┤ 4. Return schema definition
                        │    + cache headers
                        ▼
             ┌─────────────────┐
             │  Validate Event │
             │ Against Schema  │
             └────────┬────────┘
                      │
         ┌────────────┴────────────┐
         ▼                         ▼
    ┌─────────┐              ┌──────────┐
    │ Valid:  │              │ Invalid: │
    │ Process │              │ Log Error│
    │  Event  │              │ Emit     │
    │         │              │ Metric   │
    └─────────┘              └──────────┘
```

---

## Data Flow: Schema Evolution (CI/CD Pipeline)

```
┌────────────────┐
│   Developer    │
│  Commits New   │
│ Schema Version │
└───────┬────────┘
        │
        │ 1. git push
        ▼
┌──────────────────┐
│   CI/CD Pipeline │
│   (GitHub Actions│
│   / Jenkins)     │
└────────┬─────────┘
         │
         │ 2. Run schema compatibility check
         ▼
┌──────────────────────────────────────────┐
│       Schema Registry API                │
│  POST /compatibility/subjects/           │
│       telemetry.inference/versions/latest│
│                                          │
│  Body: { "schema": "..." }              │
└─────────────────┬────────────────────────┘
                  │
      ┌───────────┴──────────┐
      ▼                      ▼
┌──────────┐          ┌─────────────┐
│Compatible│          │Incompatible │
│  (200 OK)│          │  (409       │
└────┬─────┘          │  Conflict)  │
     │                └──────┬──────┘
     │                       │
     │ 3. Register           │ 3. Block deployment
     │    schema             │    Show breaking changes
     ▼                       ▼
┌──────────────┐      ┌──────────────┐
│POST /subjects│      │ Pipeline FAIL│
│/telemetry.   │      │ (Exit 1)     │
│inference/    │      └──────────────┘
│versions      │
└──────┬───────┘
       │
       │ 4. Return schema_id: 43
       ▼
┌──────────────┐
│  Deploy to   │
│  Production  │
│              │
│ (Apps now use│
│  schema_id   │
│     43)      │
└──────────────┘
```

---

## Data Flow: Schema Deprecation Workflow

```
Time: T-90 days
┌──────────────────┐
│ Schema Owner     │
│ Creates RFC:     │
│ Deprecate v1.x   │
│ Migrate to v2.x  │
└────────┬─────────┘
         │
         ▼
┌────────────────────────────────┐
│ Schema Registry                │
│ PUT /subjects/foo/versions/1   │
│ Body: { "state": "DEPRECATED", │
│         "sunset_date":          │
│         "2025-05-01" }          │
└────────┬───────────────────────┘
         │
         │ Webhook triggers
         ▼
┌─────────────────────┐
│ Governance Dashboard│
│ Shows deprecation   │
│ notice to consumers │
└─────────────────────┘

Time: T-60 days
┌──────────────────┐
│ Producer starts  │
│ dual-write:      │
│ • Emit v1 events │
│ • Emit v2 events │
└──────────────────┘

Time: T-30 days
┌──────────────────┐
│ Monitoring shows │
│ v1 usage dropping│
│ Send reminders   │
│ to lagging teams │
└──────────────────┘

Time: T+0 (Cutover)
┌──────────────────┐
│ Feature flag     │
│ switches to v2   │
│ as default       │
│ (v1 still valid) │
└──────────────────┘

Time: T+30 days
┌────────────────────────────────┐
│ Schema Registry                │
│ PUT /subjects/foo/versions/1   │
│ Body: { "state": "ARCHIVED" }  │
│ • No new consumers allowed     │
│ • Existing consumers still work│
└────────────────────────────────┘
```

---

## Integration Patterns by Module

### LLM-Observatory (Bidirectional)

**Producer Role** (Observatory emits schemas):
- Registers telemetry event schemas (Avro format)
- Registers metric schemas (Protobuf format)
- Notifies Registry of schema usage patterns

**Consumer Role** (Observatory uses schemas):
- Validates incoming events from LLM apps
- Retrieves schemas by ID for decoding
- Caches schemas for high-throughput validation

**API Calls**:
```
# On application startup
POST /subjects/telemetry.inference/versions
  → Returns schema_id: 42

# On event ingestion (hot path)
GET /schemas/ids/42
  → Returns cached Avro schema
```

---

### LLM-Sentinel (Producer)

**Role**: Registers security policy schemas

**Pattern**: Pre-commit validation in CI/CD

**Workflow**:
1. Security engineer defines new policy schema (JSON Schema)
2. CI/CD pipeline validates compatibility
3. On success, register schema and deploy policy
4. Sentinel validates security events against schema at runtime

**API Calls**:
```
# CI/CD pipeline (before merge)
POST /compatibility/subjects/security.policy/versions/latest
  Body: { "schema": "..." }
  → 200 OK (compatible) or 409 Conflict (breaking)

# On merge to main
POST /subjects/security.policy/versions
  → Returns schema_id: 17
```

---

### LLM-CostOps (Bidirectional)

**Producer Role**:
- Registers cost event schemas (token usage, API pricing)

**Consumer Role**:
- Retrieves schemas for cost analytics pipelines
- Validates cost events for consistency

**Special Requirements**:
- Cost calculation formulas embedded in schema metadata
- Cross-field constraints (e.g., total_cost = units * unit_price)

**API Calls**:
```
# Register cost event schema with constraints
POST /subjects/cost.token-usage/versions
  Body: {
    "schema": "...",
    "rules": [
      { "expression": "event.total_tokens > 0" },
      { "expression": "event.cost_usd >= 0" }
    ]
  }

# Validate cost event
POST /validate
  Body: { "schema_id": 55, "data": {...} }
  → Returns validation_result
```

---

### LLM-Analytics-Hub (Consumer)

**Role**: Consumes schemas for analytics pipelines

**Pattern**: Schema catalog browsing + bulk retrieval

**Use Cases**:
- Data engineers browse schema catalog
- ETL pipelines retrieve schemas for schema-on-read
- Impact analysis for schema changes (which pipelines affected?)

**API Calls**:
```
# Browse all schemas with tag "analytics"
GET /subjects?tag=analytics
  → Returns list of subjects

# Get all versions for a subject
GET /subjects/telemetry.inference/versions
  → Returns [1, 2, 3, ..., 12]

# Bulk retrieve schemas for pipeline
POST /schemas/bulk
  Body: { "ids": [42, 43, 55, 67] }
  → Returns array of schemas
```

---

### LLM-Governance-Dashboard (Consumer)

**Role**: UI for schema browsing and management

**Pattern**: Read-only API + admin operations

**Features**:
- Search schemas by name, tag, owner
- View schema lineage (which schemas reference this one?)
- Self-service deprecation workflows
- Usage analytics (top schemas by validation count)

**API Calls**:
```
# Search schemas
GET /subjects?query=telemetry&owner=observatory-team
  → Returns matching subjects

# Get schema metadata with lineage
GET /subjects/telemetry.inference/metadata
  → Returns {
      versions: [...],
      consumers: ["observatory", "analytics"],
      references: ["common.timestamp"],
      usage_stats: { validation_count: 1000000 }
    }

# Self-service deprecation (if authorized)
PUT /subjects/telemetry.inference/versions/5
  Body: { "state": "DEPRECATED", "sunset_date": "2025-06-01" }
```

---

## Technology Stack

### Core Service
- **Language**: Rust (tokio async runtime)
- **Web Framework**: Axum (HTTP) + Tonic (gRPC)
- **Serialization**: serde (JSON), prost (Protobuf), apache-avro (Avro)

### Storage
- **Metadata**: PostgreSQL 14+ (JSONB for schema definitions)
- **Cache**: Redis 7+ (Cluster mode for HA)
- **Objects** (optional): S3-compatible (MinIO, AWS S3)

### Observability
- **Metrics**: Prometheus (via prometheus-client crate)
- **Tracing**: OpenTelemetry (via opentelemetry-rust)
- **Logging**: tracing crate (JSON output)

### Deployment
- **Orchestration**: Kubernetes (Helm chart)
- **Service Mesh**: Istio (optional, for mTLS + observability)
- **Ingress**: nginx or Envoy (TLS termination, rate limiting)

---

## API Surface Summary

### REST API (Port 8081)

**Schema Operations**:
```
POST   /subjects/{subject}/versions           # Register schema
GET    /subjects                              # List subjects
GET    /subjects/{subject}/versions           # List versions
GET    /subjects/{subject}/versions/{version} # Get schema
DELETE /subjects/{subject}/versions/{version} # Soft-delete
```

**Compatibility**:
```
POST   /compatibility/subjects/{subject}/versions/{version}
  # Check if new schema compatible with specific version

GET    /config/{subject}  # Get compatibility mode
PUT    /config/{subject}  # Set compatibility mode
```

**Schema Retrieval**:
```
GET    /schemas/ids/{id}  # Get by global ID (most common, cacheable)
POST   /schemas/bulk      # Bulk retrieval
```

**Lifecycle**:
```
PUT    /subjects/{subject}/versions/{version}
  Body: { "state": "DEPRECATED", "sunset_date": "..." }

POST   /subjects/{subject}/rollback
  Body: { "target_version": 5 }
```

**Search & Metadata**:
```
GET    /subjects?query=...&tag=...&owner=...
GET    /subjects/{subject}/metadata
GET    /subjects/{subject}/consumers
```

### gRPC API (Port 9090)

**High-performance operations** for latency-sensitive use cases:
```protobuf
service SchemaRegistry {
  rpc RegisterSchema(RegisterSchemaRequest) returns (RegisterSchemaResponse);
  rpc GetSchemaById(GetSchemaByIdRequest) returns (GetSchemaByIdResponse);
  rpc CheckCompatibility(CheckCompatibilityRequest) returns (CheckCompatibilityResponse);
  rpc ValidateData(ValidateDataRequest) returns (ValidateDataResponse);
}
```

---

## Security Model

### Authentication Methods

1. **API Key** (stateless, for services):
   - Header: `X-API-Key: <key>`
   - Stored hashed in PostgreSQL
   - Per-key rate limits

2. **JWT** (for user sessions, LLM-Governance-Dashboard):
   - Header: `Authorization: Bearer <jwt>`
   - Issued by central auth service
   - Claims include roles, subject permissions

3. **mTLS** (for service-to-service):
   - Certificate-based authentication
   - Managed by service mesh (Istio)

### Authorization (RBAC)

**Roles**:
- **admin**: Full access (all subjects, all operations)
- **editor**: Register, update, deprecate (assigned subjects)
- **viewer**: Read-only (all subjects)

**Subject-Level Permissions** (future):
- Per-subject ACLs (e.g., only observatory-team can modify telemetry.* subjects)

---

## Monitoring Dashboard (Grafana)

### Key Panels

**Operations**:
- Requests/sec (by endpoint, by status code)
- Latency heatmap (p50, p95, p99, p999)
- Error rate (by error type)

**Schemas**:
- Total schemas (by type, by state)
- Schema registrations/hour
- Compatibility check results (pass/fail ratio)

**Performance**:
- Cache hit rate (client-side, Redis, CDN)
- Storage latency (PostgreSQL query time)
- Validation duration (by schema type)

**Health**:
- Storage connection status
- Cache availability
- Memory usage
- CPU usage

---

## Alerting Rules (Prometheus)

**Critical** (PagerDuty):
```yaml
- alert: SchemaRegistryHighErrorRate
  expr: rate(schema_registry_requests_total{status=~"5.."}[5m]) > 0.01
  for: 5m
  severity: critical

- alert: SchemaRegistryHighLatency
  expr: histogram_quantile(0.99, schema_registry_request_duration_seconds) > 0.5
  for: 5m
  severity: critical

- alert: SchemaRegistryStorageDown
  expr: schema_registry_storage_connection_status == 0
  for: 1m
  severity: critical
```

**Warning** (Slack):
```yaml
- alert: SchemaRegistryLowCacheHitRate
  expr: rate(schema_registry_cache_hits_total[5m]) / rate(schema_registry_cache_requests_total[5m]) < 0.80
  for: 10m
  severity: warning

- alert: SchemaRegistryHighCompatibilityFailures
  expr: rate(schema_registry_compatibility_checks_total{result="fail"}[5m]) > 0.10
  for: 10m
  severity: warning
```

---

## Deployment Architecture

### Production Deployment (Kubernetes)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-schema-registry
spec:
  replicas: 3  # HA deployment
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1  # Zero-downtime upgrades
  selector:
    matchLabels:
      app: llm-schema-registry
  template:
    metadata:
      labels:
        app: llm-schema-registry
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
    spec:
      containers:
      - name: registry
        image: llm-devops/schema-registry:v1.0.0
        ports:
        - containerPort: 8081  # REST API
        - containerPort: 9090  # gRPC API
        - containerPort: 8080  # Metrics
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: schema-registry-secrets
              key: database-url
        - name: REDIS_URL
          value: "redis://redis-cluster:6379"
        - name: LOG_LEVEL
          value: "info"
        resources:
          requests:
            cpu: 500m
            memory: 1Gi
          limits:
            cpu: 2000m
            memory: 4Gi
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
```

---

## Disaster Recovery

### Backup Strategy
- **PostgreSQL**: Daily full backup + continuous WAL archiving
- **Retention**: 30 days
- **Storage**: S3 with versioning enabled

### Recovery Procedures
1. **Database Corruption**: Restore from latest backup (RTO: 15 min, RPO: 5 min)
2. **Regional Outage**: Failover to secondary region (RTO: 5 min, RPO: 0)
3. **Schema Data Loss**: Schemas are append-only, corruption unlikely
4. **Cache Failure**: Degrade gracefully, bypass cache, fetch from DB

### Testing
- **Quarterly**: Full DR drill (restore from backup, validate data integrity)
- **Monthly**: Failover test (switch to secondary region)

---

## Cost Estimate (AWS Deployment)

**Compute** (EKS):
- 3x c5.xlarge instances (4 vCPU, 8 GB RAM each)
- ~$500/month

**Storage**:
- RDS PostgreSQL (db.r5.large, 100 GB): ~$300/month
- ElastiCache Redis (cache.r5.large): ~$200/month
- S3 (schema storage, negligible): ~$10/month

**Network**:
- ALB + data transfer: ~$100/month

**Total**: ~$1,100/month (~$13,000/year)

**Scaling** (10x load):
- 6x instances, larger RDS/Redis: ~$3,500/month

---

## Next Steps

This architecture document complements the SPECIFICATION. Together they provide:

1. **What** (SPECIFICATION): Requirements, features, success criteria
2. **How** (This Document): Integration patterns, data flows, deployment

Next phase: **PSEUDOCODE** (detailed algorithms and implementation logic)

---

**Document Version**: 1.0
**Last Updated**: 2025-11-21
**Author**: Requirements Analyst Agent
