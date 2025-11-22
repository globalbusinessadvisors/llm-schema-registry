# ARCHITECTURE - LLM-Schema-Registry

## Overview

This document defines the architectural design for the LLM-Schema-Registry, a high-performance schema registry built in Rust for managing, versioning, and validating schemas used in LLM-powered applications. The architecture follows the SPARC methodology and emphasizes type safety, performance, and reliability.

## 1. Crate Selection & Rationale

### 1.1 Schema Parsing & Serialization

#### Primary Crates

**`serde` (v1.0) + `serde_json` (v1.0)**
- **Rationale**: Industry-standard serialization framework with zero-copy deserialization
- **Use Case**: JSON Schema parsing, API request/response handling
- **Performance**: Zero-allocation deserialization for borrowed data
- **Integration**: Deep ecosystem integration with virtually all Rust libraries

**`apache-avro` (v0.16)**
- **Rationale**: Official Apache Avro implementation with schema evolution support
- **Use Case**: Binary schema format for high-throughput LLM I/O
- **Performance**: Compact binary representation, ~10x smaller than JSON
- **Features**: Built-in schema resolution, codec support (deflate, snappy, zstd)

**`prost` (v0.12) + `prost-types` (v0.12)**
- **Rationale**: Pure Rust Protocol Buffers implementation, faster than protoc
- **Use Case**: gRPC service definitions, structured LLM prompt/response schemas
- **Performance**: Code generation at compile-time, zero runtime overhead
- **Tooling**: `prost-build` for build.rs integration

**`jsonschema` (v0.17)**
- **Rationale**: Full JSON Schema Draft 7/2019-09/2020-12 implementation
- **Use Case**: Schema validation against JSON Schema standards
- **Performance**: Compiled validators with caching, ~1μs validation for simple schemas
- **Features**: Custom keywords, format validators, reference resolution

#### Supporting Crates

**`schemars` (v0.8)**
- **Rationale**: Automatic JSON Schema generation from Rust types
- **Use Case**: Generate schemas from Rust structs for LLM API contracts
- **Integration**: Seamless serde integration via derive macros

**`yaml-rust2` (v0.8)**
- **Rationale**: YAML parsing for schema definitions
- **Use Case**: Human-friendly schema authoring format
- **Safety**: Pure Rust, no C dependencies

### 1.2 Validation Engine

**`validator` (v0.18)**
- **Rationale**: Declarative validation via derive macros
- **Use Case**: Runtime validation of schema metadata, configuration
- **Features**: Email, URL, length, range, custom validators
- **Performance**: Compile-time code generation, minimal allocations

**`garde` (v0.18)** (Alternative/Supplement)
- **Rationale**: More ergonomic validation with better error messages
- **Use Case**: Complex cross-field validation rules
- **Features**: Async validation support, context-aware validation

**Custom Compatibility Checker Design**
```rust
// Core compatibility engine
pub struct CompatibilityChecker {
    rules: Arc<CompatibilityRules>,
    cache: Arc<RwLock<CompatibilityCache>>,
}

pub enum CompatibilityLevel {
    Backward,        // New schema can read old data
    Forward,         // Old schema can read new data
    Full,            // Both backward and forward
    Transitive,      // Full compatibility across all versions
    None,            // No compatibility checks
}

pub trait CompatibilityRule: Send + Sync {
    fn check(&self, old: &Schema, new: &Schema) -> Result<(), CompatibilityError>;
    fn level(&self) -> CompatibilityLevel;
}
```

### 1.3 Storage Layer

**`sqlx` (v0.8) + `sqlx-postgres` (v0.8)**
- **Rationale**: Async, compile-time checked SQL with connection pooling
- **Use Case**: Primary metadata store (PostgreSQL)
- **Performance**: Prepared statement caching, pipeline mode for batching
- **Features**:
  - Compile-time query verification via `sqlx::query!` macro
  - Automatic migration management
  - Transaction support with SERIALIZABLE isolation
- **Schema**: JSONB columns for flexible schema storage, BTrees for versioning

**`redb` (v2.1)**
- **Rationale**: Pure Rust embedded key-value store, ACID guarantees
- **Use Case**: Fast local cache, single-node deployments
- **Performance**: LMDB-like speed, ~100ns reads, copy-on-write B-trees
- **Safety**: Compile-time type checking for table definitions
- **Advantage**: Zero-copy reads, crash-safe without WAL overhead

**`sled` (v0.34)** (Alternative consideration)
- **Rationale**: Lock-free BwTree-based embedded database
- **Use Case**: High-concurrency local storage
- **Trade-offs**: Faster writes than redb, but less mature API
- **Note**: Consider for write-heavy workloads

**Architecture Decision**: Hybrid approach
```rust
pub enum StorageBackend {
    // Production multi-node
    Postgres(sqlx::PgPool),

    // Single-node / edge deployments
    Embedded(redb::Database),

    // In-memory for testing
    Memory(Arc<RwLock<HashMap<SchemaId, Schema>>>),
}
```

### 1.4 API Layer

**`axum` (v0.7)**
- **Rationale**: Ergonomic async web framework built on tokio/tower
- **Use Case**: REST API endpoints
- **Performance**: Minimal overhead, direct tower middleware integration
- **Features**:
  - Type-safe extractors
  - Automatic OpenAPI generation via `utoipa-axum`
  - WebSocket support for schema change notifications
  - Built-in request tracing integration

**`tonic` (v0.11) + `tonic-build` (v0.11)**
- **Rationale**: gRPC implementation with excellent performance
- **Use Case**: High-throughput schema validation service
- **Performance**: HTTP/2 multiplexing, protobuf binary encoding
- **Features**:
  - Bidirectional streaming for batch operations
  - Load balancing via `tower` middleware
  - TLS support with `rustls`
  - Automatic client generation

**`tower` (v0.4) + `tower-http` (v0.5)**
- **Rationale**: Modular service abstractions for middleware
- **Use Case**: Rate limiting, authentication, tracing, metrics
- **Features**:
  - `tower::ServiceBuilder` for middleware composition
  - `tower-http::cors`, `tower-http::compression`
  - `tower::limit::RateLimit` for API throttling

**`utoipa` (v4.2) + `utoipa-swagger-ui` (v6.0)**
- **Rationale**: OpenAPI 3.1 generation from Rust types
- **Use Case**: Automatic API documentation
- **Features**: Derive macros for schema/endpoint documentation

### 1.5 Versioning & Semantic Versioning

**`semver` (v1.0)**
- **Rationale**: Robust semantic versioning implementation
- **Use Case**: Schema version parsing, comparison, validation
- **Features**:
  - Version range parsing (npm/Cargo style)
  - Precedence ordering
  - Pre-release and build metadata support

