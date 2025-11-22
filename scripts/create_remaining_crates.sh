#!/bin/bash

# Security crate
cd crates/schema-registry-security
cat > Cargo.toml << 'EOF'
[package]
name = "schema-registry-security"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
schema-registry-core = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
jsonwebtoken = { workspace = true }
sha2 = { workspace = true }
argon2 = { workspace = true }
rand = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
EOF

cat > src/lib.rs << 'EOF'
//! Security layer: RBAC, ABAC, signatures, audit logging
pub mod rbac;
pub mod abac;
pub mod audit;

pub struct SecurityManager {}
impl SecurityManager {
    pub fn new() -> Self { Self {} }
}
impl Default for SecurityManager {
    fn default() -> Self { Self::new() }
}
EOF

mkdir -p src
echo "pub struct RbacManager;" > src/rbac.rs
echo "pub struct AbacManager;" > src/abac.rs
echo "pub struct AuditLogger;" > src/audit.rs

# Observability crate
cd ../schema-registry-observability
cat > Cargo.toml << 'EOF'
[package]
name = "schema-registry-observability"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
schema-registry-core = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
opentelemetry = { workspace = true }
opentelemetry-otlp = { workspace = true }
opentelemetry_sdk = { workspace = true }
tracing-opentelemetry = { workspace = true }
prometheus = { workspace = true }
metrics = { workspace = true }
serde = { workspace = true }
EOF

cat > src/lib.rs << 'EOF'
//! Observability: Prometheus metrics, OpenTelemetry tracing
pub mod metrics;
pub mod tracing_setup;

pub struct ObservabilityManager {}
impl ObservabilityManager {
    pub fn new() -> Self { Self {} }
}
impl Default for ObservabilityManager {
    fn default() -> Self { Self::new() }
}
EOF

echo "pub struct MetricsCollector;" > src/metrics.rs
echo "pub fn setup_tracing() {}" > src/tracing_setup.rs

# API crate
cd ../schema-registry-api
cat > Cargo.toml << 'EOF'
[package]
name = "schema-registry-api"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
schema-registry-core = { workspace = true }
schema-registry-storage = { workspace = true }
schema-registry-validation = { workspace = true }
schema-registry-compatibility = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
hyper = { workspace = true }
tonic = { workspace = true }
prost = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
EOF

cat > src/lib.rs << 'EOF'
//! API layer: REST (Axum) and gRPC (Tonic)
pub mod rest;
pub mod grpc;

pub struct ApiServer {}
impl ApiServer {
    pub fn new() -> Self { Self {} }
}
impl Default for ApiServer {
    fn default() -> Self { Self::new() }
}
EOF

echo "pub struct RestApi;" > src/rest.rs
echo "pub struct GrpcApi;" > src/grpc.rs

# CLI crate
cd ../schema-registry-cli
cat > Cargo.toml << 'EOF'
[package]
name = "schema-registry-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true

[[bin]]
name = "schema-cli"
path = "src/main.rs"

[dependencies]
schema-registry-core = { workspace = true }
tokio = { workspace = true }
clap = { workspace = true }
comfy-table = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
EOF

cat > src/main.rs << 'EOF'
use clap::Parser;

#[derive(Parser)]
#[command(name = "schema-cli")]
#[command(about = "Schema Registry CLI", long_about = None)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
    println!("Schema Registry CLI");
}
EOF

# Server crate
cd ../schema-registry-server
cat > Cargo.toml << 'EOF'
[package]
name = "schema-registry-server"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true

[[bin]]
name = "schema-registry-server"
path = "src/main.rs"

[dependencies]
schema-registry-core = { workspace = true }
schema-registry-api = { workspace = true }
schema-registry-storage = { workspace = true }
schema-registry-validation = { workspace = true }
schema-registry-compatibility = { workspace = true }
schema-registry-security = { workspace = true }
schema-registry-observability = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
config = { workspace = true }
anyhow = { workspace = true }
EOF

cat > src/main.rs << 'EOF'
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Schema Registry Server");
    Ok(())
}
EOF

echo "All crates created successfully!"
