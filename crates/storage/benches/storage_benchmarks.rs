//! Storage layer performance benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use schema_registry_core::{Schema, SchemaContent, SchemaMetadata, SchemaVersion};
use schema_registry_storage::{CacheConfig, CacheManager};
use std::sync::Arc;

fn create_test_schema(subject: &str, version: (u32, u32, u32)) -> Schema {
    Schema::new(
        subject.to_string(),
        SchemaVersion::new(version.0, version.1, version.2),
        SchemaContent::Json(serde_json::json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "name": {"type": "string"},
                "email": {"type": "string", "format": "email"},
                "age": {"type": "integer", "minimum": 0},
                "created_at": {"type": "string", "format": "date-time"}
            },
            "required": ["id", "name", "email"]
        })),
        SchemaMetadata::default(),
    )
}

fn bench_cache_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_operations");

    // Test different cache sizes
    for size in [100, 1_000, 10_000] {
        let config = CacheConfig {
            l1_max_entries: size,
            l1_ttl_seconds: 3600,
            ..Default::default()
        };

        group.throughput(Throughput::Elements(1));

        // Benchmark cache writes
        group.bench_with_input(
            BenchmarkId::new("cache_put", size),
            &size,
            |b, _| {
                let cache = CacheManager::new(config.clone());
                let schema = Arc::new(create_test_schema("test.subject", (1, 0, 0)));
                let id = schema.id;

                b.to_async(&rt).iter(|| async {
                    cache.put(id, schema.clone()).await;
                });
            },
        );

        // Benchmark cache reads (hot)
        group.bench_with_input(
            BenchmarkId::new("cache_get_hot", size),
            &size,
            |b, _| {
                let cache = CacheManager::new(config.clone());
                let schema = Arc::new(create_test_schema("test.subject", (1, 0, 0)));
                let id = schema.id;

                rt.block_on(async {
                    cache.put(id, schema.clone()).await;
                });

                b.to_async(&rt).iter(|| async {
                    black_box(cache.get(id).await);
                });
            },
        );

        // Benchmark cache reads (cold)
        group.bench_with_input(
            BenchmarkId::new("cache_get_cold", size),
            &size,
            |b, _| {
                let cache = CacheManager::new(config.clone());
                let schema = create_test_schema("test.subject", (1, 0, 0));
                let id = schema.id;

                b.to_async(&rt).iter(|| async {
                    black_box(cache.get(id).await);
                });
            },
        );
    }

    group.finish();
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    let schemas = vec![
        create_test_schema("small", (1, 0, 0)),
        create_test_schema("medium", (1, 0, 0)),
        create_test_schema("large", (1, 0, 0)),
    ];

    for (i, schema) in schemas.iter().enumerate() {
        let size = match i {
            0 => "small",
            1 => "medium",
            2 => "large",
            _ => "unknown",
        };

        // JSON serialization
        group.bench_with_input(
            BenchmarkId::new("json_serialize", size),
            schema,
            |b, s| {
                b.iter(|| {
                    black_box(serde_json::to_string(s).unwrap());
                });
            },
        );

        // Bincode serialization
        group.bench_with_input(
            BenchmarkId::new("bincode_serialize", size),
            schema,
            |b, s| {
                b.iter(|| {
                    black_box(bincode::serialize(s).unwrap());
                });
            },
        );

        // JSON deserialization
        let json_data = serde_json::to_string(schema).unwrap();
        group.bench_with_input(
            BenchmarkId::new("json_deserialize", size),
            &json_data,
            |b, s| {
                b.iter(|| {
                    black_box(serde_json::from_str::<Schema>(s).unwrap());
                });
            },
        );

        // Bincode deserialization
        let bincode_data = bincode::serialize(schema).unwrap();
        group.bench_with_input(
            BenchmarkId::new("bincode_deserialize", size),
            &bincode_data,
            |b, s| {
                b.iter(|| {
                    black_box(bincode::deserialize::<Schema>(s).unwrap());
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_access(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_access");

    let config = CacheConfig::default();
    let cache = Arc::new(CacheManager::new(config));

    // Pre-populate cache
    rt.block_on(async {
        for i in 0..100 {
            let schema = Arc::new(create_test_schema(
                &format!("test.subject.{}", i),
                (1, 0, 0),
            ));
            cache.put(schema.id, schema.clone()).await;
        }
    });

    for concurrency in [1, 10, 50, 100] {
        group.throughput(Throughput::Elements(concurrency));

        group.bench_with_input(
            BenchmarkId::new("concurrent_reads", concurrency),
            &concurrency,
            |b, &n| {
                b.to_async(&rt).iter(|| {
                    let cache = cache.clone();
                    async move {
                        let mut handles = vec![];
                        for i in 0..n {
                            let cache = cache.clone();
                            let handle = tokio::spawn(async move {
                                let schema = create_test_schema(
                                    &format!("test.subject.{}", i % 100),
                                    (1, 0, 0),
                                );
                                cache.get(schema.id).await
                            });
                            handles.push(handle);
                        }
                        for handle in handles {
                            black_box(handle.await.unwrap());
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_cache_hit_rates(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_hit_rates");

    let config = CacheConfig {
        l1_max_entries: 1000,
        enable_statistics: true,
        ..Default::default()
    };
    let cache = CacheManager::new(config);

    // Simulate realistic access patterns
    group.bench_function("realistic_access_pattern", |b| {
        b.to_async(&rt).iter(|| async {
            // 80% of requests go to 20% of schemas (Pareto principle)
            for i in 0..100 {
                let schema = Arc::new(create_test_schema(
                    &format!("hot.schema.{}", i % 20), // Hot schemas
                    (1, 0, 0),
                ));
                
                if let Some(_) = cache.get(schema.id).await {
                    // Cache hit
                } else {
                    // Cache miss - populate
                    cache.put(schema.id, schema.clone()).await;
                }
            }

            // Remaining 20% requests to 80% of schemas
            for i in 0..25 {
                let schema = Arc::new(create_test_schema(
                    &format!("cold.schema.{}", i + 20), // Cold schemas
                    (1, 0, 0),
                ));
                
                if let Some(_) = cache.get(schema.id).await {
                    // Cache hit
                } else {
                    // Cache miss - populate
                    cache.put(schema.id, schema.clone()).await;
                }
            }

            black_box(cache.hit_rate());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_operations,
    bench_serialization,
    bench_concurrent_access,
    bench_cache_hit_rates
);
criterion_main!(benches);