**Custom Extension**:
```rust
pub struct SchemaVersion {
    semver: semver::Version,
    git_sha: Option<String>,      // Optional git commit
    timestamp: chrono::DateTime<chrono::Utc>,
    compatibility_level: CompatibilityLevel,
}
```

### 1.6 Async Runtime & Concurrency

**`tokio` (v1.35) - Full features**
- **Rationale**: Industry-standard async runtime
- **Use Case**: All async I/O operations
- **Features**: Multi-threaded scheduler, work-stealing, async FS I/O

**`rayon` (v1.8)**
- **Rationale**: Data parallelism for CPU-bound tasks
- **Use Case**: Parallel schema validation, batch compatibility checks
- **Integration**: Complements tokio for blocking operations

**`parking_lot` (v0.12)**
- **Rationale**: Faster RwLock/Mutex than std
- **Use Case**: Shared state protection (caches, connection pools)
- **Performance**: ~2x faster than std::sync in uncontended cases

### 1.7 Observability & Monitoring

**`tracing` (v0.1) + `tracing-subscriber` (v0.3)**
- **Rationale**: Structured logging with span-based context
- **Use Case**: Request tracing, performance profiling
- **Integration**: Native axum/tonic support

**`metrics` (v0.22) + `metrics-exporter-prometheus` (v0.13)**
- **Rationale**: Prometheus-compatible metrics
- **Use Case**: Schema operation counters, latency histograms
- **Metrics**:
  - `schema_validations_total` (counter)
  - `schema_validation_duration_seconds` (histogram)
  - `active_schemas` (gauge)
  - `compatibility_check_failures_total` (counter)

**`opentelemetry` (v0.21) + `opentelemetry-jaeger` (v0.20)**
- **Rationale**: Distributed tracing for microservices
- **Use Case**: Cross-service request tracing in LLM DevOps pipeline

### 1.8 Configuration & CLI

**`clap` (v4.5) - derive features**
- **Rationale**: Full-featured CLI parsing with derive macros
- **Use Case**: Schema registry CLI tool
- **Features**: Subcommands, validation, help generation

**`config` (v0.14)**
- **Rationale**: Layered configuration (files, env vars, CLI)
- **Use Case**: Runtime configuration management
- **Formats**: TOML, YAML, JSON support

**`dotenvy` (v0.15)**
- **Rationale**: .env file loading
- **Use Case**: Development environment configuration

### 1.9 Error Handling

**`thiserror` (v1.0)**
- **Rationale**: Ergonomic error derive macros
- **Use Case**: Domain error types
```rust
#[derive(thiserror::Error, Debug)]
pub enum RegistryError {
    #[error("Schema not found: {0}")]
    SchemaNotFound(SchemaId),

    #[error("Compatibility check failed: {0}")]
    IncompatibleSchema(String),

    #[error("Storage error: {0}")]
    Storage(#[from] sqlx::Error),
}
```

**`anyhow` (v1.0)**
- **Rationale**: Dynamic error handling for applications
- **Use Case**: CLI tool, main application errors

### 1.10 Testing & Mocking

**`mockall` (v0.12)**
- **Rationale**: Mock object generation
- **Use Case**: Unit testing with mocked storage/validators

**`proptest` (v1.4)**
- **Rationale**: Property-based testing
- **Use Case**: Fuzz testing schema parsers, compatibility checkers

**`criterion` (v0.5)**
- **Rationale**: Statistical benchmarking
- **Use Case**: Performance regression testing

## 2. API Design

### 2.1 REST API (OpenAPI 3.1)

```rust
use axum::{Router, routing::{get, post, put, delete}};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        register_schema,
        get_schema,
        list_schemas,
        validate_compatibility,
        search_schemas,
    ),
    components(schemas(
        Schema,
        SchemaMetadata,
        CompatibilityReport,
    ))
)]
pub struct ApiDoc;

pub fn create_router() -> Router {
    Router::new()
        // Schema Management
        .route("/api/v1/schemas", post(register_schema))
        .route("/api/v1/schemas", get(list_schemas))
        .route("/api/v1/schemas/:id", get(get_schema))
        .route("/api/v1/schemas/:id/versions", get(list_versions))
        .route("/api/v1/schemas/:id/versions/:version", get(get_schema_version))

        // Validation
        .route("/api/v1/validate/schema", post(validate_schema))
        .route("/api/v1/validate/data", post(validate_data))
        .route("/api/v1/compatibility/check", post(validate_compatibility))

        // Search & Discovery
        .route("/api/v1/search", get(search_schemas))
        .route("/api/v1/schemas/:id/dependents", get(get_dependents))
        .route("/api/v1/schemas/:id/dependencies", get(get_dependencies))

        // Metadata
        .route("/api/v1/subjects", get(list_subjects))
        .route("/api/v1/subjects/:subject/versions", get(list_subject_versions))

        // Health & Metrics
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))

        // WebSocket for real-time updates
        .route("/api/v1/stream/changes", get(schema_change_stream))
}
```

#### Key Endpoints

**POST /api/v1/schemas** - Register new schema
```rust
#[utoipa::path(
    post,
    path = "/api/v1/schemas",
    request_body = RegisterSchemaRequest,
    responses(
        (status = 201, description = "Schema registered", body = SchemaResponse),
        (status = 409, description = "Incompatible schema"),
    )
)]
async fn register_schema(
    State(registry): State<Arc<SchemaRegistry>>,
    Json(req): Json<RegisterSchemaRequest>,
) -> Result<Json<SchemaResponse>, RegistryError> {
    // Implementation
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct RegisterSchemaRequest {
    pub subject: String,
    pub schema: serde_json::Value,
    pub schema_type: SchemaType,
    pub compatibility_level: Option<CompatibilityLevel>,
    pub metadata: HashMap<String, String>,
}
```

**POST /api/v1/validate/data** - Validate data against schema
```rust
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct ValidateDataRequest {
    pub schema_id: SchemaId,
    pub data: serde_json::Value,
    pub strict: bool,  // Fail on unknown fields
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub validation_time_ms: f64,
}
```

**POST /api/v1/compatibility/check** - Check compatibility
```rust
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CompatibilityCheckRequest {
    pub subject: String,
    pub new_schema: serde_json::Value,
    pub level: CompatibilityLevel,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct CompatibilityReport {
    pub compatible: bool,
    pub level: CompatibilityLevel,
    pub violations: Vec<CompatibilityViolation>,
    pub compared_versions: Vec<semver::Version>,
}
```

### 2.2 gRPC Service Definition

