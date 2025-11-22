//! S3 storage backend for large schema content

use crate::error::{Result, StorageError};
use aws_config::BehaviorVersion;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use schema_registry_core::{Schema, SchemaContent, SchemaId};
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};

/// S3 storage backend for large schema content
///
/// This backend is used to store large schema content (> threshold size) in S3,
/// while keeping metadata and small schemas in PostgreSQL.
pub struct S3Backend {
    client: S3Client,
    bucket: String,
    prefix: String,
    min_size_bytes: usize,
}

impl S3Backend {
    /// Create a new S3 backend
    ///
    /// # Arguments
    /// * `bucket` - S3 bucket name
    /// * `region` - AWS region
    /// * `prefix` - Key prefix for all objects (e.g., "schemas/")
    /// * `min_size_bytes` - Minimum schema size to store in S3
    pub async fn new(
        bucket: String,
        region: &str,
        prefix: Option<String>,
        min_size_bytes: usize,
    ) -> Result<Self> {
        info!("Initializing S3 backend for bucket: {}", bucket);

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = S3Client::new(&config);

        // Verify bucket access
        match client.head_bucket().bucket(&bucket).send().await {
            Ok(_) => info!("S3 bucket '{}' is accessible", bucket),
            Err(e) => {
                error!("Failed to access S3 bucket '{}': {}", bucket, e);
                return Err(StorageError::S3Error(format!(
                    "Bucket access failed: {}",
                    e
                )));
            }
        }

        Ok(Self {
            client,
            bucket,
            prefix: prefix.unwrap_or_else(|| "schemas/".to_string()),
            min_size_bytes,
        })
    }

    /// Generate S3 key for a schema
    fn schema_key(&self, id: SchemaId) -> String {
        format!("{}{}.bin", self.prefix, id)
    }

    /// Check if schema content should be stored in S3
    pub fn should_use_s3(&self, content: &SchemaContent) -> bool {
        content.size_bytes() >= self.min_size_bytes
    }

    /// Store schema content in S3
    ///
    /// # Arguments
    /// * `id` - Schema ID
    /// * `content` - Schema content to store
    ///
    /// # Returns
    /// S3 object key
    #[instrument(skip(self, content), fields(schema_id = %id, size = content.size_bytes()))]
    pub async fn put_schema_content(
        &self,
        id: SchemaId,
        content: &SchemaContent,
    ) -> Result<String> {
        let key = self.schema_key(id);
        let size = content.size_bytes();
        
        debug!("Storing schema content in S3: {} ({} bytes)", key, size);

        // Serialize content
        let bytes = bincode::serialize(content)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let body = ByteStream::from(bytes);

        // Upload to S3
        match self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(body)
            .content_type("application/octet-stream")
            .metadata("schema_id", id.to_string())
            .metadata("original_size", size.to_string())
            .send()
            .await
        {
            Ok(_) => {
                info!("Schema content stored in S3: {}", key);
                metrics::counter!("schema_registry.s3.uploads").increment(1);
                metrics::histogram!("schema_registry.s3.upload_size_bytes").record(size as f64);
                Ok(key)
            }
            Err(e) => {
                error!("Failed to store schema content in S3: {}", e);
                metrics::counter!("schema_registry.s3.upload_errors").increment(1);
                Err(StorageError::S3Error(format!("Upload failed: {}", e)))
            }
        }
    }

