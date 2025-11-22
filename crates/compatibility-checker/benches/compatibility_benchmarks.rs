//! Performance benchmarks for compatibility checker
//!
//! Target: p95 latency < 25ms

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use llm_schema_registry_compatibility::*;
use chrono::Utc;
use uuid::Uuid;

fn create_test_schema(
    name: &str,
    version: &str,
    format: SchemaFormat,
    content: &str,
) -> types::Schema {
    types::Schema {
        id: Uuid::new_v4(),
        name: name.to_string(),
        namespace: "test".to_string(),
        version: types::SemanticVersion::parse(version).unwrap(),
        format,
        content: content.to_string(),
        content_hash: types::Schema::calculate_hash(content),
        description: "Test schema".to_string(),
        compatibility_mode: types::CompatibilityMode::Backward,
        created_at: Utc::now(),
        metadata: Default::default(),
    }
}

fn benchmark_backward_compatibility(c: &mut Criterion) {
    let mut group = c.benchmark_group("backward_compatibility");

    // Simple JSON Schema
    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            }
        }"#,
    );

    let new_schema = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"},
                "age": {"type": "integer"}
            }
        }"#,
    );

    group.bench_function("json_schema_simple", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let checker = CompatibilityChecker::new(CompatibilityCheckerConfig {
            enable_cache: false,
            ..Default::default()
        });

        b.to_async(&rt).iter(|| async {
            black_box(
                checker
                    .check_compatibility(
                        &new_schema,
                        &old_schema,
                        types::CompatibilityMode::Backward,
                    )
                    .await
                    .unwrap(),
            )
        });
    });

    // Complex JSON Schema with many fields
    let complex_old = create_complex_schema("1.0.0", 50);
    let complex_new = create_complex_schema("1.1.0", 55);

    group.bench_function("json_schema_complex", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let checker = CompatibilityChecker::new(CompatibilityCheckerConfig {
            enable_cache: false,
            ..Default::default()
        });

        b.to_async(&rt).iter(|| async {
            black_box(
                checker
                    .check_compatibility(
                        &complex_new,
                        &complex_old,
                        types::CompatibilityMode::Backward,
                    )
                    .await
                    .unwrap(),
            )
        });
    });

    group.finish();
}

fn benchmark_with_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("caching");

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }"#,
    );

    let new_schema = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::JsonSchema,
        r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            }
        }"#,
    );

    group.bench_function("cache_miss", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();

        b.to_async(&rt).iter(|| async {
            // Create new checker each time to ensure cache miss
            let checker = CompatibilityChecker::new(CompatibilityCheckerConfig {
                enable_cache: true,
                ..Default::default()
            });

            black_box(
                checker
                    .check_compatibility(
                        &new_schema,
                        &old_schema,
                        types::CompatibilityMode::Backward,
                    )
                    .await
                    .unwrap(),
            )
        });
    });

    group.bench_function("cache_hit", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let checker = CompatibilityChecker::new(CompatibilityCheckerConfig {
            enable_cache: true,
            ..Default::default()
        });

        // Warm up cache
        rt.block_on(async {
            checker
                .check_compatibility(
                    &new_schema,
                    &old_schema,
                    types::CompatibilityMode::Backward,
                )
                .await
                .unwrap();
        });

        b.to_async(&rt).iter(|| async {
            black_box(
                checker
                    .check_compatibility(
                        &new_schema,
                        &old_schema,
                        types::CompatibilityMode::Backward,
                    )
                    .await
                    .unwrap(),
            )
        });
    });

    group.finish();
}

fn benchmark_format_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_comparison");

    // JSON Schema
    let json_old = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#,
    );

    let json_new = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::JsonSchema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}, "email": {"type": "string"}}}"#,
    );

    group.bench_function(BenchmarkId::new("format", "json_schema"), |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

        b.to_async(&rt).iter(|| async {
            black_box(
                checker
                    .check_compatibility(&json_new, &json_old, types::CompatibilityMode::Backward)
                    .await
                    .unwrap(),
            )
        });
    });

    // Avro
    let avro_old = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::Avro,
        r#"{"type": "record", "name": "User", "fields": [{"name": "name", "type": "string"}]}"#,
    );

    let avro_new = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::Avro,
        r#"{"type": "record", "name": "User", "fields": [{"name": "name", "type": "string"}, {"name": "email", "type": "string", "default": ""}]}"#,
    );

    group.bench_function(BenchmarkId::new("format", "avro"), |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

        b.to_async(&rt).iter(|| async {
            black_box(
                checker
                    .check_compatibility(&avro_new, &avro_old, types::CompatibilityMode::Backward)
                    .await
                    .unwrap(),
            )
        });
    });

    // Protobuf
    let proto_old = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::Protobuf,
        "message User { optional string name = 1; }",
    );

    let proto_new = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::Protobuf,
        "message User { optional string name = 1; optional string email = 2; }",
    );

    group.bench_function(BenchmarkId::new("format", "protobuf"), |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

        b.to_async(&rt).iter(|| async {
            black_box(
                checker
                    .check_compatibility(&proto_new, &proto_old, types::CompatibilityMode::Backward)
                    .await
                    .unwrap(),
            )
        });
    });

    group.finish();
}

fn benchmark_compatibility_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("compatibility_modes");

    let old_schema = create_test_schema(
        "User",
        "1.0.0",
        SchemaFormat::JsonSchema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#,
    );

    let new_schema = create_test_schema(
        "User",
        "1.1.0",
        SchemaFormat::JsonSchema,
        r#"{"type": "object", "properties": {"name": {"type": "string"}, "email": {"type": "string", "default": ""}}}"#,
    );

    for mode in &[
        types::CompatibilityMode::Backward,
        types::CompatibilityMode::Forward,
        types::CompatibilityMode::Full,
        types::CompatibilityMode::None,
    ] {
        group.bench_with_input(BenchmarkId::new("mode", format!("{:?}", mode)), mode, |b, mode| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let checker = CompatibilityChecker::new(CompatibilityCheckerConfig::default());

            b.to_async(&rt).iter(|| async {
                black_box(
                    checker
                        .check_compatibility(&new_schema, &old_schema, *mode)
                        .await
                        .unwrap(),
                )
            });
        });
    }

    group.finish();
}

fn create_complex_schema(version: &str, num_fields: usize) -> types::Schema {
    let mut properties = String::from("{");

    for i in 0..num_fields {
        if i > 0 {
            properties.push_str(", ");
        }
        properties.push_str(&format!(
            r#""field{}": {{"type": "string", "minLength": {}, "maxLength": {}}}"#,
            i,
            i,
            i + 100
        ));
    }

    properties.push('}');

    let content = format!(
        r#"{{"type": "object", "properties": {}}}"#,
        properties
    );

    create_test_schema("ComplexSchema", version, SchemaFormat::JsonSchema, &content)
}

criterion_group!(
    benches,
    benchmark_backward_compatibility,
    benchmark_with_cache,
    benchmark_format_comparison,
    benchmark_compatibility_modes
);
criterion_main!(benches);