**proto/schema_registry.proto**
```protobuf
syntax = "proto3";

package schema_registry.v1;

service SchemaRegistry {
  // Schema Management
  rpc RegisterSchema(RegisterSchemaRequest) returns (RegisterSchemaResponse);
  rpc GetSchema(GetSchemaRequest) returns (GetSchemaResponse);
  rpc ListSchemas(ListSchemasRequest) returns (stream Schema);

  // Validation
  rpc ValidateData(ValidateDataRequest) returns (ValidationReport);
  rpc ValidateSchema(ValidateSchemaRequest) returns (SchemaValidationReport);

  // Compatibility
  rpc CheckCompatibility(CompatibilityCheckRequest) returns (CompatibilityReport);

  // Batch Operations
  rpc BatchValidate(stream ValidateDataRequest) returns (stream ValidationReport);
  rpc BatchRegister(stream RegisterSchemaRequest) returns (stream RegisterSchemaResponse);

  // Real-time Streaming
  rpc StreamSchemaChanges(StreamRequest) returns (stream SchemaChangeEvent);
}

message Schema {
  string id = 1;
  string subject = 2;
  string version = 3;  // Semantic version
  SchemaType schema_type = 4;
  bytes schema_content = 5;  // Serialized schema
  map<string, string> metadata = 6;
  google.protobuf.Timestamp created_at = 7;
  CompatibilityLevel compatibility_level = 8;
}

enum SchemaType {
  SCHEMA_TYPE_UNSPECIFIED = 0;
  SCHEMA_TYPE_JSON = 1;
  SCHEMA_TYPE_AVRO = 2;
  SCHEMA_TYPE_PROTOBUF = 3;
  SCHEMA_TYPE_THRIFT = 4;
}

enum CompatibilityLevel {
  COMPATIBILITY_LEVEL_UNSPECIFIED = 0;
  COMPATIBILITY_LEVEL_BACKWARD = 1;
  COMPATIBILITY_LEVEL_FORWARD = 2;
  COMPATIBILITY_LEVEL_FULL = 3;
  COMPATIBILITY_LEVEL_TRANSITIVE = 4;
  COMPATIBILITY_LEVEL_NONE = 5;
}

message RegisterSchemaRequest {
  string subject = 1;
  bytes schema_content = 2;
  SchemaType schema_type = 3;
  map<string, string> metadata = 4;
  optional CompatibilityLevel compatibility_level = 5;
}

message ValidationReport {
  bool valid = 1;
  repeated ValidationError errors = 2;
  repeated ValidationWarning warnings = 3;
  double validation_time_ms = 4;
}

message CompatibilityReport {
  bool compatible = 1;
  CompatibilityLevel level = 2;
  repeated CompatibilityViolation violations = 3;
  repeated string compared_versions = 4;
}
```

### 2.3 Async Patterns

**Tokio Integration**
```rust
// Main server setup
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // Load configuration
    let config = Config::from_env()?;

    // Initialize storage backend
    let storage = match config.storage_backend {
        StorageBackendConfig::Postgres { url } => {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(50)
                .acquire_timeout(Duration::from_secs(3))
                .connect(&url)
                .await?;
            StorageBackend::Postgres(pool)
        }
        StorageBackendConfig::Embedded { path } => {
            let db = redb::Database::create(path)?;
            StorageBackend::Embedded(db)
        }
    };

    // Initialize registry
    let registry = Arc::new(SchemaRegistry::new(storage, config.clone()).await?);

    // Spawn background tasks
    tokio::spawn(cleanup_old_versions(registry.clone()));
    tokio::spawn(metrics_aggregation(registry.clone()));

    // Start REST API
    let rest_server = {
        let app = create_router()
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CompressionLayer::new())
                    .layer(CorsLayer::permissive())
                    .layer(RateLimitLayer::new(100, Duration::from_secs(1)))
            )
            .with_state(registry.clone());

        axum::serve(
            TcpListener::bind(&config.rest_addr).await?,
            app.into_make_service()
        )
    };

    // Start gRPC server
    let grpc_server = {
        let svc = SchemaRegistryService::new(registry.clone());
        tonic::transport::Server::builder()
            .add_service(SchemaRegistryServer::new(svc))
            .serve(config.grpc_addr)
    };

    // Run both servers concurrently
    tokio::try_join!(rest_server, grpc_server)?;

    Ok(())
}
```

**Async Validation Pipeline**
```rust
pub async fn validate_batch(
    &self,
    requests: Vec<ValidateDataRequest>,
) -> Vec<ValidationReport> {
    // Parallel validation using futures
    let futures = requests.into_iter().map(|req| {
        let validator = self.validator_cache.get(&req.schema_id);
        async move {
            validator.validate(&req.data).await
        }
    });

    // Execute concurrently with bounded parallelism
    futures::stream::iter(futures)
        .buffer_unordered(50)  // Max 50 concurrent validations
        .collect()
        .await
}
```

## 3. Module Architecture

### 3.1 Crate Structure

```
llm-schema-registry/
├── Cargo.toml                    # Workspace manifest
├── crates/
│   ├── registry-core/            # Core registry engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── schema.rs         # Schema representation
│   │       ├── registry.rs       # Main registry logic
│   │       ├── versioning.rs     # Version management
│   │       └── subject.rs        # Subject management
│   │
│   ├── validation/               # Validation engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── validator.rs      # Generic validator trait
│   │       ├── json_validator.rs # JSON Schema validation
│   │       ├── avro_validator.rs # Avro validation
│   │       ├── proto_validator.rs# Protobuf validation
│   │       └── cache.rs          # Compiled validator cache
│   │
│   ├── compatibility/            # Compatibility checker
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── checker.rs        # Main checker logic
│   │       ├── rules/
│   │       │   ├── mod.rs
│   │       │   ├── json.rs       # JSON Schema rules
│   │       │   ├── avro.rs       # Avro rules
│   │       │   └── proto.rs      # Protobuf rules
│   │       └── levels.rs         # Compatibility level logic
│   │
│   ├── storage/                  # Storage layer abstraction
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── backend.rs        # Storage trait
│   │       ├── postgres.rs       # PostgreSQL impl
│   │       ├── redb_impl.rs      # Embedded DB impl
│   │       ├── memory.rs         # In-memory impl
│   │       └── migrations/       # SQL migrations
│   │
│   ├── api/                      # API layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── rest/
│   │       │   ├── mod.rs
│   │       │   ├── handlers.rs
│   │       │   └── routes.rs
│   │       ├── grpc/
│   │       │   ├── mod.rs
│   │       │   └── service.rs
│   │       └── models.rs         # API models
│   │
│   ├── cli/                      # CLI tool
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       └── commands/
│   │           ├── register.rs
│   │           ├── validate.rs
│   │           └── search.rs
│   │
│   └── server/                   # Server binary
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
│
├── proto/                        # Protocol buffer definitions
│   └── schema_registry.proto
│
└── benches/                      # Criterion benchmarks
    ├── validation.rs
    └── compatibility.rs
```

