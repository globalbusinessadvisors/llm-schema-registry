# LLM-Schema-Registry: SPECIFICATION Phase (SPARC Methodology)

## Executive Summary

LLM-Schema-Registry is a centralized, canonical data-contract and schema version-control service designed to enforce data integrity, compatibility, and governance across the LLM DevOps ecosystem. As a critical infrastructure component, it ensures consistent telemetry formats, event schemas, and inter-module communication contracts across 20+ modules organized into 8 functional cores.

---

## 1. Purpose and Vision

### 1.1 Strategic Purpose

LLM-Schema-Registry serves as the **single source of truth** for all data structures flowing through the LLM DevOps platform. It addresses the fundamental challenge of maintaining data consistency and compatibility in a distributed, polyglot microservices architecture where multiple modules (LLM-Observatory, LLM-Sentinel, LLM-CostOps, etc.) must communicate using standardized, evolvable data contracts.

### 1.2 Vision Statement

To provide a robust, Rust-based schema registry that:
- **Eliminates schema drift** across telemetry, events, and API contracts
- **Enables safe evolution** of data structures without breaking existing integrations
- **Enforces data quality** at the platform level through validation and compatibility checks
- **Reduces operational risk** by preventing incompatible schema deployments
- **Accelerates development** by providing clear, versioned data contracts

### 1.3 Core Value Propositions

1. **Operational Safety**: Prevent production incidents caused by schema incompatibilities
2. **Development Velocity**: Enable teams to evolve schemas independently with confidence
3. **Data Governance**: Centralized control over data structures and evolution policies
4. **Observability Foundation**: Standardized telemetry schemas enable consistent monitoring
5. **Cost Optimization**: Track and validate cost-related data structures (CostOps integration)
6. **Security Assurance**: Enforce schema-level security policies (Sentinel integration)

---

## 2. Scope and Boundaries

### 2.1 In Scope

**Schema Management:**
- Registration and storage of schema definitions (Avro, Protobuf, JSON Schema)
- Version control with semantic versioning (major.minor.patch)
- Schema metadata management (ownership, tags, documentation)
- Schema deprecation workflows with sunset timelines

**Validation and Compatibility:**
- Pre-registration compatibility validation
- Runtime schema validation for producers and consumers
- Compatibility mode enforcement (BACKWARD, FORWARD, FULL, TRANSITIVE variants)
- Breaking change detection and blocking

**Data Contract Enforcement:**
- Field-level constraints (type, range, format, nullability)
- Declarative data quality rules using CEL (Common Expression Language)
- Semantic validation beyond structural conformance
- Cross-schema referencing and composition

**Lifecycle Management:**
- Schema state transitions (draft → active → deprecated → archived)
- Rollback capabilities for failed schema deployments
- Impact analysis for schema changes
- Usage tracking and dependency mapping

**Integration Services:**
- RESTful API for schema operations
- Rust SDK for high-performance integrations
- gRPC endpoints for low-latency operations
- Webhook notifications for schema events

### 2.2 Out of Scope

- **Data Storage**: Registry stores schemas, not application data
- **Message Routing**: No event/message broker functionality
- **Data Transformation**: No ETL or data mapping services
- **Schema Inference**: No automatic schema generation from data samples
- **UI/Dashboard**: Covered by LLM-Governance-Dashboard integration

### 2.3 Architectural Boundaries

**North Boundary** (Producers):
- All LLM DevOps modules registering or validating schemas
- External systems integrating via API

**South Boundary** (Dependencies):
- Persistent storage layer (PostgreSQL/ClickHouse for metadata)
- Object storage (S3-compatible) for schema artifacts
- Optional caching layer (Redis) for performance

**East/West Boundaries** (Peer Integrations):
- LLM-Observatory: Telemetry schema definitions
- LLM-Sentinel: Security policy schemas
- LLM-CostOps: Cost event schemas
- LLM-Analytics-Hub: Analytics data schemas
- LLM-Governance-Dashboard: Schema browsing and management UI

---

## 3. Core Requirements

### 3.1 Functional Requirements

#### FR-1: Schema Registration

**FR-1.1**: Support registration of schemas in multiple formats:
- Apache Avro (recommended for telemetry and events)
- Protocol Buffers (recommended for high-performance inter-service communication)
- JSON Schema (recommended for REST APIs and human-readable contracts)

