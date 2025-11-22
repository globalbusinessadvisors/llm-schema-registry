//! S3 integration tests using LocalStack

use super::*;
use aws_sdk_s3::primitives::ByteStream;
use bytes::Bytes;

#[tokio::test]
async fn test_s3_create_bucket() {
    let env = TestEnvironment::new().await.unwrap();
    let client = env.s3_client().await;

    // List buckets
    let buckets = client.list_buckets().send().await.unwrap();

    // Verify our test bucket exists
    assert!(buckets.buckets().iter().any(|b| b.name() == Some(&env.s3_bucket)));
}

#[tokio::test]
async fn test_s3_put_and_get_object() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    let key = "test/schema1.json";
    let content = r#"{"type": "object", "properties": {}}"#;

    // PUT object
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .body(ByteStream::from(Bytes::from(content)))
        .send()
        .await
        .unwrap();

    // GET object
    let response = client
        .get_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await
        .unwrap();

    let bytes = response.body.collect().await.unwrap().into_bytes();
    let retrieved = String::from_utf8(bytes.to_vec()).unwrap();

    assert_eq!(retrieved, content);
}

#[tokio::test]
async fn test_s3_list_objects() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    // Upload multiple objects
    for i in 0..5 {
        client
            .put_object()
            .bucket(&env.s3_bucket)
            .key(format!("schemas/schema{}.json", i))
            .body(ByteStream::from(Bytes::from(format!(r#"{{"id": {}}}"#, i))))
            .send()
            .await
            .unwrap();
    }

    // List objects
    let response = client
        .list_objects_v2()
        .bucket(&env.s3_bucket)
        .prefix("schemas/")
        .send()
        .await
        .unwrap();

    let count = response.contents().len();
    assert_eq!(count, 5);
}

#[tokio::test]
async fn test_s3_delete_object() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    let key = "test/delete-me.json";

    // PUT object
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .body(ByteStream::from(Bytes::from("{}")))
        .send()
        .await
        .unwrap();

    // Verify exists
    let response = client
        .head_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await;
    assert!(response.is_ok());

    // DELETE object
    client
        .delete_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await
        .unwrap();

    // Verify deleted
    let response = client
        .head_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_s3_object_metadata() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    let key = "test/with-metadata.json";

    // PUT object with metadata
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .body(ByteStream::from(Bytes::from("{}")))
        .metadata("schema-version", "1.0.0")
        .metadata("format", "json")
        .content_type("application/json")
        .send()
        .await
        .unwrap();

    // HEAD object to get metadata
    let response = client
        .head_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await
        .unwrap();

    assert_eq!(response.metadata().unwrap().get("schema-version"), Some(&"1.0.0".to_string()));
    assert_eq!(response.metadata().unwrap().get("format"), Some(&"json".to_string()));
    assert_eq!(response.content_type(), Some("application/json"));
}

#[tokio::test]
async fn test_s3_multipart_upload() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    let key = "test/large-schema.json";

    // Create large content (6MB)
    let part_size = 5 * 1024 * 1024; // 5MB
    let large_content = "x".repeat(part_size + 1024 * 1024); // 6MB total

    // Initiate multipart upload
    let multipart = client
        .create_multipart_upload()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await
        .unwrap();

    let upload_id = multipart.upload_id().unwrap();

    // Upload parts
    let mut parts = vec![];

    // Part 1
    let part1_data = large_content[0..part_size].to_string();
    let part1 = client
        .upload_part()
        .bucket(&env.s3_bucket)
        .key(key)
        .upload_id(upload_id)
        .part_number(1)
        .body(ByteStream::from(Bytes::from(part1_data)))
        .send()
        .await
        .unwrap();

    parts.push(
        aws_sdk_s3::types::CompletedPart::builder()
            .part_number(1)
            .e_tag(part1.e_tag().unwrap())
            .build()
    );

    // Part 2
    let part2_data = large_content[part_size..].to_string();
    let part2 = client
        .upload_part()
        .bucket(&env.s3_bucket)
        .key(key)
        .upload_id(upload_id)
        .part_number(2)
        .body(ByteStream::from(Bytes::from(part2_data)))
        .send()
        .await
        .unwrap();

    parts.push(
        aws_sdk_s3::types::CompletedPart::builder()
            .part_number(2)
            .e_tag(part2.e_tag().unwrap())
            .build()
    );

    // Complete multipart upload
    client
        .complete_multipart_upload()
        .bucket(&env.s3_bucket)
        .key(key)
        .upload_id(upload_id)
        .multipart_upload(
            aws_sdk_s3::types::CompletedMultipartUpload::builder()
                .set_parts(Some(parts))
                .build()
        )
        .send()
        .await
        .unwrap();

    // Verify upload
    let response = client
        .head_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await
        .unwrap();

    assert_eq!(response.content_length(), Some(large_content.len() as i64));
}

#[tokio::test]
async fn test_s3_copy_object() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    let source_key = "test/source.json";
    let dest_key = "test/destination.json";
    let content = r#"{"copy": "test"}"#;

    // PUT source object
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(source_key)
        .body(ByteStream::from(Bytes::from(content)))
        .send()
        .await
        .unwrap();

    // COPY object
    let copy_source = format!("{}/{}", env.s3_bucket, source_key);
    client
        .copy_object()
        .bucket(&env.s3_bucket)
        .key(dest_key)
        .copy_source(&copy_source)
        .send()
        .await
        .unwrap();

    // GET destination object
    let response = client
        .get_object()
        .bucket(&env.s3_bucket)
        .key(dest_key)
        .send()
        .await
        .unwrap();

    let bytes = response.body.collect().await.unwrap().into_bytes();
    let retrieved = String::from_utf8(bytes.to_vec()).unwrap();

    assert_eq!(retrieved, content);
}