### 3.2 Module Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│                         SERVER                              │
│                    (Binary Crate)                           │
└──────────────┬──────────────────────────────────┬───────────┘
               │                                  │
               ▼                                  ▼
       ┌──────────────┐                  ┌──────────────┐
       │   API REST   │                  │   API gRPC   │
       └──────┬───────┘                  └──────┬───────┘
              │                                  │
              └─────────────┬────────────────────┘
                            │
                            ▼
                   ┌────────────────┐
                   │ REGISTRY-CORE  │
                   │   (Main Logic) │
                   └────────┬───────┘
                            │
          ┌─────────────────┼─────────────────┐
          │                 │                 │
          ▼                 ▼                 ▼
  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
  │  VALIDATION  │  │COMPATIBILITY │  │   STORAGE    │
  │    ENGINE    │  │   CHECKER    │  │   BACKEND    │
  └──────────────┘  └──────────────┘  └──────────────┘
          │                 │                 │
          │                 │                 │
          └─────────────────┴─────────────────┘
                            │
                    External Crates
              (serde, sqlx, jsonschema, etc.)
```

### 3.3 Core Module: registry-core

**Key Types**
```rust
// src/schema.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub id: SchemaId,
    pub subject: String,
    pub version: SchemaVersion,
    pub schema_type: SchemaType,
    pub content: SchemaContent,
    pub metadata: SchemaMetadata,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchemaId(uuid::Uuid);

#[derive(Debug, Clone)]
pub enum SchemaContent {
    Json(serde_json::Value),
    Avro(apache_avro::Schema),
    Protobuf(Vec<u8>),  // Serialized FileDescriptorProto
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub owner: String,
    pub compatibility_level: CompatibilityLevel,
    pub custom: HashMap<String, serde_json::Value>,
}

// src/registry.rs
pub struct SchemaRegistry {
    storage: Arc<dyn StorageBackend>,
    validator: Arc<ValidatorEngine>,
    compatibility: Arc<CompatibilityChecker>,
    config: RegistryConfig,

    // Caches
    schema_cache: Arc<RwLock<LruCache<SchemaId, Arc<Schema>>>>,
    subject_cache: Arc<RwLock<HashMap<String, Vec<SchemaVersion>>>>,
}

impl SchemaRegistry {
    pub async fn register_schema(
        &self,
        subject: String,
        schema: SchemaContent,
        metadata: SchemaMetadata,
    ) -> Result<Schema, RegistryError> {
        // 1. Validate schema is well-formed
        self.validator.validate_schema(&schema).await?;

        // 2. Get existing versions for subject
        let existing = self.storage.get_versions(&subject).await?;

        // 3. Check compatibility
        if let Some(latest) = existing.last() {
            self.compatibility
                .check(&latest.content, &schema, &metadata.compatibility_level)
                .await?;
        }

        // 4. Determine next version
        let version = self.next_version(&existing, &schema)?;

        // 5. Store schema
        let id = SchemaId::new();
        let schema_obj = Schema {
            id,
            subject: subject.clone(),
            version,
            schema_type: schema.schema_type(),
            content: schema,
            metadata,
            created_at: chrono::Utc::now(),
        };

        self.storage.store_schema(&schema_obj).await?;

        // 6. Invalidate caches
        self.invalidate_caches(&subject);

        // 7. Emit event
        self.emit_schema_registered(&schema_obj).await;

        Ok(schema_obj)
    }

    pub async fn validate_data(
        &self,
        schema_id: SchemaId,
        data: &serde_json::Value,
    ) -> Result<ValidationReport, RegistryError> {
        // Get schema (cached)
        let schema = self.get_schema(schema_id).await?;

        // Validate
        self.validator.validate(data, &schema).await
    }
}
```

### 3.4 Validation Module

```rust
// validation/src/validator.rs
#[async_trait]
pub trait Validator: Send + Sync {
    async fn validate(
        &self,
        data: &serde_json::Value,
        schema: &Schema,
    ) -> Result<ValidationReport, ValidationError>;

    fn schema_type(&self) -> SchemaType;
}

// validation/src/json_validator.rs
pub struct JsonValidator {
    // Cache of compiled validators
    cache: Arc<RwLock<LruCache<SchemaId, jsonschema::Validator>>>,
}

impl JsonValidator {
    pub fn compile(&self, schema: &serde_json::Value) -> Result<jsonschema::Validator, Error> {
        jsonschema::validator_for(schema)
            .map_err(|e| Error::InvalidSchema(e.to_string()))
    }
}

#[async_trait]
impl Validator for JsonValidator {
    async fn validate(
        &self,
        data: &serde_json::Value,
        schema: &Schema,
    ) -> Result<ValidationReport, ValidationError> {
        let start = std::time::Instant::now();

        // Get or compile validator
        let validator = {
            let mut cache = self.cache.write();
            match cache.get(&schema.id) {
                Some(v) => v.clone(),
                None => {
                    let v = self.compile(schema.content.as_json())?;
                    cache.put(schema.id, v.clone());
                    v
                }
            }
        };

        // Validate
        let result = validator.validate(data);

        let errors = result
            .err()
            .map(|e| e.map(|error| ValidationError {
                path: error.instance_path.to_string(),
                message: error.to_string(),
            }).collect())
            .unwrap_or_default();

        Ok(ValidationReport {
            valid: errors.is_empty(),
            errors,
            warnings: vec![],
            validation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        })
    }
}
```

### 3.5 Compatibility Module

```rust
// compatibility/src/checker.rs
pub struct CompatibilityChecker {
    rules: HashMap<SchemaType, Arc<dyn CompatibilityRuleSet>>,
}

impl CompatibilityChecker {
    pub async fn check(
        &self,
        old: &SchemaContent,
        new: &SchemaContent,
        level: &CompatibilityLevel,
    ) -> Result<CompatibilityReport, Error> {
        if old.schema_type() != new.schema_type() {
            return Err(Error::TypeMismatch);
        }

        let ruleset = self.rules.get(&old.schema_type())
            .ok_or(Error::UnsupportedSchemaType)?;

        ruleset.check(old, new, level).await
    }
}

// compatibility/src/rules/json.rs
pub struct JsonCompatibilityRules;