**FR-1.2**: Each schema registration must include:
- Schema definition (in supported format)
- Subject/namespace identifier (e.g., `telemetry.llm-observatory.inference-event`)
- Version number (auto-incremented or explicit)
- Compatibility mode (defaults to BACKWARD)
- Metadata: owner team, description, tags, documentation links

**FR-1.3**: Schema registration must be atomic and fail if:
- Schema definition is malformed
- Compatibility check fails against existing versions
- Subject-version combination already exists

**FR-1.4**: Return schema ID (globally unique, monotonically increasing) upon successful registration

#### FR-2: Compatibility Validation

**FR-2.1**: Implement compatibility modes aligned with Confluent Schema Registry:

| Mode | Description | Use Case |
|------|-------------|----------|
| BACKWARD | New schema can read data written with previous schema | Consumer upgrades first |
| BACKWARD_TRANSITIVE | New schema compatible with ALL previous versions | Strict backward compatibility |
| FORWARD | Old schema can read data written with new schema | Producer upgrades first |
| FORWARD_TRANSITIVE | Old schema compatible with ALL future versions | Strict forward compatibility |
| FULL | New schema is both backward and forward compatible | Safest, most restrictive |
| FULL_TRANSITIVE | Full compatibility with ALL versions | Maximum safety |
| NONE | No compatibility checks | Development/testing only |

**FR-2.2**: Compatibility checks must be performed:
- During schema registration (pre-commit validation)
- Asynchronously for bulk validation jobs
- On-demand via API for CI/CD integration

**FR-2.3**: Provide detailed compatibility reports including:
- List of breaking changes detected
- Affected fields and their changes
- Recommended remediation steps

#### FR-3: Schema Retrieval

**FR-3.1**: Retrieve schemas by:
- Schema ID (global identifier)
- Subject + version number
- Subject + "latest" version
- Subject + compatibility filter (e.g., "latest FULL-compatible")

**FR-3.2**: Support bulk retrieval operations:
- All versions for a subject
- All subjects matching a pattern (e.g., `telemetry.*`)
- Schemas by tag or metadata filter

**FR-3.3**: Include caching headers in HTTP responses for client-side caching

**FR-3.4**: Support content negotiation for different schema formats in API responses

#### FR-4: Schema Lifecycle Management

**FR-4.1**: Support schema states:
- **DRAFT**: Under development, not available for production use
- **ACTIVE**: Current and usable in production
- **DEPRECATED**: Marked for removal, usage discouraged
- **ARCHIVED**: Retained for historical reference, not usable

**FR-4.2**: Implement deprecation workflows:
- Deprecation announcement with target sunset date
- Notification to consumers (via webhooks/events)
- Grace period enforcement (minimum 90 days recommended)
- Automated checks preventing new consumer registrations

**FR-4.3**: Rollback capabilities:
- Mark current version as deprecated
- Reactivate previous version as "latest"
- Maintain audit trail of rollback operations
- Notify consumers of rollback event

**FR-4.4**: Soft-delete semantics:
- Schemas never physically deleted (auditability)
- Archived schemas not returned in standard queries
- Superuser capability to view archived schemas

#### FR-5: Data Contract Enforcement

**FR-5.1**: Support field-level constraints beyond schema structure:
- Type constraints (primitive types, enums, unions)
- Numeric constraints (min, max, precision)
- String constraints (pattern, length, format)
- Nullability constraints
- Default value specifications

**FR-5.2**: Implement declarative rules using CEL expressions:
```cel
// Example: Cost event must have positive amount
event.cost_amount > 0

// Example: Inference event timestamp within 5 minutes of processing
timestamp(event.inferred_at) < timestamp(now) + duration("5m")
```

**FR-5.3**: Support semantic validation:
- Cross-field constraints (e.g., end_time > start_time)
- Conditional constraints based on event type
- Reference data validation (e.g., model_id exists in catalog)

**FR-5.4**: Validation levels:
- **STRICT**: Block invalid data, return error
- **WARN**: Log warning, allow data with annotations
- **MONITOR**: Track violations in metrics, allow data

#### FR-6: Integration Points

**FR-6.1 LLM-Observatory Integration**:
- Define canonical schemas for telemetry events:
  - Inference request/response events
  - Model performance metrics
  - Latency and throughput measurements
  - Error and exception traces
- Support OpenTelemetry semantic convention alignment
- Provide schema evolution notifications to Observatory