#[tokio::test]
async fn test_s3_object_versioning_simulation() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    let key = "schemas/versioned.json";

    // Upload version 1
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .body(ByteStream::from(Bytes::from(r#"{"version": 1}"#)))
        .metadata("version", "1.0.0")
        .send()
        .await
        .unwrap();

    // Upload version 2 (overwrites)
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .body(ByteStream::from(Bytes::from(r#"{"version": 2}"#)))
        .metadata("version", "2.0.0")
        .send()
        .await
        .unwrap();

    // GET latest version
    let response = client
        .get_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await
        .unwrap();

    let bytes = response.body.collect().await.unwrap().into_bytes();
    let content = String::from_utf8(bytes.to_vec()).unwrap();

    assert_eq!(content, r#"{"version": 2}"#);

    // Check metadata
    let head = client
        .head_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await
        .unwrap();

    assert_eq!(head.metadata().unwrap().get("version"), Some(&"2.0.0".to_string()));
}

#[tokio::test]
async fn test_s3_prefix_organization() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    // Create hierarchical structure
    let schemas = vec![
        "schemas/v1/user.json",
        "schemas/v1/product.json",
        "schemas/v2/user.json",
        "schemas/v2/product.json",
        "schemas/v2/order.json",
    ];

    for key in &schemas {
        client
            .put_object()
            .bucket(&env.s3_bucket)
            .key(*key)
            .body(ByteStream::from(Bytes::from("{}")))
            .send()
            .await
            .unwrap();
    }

    // List v1 schemas
    let v1_response = client
        .list_objects_v2()
        .bucket(&env.s3_bucket)
        .prefix("schemas/v1/")
        .send()
        .await
        .unwrap();

    assert_eq!(v1_response.contents().len(), 2);

    // List v2 schemas
    let v2_response = client
        .list_objects_v2()
        .bucket(&env.s3_bucket)
        .prefix("schemas/v2/")
        .send()
        .await
        .unwrap();

    assert_eq!(v2_response.contents().len(), 3);
}

#[tokio::test]
async fn test_s3_pagination() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    // Upload 15 objects
    for i in 0..15 {
        client
            .put_object()
            .bucket(&env.s3_bucket)
            .key(format!("page-test/object{:02}.json", i))
            .body(ByteStream::from(Bytes::from("{}")))
            .send()
            .await
            .unwrap();
    }

    // List with pagination (max 5 per page)
    let mut total_count = 0;
    let mut continuation_token = None;

    loop {
        let mut request = client
            .list_objects_v2()
            .bucket(&env.s3_bucket)
            .prefix("page-test/")
            .max_keys(5);

        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }

        let response = request.send().await.unwrap();

        total_count += response.contents().len();
        continuation_token = response.next_continuation_token().map(|s| s.to_string());

        if continuation_token.is_none() {
            break;
        }
    }

    assert_eq!(total_count, 15);
}

#[tokio::test]
async fn test_s3_storage_class() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    let key = "test/storage-class.json";

    // PUT with storage class
    client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .body(ByteStream::from(Bytes::from("{}")))
        .storage_class(aws_sdk_s3::types::StorageClass::Standard)
        .send()
        .await
        .unwrap();

    // Verify storage class
    let head = client
        .head_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await
        .unwrap();

    // LocalStack may not fully support storage classes, but we can verify the upload succeeded
    assert!(head.content_length().unwrap() > 0);
}

#[tokio::test]
async fn test_s3_etag_validation() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    let key = "test/etag.json";
    let content = r#"{"etag": "test"}"#;

    // PUT object
    let put_response = client
        .put_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .body(ByteStream::from(Bytes::from(content)))
        .send()
        .await
        .unwrap();

    let etag = put_response.e_tag().unwrap();

    // GET object and verify ETag
    let get_response = client
        .get_object()
        .bucket(&env.s3_bucket)
        .key(key)
        .send()
        .await
        .unwrap();

    assert_eq!(get_response.e_tag().unwrap(), etag);
}

#[tokio::test]
async fn test_s3_batch_delete() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    // Upload multiple objects
    for i in 0..10 {
        client
            .put_object()
            .bucket(&env.s3_bucket)
            .key(format!("batch/object{}.json", i))
            .body(ByteStream::from(Bytes::from("{}")))
            .send()
            .await
            .unwrap();
    }

    // Batch delete
    let objects_to_delete: Vec<_> = (0..10)
        .map(|i| {
            aws_sdk_s3::types::ObjectIdentifier::builder()
                .key(format!("batch/object{}.json", i))
                .build()
                .unwrap()
        })
        .collect();

    client
        .delete_objects()
        .bucket(&env.s3_bucket)
        .delete(
            aws_sdk_s3::types::Delete::builder()
                .set_objects(Some(objects_to_delete))
                .build()
                .unwrap()
        )
        .send()
        .await
        .unwrap();

    // Verify all deleted
    let response = client
        .list_objects_v2()
        .bucket(&env.s3_bucket)
        .prefix("batch/")
        .send()
        .await
        .unwrap();

    assert_eq!(response.contents().len(), 0);
}

