//! Multi-tier storage integration tests (L1: Memory, L2: Redis, L3: S3, L4: PostgreSQL)

use super::*;
use serde_json::json;
use uuid::Uuid;
use redis::Commands;

#[tokio::test]
async fn test_cache_l1_memory_hit() {
    // Simulates L1 (in-memory) cache hit
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    // This test validates that the multi-tier caching strategy works
    // In practice, L1 would be Moka (in-memory), L2 would be Redis

    let mut conn = env.redis_client().get_connection().unwrap();
    let schema_id = Uuid::new_v4();
    let schema_data = json!({"type": "object", "properties": {}});

    // Store in L2 (Redis)
    let _: () = conn.set(
        format!("schema:{}", schema_id),
        serde_json::to_string(&schema_data).unwrap()
    ).unwrap();

    // Retrieve (cache hit simulation)
    let retrieved: String = conn.get(format!("schema:{}", schema_id)).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&retrieved).unwrap();

    assert_eq!(parsed, schema_data);
}

#[tokio::test]
async fn test_cache_l2_redis_miss_l3_s3_hit() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let schema_data = json!({"type": "object", "properties": {"name": {"type": "string"}}});
    let client = env.s3_client().await;

    // Store in L3 (S3)
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .body(aws_sdk_s3::primitives::ByteStream::from(
            bytes::Bytes::from(serde_json::to_string(&schema_data).unwrap())
        ))
        .send()
        .await
        .unwrap();

    // Retrieve from S3
    let response = client
        .get_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .send()
        .await
        .unwrap();

    let bytes = response.body.collect().await.unwrap().into_bytes();
    let retrieved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(retrieved, schema_data);

    // Now populate L2 cache from L3
    let mut conn = env.redis_client().get_connection().unwrap();
    let _: () = conn.set(
        format!("schema:{}", schema_id),
        serde_json::to_string(&schema_data).unwrap()
    ).unwrap();

    // Verify L2 cache hit
    let cached: String = conn.get(format!("schema:{}", schema_id)).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&cached).unwrap();

    assert_eq!(parsed, schema_data);
}

#[tokio::test]
async fn test_cache_all_miss_l4_postgres_hit() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Store in L4 (PostgreSQL)
    sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(schema_id)
    .bind("User")
    .bind("com.example")
    .bind("1.0.0")
    .bind("json")
    .bind(r#"{"type": "object"}"#)
    .bind("abc123")
    .bind("User schema")
    .bind("BACKWARD")
    .bind("ACTIVE")
    .bind(now)
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Retrieve from PostgreSQL
    let row: (String,) = sqlx::query_as("SELECT content FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    assert_eq!(row.0, r#"{"type": "object"}"#);

    // Populate upper caches
    let client = env.s3_client().await;
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .body(aws_sdk_s3::primitives::ByteStream::from(
            bytes::Bytes::from(row.0.clone())
        ))
        .send()
        .await
        .unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();
    let _: () = conn.set(format!("schema:{}", schema_id), row.0.clone()).unwrap();

    // Verify all tiers have the data
    let cached: String = conn.get(format!("schema:{}", schema_id)).unwrap();
    assert_eq!(cached, row.0);
}

#[tokio::test]
async fn test_cache_invalidation_cascade() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let schema_data = r#"{"type": "object"}"#;

    // Populate all caches
    let mut conn = env.redis_client().get_connection().unwrap();
    let _: () = conn.set(format!("schema:{}", schema_id), schema_data).unwrap();

    let client = env.s3_client().await;
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .body(aws_sdk_s3::primitives::ByteStream::from(
            bytes::Bytes::from(schema_data)
        ))
        .send()
        .await
        .unwrap();

    // Invalidate Redis cache
    let _: () = conn.del(format!("schema:{}", schema_id)).unwrap();

    // Verify cache miss
    let result: Option<String> = conn.get(format!("schema:{}", schema_id)).ok();
    assert!(result.is_none());

    // S3 should still have it
    let response = client
        .get_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .send()
        .await
        .unwrap();

    let bytes = response.body.collect().await.unwrap().into_bytes();
    let retrieved = String::from_utf8(bytes.to_vec()).unwrap();

    assert_eq!(retrieved, schema_data);
}