**FR-6.2 LLM-Sentinel Integration**:
- Define schemas for security events:
  - Authentication/authorization events
  - Policy violation alerts
  - Threat detection signals
- Enforce encryption requirements on sensitive fields
- Support PII detection rules at schema level

**FR-6.3 LLM-CostOps Integration**:
- Define schemas for cost tracking:
  - Token usage events
  - API call pricing events
  - Resource allocation events
- Validate cost calculation consistency across modules
- Enable cost attribution schema standards

**FR-6.4 LLM-Analytics-Hub Integration**:
- Provide schema catalog for analytics pipelines
- Support schema-on-read scenarios with compatibility guarantees
- Enable impact analysis for schema changes affecting analytics

**FR-6.5 LLM-Governance-Dashboard Integration**:
- Expose schema browsing and search APIs
- Provide schema lineage and dependency graphs
- Enable self-service schema management workflows
- Generate human-readable schema documentation

#### FR-7: Versioning and Evolution

**FR-7.1**: Semantic versioning enforcement:
- **Major**: Breaking changes (remove field, change type)
- **Minor**: Backward-compatible additions (add optional field)
- **Patch**: Non-functional changes (documentation, examples)

**FR-7.2**: Version comparison API:
- Diff two schema versions (field-by-field comparison)
- Highlight breaking vs. non-breaking changes
- Generate migration guides automatically

**FR-7.3**: Version limits and cleanup:
- Configurable max versions per subject (default: 100)
- Automatic archival of old versions exceeding limit
- Retention policy enforcement

**FR-7.4**: Schema references and composition:
- Support schema references (Avro, Protobuf, JSON Schema)
- Validate transitive dependencies
- Prevent circular references

#### FR-8: Audit and Observability

**FR-8.1**: Comprehensive audit logging:
- All schema CRUD operations
- Compatibility check results
- Validation failures
- State transitions (activation, deprecation)

**FR-8.2**: Metrics emission:
- Schema registration rate
- Validation failure rate (by subject, by error type)
- Schema retrieval latency (p50, p95, p99)
- Cache hit rate
- Storage utilization

**FR-8.3**: Health checks and diagnostics:
- Liveness probe (process health)
- Readiness probe (dependencies available)
- Storage connectivity checks
- Cache connectivity checks

### 3.2 Non-Functional Requirements

#### NFR-1: Performance

**NFR-1.1 Latency**:
- Schema retrieval by ID: p95 < 10ms, p99 < 50ms
- Schema registration: p95 < 100ms, p99 < 500ms
- Compatibility validation: p95 < 200ms, p99 < 1s

**NFR-1.2 Throughput**:
- Support 10,000+ schema retrievals per second (with caching)
- Support 100+ schema registrations per second
- Handle 1,000+ concurrent clients

**NFR-1.3 Caching Strategy**:
- Client-side caching with HTTP cache headers
- Server-side Redis cache for frequently accessed schemas
- Cache invalidation on schema updates
- Configurable TTL (default: 5 minutes for schema metadata, 1 hour for schema definitions)

#### NFR-2: Scalability

**NFR-2.1 Horizontal Scaling**:
- Stateless API layer (scale via load balancer)
- Read replicas for database (read-heavy workload)
- Cache sharding for large deployments

**NFR-2.2 Capacity Planning**:
- Support 10,000+ subjects (unique schema namespaces)
- Support 1,000,000+ total schema versions
- Support 100GB+ schema storage

**NFR-2.3 Multi-Tenancy** (future consideration):
- Namespace isolation for different teams/projects
- Per-tenant quotas and rate limits

#### NFR-3: Reliability

**NFR-3.1 Availability**:
- Target: 99.9% uptime (43 minutes downtime/month)
- No single point of failure (multi-instance deployment)
- Graceful degradation (read-only mode if storage unavailable)

**NFR-3.2 Durability**:
- Persistent storage with replication (3+ replicas)
- Regular backups (daily, retained for 30 days)
- Point-in-time recovery capability

**NFR-3.3 Fault Tolerance**:
- Retry logic with exponential backoff for storage operations
- Circuit breaker for dependency failures
- Bulkhead pattern for resource isolation

#### NFR-4: Security

**NFR-4.1 Authentication**:
- API key-based authentication (lightweight)
- OAuth 2.0 / JWT support for enterprise SSO integration
- Service-to-service authentication via mTLS

