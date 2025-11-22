use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use schema_registry_core::models::{SchemaFormat, SchemaState};
use std::sync::Arc;
use tokio::runtime::Runtime;

// Mock data for benchmarking
fn generate_json_schema(size: usize) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    for i in 0..size {
        properties.insert(
            format!("field_{}", i),
            serde_json::json!({
                "type": "string",
                "description": format!("Field number {}", i)
            }),
        );
    }

    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": properties,
        "required": (0..size.min(10)).map(|i| format!("field_{}", i)).collect::<Vec<_>>()
    })
}

fn generate_avro_schema(size: usize) -> String {
    let mut fields = Vec::new();
    for i in 0..size {
        fields.push(serde_json::json!({
            "name": format!("field_{}", i),
            "type": "string",
            "doc": format!("Field number {}", i)
        }));
    }

    serde_json::json!({
        "type": "record",
        "name": "TestRecord",
        "namespace": "com.example",
        "fields": fields
    })
    .to_string()
}

// Benchmark: Schema validation
fn bench_schema_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_validation");

    for size in [10, 50, 100, 500].iter() {
        let schema = generate_json_schema(*size);
        let data = serde_json::json!({
            "field_0": "test",
            "field_1": "test",
            "field_2": "test",
        });

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("json_schema", size),
            size,
            |b, _| {
                b.iter(|| {
                    // Simulate JSON Schema validation
                    black_box(&schema);
                    black_box(&data);
                });
            },
        );
    }

    group.finish();
}

// Benchmark: Schema serialization/deserialization
fn bench_schema_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_serialization");

    for size in [10, 50, 100, 500].iter() {
        let schema = generate_json_schema(*size);

        group.throughput(Throughput::Bytes(schema.to_string().len() as u64));

        // JSON serialization
        group.bench_with_input(
            BenchmarkId::new("json_to_string", size),
            size,
            |b, _| {
                b.iter(|| {
                    let serialized = serde_json::to_string(black_box(&schema)).unwrap();
                    black_box(serialized);
                });
            },
        );

        // JSON deserialization
        let schema_str = serde_json::to_string(&schema).unwrap();
        group.bench_with_input(
            BenchmarkId::new("json_from_string", size),
            size,
            |b, _| {
                b.iter(|| {
                    let deserialized: serde_json::Value =
                        serde_json::from_str(black_box(&schema_str)).unwrap();
                    black_box(deserialized);
                });
            },
        );

        // Bincode serialization
        group.bench_with_input(
            BenchmarkId::new("bincode_serialize", size),
            size,
            |b, _| {
                b.iter(|| {
                    let serialized = bincode::serialize(black_box(&schema)).unwrap();
                    black_box(serialized);
                });
            },
        );
    }

    group.finish();
}

// Benchmark: Hash computation
fn bench_hash_computation(c: &mut Criterion) {
    use sha2::{Digest, Sha256};

    let mut group = c.benchmark_group("hash_computation");

    for size in [100, 1000, 10000, 100000].iter() {
        let data = vec![0u8; *size];

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("sha256", size), size, |b, _| {
            b.iter(|| {
                let mut hasher = Sha256::new();
                hasher.update(black_box(&data));
                let result = hasher.finalize();
                black_box(result);
            });
        });
    }

    group.finish();
}

// Benchmark: Compatibility checking (simplified)
fn bench_compatibility_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("compatibility_check");

    for size in [10, 50, 100].iter() {
        let old_schema = generate_json_schema(*size);
        let mut new_schema = old_schema.clone();

        // Add a new optional field
        if let Some(props) = new_schema.get_mut("properties").and_then(|p| p.as_object_mut()) {
            props.insert(
                format!("new_field_{}", size),
                serde_json::json!({
                    "type": "string",
                    "description": "New optional field"
                }),
            );
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("backward_compatible", size),
            size,
            |b, _| {
                b.iter(|| {
                    // Simulate compatibility check
                    black_box(&old_schema);
                    black_box(&new_schema);
                });
            },
        );
    }

    group.finish();
}

// Benchmark: Cache operations (in-memory simulation)
fn bench_cache_operations(c: &mut Criterion) {
    use std::collections::HashMap;
    use parking_lot::RwLock;

    let mut group = c.benchmark_group("cache_operations");

    // Prepare cache with 1000 entries
    let cache = Arc::new(RwLock::new(HashMap::new()));
    for i in 0..1000 {
        let schema = generate_json_schema(10);
        cache.write().insert(format!("schema_{}", i), schema);
    }

    // Benchmark: Cache read (hit)
    group.bench_function("cache_read_hit", |b| {
        b.iter(|| {
            let result = cache.read().get("schema_500").cloned();
            black_box(result);
        });
    });

    // Benchmark: Cache read (miss)
    group.bench_function("cache_read_miss", |b| {
        b.iter(|| {
            let result = cache.read().get("nonexistent").cloned();
            black_box(result);
        });
    });

    // Benchmark: Cache write
    group.bench_function("cache_write", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let schema = generate_json_schema(10);
            cache.write().insert(format!("new_schema_{}", counter), schema);
        });
    });

    group.finish();
}

// Benchmark: Concurrent schema access
fn bench_concurrent_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_access");

    group.bench_function("single_thread_sequential", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let schema = generate_json_schema(10);
                black_box(schema);
            }
        });
    });

    group.bench_function("multi_thread_parallel", |b| {
        b.to_async(&rt).iter(|| async {
            let tasks: Vec<_> = (0..100)
                .map(|_| {
                    tokio::spawn(async {
                        let schema = generate_json_schema(10);
                        black_box(schema);
                    })
                })
                .collect();

            for task in tasks {
                task.await.unwrap();
            }
        });
    });

    group.finish();
}

// Benchmark: Avro schema parsing
fn bench_avro_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("avro_parsing");

    for size in [10, 50, 100].iter() {
        let schema_str = generate_avro_schema(*size);

        group.throughput(Throughput::Bytes(schema_str.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_avro_schema", size),
            size,
            |b, _| {
                b.iter(|| {
                    let parsed = apache_avro::Schema::parse_str(black_box(&schema_str));
                    black_box(parsed);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_schema_validation,
    bench_schema_serialization,
    bench_hash_computation,
    bench_compatibility_check,
    bench_cache_operations,
    bench_concurrent_access,
    bench_avro_parsing,
);

criterion_main!(benches);