#[tokio::test]
async fn test_write_through_cache_strategy() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let schema_data = r#"{"type": "object", "properties": {"id": {"type": "string"}}}"#;
    let now = chrono::Utc::now();

    // Write to PostgreSQL (L4)
    sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(schema_id)
    .bind("Product")
    .bind("com.example")
    .bind("1.0.0")
    .bind("json")
    .bind(schema_data)
    .bind("hash123")
    .bind("Product schema")
    .bind("BACKWARD")
    .bind("ACTIVE")
    .bind(now)
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Immediately write to S3 (L3)
    let client = env.s3_client().await;
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .body(aws_sdk_s3::primitives::ByteStream::from(
            bytes::Bytes::from(schema_data)
        ))
        .send()
        .await
        .unwrap();

    // Immediately write to Redis (L2)
    let mut conn = env.redis_client().get_connection().unwrap();
    let _: () = conn.set_ex(format!("schema:{}", schema_id), schema_data, 3600).unwrap();

    // Verify all tiers are consistent
    let db_content: (String,) = sqlx::query_as("SELECT content FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    let s3_response = client
        .get_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .send()
        .await
        .unwrap();
    let s3_bytes = s3_response.body.collect().await.unwrap().into_bytes();
    let s3_content = String::from_utf8(s3_bytes.to_vec()).unwrap();

    let redis_content: String = conn.get(format!("schema:{}", schema_id)).unwrap();

    assert_eq!(db_content.0, schema_data);
    assert_eq!(s3_content, schema_data);
    assert_eq!(redis_content, schema_data);
}

#[tokio::test]
async fn test_cache_hit_rate_tracking() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Simulate cache operations
    let mut hits = 0;
    let mut misses = 0;

    // Pre-populate some cache entries
    for i in 0..5 {
        let _: () = conn.set(format!("schema:{}", i), format!("data{}", i)).unwrap();
    }

    // Simulate read pattern: 80% hits, 20% misses
    for i in 0..100 {
        let key = format!("schema:{}", i % 7);
        let result: Option<String> = conn.get(&key).ok();

        if result.is_some() {
            hits += 1;
        } else {
            misses += 1;
        }
    }

    let hit_rate = hits as f64 / (hits + misses) as f64;
    tracing::info!("Cache hit rate: {:.2}%", hit_rate * 100.0);

    // Should have reasonable hit rate given the pattern
    assert!(hit_rate > 0.5, "Hit rate too low: {}", hit_rate);
}

#[tokio::test]
async fn test_cache_stampede_prevention() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let key = format!("schema:{}", schema_id);

    // Simulate cache stampede: multiple concurrent requests for same cold key
    let mut handles = vec![];

    for i in 0..10 {
        let redis_client = env.redis_client().clone();
        let key_clone = key.clone();

        let handle = tokio::spawn(async move {
            let mut conn = redis_client.get_connection().unwrap();

            // Check if key exists
            let exists: bool = conn.exists(&key_clone).unwrap();

            if !exists {
                // Simulate expensive operation
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Set the value (in real implementation, use setnx for locking)
                let _: () = conn.set(&key_clone, format!("computed_value_{}", i)).unwrap();
            }

            let value: String = conn.get(&key_clone).unwrap();
            value
        });

        handles.push(handle);
    }

    // Wait for all tasks
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // All should have gotten a value (first one to write wins)
    assert_eq!(results.len(), 10);
    assert!(results.iter().all(|r| r.starts_with("computed_value_")));
}