**NFR-4.2 Authorization**:
- Role-based access control (RBAC):
  - **Admin**: Full access (all operations)
  - **Editor**: Register, update, deprecate schemas
  - **Viewer**: Read-only access
- Subject-level permissions (future enhancement)

**NFR-4.3 Data Protection**:
- TLS 1.3 for all API communications
- Encryption at rest for sensitive schema metadata
- Audit logs for compliance (SOC 2, GDPR)

**NFR-4.4 Rate Limiting**:
- Per-client rate limits (configurable, default: 1000 req/min)
- DDoS protection via reverse proxy (nginx/envoy)

#### NFR-5: Maintainability

**NFR-5.1 Code Quality**:
- Rust implementation leveraging type safety
- Comprehensive unit tests (>80% coverage)
- Integration tests for critical workflows
- Property-based testing for schema validation logic

**NFR-5.2 Operational Excellence**:
- Structured logging (JSON format)
- Correlation IDs for request tracing
- Runbook documentation for common operations
- Alerting on critical metrics (high error rate, high latency)

**NFR-5.3 Versioning Strategy**:
- Semantic versioning for registry service itself
- API versioning (v1, v2) for backward compatibility
- Deprecation policy for old API versions (minimum 1 year notice)

#### NFR-6: Compatibility

**NFR-6.1 Standards Compliance**:
- API compatibility with Confluent Schema Registry REST API (where applicable)
- Support for standard schema formats (Avro, Protobuf, JSON Schema)
- OpenTelemetry semantic convention alignment

**NFR-6.2 Interoperability**:
- REST API for polyglot clients
- gRPC API for high-performance Rust/Go/Java clients
- Schema export/import for migration scenarios

#### NFR-7: Observability

**NFR-7.1 Metrics**:
- Prometheus-compatible metrics endpoint
- Custom metrics for schema operations
- Integration with LLM-Observatory for centralized monitoring

**NFR-7.2 Tracing**:
- OpenTelemetry instrumentation
- Distributed tracing for cross-module operations
- Trace sampling for production efficiency

**NFR-7.3 Logging**:
- Structured JSON logs
- Log levels: ERROR, WARN, INFO, DEBUG, TRACE
- Log aggregation integration (e.g., Loki, Elasticsearch)

---

## 4. Integration Architecture

### 4.1 Module Integration Matrix

| Module | Direction | Schema Types | Integration Pattern |
|--------|-----------|--------------|---------------------|
| LLM-Observatory | Bidirectional | Telemetry events (Avro), Metrics (Protobuf) | Schema validation on event ingestion, schema updates via webhooks |
| LLM-Sentinel | Unidirectional (to Registry) | Security events (Avro), Policy schemas (JSON) | Pre-commit schema validation in CI/CD |
| LLM-CostOps | Bidirectional | Cost events (Avro), Pricing schemas (JSON) | Schema validation on cost event publishing, schema retrieval for analytics |
| LLM-Analytics-Hub | Unidirectional (from Registry) | All analytics schemas | Schema catalog API, bulk schema retrieval |
| LLM-Governance-Dashboard | Unidirectional (from Registry) | All schemas (metadata) | Read-only API for browsing, search, and visualization |

### 4.2 Communication Patterns

**Synchronous (REST/gRPC)**:
- Schema registration and retrieval
- Compatibility checks during development
- Schema search and metadata queries

**Asynchronous (Events/Webhooks)**:
- Schema update notifications (publish to message bus)
- Deprecation announcements (webhook to subscribers)
- Breaking change alerts (webhook to affected consumers)

### 4.3 Data Flow Examples

**Example 1: LLM-Observatory Telemetry Ingestion**
```
1. Observatory receives inference event from LLM application
2. Observatory extracts schema ID from event header
3. Observatory queries Schema Registry for schema by ID (cached)
4. Observatory validates event against retrieved schema
5. If valid: process event; If invalid: log error, emit metric, optionally drop
```

**Example 2: LLM-Sentinel Policy Deployment**
```
1. Sentinel team develops new security policy schema
2. CI/CD pipeline validates schema compatibility (API call to Registry)
3. On success: register schema, receive schema ID
4. Sentinel deploys policy enforcement logic with schema ID reference
5. Runtime: validate security events against registered schema
```