    /// Retrieve schema content from S3
    ///
    /// # Arguments
    /// * `id` - Schema ID
    ///
    /// # Returns
    /// Schema content if found
    #[instrument(skip(self), fields(schema_id = %id))]
    pub async fn get_schema_content(&self, id: SchemaId) -> Result<Option<SchemaContent>> {
        let key = self.schema_key(id);
        debug!("Fetching schema content from S3: {}", key);

        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(output) => {
                let bytes = output
                    .body
                    .collect()
                    .await
                    .map_err(|e| StorageError::S3Error(format!("Failed to read S3 body: {}", e)))?
                    .into_bytes();

                let content: SchemaContent = bincode::deserialize(&bytes)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

                debug!("Schema content retrieved from S3: {} bytes", bytes.len());
                metrics::counter!("schema_registry.s3.downloads").increment(1);
                metrics::histogram!("schema_registry.s3.download_size_bytes").record(bytes.len() as f64);

                Ok(Some(content))
            }
            Err(e) => {
                if e.to_string().contains("NoSuchKey") {
                    debug!("Schema content not found in S3: {}", key);
                    Ok(None)
                } else {
                    error!("Failed to retrieve schema content from S3: {}", e);
                    metrics::counter!("schema_registry.s3.download_errors").increment(1);
                    Err(StorageError::S3Error(format!("Download failed: {}", e)))
                }
            }
        }
    }

    /// Delete schema content from S3
    ///
    /// # Arguments
    /// * `id` - Schema ID
    #[instrument(skip(self), fields(schema_id = %id))]
    pub async fn delete_schema_content(&self, id: SchemaId) -> Result<()> {
        let key = self.schema_key(id);
        debug!("Deleting schema content from S3: {}", key);

        match self
            .client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => {
                info!("Schema content deleted from S3: {}", key);
                metrics::counter!("schema_registry.s3.deletes").increment(1);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to delete schema content from S3: {}", e);
                metrics::counter!("schema_registry.s3.delete_errors").increment(1);
                // Don't fail if delete fails (idempotent operation)
                Ok(())
            }
        }
    }

    /// Generate a presigned URL for direct access
    ///
    /// # Arguments
    /// * `id` - Schema ID
    /// * `expires_in` - URL expiration duration
    ///
    /// # Returns
    /// Presigned URL as a string
    #[instrument(skip(self), fields(schema_id = %id))]
    pub async fn generate_presigned_url(
        &self,
        id: SchemaId,
        expires_in: Duration,
    ) -> Result<String> {
        let key = self.schema_key(id);
        debug!("Generating presigned URL for schema: {}", key);

        let presigning_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| StorageError::S3Error(format!("Invalid presigning config: {}", e)))?;

        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .presigned(presigning_config)
            .await
        {
            Ok(presigned) => {
                info!("Presigned URL generated for schema: {}", id);
                metrics::counter!("schema_registry.s3.presigned_urls").increment(1);
                Ok(presigned.uri().to_string())
            }
            Err(e) => {
                error!("Failed to generate presigned URL: {}", e);
                Err(StorageError::S3Error(format!(
                    "Presigned URL generation failed: {}",
                    e
                )))
            }
        }
    }

    /// Check if schema content exists in S3
    pub async fn exists(&self, id: SchemaId) -> Result<bool> {
        let key = self.schema_key(id);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(StorageError::S3Error(format!("Head object failed: {}", e)))
                }
            }
        }
    }

    /// List all schema objects with the configured prefix
    pub async fn list_all(&self) -> Result<Vec<String>> {
        debug!("Listing all schema objects in S3");

        let mut objects = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut req = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&self.prefix);

            if let Some(token) = continuation_token {
                req = req.continuation_token(token);
            }

            match req.send().await {
                Ok(output) => {
                    if let Some(contents) = output.contents {
                        for obj in contents {
                            if let Some(key) = obj.key {
                                objects.push(key);
                            }
                        }
                    }

                    if output.is_truncated.unwrap_or(false) {
                        continuation_token = output.next_continuation_token;
                    } else {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to list S3 objects: {}", e);
                    return Err(StorageError::S3Error(format!("List objects failed: {}", e)));
                }
            }
        }

        info!("Found {} schema objects in S3", objects.len());
        Ok(objects)
    }

    /// Get storage statistics from S3
    pub async fn statistics(&self) -> Result<S3Statistics> {
        let objects = self.list_all().await?;
        let total_objects = objects.len() as i64;

        // This is a simplified version - in production you'd want to batch this
        // or use S3 inventory/analytics
        Ok(S3Statistics {
            total_objects,
            bucket: self.bucket.clone(),
            prefix: self.prefix.clone(),
        })
    }
}

/// S3 storage statistics
#[derive(Debug, Clone)]
pub struct S3Statistics {
    pub total_objects: i64,
    pub bucket: String,
    pub prefix: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require AWS credentials and a test S3 bucket
    // Run with: AWS_PROFILE=test cargo test --features test-s3
    
    #[tokio::test]
    #[ignore]
    async fn test_s3_basic_operations() {
        let backend = S3Backend::new(
            "test-schema-registry".to_string(),
            "us-east-1",
            Some("test/".to_string()),
            1024,
        )
        .await
        .unwrap();

        let id = SchemaId::new();
        let content = SchemaContent::Json(serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }));

        // Should not exist initially
        assert!(!backend.exists(id).await.unwrap());

        // Upload
        let key = backend.put_schema_content(id, &content).await.unwrap();
        assert!(!key.is_empty());

        // Should now exist
        assert!(backend.exists(id).await.unwrap());

        // Download
        let retrieved = backend.get_schema_content(id).await.unwrap();
        assert!(retrieved.is_some());

        // Generate presigned URL
        let url = backend
            .generate_presigned_url(id, Duration::from_secs(3600))
            .await
            .unwrap();
        assert!(url.starts_with("https://"));

        // Delete
        backend.delete_schema_content(id).await.unwrap();
        assert!(!backend.exists(id).await.unwrap());
    }
}
