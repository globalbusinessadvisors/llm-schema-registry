use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use tokio::runtime::Runtime;

// Benchmark Redis-like cache operations
fn bench_redis_serialization(c: &mut Criterion) {
    use serde_json::json;

    let mut group = c.benchmark_group("redis_serialization");

    let small_schema = json!({
        "type": "object",
        "properties": {
            "id": {"type": "string"},
            "name": {"type": "string"}
        }
    });

    let large_schema = json!({
        "type": "object",
        "properties": (0..100).map(|i| {
            (format!("field_{}", i), json!({"type": "string", "description": format!("Field {}", i)}))
        }).collect::<serde_json::Map<String, serde_json::Value>>()
    });

    // Small schema serialization
    group.throughput(Throughput::Bytes(small_schema.to_string().len() as u64));
    group.bench_function("small_schema_serialize", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(black_box(&small_schema)).unwrap();
            black_box(serialized);
        });
    });

    group.bench_function("small_schema_deserialize", |b| {
        let serialized = serde_json::to_vec(&small_schema).unwrap();
        b.iter(|| {
            let deserialized: serde_json::Value =
                serde_json::from_slice(black_box(&serialized)).unwrap();
            black_box(deserialized);
        });
    });

    // Large schema serialization
    group.throughput(Throughput::Bytes(large_schema.to_string().len() as u64));
    group.bench_function("large_schema_serialize", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(black_box(&large_schema)).unwrap();
            black_box(serialized);
        });
    });

    group.bench_function("large_schema_deserialize", |b| {
        let serialized = serde_json::to_vec(&large_schema).unwrap();
        b.iter(|| {
            let deserialized: serde_json::Value =
                serde_json::from_slice(black_box(&serialized)).unwrap();
            black_box(deserialized);
        });
    });

    group.finish();
}

// Benchmark in-memory cache (Moka) operations
fn bench_in_memory_cache(c: &mut Criterion) {
    use moka::future::Cache;

    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("in_memory_cache");

    // Create cache with 10,000 capacity
    let cache: Cache<String, String> = Cache::builder()
        .max_capacity(10_000)
        .build();

    // Pre-populate cache
    rt.block_on(async {
        for i in 0..1000 {
            cache
                .insert(format!("key_{}", i), format!("value_{}", i))
                .await;
        }
    });

    // Benchmark: Cache get (hit)
    group.bench_function("get_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let result = cache.get(&"key_500".to_string()).await;
            black_box(result);
        });
    });

    // Benchmark: Cache get (miss)
    group.bench_function("get_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let result = cache.get(&"nonexistent".to_string()).await;
            black_box(result);
        });
    });

    // Benchmark: Cache insert
    group.bench_function("insert", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            cache
                .insert(
                    format!("new_key_{}", counter),
                    format!("new_value_{}", counter),
                )
                .await;
        });
    });

    // Benchmark: Cache invalidate
    group.bench_function("invalidate", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            cache.invalidate(&format!("key_{}", counter % 1000)).await;
        });
    });

    group.finish();
}

// Benchmark cache key generation
fn bench_cache_key_generation(c: &mut Criterion) {
    use sha2::{Digest, Sha256};

    let mut group = c.benchmark_group("cache_key_generation");

    let namespace = "com.example.schemas";
    let name = "user.profile";
    let version = "1.2.3";

    // Simple concatenation
    group.bench_function("simple_concat", |b| {
        b.iter(|| {
            let key = format!("{}:{}:{}", namespace, name, version);
            black_box(key);
        });
    });

    // Hash-based key
    group.bench_function("hash_based", |b| {
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(namespace.as_bytes());
            hasher.update(name.as_bytes());
            hasher.update(version.as_bytes());
            let hash = hasher.finalize();
            let key = format!("schema:{}", hex::encode(hash));
            black_box(key);
        });
    });

    group.finish();
}

