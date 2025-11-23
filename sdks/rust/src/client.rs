//! Main client implementation for the LLM Schema Registry SDK.
//!
//! This module provides the primary `SchemaRegistryClient` for interacting with the
//! Schema Registry API. The client uses tokio for async operations and reqwest for
//! HTTP communication, providing zero-cost abstractions and high performance.

use crate::cache::{CacheConfig, SchemaCache};
use crate::errors::{Result, SchemaRegistryError};
use crate::models::*;
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};
use url::Url;

/// Default timeout for HTTP requests (30 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default maximum number of retries
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default initial retry delay (500ms)
const DEFAULT_INITIAL_RETRY_DELAY_MS: u64 = 500;

/// Configuration for the Schema Registry client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL of the Schema Registry API
    pub base_url: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial retry delay (exponential backoff)
    pub initial_retry_delay: Duration,
    /// Cache configuration
    pub cache_config: CacheConfig,
}

impl ClientConfig {
    /// Creates a new client configuration with the given base URL.
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_schema_registry_sdk::client::ClientConfig;
    ///
    /// let config = ClientConfig::new("http://localhost:8080");
    /// ```
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_retries: DEFAULT_MAX_RETRIES,
            initial_retry_delay: Duration::from_millis(DEFAULT_INITIAL_RETRY_DELAY_MS),
            cache_config: CacheConfig::default(),
        }
    }

    /// Sets the API key for authentication.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Sets the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the maximum number of retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Sets the initial retry delay.
    pub fn with_initial_retry_delay(mut self, delay: Duration) -> Self {
        self.initial_retry_delay = delay;
        self
    }

    /// Sets the cache configuration.
    pub fn with_cache_config(mut self, cache_config: CacheConfig) -> Self {
        self.cache_config = cache_config;
        self
    }
}

/// The main Schema Registry client.
///
/// This client provides all operations for interacting with the Schema Registry,
/// including schema registration, validation, compatibility checking, and more.
/// It features automatic retries, caching, and comprehensive error handling.
///
/// # Examples
///
/// ```no_run
/// use llm_schema_registry_sdk::{SchemaRegistryClient, Schema, SchemaFormat};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = SchemaRegistryClient::builder()
///         .base_url("http://localhost:8080")
///         .api_key("your-api-key")
///         .build()?;
///
///     let schema = Schema::new(
///         "telemetry",
///         "InferenceEvent",
///         "1.0.0",
///         SchemaFormat::JsonSchema,
///         r#"{"type": "object"}"#,
///     );
///
///     let result = client.register_schema(schema).await?;
///     println!("Schema ID: {}", result.schema_id);
///
///     Ok(())
/// }
/// ```
pub struct SchemaRegistryClient {
    config: ClientConfig,
    http_client: Client,
    cache: SchemaCache,
}

impl SchemaRegistryClient {
    /// Creates a new client builder.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Creates a new client with the given configuration.
    pub fn new(config: ClientConfig) -> Result<Self> {
        // Validate the base URL
        Url::parse(&config.base_url)
            .map_err(|e| SchemaRegistryError::ConfigError(format!("Invalid base URL: {}", e)))?;

        let http_client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| SchemaRegistryError::ConfigError(format!("Failed to build HTTP client: {}", e)))?;

        let cache = SchemaCache::new(config.cache_config.clone());

