//! Database integration tests

use super::*;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_database_connection() {
    let env = TestEnvironment::new().await.expect("Failed to create test environment");

    // Simple query to verify connection
    let result: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(env.db_pool())
        .await
        .expect("Query failed");

    assert_eq!(result.0, 1);
}

#[tokio::test]
async fn test_schema_crud_operations() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let schema_id = Uuid::new_v4();
    let now = Utc::now();

    // CREATE
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

    // READ
    let row: (Uuid, String, String) = sqlx::query_as(
        "SELECT id, name, namespace FROM schemas WHERE id = $1"
    )
    .bind(schema_id)
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert_eq!(row.0, schema_id);
    assert_eq!(row.1, "User");
    assert_eq!(row.2, "com.example");

    // UPDATE
    sqlx::query("UPDATE schemas SET state = $1 WHERE id = $2")
        .bind("DEPRECATED")
        .bind(schema_id)
        .execute(env.db_pool())
        .await
        .unwrap();

    let state: (String,) = sqlx::query_as("SELECT state FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    assert_eq!(state.0, "DEPRECATED");

    // DELETE
    sqlx::query("DELETE FROM schemas WHERE id = $1")
        .bind(schema_id)
        .execute(env.db_pool())
        .await
        .unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn test_schema_unique_constraint() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();

    // Insert first schema
    sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(Uuid::new_v4())
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

    // Try to insert duplicate (same namespace, name, version)
    let result = sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(Uuid::new_v4())
    .bind("User")
    .bind("com.example")
    .bind("1.0.0")
    .bind("json")
    .bind(r#"{"type": "object", "properties": {}}"#)
    .bind("def456")
    .bind("Duplicate schema")
    .bind("BACKWARD")
    .bind("ACTIVE")
    .bind(now)
    .bind(now)
    .execute(env.db_pool())
    .await;

    assert!(result.is_err(), "Should fail due to unique constraint");
}

#[tokio::test]
async fn test_schema_versioning_query() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();

    // Insert multiple versions
    for version in ["1.0.0", "1.1.0", "1.2.0", "2.0.0"] {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                description, compatibility_mode, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(Uuid::new_v4())
        .bind("User")
        .bind("com.example")
        .bind(version)
        .bind("json")
        .bind(r#"{"type": "object"}"#)
        .bind(format!("hash_{}", version))
        .bind("User schema")
        .bind("BACKWARD")
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Query all versions
    let versions: Vec<(String,)> = sqlx::query_as(
        "SELECT version FROM schemas WHERE namespace = $1 AND name = $2 ORDER BY version"
    )
    .bind("com.example")
    .bind("User")
    .fetch_all(env.db_pool())
    .await
    .unwrap();

    assert_eq!(versions.len(), 4);
    assert_eq!(versions[0].0, "1.0.0");
    assert_eq!(versions[3].0, "2.0.0");
}

#[tokio::test]
async fn test_compatibility_check_tracking() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();

    // Create two schemas
    let schema1_id = Uuid::new_v4();
    let schema2_id = Uuid::new_v4();

    for (id, version) in [(schema1_id, "1.0.0"), (schema2_id, "2.0.0")] {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                description, compatibility_mode, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(id)
        .bind("User")
        .bind("com.example")
        .bind(version)
        .bind("json")
        .bind(r#"{"type": "object"}"#)
        .bind(format!("hash_{}", version))
        .bind("User schema")
        .bind("BACKWARD")
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Record compatibility check
    let check_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO compatibility_checks (id, old_schema_id, new_schema_id, mode, is_compatible, violations, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    )
    .bind(check_id)
    .bind(schema1_id)
    .bind(schema2_id)
    .bind("BACKWARD")
    .bind(true)
    .bind(json!([]))
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Query compatibility check
    let row: (Uuid, bool) = sqlx::query_as(
        "SELECT id, is_compatible FROM compatibility_checks WHERE old_schema_id = $1"
    )
    .bind(schema1_id)
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert_eq!(row.0, check_id);
    assert!(row.1);
}

#[tokio::test]
async fn test_validation_results_storage() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();
    let schema_id = Uuid::new_v4();

    // Create schema
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
    .bind(r#"{"type": "object", "required": ["name"]}"#)
    .bind("abc123")
    .bind("User schema")
    .bind("BACKWARD")
    .bind("ACTIVE")
    .bind(now)
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Store validation result
    let validation_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO validation_results (id, schema_id, data_hash, is_valid, errors, validated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(validation_id)
    .bind(schema_id)
    .bind("data_hash_123")
    .bind(false)
    .bind(json!([{"error": "Missing required field: name"}]))
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Query validation result
    let row: (bool, serde_json::Value) = sqlx::query_as(
        "SELECT is_valid, errors FROM validation_results WHERE id = $1"
    )
    .bind(validation_id)
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert!(!row.0);
    assert!(row.1.is_array());
}

#[tokio::test]
async fn test_transaction_rollback() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();
    let schema_id = Uuid::new_v4();

    // Start transaction
    let mut tx = env.db_pool().begin().await.unwrap();

    // Insert schema
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
    .execute(&mut *tx)
    .await
    .unwrap();

    // Rollback
    tx.rollback().await.unwrap();

    // Verify schema doesn't exist
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn test_transaction_commit() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();
    let schema_id = Uuid::new_v4();

    // Start transaction
    let mut tx = env.db_pool().begin().await.unwrap();

    // Insert schema
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
    .execute(&mut *tx)
    .await
    .unwrap();

    // Commit
    tx.commit().await.unwrap();

    // Verify schema exists
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM schemas WHERE id = $1")
        .bind(schema_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn test_index_performance() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();

    // Insert 1000 schemas
    for i in 0..1000 {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                description, compatibility_mode, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(format!("Schema{}", i))
        .bind("com.example")
        .bind("1.0.0")
        .bind("json")
        .bind(r#"{"type": "object"}"#)
        .bind(format!("hash_{}", i))
        .bind("Test schema")
        .bind("BACKWARD")
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Query with index (should be fast)
    let start = std::time::Instant::now();
    let _schemas: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM schemas WHERE namespace = $1 AND name = $2"
    )
    .bind("com.example")
    .bind("Schema500")
    .fetch_all(env.db_pool())
    .await
    .unwrap();
    let elapsed = start.elapsed();

    // Should be very fast with index (<10ms)
    assert!(elapsed.as_millis() < 10, "Query too slow: {:?}", elapsed);
}

#[tokio::test]
async fn test_jsonb_metadata_query() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();
    let schema_id = Uuid::new_v4();

    // Insert schema with JSONB metadata
    sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
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
    .bind(json!({"team": "backend", "priority": "high"}))
    .execute(env.db_pool())
    .await
    .unwrap();

    // Query JSONB field
    let metadata: (serde_json::Value,) = sqlx::query_as(
        "SELECT metadata FROM schemas WHERE id = $1"
    )
    .bind(schema_id)
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert_eq!(metadata.0["team"], "backend");
    assert_eq!(metadata.0["priority"], "high");

    // Query using JSONB operators
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM schemas WHERE metadata->>'team' = $1"
    )
    .bind("backend")
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn test_concurrent_inserts() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();
    let mut handles = vec![];

    // Spawn 10 concurrent insert tasks
    for i in 0..10 {
        let pool = env.db_pool().clone();
        let handle = tokio::spawn(async move {
            sqlx::query(
                r#"
                INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                    description, compatibility_mode, state, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                "#
            )
            .bind(Uuid::new_v4())
            .bind(format!("Schema{}", i))
            .bind("com.example")
            .bind("1.0.0")
            .bind("json")
            .bind(r#"{"type": "object"}"#)
            .bind(format!("hash_{}", i))
            .bind("Test schema")
            .bind("BACKWARD")
            .bind("ACTIVE")
            .bind(now)
            .bind(now)
            .execute(&pool)
            .await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all inserts
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM schemas")
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    assert_eq!(count.0, 10);
}

#[tokio::test]
async fn test_database_pool_health() {
    let env = TestEnvironment::new().await.unwrap();

    // Verify pool is healthy
    assert!(env.db_pool().is_closed() == false);

    // Execute a simple query
    let result: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    assert_eq!(result.0, 1);
}

#[tokio::test]
async fn test_cascade_delete_compatibility_checks() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();
    let schema_id = Uuid::new_v4();

    // Create schema
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

    // Create compatibility check
    let check_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO compatibility_checks (id, old_schema_id, new_schema_id, mode, is_compatible, violations, created_at)
        VALUES ($1, $2, $2, $3, $4, $5, $6)
        "#
    )
    .bind(check_id)
    .bind(schema_id)
    .bind("BACKWARD")
    .bind(true)
    .bind(serde_json::json!([]))
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Delete schema (should cascade to compatibility checks)
    sqlx::query("DELETE FROM schemas WHERE id = $1")
        .bind(schema_id)
        .execute(env.db_pool())
        .await
        .unwrap();

    // Verify compatibility check was also deleted
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM compatibility_checks WHERE id = $1")
        .bind(check_id)
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn test_partial_text_search() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();

    // Insert schemas with different descriptions
    for (name, description) in [
        ("UserProfile", "User profile schema for authentication"),
        ("ProductCatalog", "Product catalog schema"),
        ("OrderManagement", "Schema for order management system"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                description, compatibility_mode, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind("com.example")
        .bind("1.0.0")
        .bind("json")
        .bind(r#"{"type": "object"}"#)
        .bind(format!("hash_{}", name))
        .bind(description)
        .bind("BACKWARD")
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Search for schemas containing "schema"
    let results: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM schemas WHERE description ILIKE $1"
    )
    .bind("%schema%")
    .fetch_all(env.db_pool())
    .await
    .unwrap();

    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_batch_insert_performance() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();
    let start = std::time::Instant::now();

    // Batch insert 100 schemas in a transaction
    let mut tx = env.db_pool().begin().await.unwrap();

    for i in 0..100 {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                description, compatibility_mode, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(format!("Schema{}", i))
        .bind("com.batch")
        .bind("1.0.0")
        .bind("json")
        .bind(r#"{"type": "object"}"#)
        .bind(format!("hash_{}", i))
        .bind("Batch test")
        .bind("BACKWARD")
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .unwrap();
    }

    tx.commit().await.unwrap();
    let elapsed = start.elapsed();

    // Should complete quickly
    assert!(elapsed.as_secs() < 5, "Batch insert too slow: {:?}", elapsed);

    // Verify count
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM schemas WHERE namespace = $1")
        .bind("com.batch")
        .fetch_one(env.db_pool())
        .await
        .unwrap();

    assert_eq!(count.0, 100);
}

#[tokio::test]
async fn test_query_with_limit_offset() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();

    // Insert 20 schemas
    for i in 0..20 {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                description, compatibility_mode, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(format!("Schema{:02}", i))
        .bind("com.pagination")
        .bind("1.0.0")
        .bind("json")
        .bind(r#"{"type": "object"}"#)
        .bind(format!("hash_{}", i))
        .bind("Pagination test")
        .bind("BACKWARD")
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Query with pagination
    let page1: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM schemas WHERE namespace = $1 ORDER BY name LIMIT 5 OFFSET 0"
    )
    .bind("com.pagination")
    .fetch_all(env.db_pool())
    .await
    .unwrap();

    let page2: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM schemas WHERE namespace = $1 ORDER BY name LIMIT 5 OFFSET 5"
    )
    .bind("com.pagination")
    .fetch_all(env.db_pool())
    .await
    .unwrap();

    assert_eq!(page1.len(), 5);
    assert_eq!(page2.len(), 5);
    assert_ne!(page1[0].0, page2[0].0);
}

#[tokio::test]
async fn test_complex_join_query() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();
    let schema1_id = Uuid::new_v4();
    let schema2_id = Uuid::new_v4();

    // Create two schemas
    for (id, version) in [(schema1_id, "1.0.0"), (schema2_id, "2.0.0")] {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                                description, compatibility_mode, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(id)
        .bind("Product")
        .bind("com.join")
        .bind(version)
        .bind("json")
        .bind(r#"{"type": "object"}"#)
        .bind(format!("hash_{}", version))
        .bind("Join test")
        .bind("BACKWARD")
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Create validation results for both schemas
    for schema_id in [schema1_id, schema2_id] {
        sqlx::query(
            r#"
            INSERT INTO validation_results (id, schema_id, data_hash, is_valid, errors, validated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(schema_id)
        .bind("data_hash_123")
        .bind(true)
        .bind(serde_json::json!([]))
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Join query: schemas with validation results
    let results: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT s.version, v.data_hash
        FROM schemas s
        INNER JOIN validation_results v ON s.id = v.schema_id
        WHERE s.namespace = $1
        ORDER BY s.version
        "#
    )
    .bind("com.join")
    .fetch_all(env.db_pool())
    .await
    .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "1.0.0");
    assert_eq!(results[1].0, "2.0.0");
}

#[tokio::test]
async fn test_null_handling() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();
    let schema_id = Uuid::new_v4();

    // Insert schema with NULL metadata
    sqlx::query(
        r#"
        INSERT INTO schemas (id, name, namespace, version, format, content, content_hash,
                            description, compatibility_mode, state, created_at, updated_at, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#
    )
    .bind(schema_id)
    .bind("NullTest")
    .bind("com.example")
    .bind("1.0.0")
    .bind("json")
    .bind(r#"{"type": "object"}"#)
    .bind("abc123")
    .bind("Null test")
    .bind("BACKWARD")
    .bind("ACTIVE")
    .bind(now)
    .bind(now)
    .bind(None::<serde_json::Value>)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Query with NULL check
    let row: (Option<serde_json::Value>,) = sqlx::query_as(
        "SELECT metadata FROM schemas WHERE id = $1"
    )
    .bind(schema_id)
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert!(row.0.is_none());
}