**Example 3: Schema Evolution (Breaking Change)**
```
1. Developer proposes schema change (remove field)
2. CI/CD performs compatibility check (API call to Registry)
3. Registry detects BACKWARD incompatibility, returns error
4. Developer refactors: deprecate field instead of removing
5. CI/CD re-checks, passes compatibility
6. Schema registered, webhook notifies LLM-Governance-Dashboard
7. Dashboard displays deprecation notice to consumers
```

### 4.4 API Design Patterns

**REST API Endpoints** (Confluent-compatible):
```
POST   /subjects/{subject}/versions          # Register new schema version
GET    /subjects/{subject}/versions          # List all versions for subject
GET    /subjects/{subject}/versions/{version} # Get specific version
GET    /subjects/{subject}/versions/latest   # Get latest version
DELETE /subjects/{subject}/versions/{version} # Soft-delete version
POST   /compatibility/subjects/{subject}/versions/{version} # Check compatibility
GET    /schemas/ids/{id}                     # Get schema by global ID
GET    /subjects                             # List all subjects
GET    /config/{subject}                     # Get compatibility config
PUT    /config/{subject}                     # Update compatibility config
```

**Rust SDK API** (ergonomic, type-safe):
```rust
use llm_schema_registry::{SchemaRegistry, Schema, CompatibilityMode};

let registry = SchemaRegistry::new("http://registry:8081")?;

// Register schema
let schema = Schema::avro(r#"{"type": "record", ...}"#)?;
let id = registry.register("telemetry.inference", schema).await?;

// Retrieve and validate
let schema = registry.get_by_id(id).await?;
let is_valid = schema.validate(&event_bytes)?;

// Check compatibility
let new_schema = Schema::avro(r#"..."#)?;
let compat = registry.check_compatibility(
    "telemetry.inference",
    &new_schema,
    CompatibilityMode::Backward
).await?;
```

---

## 5. Success Criteria

### 5.1 Technical Success Metrics

**Schema Stability**:
- Zero undetected breaking changes in production (100% compatibility check coverage)
- Schema-related production incidents: < 1 per quarter
- Mean time to detect schema incompatibility: < 5 minutes

**Performance**:
- 99th percentile retrieval latency: < 50ms
- Cache hit rate: > 95%
- Schema validation overhead: < 5% of total event processing time

**Adoption**:
- All 5 core integrations (Observatory, Sentinel, CostOps, Analytics, Governance) fully integrated within 6 months
- 100% of inter-module events schema-validated within 1 year
- 90%+ developer satisfaction score (schema registry usability survey)

### 5.2 Business Success Metrics

**Risk Reduction**:
- 80% reduction in data-format-related production incidents (year-over-year)
- 100% auditability of schema changes (compliance requirement)

**Developer Productivity**:
- 50% reduction in time spent debugging data format mismatches
- Schema evolution cycle time: < 1 day (from proposal to production)

**Platform Reliability**:
- Contribution to overall platform 99.9% uptime target
- Zero schema-registry-caused downtime for dependent modules

### 5.3 Quality Gates

**Alpha Release (Months 1-2)**:
- Core API implemented (register, retrieve, compatibility check)
- Support for Avro schemas
- Integration with LLM-Observatory (telemetry schemas)
- Basic compatibility modes (BACKWARD, FORWARD, FULL)

**Beta Release (Months 3-4)**:
- Protobuf and JSON Schema support
- Advanced compatibility modes (transitive variants)
- Deprecation and rollback workflows
- Integration with LLM-Sentinel and LLM-CostOps
- Performance optimization (caching, indexing)

**GA Release (Month 5-6)**:
- All NFRs met (performance, reliability, security)
- Complete integration with all 5 core modules
- Production-grade monitoring and alerting
- Documentation and runbooks complete
- Load testing validated at target scale

**Post-GA Enhancements** (Months 7-12):
- Data contract features (CEL rules, semantic validation)
- Schema lineage and impact analysis
- Multi-tenancy support
- Advanced caching strategies (CDN for schema distribution)

---

## 6. Constraints and Assumptions

### 6.1 Technical Constraints

**C-1: Technology Stack**:
- Must be implemented in Rust (ecosystem standard)
- Must use PostgreSQL or ClickHouse for metadata storage
- Must integrate with existing LLM DevOps infrastructure (Kubernetes, Prometheus, OpenTelemetry)

**C-2: Compatibility**:
- API should be compatible with Confluent Schema Registry where possible (minimize migration friction for teams familiar with Confluent)
- Must not introduce vendor lock-in (standard schema formats only)