        Ok(Self {
            config,
            http_client,
            cache,
        })
    }

    /// Registers a new schema or retrieves an existing one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::{SchemaRegistryClient, Schema, SchemaFormat};
    /// # async fn example(client: SchemaRegistryClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let schema = Schema::new(
    ///     "telemetry",
    ///     "InferenceEvent",
    ///     "1.0.0",
    ///     SchemaFormat::JsonSchema,
    ///     r#"{"type": "object", "properties": {"model": {"type": "string"}}}"#,
    /// );
    ///
    /// let result = client.register_schema(schema).await?;
    /// println!("Registered schema with ID: {}", result.schema_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_schema(&self, schema: Schema) -> Result<RegisterSchemaResponse> {
        let url = self.build_url("/api/v1/schemas")?;

        info!(
            "Registering schema: {}.{} v{}",
            schema.namespace, schema.name, schema.version
        );

        let response = self
            .retry_request(|| async {
                self.add_auth_header(self.http_client.post(&url).json(&schema))
                    .send()
                    .await
            })
            .await?;

        let result: RegisterSchemaResponse = response.json().await?;

        info!("Schema registered successfully: {}", result.schema_id);

        Ok(result)
    }

    /// Retrieves a schema by its ID.
    ///
    /// This method uses the cache for improved performance.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::SchemaRegistryClient;
    /// # async fn example(client: SchemaRegistryClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let schema = client.get_schema("schema-id-123").await?;
    /// println!("Retrieved schema: {}.{}", schema.metadata.namespace, schema.metadata.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_schema(&self, schema_id: &str) -> Result<GetSchemaResponse> {
        // Check cache first
        if let Some(cached) = self.cache.get(schema_id).await {
            debug!("Cache hit for schema ID: {}", schema_id);
            return Ok(cached);
        }

        debug!("Cache miss for schema ID: {}", schema_id);

        let url = self.build_url(&format!("/api/v1/schemas/{}", schema_id))?;

        let response = self
            .retry_request(|| async {
                self.add_auth_header(self.http_client.get(&url))
                    .send()
                    .await
            })
            .await?;

        let result: GetSchemaResponse = response.json().await?;

        // Cache the result
        self.cache.insert(schema_id, result.clone()).await;

        Ok(result)
    }

    /// Retrieves a schema by namespace, name, and version.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::SchemaRegistryClient;
    /// # async fn example(client: SchemaRegistryClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let schema = client.get_schema_by_version("telemetry", "InferenceEvent", "1.0.0").await?;
    /// println!("Retrieved schema: {}", schema.metadata.schema_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_schema_by_version(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
    ) -> Result<GetSchemaResponse> {
        let url = self.build_url(&format!(
            "/api/v1/schemas/{}/{}/versions/{}",
            namespace, name, version
        ))?;

        let response = self
            .retry_request(|| async {
                self.add_auth_header(self.http_client.get(&url))
                    .send()
                    .await
            })
            .await?;

        let result: GetSchemaResponse = response.json().await?;

        // Cache the result by schema_id
        self.cache.insert(&result.metadata.schema_id, result.clone()).await;

        Ok(result)
    }

    /// Validates data against a schema.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::SchemaRegistryClient;
    /// # async fn example(client: SchemaRegistryClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let validation = client.validate_data(
    ///     "schema-id-123",
    ///     r#"{"model": "gpt-4", "timestamp": "2025-01-01T00:00:00Z"}"#
    /// ).await?;
    ///
    /// if validation.is_valid() {
    ///     println!("Data is valid!");
    /// } else {
    ///     println!("Validation errors: {:?}", validation.errors());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_data(&self, schema_id: &str, data: &str) -> Result<ValidateResponse> {
        let url = self.build_url(&format!("/api/v1/schemas/{}/validate", schema_id))?;

        let payload = serde_json::json!({ "data": data });

        let response = self
            .retry_request(|| async {
                self.add_auth_header(self.http_client.post(&url).json(&payload))
                    .send()
                    .await
            })
            .await?;

        let result: ValidateResponse = response.json().await?;

        Ok(result)
    }

    /// Checks compatibility between a new schema and existing versions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::{SchemaRegistryClient, Schema, SchemaFormat, CompatibilityMode};
    /// # async fn example(client: SchemaRegistryClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let new_schema = Schema::new(
    ///     "telemetry",
    ///     "InferenceEvent",
    ///     "2.0.0",
    ///     SchemaFormat::JsonSchema,
    ///     r#"{"type": "object", "properties": {"model": {"type": "string"}, "version": {"type": "string"}}}"#,
    /// );
    ///
    /// let result = client.check_compatibility(new_schema, CompatibilityMode::Backward).await?;
    ///
    /// if result.is_compatible() {
    ///     println!("Schema is compatible!");
    /// } else {
    ///     println!("Compatibility issues: {:?}", result.issues());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_compatibility(
        &self,
        schema: Schema,
        mode: CompatibilityMode,
    ) -> Result<CompatibilityResult> {
        let url = self.build_url("/api/v1/compatibility/check")?;

        let request = CheckCompatibilityRequest { schema, mode };

        let response = self
            .retry_request(|| async {
                self.add_auth_header(self.http_client.post(&url).json(&request))
                    .send()
                    .await
            })
            .await?;

        let result: CompatibilityResult = response.json().await?;

        Ok(result)
    }

    /// Lists all versions of a schema.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::SchemaRegistryClient;
    /// # async fn example(client: SchemaRegistryClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let versions = client.list_versions("telemetry", "InferenceEvent").await?;
    /// for version in versions.versions {
    ///     println!("Version: {} (ID: {})", version.version, version.schema_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_versions(&self, namespace: &str, name: &str) -> Result<ListVersionsResponse> {
        let url = self.build_url(&format!("/api/v1/schemas/{}/{}/versions", namespace, name))?;

        let response = self
            .retry_request(|| async {
                self.add_auth_header(self.http_client.get(&url))
                    .send()
                    .await
            })
            .await?;

        let result: ListVersionsResponse = response.json().await?;

        Ok(result)
    }

    /// Searches for schemas matching a query.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::{SchemaRegistryClient, SearchQuery};
    /// # async fn example(client: SchemaRegistryClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let query = SearchQuery::new("inference")
    ///     .with_namespace("telemetry")
    ///     .with_limit(20);
    ///
    /// let results = client.search_schemas(query).await?;
    /// println!("Found {} schemas", results.total);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_schemas(&self, query: SearchQuery) -> Result<SearchResponse> {
        let url = self.build_url("/api/v1/schemas/search")?;

        let response = self
            .retry_request(|| async {
                self.add_auth_header(self.http_client.post(&url).json(&query))
                    .send()
                    .await
            })
            .await?;

        let result: SearchResponse = response.json().await?;

        Ok(result)
    }

    /// Deletes a schema by ID.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::SchemaRegistryClient;
    /// # async fn example(client: SchemaRegistryClient) -> Result<(), Box<dyn std::error::Error>> {
    /// client.delete_schema("schema-id-123").await?;
    /// println!("Schema deleted successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_schema(&self, schema_id: &str) -> Result<()> {
        let url = self.build_url(&format!("/api/v1/schemas/{}", schema_id))?;

        self.retry_request(|| async {
            self.add_auth_header(self.http_client.delete(&url))
                .send()
                .await
        })
        .await?;

        // Invalidate cache
        self.cache.invalidate(schema_id).await;

        Ok(())
    }

    /// Performs a health check on the Schema Registry service.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_schema_registry_sdk::SchemaRegistryClient;
    /// # async fn example(client: SchemaRegistryClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let health = client.health_check().await?;
    /// if health.is_healthy() {
    ///     println!("Service is healthy");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health_check(&self) -> Result<HealthCheckResponse> {
        let url = self.build_url("/health")?;

        let response = self.add_auth_header(self.http_client.get(&url)).send().await?;

        let result: HealthCheckResponse = response.json().await?;

        Ok(result)
    }

    /// Invalidates the entire cache.
    pub async fn clear_cache(&self) {
        self.cache.invalidate_all().await;
    }

    // Private helper methods

    fn build_url(&self, path: &str) -> Result<String> {
        let base = Url::parse(&self.config.base_url)?;
        let url = base.join(path)?;
        Ok(url.to_string())
    }

    fn add_auth_header(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref api_key) = self.config.api_key {
            request.header("Authorization", format!("Bearer {}", api_key))
        } else {
            request
        }
    }

    async fn retry_request<F, Fut>(&self, request_fn: F) -> Result<reqwest::Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<reqwest::Response, reqwest::Error>>,
    {
        let mut attempts = 0;
        let mut delay = self.config.initial_retry_delay;

        loop {
            attempts += 1;

            let request = request_fn().await;

            match request {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        return Ok(response);
                    }

                    let error = self.handle_error_response(response).await;

                    if attempts >= self.config.max_retries || !error.is_retryable() {
                        return Err(error);
                    }

                    warn!(
                        "Request failed (attempt {}/{}): {}. Retrying in {:?}...",
                        attempts, self.config.max_retries, error, delay
                    );
                }
                Err(e) => {
                    let error: SchemaRegistryError = e.into();

                    if attempts >= self.config.max_retries || !error.is_retryable() {
                        return Err(error);
                    }

                    warn!(
                        "Request failed (attempt {}/{}): {}. Retrying in {:?}...",
                        attempts, self.config.max_retries, error, delay
                    );
                }
            }

            sleep(delay).await;
            delay *= 2; // Exponential backoff
        }
    }

    async fn handle_error_response(&self, response: reqwest::Response) -> SchemaRegistryError {
        let status = response.status();
        let status_code = status.as_u16();

        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read response body".to_string());

        match status {
            StatusCode::NOT_FOUND => SchemaRegistryError::SchemaNotFound(body),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                SchemaRegistryError::AuthenticationError(body)
            }
            StatusCode::TOO_MANY_REQUESTS => SchemaRegistryError::RateLimitError(body),
            StatusCode::BAD_REQUEST => SchemaRegistryError::ValidationError(body),
            StatusCode::CONFLICT => SchemaRegistryError::IncompatibleSchema(body),
            _ => SchemaRegistryError::ServerError {
                status: status_code,
                message: body,
            },
        }
    }
}

