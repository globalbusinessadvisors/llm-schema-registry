//! Storage abstraction layer for the LLM Schema Registry
//!
//! This crate provides a flexible storage backend abstraction with implementations for:
//! - PostgreSQL (primary metadata store)
//! - Redis (caching layer)
//! - S3 (large schema content storage)

pub mod backend;
pub mod cache;
pub mod error;
pub mod postgres;
pub mod redis_cache;
pub mod s3;
pub mod query;

pub use backend::{StorageBackend, StorageConfig};
pub use cache::{CacheManager, CacheConfig};
pub use error::{StorageError, Result};
pub use postgres::PostgresBackend;
pub use redis_cache::RedisCache;
pub use s3::S3Backend;
pub use query::{SearchQuery, SchemaFilter};