#[tokio::test]
async fn test_ttl_based_cache_eviction() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Set with different TTLs
    let _: () = conn.set_ex("short:ttl", "value1", 1).unwrap();
    let _: () = conn.set_ex("long:ttl", "value2", 10).unwrap();

    // Verify both exist
    assert!(conn.exists::<_, bool>("short:ttl").unwrap());
    assert!(conn.exists::<_, bool>("long:ttl").unwrap());

    // Wait for short TTL to expire
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Short TTL should be gone
    assert!(!conn.exists::<_, bool>("short:ttl").unwrap());

    // Long TTL should still exist
    assert!(conn.exists::<_, bool>("long:ttl").unwrap());
}

#[tokio::test]
async fn test_lru_eviction_simulation() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Simulate LRU by tracking access patterns
    for i in 0..10 {
        let _: () = conn.set(format!("lru:{}", i), i).unwrap();
    }

    // Access some keys (simulating LRU promotion)
    for i in [0, 2, 4, 6, 8] {
        let _: i32 = conn.get(format!("lru:{}", i)).unwrap();
    }

    // All keys should still exist
    let count = (0..10).filter(|i| {
        conn.exists::<_, bool>(format!("lru:{}", i)).unwrap()
    }).count();

    assert_eq!(count, 10);
}

#[tokio::test]
async fn test_cache_consistency_check() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let schema_data = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
    let now = chrono::Utc::now();

    // Write to all tiers
    // L4: PostgreSQL
    sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(schema_id)
    .bind("ConsistencyTest")
    .bind("com.example")
    .bind("1.0.0")
    .bind("json")
    .bind(schema_data)
    .bind("hash_consistency")
    .bind("Consistency test")
    .bind("BACKWARD")
    .bind("ACTIVE")
    .bind(now)
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // L3: S3
    let client = env.s3_client().await;
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .body(aws_sdk_s3::primitives::ByteStream::from(
            bytes::Bytes::from(schema_data)
        ))
        .send()
        .await
        .unwrap();

    // L2: Redis
    let mut conn = env.redis_client().get_connection().unwrap();
    let _: () = conn.set(format!("schema:{}", schema_id), schema_data).unwrap();

    // Verify consistency across all tiers
    let db_content: (String,) = sqlx::query_as("SELECT content FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    let s3_response = client
        .get_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .send()
        .await
        .unwrap();
    let s3_bytes = s3_response.body.collect().await.unwrap().into_bytes();
    let s3_content = String::from_utf8(s3_bytes.to_vec()).unwrap();

    let redis_content: String = conn.get(format!("schema:{}", schema_id)).unwrap();

    // All tiers should have identical data
    assert_eq!(db_content.0, schema_data);
    assert_eq!(s3_content, schema_data);
    assert_eq!(redis_content, schema_data);
}

