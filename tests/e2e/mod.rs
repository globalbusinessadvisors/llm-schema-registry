//! End-to-end workflow tests

mod schema_lifecycle_tests;
mod validation_workflow_tests;
mod compatibility_workflow_tests;
mod multi_version_tests;
mod error_handling_tests;

#[ctor::ctor]
fn init() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();
}