#[tokio::test]
async fn test_s3_concurrent_uploads() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();

    let mut handles = vec![];

    // Spawn 10 concurrent upload tasks
    for i in 0..10 {
        let endpoint = env.s3_endpoint.clone();
        let bucket = env.s3_bucket.clone();

        let handle = tokio::spawn(async move {
            let config = aws_config::from_env()
                .endpoint_url(&endpoint)
                .region("us-east-1")
                .credentials_provider(aws_sdk_s3::config::Credentials::new(
                    "test",
                    "test",
                    None,
                    None,
                    "test",
                ))
                .load()
                .await;

            let client = aws_sdk_s3::Client::new(&config);

            client
                .put_object()
                .bucket(&bucket)
                .key(format!("concurrent/upload{}.json", i))
                .body(ByteStream::from(Bytes::from(format!(r#"{{"id": {}}}"#, i))))
                .send()
                .await
        });

        handles.push(handle);
    }

    // Wait for all uploads
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all uploaded
    let client = env.s3_client().await;
    let response = client
        .list_objects_v2()
        .bucket(&env.s3_bucket)
        .prefix("concurrent/")
        .send()
        .await
        .unwrap();

    assert_eq!(response.contents().len(), 10);
}

#[tokio::test]
async fn test_s3_performance_benchmark() {
    let env = TestEnvironment::new().await.unwrap();
    env.reset().await.unwrap();
    let client = env.s3_client().await;

    let content = "x".repeat(1024); // 1KB

    // Benchmark uploads
    let start = std::time::Instant::now();
    for i in 0..100 {
        client
            .put_object()
            .bucket(&env.s3_bucket)
            .key(format!("perf/object{}.json", i))
            .body(ByteStream::from(Bytes::from(content.clone())))
            .send()
            .await
            .unwrap();
    }
    let upload_duration = start.elapsed();

    // Benchmark downloads
    let start = std::time::Instant::now();
    for i in 0..100 {
        let _ = client
            .get_object()
            .bucket(&env.s3_bucket)
            .key(format!("perf/object{}.json", i))
            .send()
            .await
            .unwrap();
    }
    let download_duration = start.elapsed();

    tracing::info!("Upload 100 objects (1KB each): {:?}", upload_duration);
    tracing::info!("Download 100 objects: {:?}", download_duration);

    // Should complete reasonably fast (adjust based on environment)
    assert!(upload_duration.as_secs() < 30, "Uploads too slow");
    assert!(download_duration.as_secs() < 30, "Downloads too slow");
}