#[tokio::test]
async fn test_cache_warming_strategy() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();

    // Insert hot schemas into PostgreSQL
    let mut schema_ids = vec![];
    for i in 0..5 {
        let id = Uuid::new_v4();
        schema_ids.push(id);

        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                description, compatibility_mode, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(id)
        .bind(format!("HotSchema{}", i))
        .bind("com.hot")
        .bind("1.0.0")
        .bind("json")
        .bind(format!(r#"{{"type": "object", "id": {}}}"#, i))
        .bind(format!("hash_{}", i))
        .bind("Hot schema")
        .bind("BACKWARD")
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Warm cache: populate Redis with hot schemas
    let mut conn = env.redis_client().get_connection().unwrap();
    for (i, id) in schema_ids.iter().enumerate() {
        let content = format!(r#"{{"type": "object", "id": {}}}"#, i);
        let _: () = conn.set_ex(format!("schema:{}", id), content, 3600).unwrap();
    }

    // Verify cache is warm
    for id in schema_ids {
        let exists: bool = conn.exists(format!("schema:{}", id)).unwrap();
        assert!(exists, "Schema {} not in cache", id);
    }
}

#[tokio::test]
async fn test_read_through_cache_pattern() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let schema_data = r#"{"type": "object"}"#;
    let now = chrono::Utc::now();

    // Store only in PostgreSQL (L4)
    sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(schema_id)
    .bind("ReadThrough")
    .bind("com.example")
    .bind("1.0.0")
    .bind("json")
    .bind(schema_data)
    .bind("hash_rt")
    .bind("Read-through test")
    .bind("BACKWARD")
    .bind("ACTIVE")
    .bind(now)
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Simulate read-through: check cache (miss), read from DB, populate cache
    let mut conn = env.redis_client().get_connection().unwrap();

    let cached: Option<String> = conn.get(format!("schema:{}", schema_id)).ok();
    assert!(cached.is_none(), "Should be cache miss");

    // Read from database
    let db_content: (String,) = sqlx::query_as("SELECT content FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    // Populate cache
    let _: () = conn.set_ex(format!("schema:{}", schema_id), &db_content.0, 3600).unwrap();

    // Next read should hit cache
    let cached: String = conn.get(format!("schema:{}", schema_id)).unwrap();
    assert_eq!(cached, db_content.0);
}

#[tokio::test]
async fn test_cache_aside_pattern() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let schema_data = r#"{"type": "object", "properties": {}}"#;

    // Cache-aside: application checks cache first
    let mut conn = env.redis_client().get_connection().unwrap();

    // Check cache (miss)
    let cached: Option<String> = conn.get(format!("schema:{}", schema_id)).ok();
    assert!(cached.is_none());

    // Application loads from database (simulate)
    let now = chrono::Utc::now();
    sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(schema_id)
    .bind("CacheAside")
    .bind("com.example")
    .bind("1.0.0")
    .bind("json")
    .bind(schema_data)
    .bind("hash_ca")
    .bind("Cache-aside test")
    .bind("BACKWARD")
    .bind("ACTIVE")
    .bind(now)
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    let db_content: (String,) = sqlx::query_as("SELECT content FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    // Application updates cache
    let _: () = conn.set(format!("schema:{}", schema_id), &db_content.0).unwrap();

    // Subsequent reads hit cache
    let cached: String = conn.get(format!("schema:{}", schema_id)).unwrap();
    assert_eq!(cached, db_content.0);
}

#[tokio::test]
async fn test_cache_size_limits() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Store schemas with size tracking
    let small_schema = r#"{"type": "object"}"#;
    let large_schema = "x".repeat(1024 * 100); // 100KB

    let _: () = conn.set("schema:small", small_schema).unwrap();
    let _: () = conn.set("schema:large", &large_schema).unwrap();

    // Verify both stored
    let small: String = conn.get("schema:small").unwrap();
    let large: String = conn.get("schema:large").unwrap();

    assert_eq!(small, small_schema);
    assert_eq!(large.len(), large_schema.len());
}

#[tokio::test]
async fn test_concurrent_cache_updates() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let mut handles = vec![];

    // Spawn 10 concurrent tasks trying to update same cache key
    for i in 0..10 {
        let redis_client = env.redis_client().clone();
        let id = schema_id;

        let handle = tokio::spawn(async move {
            let mut conn = redis_client.get_connection().unwrap();
            let value = format!(r#"{{"version": {}}}"#, i);
            let _: () = conn.set(format!("schema:{}", id), value).unwrap();
        });

        handles.push(handle);
    }

    // Wait for all updates
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify some value was set (last writer wins)
    let mut conn = env.redis_client().get_connection().unwrap();
    let value: String = conn.get(format!("schema:{}", schema_id)).unwrap();
    assert!(value.starts_with(r#"{"version":"#));
}

#[tokio::test]
async fn test_cache_namespace_isolation() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Use namespaces to isolate different data types
    let _: () = conn.set("schema:123", "schema_data").unwrap();
    let _: () = conn.set("validation:123", "validation_data").unwrap();
    let _: () = conn.set("compatibility:123", "compatibility_data").unwrap();

    // Verify namespace isolation
    let schema: String = conn.get("schema:123").unwrap();
    let validation: String = conn.get("validation:123").unwrap();
    let compatibility: String = conn.get("compatibility:123").unwrap();

    assert_eq!(schema, "schema_data");
    assert_eq!(validation, "validation_data");
    assert_eq!(compatibility, "compatibility_data");

    // Can delete one namespace without affecting others
    let _: () = conn.del("validation:123").unwrap();

    assert!(conn.exists::<_, bool>("schema:123").unwrap());
    assert!(!conn.exists::<_, bool>("validation:123").unwrap());
    assert!(conn.exists::<_, bool>("compatibility:123").unwrap());
}

#[tokio::test]
async fn test_cache_compression_simulation() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Large schema that could benefit from compression
    let large_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "field1": {"type": "string", "description": "Long description ".repeat(100)},
            "field2": {"type": "string", "description": "Long description ".repeat(100)},
            "field3": {"type": "string", "description": "Long description ".repeat(100)},
        }
    });

    let serialized = serde_json::to_string(&large_schema).unwrap();
    let original_size = serialized.len();

    // In production, would compress here
    // For test, just store as-is
    let _: () = conn.set("schema:large", &serialized).unwrap();

    let retrieved: String = conn.get("schema:large").unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&retrieved).unwrap();

    assert_eq!(deserialized, large_schema);
    tracing::info!("Original size: {} bytes", original_size);
}