#[async_trait]
impl CompatibilityRuleSet for JsonCompatibilityRules {
    async fn check(
        &self,
        old: &SchemaContent,
        new: &SchemaContent,
        level: &CompatibilityLevel,
    ) -> Result<CompatibilityReport, Error> {
        let old_schema = old.as_json()?;
        let new_schema = new.as_json()?;

        let mut violations = Vec::new();

        match level {
            CompatibilityLevel::Backward => {
                // New schema can read old data
                violations.extend(self.check_backward(old_schema, new_schema)?);
            }
            CompatibilityLevel::Forward => {
                // Old schema can read new data
                violations.extend(self.check_forward(old_schema, new_schema)?);
            }
            CompatibilityLevel::Full => {
                violations.extend(self.check_backward(old_schema, new_schema)?);
                violations.extend(self.check_forward(old_schema, new_schema)?);
            }
            CompatibilityLevel::Transitive => {
                // Check against all previous versions (handled by caller)
                violations.extend(self.check_backward(old_schema, new_schema)?);
                violations.extend(self.check_forward(old_schema, new_schema)?);
            }
            CompatibilityLevel::None => {}
        }

        Ok(CompatibilityReport {
            compatible: violations.is_empty(),
            level: level.clone(),
            violations,
            compared_versions: vec![],
        })
    }
}

impl JsonCompatibilityRules {
    fn check_backward(
        &self,
        old: &serde_json::Value,
        new: &serde_json::Value,
    ) -> Result<Vec<CompatibilityViolation>, Error> {
        let mut violations = Vec::new();

        // Rule: New required fields violate backward compatibility
        if let (Some(old_req), Some(new_req)) = (
            old.get("required").and_then(|v| v.as_array()),
            new.get("required").and_then(|v| v.as_array()),
        ) {
            for req in new_req {
                if !old_req.contains(req) {
                    violations.push(CompatibilityViolation {
                        rule: "new_required_field".to_string(),
                        path: format!("/required/{}", req),
                        message: format!("New required field '{}' breaks backward compatibility", req),
                        severity: Severity::Error,
                    });
                }
            }
        }

        // Rule: Removing fields violates backward compatibility
        if let (Some(old_props), Some(new_props)) = (
            old.get("properties").and_then(|v| v.as_object()),
            new.get("properties").and_then(|v| v.as_object()),
        ) {
            for (key, _) in old_props {
                if !new_props.contains_key(key) {
                    violations.push(CompatibilityViolation {
                        rule: "field_removed".to_string(),
                        path: format!("/properties/{}", key),
                        message: format!("Removed field '{}' breaks backward compatibility", key),
                        severity: Severity::Error,
                    });
                }
            }
        }

        // Rule: Type changes
        self.check_type_compatibility(old, new, &mut violations)?;

        Ok(violations)
    }
}
```

### 3.6 Storage Module

```rust
// storage/src/backend.rs
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store_schema(&self, schema: &Schema) -> Result<(), StorageError>;
    async fn get_schema(&self, id: SchemaId) -> Result<Option<Schema>, StorageError>;
    async fn get_versions(&self, subject: &str) -> Result<Vec<Schema>, StorageError>;
    async fn get_schema_by_version(
        &self,
        subject: &str,
        version: &SchemaVersion,
    ) -> Result<Option<Schema>, StorageError>;
    async fn list_subjects(&self) -> Result<Vec<String>, StorageError>;
    async fn delete_schema(&self, id: SchemaId) -> Result<(), StorageError>;
    async fn search_schemas(&self, query: &SearchQuery) -> Result<Vec<Schema>, StorageError>;
}

// storage/src/postgres.rs
pub struct PostgresBackend {
    pool: sqlx::PgPool,
}