**C-3: Performance**:
- Cannot add more than 10ms p95 latency to event processing pipelines
- Schema storage must be efficient (no unbounded growth without cleanup)

**C-4: Operational**:
- Must be deployable via Helm chart (Kubernetes-native)
- Must support zero-downtime upgrades (rolling deployments)
- Must integrate with standard observability stack (Prometheus, Grafana, Jaeger)

### 6.2 Assumptions

**A-1: Schema Discipline**:
- Teams will follow schema evolution best practices (deprecate vs. delete)
- Breaking changes will go through proper approval workflows
- Schema ownership is clearly defined (RACI matrix)

**A-2: Infrastructure**:
- Kubernetes cluster available with sufficient resources
- PostgreSQL/ClickHouse available as managed service or stable deployment
- Redis available for caching (optional but recommended)

**A-3: Integration Readiness**:
- Integrating modules (Observatory, Sentinel, etc.) have APIs for schema validation
- Teams are willing to adopt schema validation in their pipelines
- CI/CD pipelines can incorporate schema compatibility checks

**A-4: Governance**:
- Schema governance policies are defined (compatibility modes per subject)
- Deprecation timelines are enforced organizationally (not just technically)
- Schema Registry has dedicated operational ownership (on-call rotation)

**A-5: Volume Projections**:
- 1,000 subjects at launch, growing to 10,000 within 2 years
- 10 versions per subject on average
- 10,000 schema retrievals/sec at peak (Observatory + Analytics workloads)
- 100 schema registrations/sec at peak (CI/CD deployments)

### 6.3 Risks and Mitigations

**R-1: Adoption Resistance**
- Risk: Teams bypass schema validation due to perceived friction
- Mitigation: Provide excellent DX (fast APIs, helpful errors), mandate validation via policy

**R-2: Performance Bottleneck**
- Risk: Schema Registry becomes critical path bottleneck
- Mitigation: Aggressive caching, client-side caching, read replicas

**R-3: Breaking Change Slips Through**
- Risk: Compatibility logic has bugs, allows incompatible schemas
- Mitigation: Comprehensive test suite, canary deployments, rollback procedures

**R-4: Storage Growth**
- Risk: Unlimited schema versions cause storage bloat
- Mitigation: Version limits, archival policies, compression

**R-5: Single Point of Failure**
- Risk: Schema Registry downtime breaks all dependent services
- Mitigation: High availability deployment, client-side fallback logic, graceful degradation

**R-6: Schema Format Limitations**
- Risk: Chosen formats (Avro/Protobuf/JSON) don't support future requirements
- Mitigation: Extensible architecture, plugin system for new formats

---

## 7. Research Insights and Industry Best Practices

### 7.1 Key Learnings from Industry Solutions

**From Confluent Schema Registry**:
- Single-primary architecture ensures consistency (write to Kafka log)
- Transitive compatibility modes are critical for long-term evolution
- Schema IDs should be globally unique and monotonic (simplifies caching)
- Soft-delete semantics preserve auditability

**From AWS Glue Schema Registry**:
- Serverless model reduces operational overhead
- Auto-registration reduces friction for developers (but requires governance)
- 10,000 version limit per schema is reasonable upper bound
- IAM integration is table stakes for enterprise adoption

**From Azure Schema Registry**:
- Tight integration with event streaming platforms (Event Hubs) drives adoption
- Schema groups (namespaces) enable multi-tenancy
- Developer-friendly documentation is critical for adoption

**From OpenTelemetry Schema Spec**:
- Telemetry schemas require special handling (high volume, low latency)
- Schema transformation rules enable backward compatibility at consumer side
- Semantic conventions alignment reduces fragmentation

### 7.2 Schema Format Recommendations

**Avro** (recommended for events/telemetry):
- Best balance of performance, schema evolution, and ecosystem support
- Strong backward/forward compatibility semantics
- Compact binary encoding (important for high-volume telemetry)

**Protobuf** (recommended for inter-service APIs):
- Fastest serialization/deserialization
- Excellent Rust support (prost crate)
- Field numbers enable safe evolution

**JSON Schema** (recommended for REST APIs, human-readable contracts):
- Highest interoperability (every language supports JSON)
- Self-documenting (schema is human-readable)
- Slower than binary formats (trade-off for convenience)