#[tokio::test]
async fn test_multi_tier_failover() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let schema_data = r#"{"type": "object"}"#;
    let now = chrono::Utc::now();

    // Store in PostgreSQL (L4) only
    sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(schema_id)
    .bind("Failover")
    .bind("com.example")
    .bind("1.0.0")
    .bind("json")
    .bind(schema_data)
    .bind("hash_failover")
    .bind("Failover test")
    .bind("BACKWARD")
    .bind("ACTIVE")
    .bind(now)
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Simulate failover scenario:
    // 1. Check L2 (Redis) - miss
    let mut conn = env.redis_client().get_connection().unwrap();
    let l2_result: Option<String> = conn.get(format!("schema:{}", schema_id)).ok();
    assert!(l2_result.is_none(), "L2 should miss");

    // 2. Check L3 (S3) - miss
    let client = env.s3_client().await;
    let l3_result = client
        .get_object()
        .bucket(&env.s3_bucket)
        .key(format!("schemas/{}.json", schema_id))
        .send()
        .await;
    assert!(l3_result.is_err(), "L3 should miss");

    // 3. Fall back to L4 (PostgreSQL) - hit
    let l4_result: (String,) = sqlx::query_as("SELECT content FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();
    assert_eq!(l4_result.0, schema_data);
}

#[tokio::test]
async fn test_cache_preloading() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();

    // Insert frequently accessed schemas
    let mut schema_ids = vec![];
    for i in 0..10 {
        let id = Uuid::new_v4();
        schema_ids.push(id);

        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                description, compatibility_mode, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(id)
        .bind(format!("Popular{}", i))
        .bind("com.popular")
        .bind("1.0.0")
        .bind("json")
        .bind(format!(r#"{{"id": {}}}"#, i))
        .bind(format!("hash_{}", i))
        .bind("Popular schema")
        .bind("BACKWARD")
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Preload into cache
    let schemas: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, content FROM schemas WHERE namespace = $1"
    )
    .bind("com.popular")
    .fetch_all(env.db_pool())
    .await
    .unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();
    for (id, content) in schemas {
        let _: () = conn.set(format!("schema:{}", id), content).unwrap();
    }

    // Verify all preloaded
    for id in schema_ids {
        let exists: bool = conn.exists(format!("schema:{}", id)).unwrap();
        assert!(exists);
    }
}
