//! Basic usage examples for the storage layer

use schema_registry_core::{Schema, SchemaContent, SchemaMetadata, SchemaVersion};
use schema_registry_storage::{
    backend::{BackendConfig, PoolConfig, StorageBackend, StorageConfig},
    query::{SchemaFilter, SearchQuery},
    CacheConfig, CacheManager, PostgresBackend,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== LLM Schema Registry Storage Layer Examples ===\n");

    // Example 1: Initialize PostgreSQL backend
    example_postgres_setup().await?;

    // Example 2: Basic CRUD operations
    example_crud_operations().await?;

    // Example 3: Cache usage
    example_cache_usage().await?;

    // Example 4: Search and filtering
    example_search().await?;

    println!("\n=== All examples completed successfully ===");
    Ok(())
}

async fn example_postgres_setup() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: PostgreSQL Backend Setup\n");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/schema_registry".to_string());

    let pool_config = PoolConfig {
        max_connections: 10,
        min_connections: 2,
        acquire_timeout_secs: 3,
        ..Default::default()
    };

    println!("Connecting to database: {}", database_url);
    
    match PostgresBackend::new(&database_url, &pool_config).await {
        Ok(backend) => {
            println!("✓ Connected successfully");
            
            // Run migrations
            if let Err(e) = backend.migrate().await {
                println!("✗ Migration error (may be expected if DB doesn't exist): {}", e);
            } else {
                println!("✓ Migrations completed");
            }

            // Get statistics
            if let Ok(stats) = backend.statistics().await {
                println!("✓ Storage statistics:");
                println!("  - Total schemas: {}", stats.total_schemas);
                println!("  - Active schemas: {}", stats.active_schemas);
            }
        }
        Err(e) => {
            println!("✗ Connection error (expected if PostgreSQL not running): {}", e);
        }
    }

    println!();
    Ok(())
}

async fn example_crud_operations() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: CRUD Operations (Mock)\n");

    // Create a test schema
    let schema = Schema::new(
        "com.example.user".to_string(),
        SchemaVersion::new(1, 0, 0),
        SchemaContent::Json(serde_json::json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"}
            }
        })),
        SchemaMetadata::default(),
    );

    println!("Created schema:");
    println!("  - ID: {}", schema.id);
    println!("  - Subject: {}", schema.subject);
    println!("  - Version: {}", schema.version);

    println!();
    Ok(())
}

async fn example_cache_usage() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Cache Usage\n");

    let config = CacheConfig::default();
    let cache = CacheManager::new(config);
    
    let schema = Schema::new(
        "com.example.product".to_string(),
        SchemaVersion::new(1, 0, 0),
        SchemaContent::Json(serde_json::json!({"type": "object"})),
        SchemaMetadata::default(),
    );

    let id = schema.id;
    cache.put(id, Arc::new(schema)).await;
    
    let stats = cache.statistics();
    println!("✓ Cache statistics:");
    println!("  - Hit rate: {:.2}%", stats.hit_rate * 100.0);

    println!();
    Ok(())
}

async fn example_search() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: Search and Filtering\n");

    let query = SearchQuery {
        subject_pattern: Some("com.example.*".to_string()),
        ..Default::default()
    };

    println!("✓ Search query created");
    println!();
    Ok(())
}