### 7.3 Deprecation Best Practices (Adapted from Industry)

**Timeline Recommendation**:
- T-90 days: Publish deprecation RFC with migration guide
- T-60 days: Enable dual-write (old and new schemas)
- T-30 days: Start monitoring for old schema usage, send reminders
- T+0: Cut over (new schema becomes default)
- T+30 days: Old schema marked ARCHIVED (still readable, not writable)

**Shadow/Dual-Run Pattern**:
- Run v1 and v2 schemas in parallel during migration
- Compare outputs to detect discrepancies
- Provides confidence before full cutover

---

## 8. Technical Specification Details

### 8.1 Schema Metadata Model

```rust
pub struct SchemaMetadata {
    pub id: SchemaId,              // Global unique ID (i64)
    pub subject: String,           // Namespace (e.g., "telemetry.inference")
    pub version: u32,              // Version number within subject
    pub schema_type: SchemaType,   // Avro, Protobuf, JsonSchema
    pub definition: String,        // Schema definition (JSON/proto text)
    pub compatibility_mode: CompatibilityMode,
    pub state: SchemaState,        // Draft, Active, Deprecated, Archived
    pub references: Vec<SchemaReference>, // For composed schemas

    // Metadata
    pub owner_team: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub documentation_url: Option<String>,

    // Lifecycle
    pub registered_at: DateTime<Utc>,
    pub registered_by: String,
    pub deprecated_at: Option<DateTime<Utc>>,
    pub sunset_date: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,

    // Usage tracking
    pub producer_count: u32,       // Active producers
    pub consumer_count: u32,       // Active consumers
    pub validation_count: u64,     // Lifetime validations
}
```

### 8.2 Storage Schema Design

**PostgreSQL Tables**:

```sql
CREATE TABLE schemas (
    id BIGSERIAL PRIMARY KEY,
    subject VARCHAR(255) NOT NULL,
    version INTEGER NOT NULL,
    schema_type VARCHAR(50) NOT NULL,
    definition TEXT NOT NULL,
    definition_hash VARCHAR(64) NOT NULL, -- SHA256 for deduplication
    compatibility_mode VARCHAR(50) NOT NULL,
    state VARCHAR(50) NOT NULL DEFAULT 'ACTIVE',

    owner_team VARCHAR(255) NOT NULL,
    description TEXT,
    documentation_url TEXT,

    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    registered_by VARCHAR(255) NOT NULL,
    deprecated_at TIMESTAMPTZ,
    sunset_date TIMESTAMPTZ,
    archived_at TIMESTAMPTZ,

    UNIQUE(subject, version),
    INDEX(subject, state),
    INDEX(definition_hash) -- For finding duplicates
);

CREATE TABLE schema_references (
    schema_id BIGINT NOT NULL REFERENCES schemas(id),
    referenced_subject VARCHAR(255) NOT NULL,
    referenced_version INTEGER NOT NULL,
    FOREIGN KEY (referenced_subject, referenced_version)
        REFERENCES schemas(subject, version)
);

CREATE TABLE schema_tags (
    schema_id BIGINT NOT NULL REFERENCES schemas(id),
    tag VARCHAR(100) NOT NULL,
    PRIMARY KEY (schema_id, tag)
);

CREATE TABLE schema_consumers (
    schema_id BIGINT NOT NULL REFERENCES schemas(id),
    consumer_id VARCHAR(255) NOT NULL,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (schema_id, consumer_id)
);

CREATE TABLE schema_audit_log (
    id BIGSERIAL PRIMARY KEY,
    schema_id BIGINT REFERENCES schemas(id),
    action VARCHAR(50) NOT NULL, -- REGISTER, DEPRECATE, ARCHIVE, etc.
    actor VARCHAR(255) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);
```

### 8.3 Compatibility Algorithm Pseudocode

