//! Property-based tests using proptest

mod schema_properties;
mod compatibility_properties;
mod validation_properties;

#[ctor::ctor]
fn init() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();
}
