//! Redis integration tests

use super::*;
use redis::Commands;
use serde_json::json;

#[tokio::test]
async fn test_redis_connection() {
    let env = TestEnvironment::new().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();
    let pong: String = redis::cmd("PING").query(&mut conn).unwrap();

    assert_eq!(pong, "PONG");
}

#[tokio::test]
async fn test_redis_set_get() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // SET
    let _: () = conn.set("test:key", "test:value").unwrap();

    // GET
    let value: String = conn.get("test:key").unwrap();
    assert_eq!(value, "test:value");
}

#[tokio::test]
async fn test_redis_expiration() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // SET with expiration
    let _: () = conn.set_ex("test:expire", "value", 1).unwrap();

    // Verify key exists
    let exists: bool = conn.exists("test:expire").unwrap();
    assert!(exists);

    // Wait for expiration
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Verify key expired
    let exists: bool = conn.exists("test:expire").unwrap();
    assert!(!exists);
}

#[tokio::test]
async fn test_redis_hash_operations() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // HSET
    let _: () = conn.hset_multiple("test:hash", &[
        ("field1", "value1"),
        ("field2", "value2"),
        ("field3", "value3"),
    ]).unwrap();

    // HGET
    let value: String = conn.hget("test:hash", "field1").unwrap();
    assert_eq!(value, "value1");

    // HGETALL
    let all: Vec<(String, String)> = conn.hgetall("test:hash").unwrap();
    assert_eq!(all.len(), 3);

    // HDEL
    let _: () = conn.hdel("test:hash", "field2").unwrap();

    let exists: bool = conn.hexists("test:hash", "field2").unwrap();
    assert!(!exists);
}

#[tokio::test]
async fn test_redis_list_operations() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // LPUSH
    let _: () = conn.lpush("test:list", &["item1", "item2", "item3"]).unwrap();

    // LLEN
    let len: usize = conn.llen("test:list").unwrap();
    assert_eq!(len, 3);

    // LRANGE
    let items: Vec<String> = conn.lrange("test:list", 0, -1).unwrap();
    assert_eq!(items, vec!["item3", "item2", "item1"]);

    // LPOP
    let item: String = conn.lpop("test:list", None).unwrap();
    assert_eq!(item, "item3");
}

#[tokio::test]
async fn test_redis_set_operations() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // SADD
    let _: () = conn.sadd("test:set", &["member1", "member2", "member3"]).unwrap();

    // SCARD
    let size: usize = conn.scard("test:set").unwrap();
    assert_eq!(size, 3);

    // SISMEMBER
    let is_member: bool = conn.sismember("test:set", "member1").unwrap();
    assert!(is_member);

    // SMEMBERS
    let members: Vec<String> = conn.smembers("test:set").unwrap();
    assert_eq!(members.len(), 3);

    // SREM
    let _: () = conn.srem("test:set", "member2").unwrap();

    let size: usize = conn.scard("test:set").unwrap();
    assert_eq!(size, 2);
}

#[tokio::test]
async fn test_redis_sorted_set_operations() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // ZADD
    let _: () = conn.zadd_multiple("test:zset", &[
        (1.0, "one"),
        (2.0, "two"),
        (3.0, "three"),
    ]).unwrap();

    // ZCARD
    let size: usize = conn.zcard("test:zset").unwrap();
    assert_eq!(size, 3);

    // ZRANGE
    let members: Vec<String> = conn.zrange("test:zset", 0, -1).unwrap();
    assert_eq!(members, vec!["one", "two", "three"]);

    // ZRANK
    let rank: Option<usize> = conn.zrank("test:zset", "two").unwrap();
    assert_eq!(rank, Some(1));

    // ZSCORE
    let score: f64 = conn.zscore("test:zset", "three").unwrap();
    assert_eq!(score, 3.0);
}

#[tokio::test]
async fn test_redis_transaction() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // MULTI/EXEC
    let (v1, v2): (String, String) = redis::pipe()
        .atomic()
        .set("key1", "value1")
        .get("key1")
        .set("key2", "value2")
        .get("key2")
        .query(&mut conn)
        .unwrap();

    assert_eq!(v1, "value1");
    assert_eq!(v2, "value2");
}

#[tokio::test]
async fn test_redis_pub_sub() {
    let env = TestEnvironment::new().await.unwrap();

    let client = env.redis_client().clone();

    // Spawn subscriber task
    let handle = tokio::spawn(async move {
        let mut conn = client.get_connection().unwrap();
        let mut pubsub = conn.as_pubsub();

        pubsub.subscribe("test:channel").unwrap();

        // Wait for message
        let msg = pubsub.get_message().unwrap();
        let payload: String = msg.get_payload().unwrap();

        payload
    });

    // Give subscriber time to connect
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Publish message
    let mut conn = env.redis_client().get_connection().unwrap();
    let _: () = conn.publish("test:channel", "hello world").unwrap();

    // Wait for result
    let payload = handle.await.unwrap();
    assert_eq!(payload, "hello world");
}

#[tokio::test]
async fn test_redis_json_storage() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"}
        }
    });

    let serialized = serde_json::to_string(&schema).unwrap();

    // Store JSON as string
    let _: () = conn.set("schema:123", serialized).unwrap();

    // Retrieve and deserialize
    let retrieved: String = conn.get("schema:123").unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&retrieved).unwrap();

    assert_eq!(deserialized, schema);
}

