//! Database integration tests with production schema

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
        INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                            schema_type, content, metadata, compatibility_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(schema_id)
    .bind("com.example.User")
    .bind(1)
    .bind(0)
    .bind(0)
    .bind("JSON")
    .bind(json!({"type": "object", "properties": {"name": {"type": "string"}}}))
    .bind(json!({"state": "ACTIVE", "description": "User schema"}))
    .bind("BACKWARD")
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // READ
    let row: (Uuid, String, i32, i32, i32) = sqlx::query_as(
        "SELECT id, subject, version_major, version_minor, version_patch FROM schemas WHERE id = $1"
    )
    .bind(schema_id)
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert_eq!(row.0, schema_id);
    assert_eq!(row.1, "com.example.User");
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 0);

    // UPDATE
    sqlx::query("UPDATE schemas SET metadata = jsonb_set(metadata, '{state}', '\"DEPRECATED\"') WHERE id = $1")
        .bind(schema_id)
        .execute(env.db_pool())
        .await
        .unwrap();

    let state: (String,) = sqlx::query_as("SELECT metadata->>'state' FROM schemas WHERE id = $1")
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
        INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                            schema_type, content, metadata, compatibility_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(Uuid::new_v4())
    .bind("com.example.User")
    .bind(1)
    .bind(0)
    .bind(0)
    .bind("JSON")
    .bind(json!({"type": "object"}))
    .bind(json!({}))
    .bind("BACKWARD")
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Try to insert duplicate (same subject and version)
    let result = sqlx::query(
        r#"
        INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                            schema_type, content, metadata, compatibility_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(Uuid::new_v4())
    .bind("com.example.User")
    .bind(1)
    .bind(0)
    .bind(0)
    .bind("JSON")
    .bind(json!({"type": "object", "properties": {}}))
    .bind(json!({}))
    .bind("BACKWARD")
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
    for (major, minor, patch) in [(1, 0, 0), (1, 1, 0), (1, 2, 0), (2, 0, 0)] {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                                schema_type, content, metadata, compatibility_level, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(Uuid::new_v4())
        .bind("com.example.User")
        .bind(major)
        .bind(minor)
        .bind(patch)
        .bind("JSON")
        .bind(json!({"type": "object"}))
        .bind(json!({}))
        .bind("BACKWARD")
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Query all versions
    let versions: Vec<(i32, i32, i32)> = sqlx::query_as(
        "SELECT version_major, version_minor, version_patch FROM schemas WHERE subject = $1 ORDER BY version_major, version_minor, version_patch"
    )
    .bind("com.example.User")
    .fetch_all(env.db_pool())
    .await
    .unwrap();

    assert_eq!(versions.len(), 4);
    assert_eq!(versions[0], (1, 0, 0));
    assert_eq!(versions[3], (2, 0, 0));
}

#[tokio::test]
async fn test_compatibility_check_tracking() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();

    // Create two schemas
    let schema1_id = Uuid::new_v4();
    let schema2_id = Uuid::new_v4();

    for (id, major, minor, patch) in [(schema1_id, 1, 0, 0), (schema2_id, 2, 0, 0)] {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                                schema_type, content, metadata, compatibility_level, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(id)
        .bind("com.example.User")
        .bind(major)
        .bind(minor)
        .bind(patch)
        .bind("JSON")
        .bind(json!({"type": "object"}))
        .bind(json!({}))
        .bind("BACKWARD")
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Record compatibility check
    sqlx::query(
        r#"
        INSERT INTO compatibility_checks (subject, old_version, new_version, compatibility_level, compatible, violations, checked_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    )
    .bind("com.example.User")
    .bind("1.0.0")
    .bind("2.0.0")
    .bind("BACKWARD")
    .bind(true)
    .bind(json!([]))
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Query compatibility check
    let row: (String, String, bool) = sqlx::query_as(
        "SELECT old_version, new_version, compatible FROM compatibility_checks WHERE subject = $1"
    )
    .bind("com.example.User")
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert_eq!(row.0, "1.0.0");
    assert_eq!(row.1, "2.0.0");
    assert!(row.2);
}

#[tokio::test]
async fn test_validation_history_storage() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = Utc::now();
    let schema_id = Uuid::new_v4();

    // Create schema
    sqlx::query(
        r#"
        INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                            schema_type, content, metadata, compatibility_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(schema_id)
    .bind("com.example.User")
    .bind(1)
    .bind(0)
    .bind(0)
    .bind("JSON")
    .bind(json!({"type": "object", "required": ["name"]}))
    .bind(json!({}))
    .bind("BACKWARD")
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Store validation result
    sqlx::query(
        r#"
        INSERT INTO validation_history (schema_id, data_hash, valid, error_count, validated_at, duration_ms)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(schema_id)
    .bind("data_hash_123")
    .bind(false)
    .bind(1)
    .bind(now)
    .bind(15.5)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Query validation result
    let row: (bool, i32, f64) = sqlx::query_as(
        "SELECT valid, error_count, duration_ms FROM validation_history WHERE schema_id = $1"
    )
    .bind(schema_id)
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert!(!row.0);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, 15.5);
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
        INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                            schema_type, content, metadata, compatibility_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(schema_id)
    .bind("com.example.User")
    .bind(1)
    .bind(0)
    .bind(0)
    .bind("JSON")
    .bind(json!({"type": "object"}))
    .bind(json!({}))
    .bind("BACKWARD")
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
        INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                            schema_type, content, metadata, compatibility_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(schema_id)
    .bind("com.example.User")
    .bind(1)
    .bind(0)
    .bind(0)
    .bind("JSON")
    .bind(json!({"type": "object"}))
    .bind(json!({}))
    .bind("BACKWARD")
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
            INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                                schema_type, content, metadata, compatibility_level, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(format!("com.example.Schema{}", i))
        .bind(1)
        .bind(0)
        .bind(0)
        .bind("JSON")
        .bind(json!({"type": "object"}))
        .bind(json!({}))
        .bind("BACKWARD")
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Query with index (should be fast)
    let start = std::time::Instant::now();
    let _schemas: Vec<(String,)> = sqlx::query_as(
        "SELECT subject FROM schemas WHERE subject = $1"
    )
    .bind("com.example.Schema500")
    .fetch_all(env.db_pool())
    .await
    .unwrap();
    let elapsed = start.elapsed();

    // Should be very fast with index (<50ms)
    assert!(elapsed.as_millis() < 50, "Query too slow: {:?}", elapsed);
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
        INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                            schema_type, content, metadata, compatibility_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(schema_id)
    .bind("com.example.User")
    .bind(1)
    .bind(0)
    .bind(0)
    .bind("JSON")
    .bind(json!({"type": "object"}))
    .bind(json!({"team": "backend", "priority": "high"}))
    .bind("BACKWARD")
    .bind(now)
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
                INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                                    schema_type, content, metadata, compatibility_level, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#
            )
            .bind(Uuid::new_v4())
            .bind(format!("com.example.Schema{}", i))
            .bind(1)
            .bind(0)
            .bind(0)
            .bind("JSON")
            .bind(json!({"type": "object"}))
            .bind(json!({}))
            .bind("BACKWARD")
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
async fn test_cascade_delete_validation_history() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();
    let schema_id = Uuid::new_v4();

    // Create schema
    sqlx::query(
        r#"
        INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                            schema_type, content, metadata, compatibility_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(schema_id)
    .bind("com.example.User")
    .bind(1)
    .bind(0)
    .bind(0)
    .bind("JSON")
    .bind(json!({"type": "object"}))
    .bind(json!({}))
    .bind("BACKWARD")
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Create validation history
    sqlx::query(
        r#"
        INSERT INTO validation_history (schema_id, data_hash, valid, error_count, validated_at, duration_ms)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(schema_id)
    .bind("hash123")
    .bind(true)
    .bind(0)
    .bind(now)
    .bind(10.0)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Delete schema (should cascade to validation history)
    sqlx::query("DELETE FROM schemas WHERE id = $1")
        .bind(schema_id)
        .execute(env.db_pool())
        .await
        .unwrap();

    // Verify validation history was also deleted
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM validation_history WHERE schema_id = $1")
        .bind(schema_id)
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

    // Insert schemas with different subjects
    for subject in [
        "com.example.UserProfile",
        "com.example.ProductCatalog",
        "com.example.OrderManagement",
    ] {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                                schema_type, content, metadata, compatibility_level, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(subject)
        .bind(1)
        .bind(0)
        .bind(0)
        .bind("JSON")
        .bind(json!({"type": "object"}))
        .bind(json!({}))
        .bind("BACKWARD")
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Search for schemas containing "example"
    let results: Vec<(String,)> = sqlx::query_as(
        "SELECT subject FROM schemas WHERE subject ILIKE $1"
    )
    .bind("%example%")
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
            INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                                schema_type, content, metadata, compatibility_level, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(format!("com.batch.Schema{}", i))
        .bind(1)
        .bind(0)
        .bind(0)
        .bind("JSON")
        .bind(json!({"type": "object"}))
        .bind(json!({}))
        .bind("BACKWARD")
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
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM schemas WHERE subject LIKE 'com.batch.%'")
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
            INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                                schema_type, content, metadata, compatibility_level, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(format!("com.pagination.Schema{:02}", i))
        .bind(1)
        .bind(0)
        .bind(0)
        .bind("JSON")
        .bind(json!({"type": "object"}))
        .bind(json!({}))
        .bind("BACKWARD")
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Query with pagination
    let page1: Vec<(String,)> = sqlx::query_as(
        "SELECT subject FROM schemas WHERE subject LIKE 'com.pagination.%' ORDER BY subject LIMIT 5 OFFSET 0"
    )
    .fetch_all(env.db_pool())
    .await
    .unwrap();

    let page2: Vec<(String,)> = sqlx::query_as(
        "SELECT subject FROM schemas WHERE subject LIKE 'com.pagination.%' ORDER BY subject LIMIT 5 OFFSET 5"
    )
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
    for (id, major, minor, patch) in [(schema1_id, 1, 0, 0), (schema2_id, 2, 0, 0)] {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                                schema_type, content, metadata, compatibility_level, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(id)
        .bind("com.join.Product")
        .bind(major)
        .bind(minor)
        .bind(patch)
        .bind("JSON")
        .bind(json!({"type": "object"}))
        .bind(json!({}))
        .bind("BACKWARD")
        .bind(now)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Create validation history for both schemas
    for schema_id in [schema1_id, schema2_id] {
        sqlx::query(
            r#"
            INSERT INTO validation_history (schema_id, data_hash, valid, error_count, validated_at, duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#
        )
        .bind(schema_id)
        .bind("data_hash_123")
        .bind(true)
        .bind(0)
        .bind(now)
        .bind(10.0)
        .execute(env.db_pool())
        .await
        .unwrap();
    }

    // Join query: schemas with validation history
    let results: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT
            s.version_major || '.' || s.version_minor || '.' || s.version_patch as version,
            v.data_hash
        FROM schemas s
        INNER JOIN validation_history v ON s.id = v.schema_id
        WHERE s.subject = $1
        ORDER BY s.version_major, s.version_minor, s.version_patch
        "#
    )
    .bind("com.join.Product")
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

    // Insert schema with NULL version_prerelease and version_build
    sqlx::query(
        r#"
        INSERT INTO schemas (id, subject, version_major, version_minor, version_patch,
                            version_prerelease, version_build,
                            schema_type, content, metadata, compatibility_level, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(schema_id)
    .bind("com.example.NullTest")
    .bind(1)
    .bind(0)
    .bind(0)
    .bind(None::<String>)
    .bind(None::<String>)
    .bind("JSON")
    .bind(json!({"type": "object"}))
    .bind(json!({}))
    .bind("BACKWARD")
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Query with NULL check
    let row: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT version_prerelease, version_build FROM schemas WHERE id = $1"
    )
    .bind(schema_id)
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert!(row.0.is_none());
    assert!(row.1.is_none());
}

#[tokio::test]
async fn test_subjects_table() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let now = chrono::Utc::now();

    // Insert subject
    sqlx::query(
        r#"
        INSERT INTO subjects (name, default_compatibility_level, description, tags, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind("com.example.User")
    .bind("BACKWARD")
    .bind("User schema subject")
    .bind(vec!["user", "authentication"])
    .bind(now)
    .bind(now)
    .execute(env.db_pool())
    .await
    .unwrap();

    // Query subject
    let row: (String, String, String, Vec<String>) = sqlx::query_as(
        "SELECT name, default_compatibility_level, description, tags FROM subjects WHERE name = $1"
    )
    .bind("com.example.User")
    .fetch_one(env.db_pool())
    .await
    .unwrap();

    assert_eq!(row.0, "com.example.User");
    assert_eq!(row.1, "BACKWARD");
    assert_eq!(row.2, "User schema subject");
    assert_eq!(row.3, vec!["user", "authentication"]);
}