```
function check_compatibility(new_schema, old_schema, mode):
    case mode:
        BACKWARD:
            # New schema can read old data
            return can_read_with(new_schema, old_schema)

        FORWARD:
            # Old schema can read new data
            return can_read_with(old_schema, new_schema)

        FULL:
            # Both directions
            return (can_read_with(new_schema, old_schema) AND
                    can_read_with(old_schema, new_schema))

        BACKWARD_TRANSITIVE:
            # New schema compatible with ALL previous versions
            for version in get_all_versions(old_schema.subject):
                if not can_read_with(new_schema, version):
                    return false
            return true

        # Similar for FORWARD_TRANSITIVE, FULL_TRANSITIVE

        NONE:
            return true

function can_read_with(reader_schema, writer_schema):
    # Avro-specific rules (simplified):
    # 1. If reader has field not in writer, reader must have default
    # 2. If writer has field not in reader, OK (reader ignores)
    # 3. Field types must match or be promotable (int -> long, etc.)

    reader_fields = get_fields(reader_schema)
    writer_fields = get_fields(writer_schema)

    for field in reader_fields:
        if field not in writer_fields:
            if not has_default(field):
                return false  # Breaking: new required field

    for field in common_fields(reader_fields, writer_fields):
        if not types_compatible(field.reader_type, field.writer_type):
            return false  # Breaking: type change

    return true
```

### 8.4 Caching Strategy

**Three-Layer Cache**:

1. **Client-Side Cache** (in-memory, SDK):
   - Cache schemas by ID for duration of application lifecycle
   - Cache subject+version for 5 minutes (configurable)
   - Invalidate on HTTP 410 Gone response

2. **Server-Side Cache** (Redis):
   - Key: `schema:id:{id}`, TTL: 1 hour
   - Key: `schema:subject:{subject}:version:{version}`, TTL: 5 minutes
   - Key: `schema:subject:{subject}:latest`, TTL: 1 minute
   - Invalidate on schema updates via pub/sub

3. **CDN Cache** (optional, for global deployments):
   - Cache GET /schemas/ids/{id} responses (immutable)
   - Cache-Control: public, max-age=86400 (24 hours)
   - Purge via API on schema deletion (rare)

### 8.5 Monitoring and Alerting

**Key Metrics**:
```
# Request metrics
schema_registry_requests_total{method, endpoint, status}
schema_registry_request_duration_seconds{method, endpoint}

# Schema metrics
schema_registry_schemas_total{schema_type, state}
schema_registry_schema_registrations_total{subject, status}
schema_registry_compatibility_checks_total{result}

# Performance metrics
schema_registry_cache_hits_total{cache_type}
schema_registry_cache_misses_total{cache_type}
schema_registry_validation_duration_seconds{schema_type}

# Health metrics
schema_registry_storage_connection_status{storage_type}
schema_registry_storage_operation_errors_total{operation}
```

**Critical Alerts**:
- Error rate > 1% (5 minutes)
- p99 latency > 500ms (5 minutes)
- Storage connection failures (immediate)
- Cache unavailable for > 5 minutes (warning)
- Compatibility check failures spike (> 10% of checks)

---

## 9. Acceptance Criteria

The LLM-Schema-Registry SPECIFICATION phase is complete when:

1. All stakeholders (Observatory, Sentinel, CostOps, Analytics, Governance teams) have reviewed and approved requirements
2. Functional requirements (FR-1 through FR-8) are unambiguous and testable
3. Non-functional requirements (NFR-1 through NFR-7) have quantifiable targets
4. Integration points with all 5 core modules are documented with API contracts
5. Success metrics are agreed upon with measurable targets
6. Risks are identified with mitigation strategies
7. Technical specification includes data models, API design, and algorithms
8. Industry best practices are researched and incorporated

---

## 10. Next Steps: Transition to PSEUDOCODE Phase

With the SPECIFICATION complete, the next phase will involve:

1. **API Definition**: Detailed OpenAPI/gRPC schema definitions
2. **Data Model Refinement**: Complete entity-relationship diagrams
3. **Algorithm Design**: Detailed pseudocode for compatibility checking, caching, and validation
4. **Component Architecture**: Module breakdown with clear responsibilities
5. **Interface Contracts**: Defined boundaries between components
6. **Error Handling Strategy**: Exception hierarchy and recovery procedures
7. **Test Strategy**: Unit, integration, and performance test plans

This SPECIFICATION serves as the canonical requirements document guiding all subsequent implementation phases.

---

## Document Metadata

- **Version**: 1.0
- **Date**: 2025-11-21
- **Author**: Requirements Analyst Agent (LLM-Schema-Registry Swarm)
- **Status**: Draft - Pending Stakeholder Review
- **Related Documents**:
  - SPARC Methodology Overview
  - LLM DevOps Architecture Reference
  - Module Integration Standards
- **Review Cycle**: Quarterly updates or upon major requirement changes