#[async_trait]
impl StorageBackend for PostgresBackend {
    async fn store_schema(&self, schema: &Schema) -> Result<(), StorageError> {
        sqlx::query!(
            r#"
            INSERT INTO schemas (
                id, subject, version_major, version_minor, version_patch,
                schema_type, content, metadata, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            schema.id.as_uuid(),
            &schema.subject,
            schema.version.semver.major as i32,
            schema.version.semver.minor as i32,
            schema.version.semver.patch as i32,
            schema.schema_type.as_str(),
            serde_json::to_value(&schema.content)?,
            serde_json::to_value(&schema.metadata)?,
            schema.created_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn search_schemas(&self, query: &SearchQuery) -> Result<Vec<Schema>, StorageError> {
        let sql = r#"
            SELECT * FROM schemas
            WHERE
                ($1::text IS NULL OR subject ILIKE '%' || $1 || '%')
                AND ($2::text IS NULL OR metadata @> $2::jsonb)
                AND ($3::text IS NULL OR schema_type = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
        "#;

        let rows = sqlx::query_as::<_, SchemaRow>(sql)
            .bind(&query.subject_pattern)
            .bind(query.metadata_filter.as_ref().map(serde_json::to_value).transpose()?)
            .bind(query.schema_type.as_ref().map(|t| t.as_str()))
            .bind(query.limit.unwrap_or(100) as i64)
            .bind(query.offset.unwrap_or(0) as i64)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(Schema::try_from).collect()
    }
}
```

## 4. Storage Schema Design

### 4.1 PostgreSQL Schema

```sql
-- migrations/001_initial_schema.sql

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";  -- For trigram similarity search

-- Main schemas table
CREATE TABLE schemas (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    subject VARCHAR(255) NOT NULL,

    -- Semantic version components
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL,
    version_prerelease VARCHAR(255),
    version_build VARCHAR(255),

    schema_type VARCHAR(50) NOT NULL,  -- 'json', 'avro', 'protobuf'
    content JSONB NOT NULL,            -- Schema content
    metadata JSONB NOT NULL DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255),

    -- Compatibility settings
    compatibility_level VARCHAR(50) NOT NULL,

    -- Soft delete
    deleted_at TIMESTAMPTZ,

    CONSTRAINT unique_subject_version UNIQUE (
        subject, version_major, version_minor, version_patch,
        COALESCE(version_prerelease, ''), COALESCE(version_build, '')
    )
);

-- Indexes for fast retrieval
CREATE INDEX idx_schemas_subject ON schemas(subject) WHERE deleted_at IS NULL;
CREATE INDEX idx_schemas_created_at ON schemas(created_at DESC);
CREATE INDEX idx_schemas_type ON schemas(schema_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_schemas_metadata_gin ON schemas USING GIN (metadata jsonb_path_ops);
CREATE INDEX idx_schemas_content_gin ON schemas USING GIN (content jsonb_path_ops);
CREATE INDEX idx_schemas_subject_trgm ON schemas USING GIN (subject gin_trgm_ops);

-- Composite index for version lookups
CREATE INDEX idx_schemas_subject_version ON schemas(
    subject, version_major DESC, version_minor DESC, version_patch DESC
) WHERE deleted_at IS NULL;

-- Schema dependencies (for graph traversal)
CREATE TABLE schema_dependencies (
    schema_id UUID NOT NULL REFERENCES schemas(id) ON DELETE CASCADE,
    depends_on_id UUID NOT NULL REFERENCES schemas(id) ON DELETE CASCADE,
    dependency_type VARCHAR(50) NOT NULL,  -- 'reference', 'import', 'extends'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (schema_id, depends_on_id)
);

CREATE INDEX idx_deps_schema ON schema_dependencies(schema_id);
CREATE INDEX idx_deps_depends_on ON schema_dependencies(depends_on_id);

-- Validation history (for auditing)
CREATE TABLE validation_history (
    id BIGSERIAL PRIMARY KEY,
    schema_id UUID NOT NULL REFERENCES schemas(id) ON DELETE CASCADE,
    data_hash VARCHAR(64) NOT NULL,  -- SHA-256 of validated data
    valid BOOLEAN NOT NULL,
    error_count INTEGER NOT NULL DEFAULT 0,
    validated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    duration_ms DOUBLE PRECISION NOT NULL
);

CREATE INDEX idx_validation_schema ON validation_history(schema_id, validated_at DESC);
CREATE INDEX idx_validation_timestamp ON validation_history(validated_at DESC);

-- Compatibility check history
CREATE TABLE compatibility_checks (
    id BIGSERIAL PRIMARY KEY,
    subject VARCHAR(255) NOT NULL,
    old_version VARCHAR(255) NOT NULL,
    new_version VARCHAR(255) NOT NULL,
    compatibility_level VARCHAR(50) NOT NULL,
    compatible BOOLEAN NOT NULL,
    violations JSONB,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_compat_subject ON compatibility_checks(subject, checked_at DESC);

-- Subjects metadata
CREATE TABLE subjects (
    name VARCHAR(255) PRIMARY KEY,
    default_compatibility_level VARCHAR(50) NOT NULL,
    description TEXT,
    tags TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Materialized view for fast subject version listings
CREATE MATERIALIZED VIEW subject_versions AS
SELECT
    subject,
    COUNT(*) as version_count,
    MAX(created_at) as latest_version_at,
    ARRAY_AGG(
        version_major || '.' || version_minor || '.' || version_patch
        ORDER BY version_major DESC, version_minor DESC, version_patch DESC
    ) as versions
FROM schemas
WHERE deleted_at IS NULL
GROUP BY subject;

CREATE UNIQUE INDEX idx_subject_versions ON subject_versions(subject);

-- Refresh function for materialized view
CREATE OR REPLACE FUNCTION refresh_subject_versions()
RETURNS TRIGGER AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY subject_versions;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_refresh_subject_versions
AFTER INSERT OR UPDATE OR DELETE ON schemas
FOR EACH STATEMENT
EXECUTE FUNCTION refresh_subject_versions();
```

### 4.2 Embedded Database Schema (redb)

```rust
// storage/src/redb_impl.rs
use redb::{Database, TableDefinition, ReadableTable};

// Table definitions
const SCHEMAS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("schemas");
const SUBJECTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("subjects");
const SUBJECT_VERSIONS_TABLE: TableDefinition<(&str, u32, u32, u32), &[u8]> =
    TableDefinition::new("subject_versions");

pub struct RedbBackend {
    db: Arc<Database>,
}

impl RedbBackend {
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let db = Database::create(path)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(SCHEMAS_TABLE)?;
            write_txn.open_table(SUBJECTS_TABLE)?;
            write_txn.open_table(SUBJECT_VERSIONS_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }
}

#[async_trait]
impl StorageBackend for RedbBackend {
    async fn store_schema(&self, schema: &Schema) -> Result<(), StorageError> {
        let key = schema.id.to_string();
        let value = bincode::serialize(schema)?;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(SCHEMAS_TABLE)?;
            table.insert(key.as_str(), value.as_slice())?;

            // Also index by subject+version
            let mut sv_table = write_txn.open_table(SUBJECT_VERSIONS_TABLE)?;
            let version_key = (
                schema.subject.as_str(),
                schema.version.semver.major as u32,
                schema.version.semver.minor as u32,
                schema.version.semver.patch as u32,
            );
            sv_table.insert(version_key, value.as_slice())?;
        }
        write_txn.commit()?;

        Ok(())
    }

    async fn get_versions(&self, subject: &str) -> Result<Vec<Schema>, StorageError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SUBJECT_VERSIONS_TABLE)?;

        let range_start = (subject, 0u32, 0u32, 0u32);
        let range_end = (subject, u32::MAX, u32::MAX, u32::MAX);

        let schemas: Result<Vec<_>, _> = table
            .range(range_start..=range_end)?
            .map(|result| {
                let (_, value) = result?;
                bincode::deserialize(value.value()).map_err(Into::into)
            })
            .collect();

        schemas
    }
}
```

### 4.3 Caching Strategy

```rust
pub struct CacheManager {
    // L1: In-memory LRU cache (hot schemas)
    schema_cache: Arc<RwLock<LruCache<SchemaId, Arc<Schema>>>>,

    // L2: Compiled validators (expensive to create)
    validator_cache: Arc<RwLock<LruCache<SchemaId, Arc<dyn Validator>>>>,

    // L3: Subject version list cache
    subject_cache: Arc<RwLock<HashMap<String, Arc<Vec<SchemaVersion>>>>>,

    // Metrics
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            schema_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(config.schema_cache_size).unwrap()
            ))),
            validator_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(config.validator_cache_size).unwrap()
            ))),
            subject_cache: Arc::new(RwLock::new(HashMap::new())),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn get_schema(&self, id: SchemaId) -> Option<Arc<Schema>> {
        let cache = self.schema_cache.read();
        let result = cache.peek(&id).cloned();

        if result.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        if hits + misses == 0.0 {
            0.0
        } else {
            hits / (hits + misses)
        }
    }
}
```

## 5. Integration Patterns

### 5.1 Integration with LLM DevOps Modules

```rust
// Integration points for LLM pipeline

/// Model Registry Integration
pub struct ModelSchemaBinding {
    registry: Arc<SchemaRegistry>,
}