// Benchmark cache stampede prevention (singleflight)
fn bench_singleflight(c: &mut Criterion) {
    use tokio::sync::Mutex;
    use std::collections::HashMap;

    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("singleflight");

    // Simulate expensive operation
    async fn expensive_operation(key: &str) -> String {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        format!("result_{}", key)
    }

    // Without singleflight
    group.bench_function("without_singleflight", |b| {
        b.to_async(&rt).iter(|| async {
            let tasks: Vec<_> = (0..10)
                .map(|_| tokio::spawn(expensive_operation("key1".to_string())))
                .collect();

            for task in tasks {
                black_box(task.await.unwrap());
            }
        });
    });

    // With singleflight simulation
    group.bench_function("with_singleflight", |b| {
        b.to_async(&rt).iter(|| async {
            let inflight: Arc<Mutex<HashMap<String, Arc<tokio::sync::Notify>>>> =
                Arc::new(Mutex::new(HashMap::new()));

            let tasks: Vec<_> = (0..10)
                .map(|_| {
                    let inflight = Arc::clone(&inflight);
                    tokio::spawn(async move {
                        let key = "key1".to_string();
                        let notify = {
                            let mut map = inflight.lock().await;
                            if let Some(notify) = map.get(&key) {
                                Some(Arc::clone(notify))
                            } else {
                                let notify = Arc::new(tokio::sync::Notify::new());
                                map.insert(key.clone(), Arc::clone(&notify));
                                None
                            }
                        };

                        if let Some(notify) = notify {
                            notify.notified().await;
                        } else {
                            let result = expensive_operation(&key).await;
                            let mut map = inflight.lock().await;
                            if let Some(notify) = map.remove(&key) {
                                notify.notify_waiters();
                            }
                            black_box(result);
                        }
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

// Benchmark cache eviction strategies
fn bench_cache_eviction(c: &mut Criterion) {
    use std::collections::VecDeque;

    let mut group = c.benchmark_group("cache_eviction");

    // LRU simulation
    let mut lru_cache: VecDeque<(String, String)> = VecDeque::new();
    for i in 0..1000 {
        lru_cache.push_back((format!("key_{}", i), format!("value_{}", i)));
    }

    group.bench_function("lru_access", |b| {
        b.iter(|| {
            // Find and move to back (LRU)
            if let Some(pos) = lru_cache.iter().position(|(k, _)| k == "key_500") {
                if let Some(item) = lru_cache.remove(pos) {
                    lru_cache.push_back(item);
                }
            }
        });
    });

    group.bench_function("lru_evict", |b| {
        b.iter(|| {
            lru_cache.pop_front();
            lru_cache.push_back(("new_key".to_string(), "new_value".to_string()));
        });
    });

    group.finish();
}

// Benchmark multi-tier cache lookup
fn bench_multi_tier_cache(c: &mut Criterion) {
    use std::collections::HashMap;
    use parking_lot::RwLock;

    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("multi_tier_cache");

    // L1: In-process cache
    let l1_cache = Arc::new(RwLock::new(HashMap::<String, String>::new()));
    for i in 0..100 {
        l1_cache
            .write()
            .insert(format!("key_{}", i), format!("value_{}", i));
    }

    // L2: Shared cache (simulated)
    let l2_cache = Arc::new(RwLock::new(HashMap::<String, String>::new()));
    for i in 0..1000 {
        l2_cache
            .write()
            .insert(format!("key_{}", i), format!("value_{}", i));
    }

    // L1 hit
    group.bench_function("l1_hit", |b| {
        b.iter(|| {
            let result = l1_cache.read().get("key_50").cloned();
            black_box(result);
        });
    });

    // L1 miss, L2 hit
    group.bench_function("l1_miss_l2_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let l1 = Arc::clone(&l1_cache);
            let l2 = Arc::clone(&l2_cache);

            let key = "key_500";
            let result = l1.read().get(key).cloned();

            if result.is_none() {
                if let Some(value) = l2.read().get(key).cloned() {
                    l1.write().insert(key.to_string(), value.clone());
                    black_box(Some(value));
                }
            }
        });
    });

    // Both miss (requires DB lookup simulation)
    group.bench_function("l1_l2_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let l1 = Arc::clone(&l1_cache);
            let l2 = Arc::clone(&l2_cache);

            let key = "nonexistent";
            let result = l1.read().get(key).cloned();

            if result.is_none() {
                if let Some(value) = l2.read().get(key).cloned() {
                    black_box(Some(value));
                } else {
                    // Simulate DB lookup
                    tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
                    black_box(None::<String>);
                }
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_redis_serialization,
    bench_in_memory_cache,
    bench_cache_key_generation,
    bench_singleflight,
    bench_cache_eviction,
    bench_multi_tier_cache,
);

criterion_main!(benches);