#[tokio::test]
async fn test_redis_cache_invalidation() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Set multiple cache keys
    let _: () = conn.set("cache:schema:1", "value1").unwrap();
    let _: () = conn.set("cache:schema:2", "value2").unwrap();
    let _: () = conn.set("cache:schema:3", "value3").unwrap();

    // Verify they exist
    let count: usize = conn.keys::<_, Vec<String>>("cache:schema:*").unwrap().len();
    assert_eq!(count, 3);

    // Invalidate all schema cache keys
    let keys: Vec<String> = conn.keys("cache:schema:*").unwrap();
    for key in keys {
        let _: () = conn.del(&key).unwrap();
    }

    // Verify all deleted
    let count: usize = conn.keys::<_, Vec<String>>("cache:schema:*").unwrap().len();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_redis_atomic_increment() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // INCR
    let v1: i32 = conn.incr("counter", 1).unwrap();
    assert_eq!(v1, 1);

    let v2: i32 = conn.incr("counter", 5).unwrap();
    assert_eq!(v2, 6);

    // DECR
    let v3: i32 = conn.decr("counter", 2).unwrap();
    assert_eq!(v3, 4);
}

#[tokio::test]
async fn test_redis_key_pattern_matching() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Set keys with patterns
    let _: () = conn.set("user:1:name", "Alice").unwrap();
    let _: () = conn.set("user:2:name", "Bob").unwrap();
    let _: () = conn.set("user:1:email", "alice@example.com").unwrap();
    let _: () = conn.set("schema:1:content", "{}").unwrap();

    // Find all user keys
    let user_keys: Vec<String> = conn.keys("user:*").unwrap();
    assert_eq!(user_keys.len(), 3);

    // Find specific pattern
    let name_keys: Vec<String> = conn.keys("user:*:name").unwrap();
    assert_eq!(name_keys.len(), 2);
}

#[tokio::test]
async fn test_redis_scan_operation() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Insert many keys
    for i in 0..100 {
        let _: () = conn.set(format!("key:{}", i), format!("value:{}", i)).unwrap();
    }

    // Use SCAN to iterate (better than KEYS for production)
    let mut cursor = 0;
    let mut count = 0;

    loop {
        let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("key:*")
            .arg("COUNT")
            .arg(10)
            .query(&mut conn)
            .unwrap();

        count += keys.len();
        cursor = new_cursor;

        if cursor == 0 {
            break;
        }
    }

    assert_eq!(count, 100);
}

#[tokio::test]
async fn test_redis_bitfield_operations() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // SETBIT
    let _: () = conn.setbit("bitmap", 0, true).unwrap();
    let _: () = conn.setbit("bitmap", 2, true).unwrap();
    let _: () = conn.setbit("bitmap", 4, true).unwrap();

    // GETBIT
    let bit: bool = conn.getbit("bitmap", 2).unwrap();
    assert!(bit);

    let bit: bool = conn.getbit("bitmap", 1).unwrap();
    assert!(!bit);

    // BITCOUNT
    let count: u32 = conn.getbit::<_, bool>("bitmap", 0).unwrap() as u32
        + conn.getbit::<_, bool>("bitmap", 1).unwrap() as u32
        + conn.getbit::<_, bool>("bitmap", 2).unwrap() as u32
        + conn.getbit::<_, bool>("bitmap", 3).unwrap() as u32
        + conn.getbit::<_, bool>("bitmap", 4).unwrap() as u32;

    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_redis_lua_script() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Lua script to atomically increment and get value
    let script = redis::Script::new(r#"
        redis.call('SET', KEYS[1], ARGV[1])
        redis.call('INCR', KEYS[1])
        return redis.call('GET', KEYS[1])
    "#);

    let result: String = script.key("counter").arg("10").invoke(&mut conn).unwrap();
    assert_eq!(result, "11");
}

#[tokio::test]
async fn test_redis_connection_pool_simulation() {
    let env = TestEnvironment::new().await.unwrap();

    let mut handles = vec![];

    // Simulate 10 concurrent connections
    for i in 0..10 {
        let client = env.redis_client().clone();
        let handle = tokio::spawn(async move {
            let mut conn = client.get_connection().unwrap();
            let key = format!("concurrent:{}", i);
            let _: () = conn.set(&key, i).unwrap();
            let value: i32 = conn.get(&key).unwrap();
            value
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for (i, handle) in handles.into_iter().enumerate() {
        let value = handle.await.unwrap();
        assert_eq!(value, i as i32);
    }
}

#[tokio::test]
async fn test_redis_performance_benchmark() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut conn = env.redis_client().get_connection().unwrap();

    // Benchmark SET operations
    let start = std::time::Instant::now();
    for i in 0..1000 {
        let _: () = conn.set(format!("perf:{}", i), i).unwrap();
    }
    let set_duration = start.elapsed();

    // Benchmark GET operations
    let start = std::time::Instant::now();
    for i in 0..1000 {
        let _: i32 = conn.get(format!("perf:{}", i)).unwrap();
    }
    let get_duration = start.elapsed();

    tracing::info!("SET 1000 keys: {:?}", set_duration);
    tracing::info!("GET 1000 keys: {:?}", get_duration);

    // Should be very fast
    assert!(set_duration.as_millis() < 1000, "SET too slow");
    assert!(get_duration.as_millis() < 1000, "GET too slow");
}
