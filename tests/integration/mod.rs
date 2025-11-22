//! Integration tests for LLM Schema Registry
//!
//! This module contains comprehensive integration tests using real services
//! via testcontainers (PostgreSQL, Redis, LocalStack/S3)

mod test_environment;
mod database_tests;
mod redis_tests;
mod s3_tests;
mod multi_tier_storage_tests;
mod api_integration_tests;

pub use test_environment::TestEnvironment;

// Test setup
#[ctor::ctor]
fn init() {
    // Initialize logging for tests
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();
}