impl ModelSchemaBinding {
    /// Validate model input/output schemas at deployment time
    pub async fn validate_model_schemas(
        &self,
        model_id: &str,
        input_schema_id: SchemaId,
        output_schema_id: SchemaId,
    ) -> Result<(), Error> {
        let input_schema = self.registry.get_schema(input_schema_id).await?;
        let output_schema = self.registry.get_schema(output_schema_id).await?;

        // Register binding in metadata
        let metadata = json!({
            "model_id": model_id,
            "input_schema": input_schema_id,
            "output_schema": output_schema_id,
            "validated_at": chrono::Utc::now(),
        });

        self.registry.store_metadata("model_bindings", &metadata).await?;
        Ok(())
    }

    /// Real-time validation of model I/O
    pub async fn validate_inference(
        &self,
        model_id: &str,
        input: &serde_json::Value,
        output: &serde_json::Value,
    ) -> Result<InferenceValidationReport, Error> {
        let binding = self.get_binding(model_id).await?;

        let input_validation = self.registry
            .validate_data(binding.input_schema_id, input)
            .await?;

        let output_validation = self.registry
            .validate_data(binding.output_schema_id, output)
            .await?;

        Ok(InferenceValidationReport {
            input: input_validation,
            output: output_validation,
        })
    }
}

/// Prompt Template Registry Integration
pub struct PromptSchemaValidator {
    registry: Arc<SchemaRegistry>,
}

impl PromptSchemaValidator {
    /// Validate prompt template variables against schema
    pub async fn validate_template(
        &self,
        template: &str,
        variables_schema_id: SchemaId,
    ) -> Result<TemplateValidation, Error> {
        // Extract template variables
        let variables = self.extract_variables(template);

        // Get schema
        let schema = self.registry.get_schema(variables_schema_id).await?;

        // Check all variables are defined in schema
        let mut missing = Vec::new();
        if let SchemaContent::Json(schema_json) = &schema.content {
            if let Some(props) = schema_json.get("properties") {
                for var in variables {
                    if !props.get(&var).is_some() {
                        missing.push(var);
                    }
                }
            }
        }

        Ok(TemplateValidation {
            valid: missing.is_empty(),
            missing_variables: missing,
        })
    }
}

/// Observability Integration
pub struct SchemaMetricsExporter {
    registry: Arc<SchemaRegistry>,
}

impl SchemaMetricsExporter {
    /// Export schema-related metrics to observability platform
    pub async fn export_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();

        // Schema counts by type
        let stats = self.registry.get_statistics().await;
        metrics.insert("schemas_total".to_string(), stats.total_schemas as f64);
        metrics.insert("subjects_total".to_string(), stats.total_subjects as f64);

        // Validation metrics
        metrics.insert("validations_per_second".to_string(), stats.validations_per_sec);
        metrics.insert("avg_validation_ms".to_string(), stats.avg_validation_ms);

        // Cache hit rate
        metrics.insert("cache_hit_rate".to_string(), stats.cache_hit_rate);

        metrics
    }
}

/// Event Bus Integration
pub struct SchemaEventPublisher {
    registry: Arc<SchemaRegistry>,
    event_bus: Arc<dyn EventBus>,
}

#[derive(Debug, Clone, Serialize)]
pub enum SchemaEvent {
    SchemaRegistered {
        schema_id: SchemaId,
        subject: String,
        version: String,
    },
    SchemaDeleted {
        schema_id: SchemaId,
    },
    ValidationFailed {
        schema_id: SchemaId,
        error_count: usize,
    },
    IncompatibilityDetected {
        subject: String,
        old_version: String,
        new_version: String,
        violations: Vec<String>,
    },
}

impl SchemaEventPublisher {
    pub async fn publish(&self, event: SchemaEvent) -> Result<(), Error> {
        self.event_bus.publish("schema.events", &event).await
    }
}
```

### 5.2 Client Libraries

```rust
// Auto-generated client library
pub struct SchemaRegistryClient {
    rest_client: reqwest::Client,
    grpc_client: Option<SchemaRegistryGrpcClient<tonic::transport::Channel>>,
    base_url: String,
    config: ClientConfig,
}

impl SchemaRegistryClient {
    /// Create REST client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            rest_client: reqwest::Client::new(),
            grpc_client: None,
            base_url: base_url.into(),
            config: ClientConfig::default(),
        }
    }

    /// Create gRPC client
    pub async fn with_grpc(mut self, endpoint: impl Into<String>) -> Result<Self, Error> {
        let channel = tonic::transport::Channel::from_shared(endpoint.into())?
            .connect()
            .await?;
        self.grpc_client = Some(SchemaRegistryGrpcClient::new(channel));
        Ok(self)
    }

    /// Register schema (auto-retries with exponential backoff)
    pub async fn register_schema(
        &self,
        request: RegisterSchemaRequest,
    ) -> Result<SchemaResponse, Error> {
        let url = format!("{}/api/v1/schemas", self.base_url);

        // Retry with exponential backoff
        let mut backoff = Duration::from_millis(100);
        for attempt in 0..self.config.max_retries {
            match self.rest_client.post(&url).json(&request).send().await {
                Ok(resp) if resp.status().is_success() => {
                    return resp.json().await.map_err(Into::into);
                }
                Ok(resp) if resp.status() == 409 => {
                    // Conflict - incompatible schema
                    let error: ApiError = resp.json().await?;
                    return Err(Error::IncompatibleSchema(error.message));
                }
                Ok(_) | Err(_) if attempt < self.config.max_retries - 1 => {
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
                Err(e) => return Err(e.into()),
                Ok(resp) => {
                    let error: ApiError = resp.json().await?;
                    return Err(Error::Api(error));
                }
            }
        }

        Err(Error::MaxRetriesExceeded)
    }

    /// Validate data (prefer gRPC for performance)
    pub async fn validate_data(
        &self,
        schema_id: SchemaId,
        data: &serde_json::Value,
    ) -> Result<ValidationReport, Error> {
        if let Some(grpc) = &self.grpc_client {
            // Use gRPC for better performance
            let request = ValidateDataRequest {
                schema_id: schema_id.to_string(),
                data: serde_json::to_vec(data)?,
                strict: true,
            };

            let response = grpc.clone().validate_data(request).await?;
            Ok(response.into_inner())
        } else {
            // Fallback to REST
            let url = format!("{}/api/v1/validate/data", self.base_url);
            self.rest_client
                .post(&url)
                .json(&json!({
                    "schema_id": schema_id,
                    "data": data,
                }))
                .send()
                .await?
                .json()
                .await
                .map_err(Into::into)
        }
    }
}
```

### 5.3 SDK Integration Examples

```rust
// Python bindings via PyO3
use pyo3::prelude::*;

