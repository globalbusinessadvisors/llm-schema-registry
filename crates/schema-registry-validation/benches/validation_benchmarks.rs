//! Performance benchmarks for the validation engine
//!
//! Target: <50ms p95 validation latency

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use schema_registry_validation::{ValidationEngine, SchemaFormat};

// Sample schemas for benchmarking
const JSON_SCHEMA_SIMPLE: &str = r#"{
    "type": "object",
    "description": "Simple user schema",
    "properties": {
        "id": {"type": "integer", "description": "User ID"},
        "name": {"type": "string", "description": "User name"},
        "email": {"type": "string", "format": "email", "description": "Email"}
    },
    "required": ["id", "name"]
}"#;

const JSON_SCHEMA_COMPLEX: &str = r#"{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "type": "object",
    "description": "Complex nested schema",
    "properties": {
        "id": {"type": "integer"},
        "profile": {
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0, "maximum": 150},
                "address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"},
                        "country": {"type": "string"}
                    }
                }
            }
        },
        "tags": {
            "type": "array",
            "items": {"type": "string"}
        },
        "metadata": {
            "type": "object",
            "additionalProperties": {"type": "string"}
        }
    },
    "required": ["id", "profile"]
}"#;

const AVRO_SCHEMA: &str = r#"{
    "type": "record",
    "name": "User",
    "namespace": "com.example",
    "doc": "User record",
    "fields": [
        {"name": "id", "type": "long", "doc": "User ID"},
        {"name": "username", "type": "string", "doc": "Username"},
        {"name": "email", "type": "string", "doc": "Email"},
        {"name": "created_at", "type": "long", "doc": "Timestamp"}
    ]
}"#;

const PROTOBUF_SCHEMA: &str = r#"
syntax = "proto3";

package example;

message User {
  int64 id = 1;
  string username = 2;
  string email = 3;
  int64 created_at = 4;
}

message Post {
  int64 id = 1;
  int64 user_id = 2;
  string title = 3;
  string content = 4;
  repeated string tags = 5;
}
"#;

fn bench_json_schema_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("json-schema");

    group.bench_function("simple", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = ValidationEngine::new();

        b.to_async(&runtime).iter(|| async {
            let result = engine
                .validate(black_box(JSON_SCHEMA_SIMPLE), SchemaFormat::JsonSchema)
                .await
                .unwrap();
            black_box(result);
        });
    });

    group.bench_function("complex", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = ValidationEngine::new();

        b.to_async(&runtime).iter(|| async {
            let result = engine
                .validate(black_box(JSON_SCHEMA_COMPLEX), SchemaFormat::JsonSchema)
                .await
                .unwrap();
            black_box(result);
        });
    });

    group.finish();
}

fn bench_avro_validation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let engine = ValidationEngine::new();

    c.bench_function("avro-validation", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine
                .validate(black_box(AVRO_SCHEMA), SchemaFormat::Avro)
                .await
                .unwrap();
            black_box(result);
        });
    });
}

fn bench_protobuf_validation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let engine = ValidationEngine::new();

    c.bench_function("protobuf-validation", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine
                .validate(black_box(PROTOBUF_SCHEMA), SchemaFormat::Protobuf)
                .await
                .unwrap();
            black_box(result);
        });
    });
}

fn bench_format_detection(c: &mut Criterion) {
    use schema_registry_validation::detect_format;

    let mut group = c.benchmark_group("format-detection");

    group.bench_function("json-schema", |b| {
        b.iter(|| {
            let format = detect_format(black_box(JSON_SCHEMA_SIMPLE)).unwrap();
            black_box(format);
        });
    });

    group.bench_function("avro", |b| {
        b.iter(|| {
            let format = detect_format(black_box(AVRO_SCHEMA)).unwrap();
            black_box(format);
        });
    });

    group.bench_function("protobuf", |b| {
        b.iter(|| {
            let format = detect_format(black_box(PROTOBUF_SCHEMA)).unwrap();
            black_box(format);
        });
    });

    group.finish();
}

fn bench_validation_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let engine = ValidationEngine::new();

            // Generate schema with N properties
            let mut properties = Vec::new();
            for i in 0..size {
                properties.push(format!(
                    r#""field_{}": {{"type": "string", "description": "Field {}"}}"#,
                    i, i
                ));
            }

            let schema = format!(
                r#"{{
                    "type": "object",
                    "description": "Benchmark schema",
                    "properties": {{
                        {}
                    }}
                }}"#,
                properties.join(",\n")
            );

            b.to_async(&runtime).iter(|| async {
                let result = engine
                    .validate(black_box(&schema), SchemaFormat::JsonSchema)
                    .await
                    .unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_json_schema_validation,
    bench_avro_validation,
    bench_protobuf_validation,
    bench_format_detection,
    bench_validation_throughput
);
criterion_main!(benches);