/// Builder for creating a SchemaRegistryClient.
#[derive(Default)]
pub struct ClientBuilder {
    config: Option<ClientConfig>,
}

impl ClientBuilder {
    /// Sets the base URL for the client.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.config = Some(ClientConfig::new(base_url));
        self
    }

    /// Sets the API key for authentication.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        if let Some(ref mut config) = self.config {
            config.api_key = Some(api_key.into());
        }
        self
    }

    /// Sets the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        if let Some(ref mut config) = self.config {
            config.timeout = timeout;
        }
        self
    }

    /// Sets the maximum number of retries.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        if let Some(ref mut config) = self.config {
            config.max_retries = max_retries;
        }
        self
    }

    /// Sets the cache configuration.
    pub fn cache_config(mut self, cache_config: CacheConfig) -> Self {
        if let Some(ref mut config) = self.config {
            config.cache_config = cache_config;
        }
        self
    }

    /// Builds the SchemaRegistryClient.
    pub fn build(self) -> Result<SchemaRegistryClient> {
        let config = self
            .config
            .ok_or_else(|| SchemaRegistryError::ConfigError("Base URL is required".to_string()))?;

        SchemaRegistryClient::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfig::new("http://localhost:8080")
            .with_api_key("test-key")
            .with_timeout(Duration::from_secs(60))
            .with_max_retries(5);

        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_builder() {
        let result = SchemaRegistryClient::builder()
            .base_url("http://localhost:8080")
            .api_key("test-key")
            .timeout(Duration::from_secs(30))
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_client_builder_missing_base_url() {
        let result = SchemaRegistryClient::builder().build();

        assert!(result.is_err());
        match result {
            Err(SchemaRegistryError::ConfigError(_)) => (),
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_client_invalid_base_url() {
        let result = SchemaRegistryClient::builder()
            .base_url("not a valid url")
            .build();

        assert!(result.is_err());
    }
}