#[pyclass]
struct PySchemaRegistry {
    inner: Arc<SchemaRegistryClient>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PySchemaRegistry {
    #[new]
    fn new(base_url: String) -> Self {
        Self {
            inner: Arc::new(SchemaRegistryClient::new(base_url)),
            runtime: tokio::runtime::Runtime::new().unwrap(),
        }
    }

    fn validate_data(&self, schema_id: String, data: &str) -> PyResult<bool> {
        let data: serde_json::Value = serde_json::from_str(data)?;
        let schema_id = SchemaId::parse(&schema_id)?;

        let result = self.runtime.block_on(async {
            self.inner.validate_data(schema_id, &data).await
        })?;

        Ok(result.valid)
    }
}

#[pymodule]
fn schema_registry(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PySchemaRegistry>()?;
    Ok(())
}
```

## 6. Performance Considerations

### 6.1 Optimization Strategies

**1. Compiled Validator Caching**
- Pre-compile JSON Schema validators at registration time
- Cache compiled validators with LRU eviction
- Target: <1μs validation for cached schemas

**2. Connection Pooling**
```rust
// PostgreSQL connection pool configuration
let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(50)
    .min_connections(10)
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

**3. Batch Operations**
```rust
// Batch validation using rayon
pub async fn validate_batch_parallel(
    &self,
    requests: Vec<(SchemaId, serde_json::Value)>,
) -> Vec<ValidationReport> {
    // Group by schema ID to maximize cache hits
    let mut grouped: HashMap<SchemaId, Vec<serde_json::Value>> = HashMap::new();
    for (schema_id, data) in requests {
        grouped.entry(schema_id).or_default().push(data);
    }

    // Parallel validation per schema
    let results: Vec<_> = grouped
        .into_par_iter()
        .flat_map(|(schema_id, data_items)| {
            let validator = self.get_validator(schema_id);
            data_items
                .par_iter()
                .map(|data| validator.validate(data))
                .collect::<Vec<_>>()
        })
        .collect();

    results
}
```

**4. Query Optimization**
- Use JSONB GIN indexes for metadata/content searches
- Materialized views for expensive aggregations
- Prepared statement caching
- Index-only scans for version listings

**5. Protocol Buffers for gRPC**
- Binary encoding reduces payload size by ~70%
- HTTP/2 multiplexing for concurrent requests
- Streaming for batch operations

### 6.2 Performance Targets

| Operation | Target Latency | Throughput |
|-----------|----------------|------------|
| Schema registration | <50ms (p99) | 1000 ops/sec |
| Schema retrieval (cached) | <1ms | 100k ops/sec |
| Data validation (cached) | <5ms (p99) | 10k ops/sec |
| Compatibility check | <100ms (p99) | 500 ops/sec |
| Search query | <200ms (p99) | 100 ops/sec |

### 6.3 Resource Limits

```rust
pub struct ResourceConfig {
    // Max schema size
    pub max_schema_size_bytes: usize,  // Default: 1MB

    // Max data payload for validation
    pub max_data_size_bytes: usize,    // Default: 10MB

    // Cache sizes
    pub schema_cache_size: usize,      // Default: 10000 entries
    pub validator_cache_size: usize,   // Default: 1000 entries

    // Rate limits
    pub rate_limit_per_ip: u32,        // Default: 100 req/sec
    pub rate_limit_global: u32,        // Default: 10000 req/sec

    // Timeouts
    pub validation_timeout: Duration,   // Default: 30s
    pub compatibility_timeout: Duration,// Default: 60s
}
```

### 6.4 Horizontal Scaling

```rust
// Distributed cache with Redis
pub struct DistributedCache {
    redis: Arc<redis::aio::MultiplexedConnection>,
    local: Arc<RwLock<LruCache<SchemaId, Arc<Schema>>>>,
}

impl DistributedCache {
    pub async fn get(&self, id: SchemaId) -> Option<Arc<Schema>> {
        // L1: Check local cache
        if let Some(schema) = self.local.read().peek(&id).cloned() {
            return Some(schema);
        }

        // L2: Check Redis
        let key = format!("schema:{}", id);
        if let Ok(Some(bytes)) = self.redis.get::<_, Option<Vec<u8>>>(&key).await {
            if let Ok(schema) = bincode::deserialize::<Schema>(&bytes) {
                let schema = Arc::new(schema);
                self.local.write().put(id, schema.clone());
                return Some(schema);
            }
        }

        None
    }

    pub async fn put(&self, id: SchemaId, schema: Arc<Schema>) {
        // Update local cache
        self.local.write().put(id, schema.clone());

        // Update Redis (fire and forget)
        let redis = self.redis.clone();
        let key = format!("schema:{}", id);
        let bytes = bincode::serialize(&*schema).unwrap();

        tokio::spawn(async move {
            let _: Result<(), _> = redis
                .set_ex(&key, bytes, 3600)  // 1 hour TTL
                .await;
        });
    }
}
```

## 7. Security Considerations

```rust
// Authentication middleware
pub struct AuthMiddleware {
    jwks: Arc<Jwks>,
}

impl AuthMiddleware {
    pub async fn verify_token(&self, token: &str) -> Result<Claims, Error> {
        let header = jsonwebtoken::decode_header(token)?;
        let kid = header.kid.ok_or(Error::MissingKid)?;

        let key = self.jwks.get_key(&kid).ok_or(Error::UnknownKey)?;

        let claims = jsonwebtoken::decode::<Claims>(
            token,
            &key,
            &jsonwebtoken::Validation::default(),
        )?;

        Ok(claims.claims)
    }
}

// RBAC authorization
pub enum Permission {
    SchemaRead,
    SchemaWrite,
    SchemaDelete,
    AdminAccess,
}

pub fn check_permission(claims: &Claims, permission: Permission) -> Result<(), Error> {
    if !claims.permissions.contains(&permission) {
        return Err(Error::Forbidden);
    }
    Ok(())
}
```

---

## Summary

This architecture provides a robust, performant, and scalable foundation for the LLM-Schema-Registry using Rust's type safety and performance characteristics. Key highlights:

1. **Type-safe crate selection**: Leveraging Rust's ecosystem for serialization, validation, and async I/O
2. **Dual API design**: REST for ease of use, gRPC for high-performance scenarios
3. **Modular architecture**: Clear separation of concerns across registry, validation, compatibility, storage, and API layers
4. **Flexible storage**: Support for PostgreSQL (distributed) and embedded databases (single-node)
5. **Performance-first**: Caching, connection pooling, parallel processing, and compiled validators
6. **Integration-ready**: Clear patterns for LLM DevOps ecosystem integration
7. **Production-ready**: Observability, security, rate limiting, and horizontal scaling support

The architecture follows SPARC principles by providing specific, actionable design decisions that can be directly implemented in the pseudocode phase.

