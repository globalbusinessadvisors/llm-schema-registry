// Configuration Management System
// Supports environment variables, config files, Kubernetes ConfigMaps, and secrets

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server configuration
    pub server: ServerSettings,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Redis configuration
    pub redis: RedisConfig,

    /// S3 configuration
    pub s3: S3Config,

    /// Security configuration
    pub security: SecurityConfig,

    /// Observability configuration
    pub observability: ObservabilityConfig,

    /// Feature flags
    pub features: FeatureFlags,

    /// Performance tuning
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// Server listen address
    #[serde(default = "default_listen_address")]
    pub listen_address: String,

    /// HTTP port
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// gRPC port
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,

    /// Graceful shutdown timeout (seconds)
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_seconds: u64,

    /// Request timeout (seconds)
    #[serde(default = "default_request_timeout")]
    pub request_timeout_seconds: u64,

    /// Enable TLS
    #[serde(default)]
    pub tls_enabled: bool,

    /// TLS certificate path
    pub tls_cert_path: Option<PathBuf>,

    /// TLS key path
    pub tls_key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub url: String,

    /// Connection pool minimum size
    #[serde(default = "default_db_pool_min")]
    pub pool_min: u32,

    /// Connection pool maximum size
    #[serde(default = "default_db_pool_max")]
    pub pool_max: u32,

    /// Connection timeout (seconds)
    #[serde(default = "default_db_timeout")]
    pub connection_timeout_seconds: u64,

    /// Statement timeout (seconds)
    #[serde(default = "default_statement_timeout")]
    pub statement_timeout_seconds: u64,

    /// Enable SSL
    #[serde(default = "default_true")]
    pub ssl_enabled: bool,

    /// SSL certificate path
    pub ssl_cert_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,

    /// Connection pool minimum size
    #[serde(default = "default_redis_pool_min")]
    pub pool_min: u32,

    /// Connection pool maximum size
    #[serde(default = "default_redis_pool_max")]
    pub pool_max: u32,

    /// Connection timeout (seconds)
    #[serde(default = "default_redis_timeout")]
    pub connection_timeout_seconds: u64,

    /// Enable TLS
    #[serde(default)]
    pub tls_enabled: bool,

    /// Key prefix for all cache keys
    #[serde(default = "default_redis_prefix")]
    pub key_prefix: String,

    /// Default TTL for cache entries (seconds)
    #[serde(default = "default_cache_ttl")]
    pub default_ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,

    /// AWS region
    #[serde(default = "default_aws_region")]
    pub region: String,

    /// S3 endpoint (for S3-compatible services)
    pub endpoint: Option<String>,

    /// Enable path-style access
    #[serde(default)]
    pub path_style: bool,

    /// Access key ID
    pub access_key_id: Option<String>,

    /// Secret access key
    pub secret_access_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    #[serde(default = "default_true")]
    pub auth_enabled: bool,

    /// JWT secret (base64 encoded)
    pub jwt_secret: Option<String>,

    /// JWT token expiration (seconds)
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_seconds: u64,

    /// Enable API key authentication
    #[serde(default = "default_true")]
    pub api_key_enabled: bool,

    /// Enable OAuth 2.0
    #[serde(default)]
    pub oauth_enabled: bool,

    /// OAuth provider URL
    pub oauth_provider_url: Option<String>,

    /// OAuth client ID
    pub oauth_client_id: Option<String>,

    /// OAuth client secret
    pub oauth_client_secret: Option<String>,

    /// Enable mTLS
    #[serde(default)]
    pub mtls_enabled: bool,

    /// CA certificate path
    pub ca_cert_path: Option<PathBuf>,

    /// Enable audit logging
    #[serde(default = "default_true")]
    pub audit_logging_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Enable Prometheus metrics
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,

    /// Metrics endpoint path
    #[serde(default = "default_metrics_path")]
    pub metrics_path: String,

    /// Enable OpenTelemetry tracing
    #[serde(default = "default_true")]
    pub tracing_enabled: bool,

    /// OTLP endpoint
    pub otlp_endpoint: Option<String>,

    /// Trace sampling rate (0.0 - 1.0)
    #[serde(default = "default_trace_sampling")]
    pub trace_sampling_rate: f64,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Log format (json or text)
    #[serde(default = "default_log_format")]
    pub log_format: String,

    /// Enable Sentry error tracking
    #[serde(default)]
    pub sentry_enabled: bool,

    /// Sentry DSN
    pub sentry_dsn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable caching
    #[serde(default = "default_true")]
    pub caching_enabled: bool,

    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub rate_limiting_enabled: bool,

    /// Enable CORS
    #[serde(default = "default_true")]
    pub cors_enabled: bool,

    /// Enable compression
    #[serde(default = "default_true")]
    pub compression_enabled: bool,

    /// Enable read-only mode
    #[serde(default)]
    pub read_only_mode: bool,

    /// Enable circuit breaker
    #[serde(default = "default_true")]
    pub circuit_breaker_enabled: bool,

    /// Enable cache warming on startup
    #[serde(default = "default_true")]
    pub cache_warming_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Rate limit: requests per second per IP
    #[serde(default = "default_rate_limit_rps")]
    pub rate_limit_rps: u32,

    /// Circuit breaker error threshold percentage
    #[serde(default = "default_circuit_breaker_threshold")]
    pub circuit_breaker_threshold_pct: u8,

    /// Circuit breaker timeout (seconds)
    #[serde(default = "default_circuit_breaker_timeout")]
    pub circuit_breaker_timeout_seconds: u64,

    /// Max request body size (bytes)
    #[serde(default = "default_max_body_size")]
    pub max_request_body_bytes: usize,

    /// Connection keep-alive (seconds)
    #[serde(default = "default_keepalive")]
    pub keepalive_seconds: u64,

    /// Worker threads
    pub worker_threads: Option<usize>,
}

// Default value functions
fn default_listen_address() -> String {
    "0.0.0.0".to_string()
}

fn default_http_port() -> u16 {
    8080
}

fn default_grpc_port() -> u16 {
    9090
}

fn default_shutdown_timeout() -> u64 {
    30
}

fn default_request_timeout() -> u64 {
    30
}

fn default_db_pool_min() -> u32 {
    5
}

fn default_db_pool_max() -> u32 {
    50
}

fn default_db_timeout() -> u64 {
    10
}

fn default_statement_timeout() -> u64 {
    30
}

fn default_redis_pool_min() -> u32 {
    5
}

fn default_redis_pool_max() -> u32 {
    25
}

fn default_redis_timeout() -> u64 {
    5
}

fn default_redis_prefix() -> String {
    "schema_registry:".to_string()
}

fn default_cache_ttl() -> u64 {
    3600 // 1 hour
}

fn default_aws_region() -> String {
    "us-east-1".to_string()
}

fn default_jwt_expiration() -> u64 {
    3600 // 1 hour
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

fn default_trace_sampling() -> f64 {
    0.1 // 10%
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_rate_limit_rps() -> u32 {
    100
}

fn default_circuit_breaker_threshold() -> u8 {
    50
}

fn default_circuit_breaker_timeout() -> u64 {
    60
}

fn default_max_body_size() -> usize {
    10 * 1024 * 1024 // 10 MB
}

fn default_keepalive() -> u64 {
    60
}

fn default_true() -> bool {
    true
}

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder();

        // Set defaults
        builder = builder
            .set_default("server.listen_address", default_listen_address())?
            .set_default("server.http_port", default_http_port() as i64)?
            .set_default("server.grpc_port", default_grpc_port() as i64)?
            .set_default("server.shutdown_timeout_seconds", default_shutdown_timeout() as i64)?
            .set_default("server.request_timeout_seconds", default_request_timeout() as i64)?
            .set_default("server.tls_enabled", false)?
            .set_default("database.pool_min", default_db_pool_min() as i64)?
            .set_default("database.pool_max", default_db_pool_max() as i64)?
            .set_default("features.caching_enabled", true)?
            .set_default("features.rate_limiting_enabled", true)?;

        // Add environment variables with prefix
        builder = builder.add_source(
            config::Environment::with_prefix("SCHEMA_REGISTRY")
                .separator("__")
                .try_parsing(true),
        );

        // Build and deserialize
        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(
                config::Environment::with_prefix("SCHEMA_REGISTRY")
                    .separator("__")
                    .try_parsing(true),
            );

        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate database URL
        if self.database.url.is_empty() {
            errors.push("Database URL is required".to_string());
        }

        // Validate Redis URL
        if self.redis.url.is_empty() {
            errors.push("Redis URL is required".to_string());
        }

        // Validate S3 bucket
        if self.s3.bucket.is_empty() {
            errors.push("S3 bucket name is required".to_string());
        }

        // Validate TLS configuration
        if self.server.tls_enabled {
            if self.server.tls_cert_path.is_none() {
                errors.push("TLS certificate path required when TLS is enabled".to_string());
            }
            if self.server.tls_key_path.is_none() {
                errors.push("TLS key path required when TLS is enabled".to_string());
            }
        }

        // Validate authentication configuration
        if self.security.auth_enabled {
            if self.security.jwt_secret.is_none() && !self.security.oauth_enabled {
                errors.push("JWT secret or OAuth must be configured when auth is enabled".to_string());
            }
        }

        // Validate pool sizes
        if self.database.pool_min > self.database.pool_max {
            errors.push("Database pool_min cannot be greater than pool_max".to_string());
        }

        if self.redis.pool_min > self.redis.pool_max {
            errors.push("Redis pool_min cannot be greater than pool_max".to_string());
        }

        // Validate trace sampling rate
        if self.observability.trace_sampling_rate < 0.0 || self.observability.trace_sampling_rate > 1.0 {
            errors.push("Trace sampling rate must be between 0.0 and 1.0".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Print configuration summary (redacting secrets)
    pub fn print_summary(&self) {
        tracing::info!("===========================================");
        tracing::info!("Configuration Summary");
        tracing::info!("===========================================");
        tracing::info!("Server:");
        tracing::info!("  HTTP: {}:{}", self.server.listen_address, self.server.http_port);
        tracing::info!("  gRPC: {}:{}", self.server.listen_address, self.server.grpc_port);
        tracing::info!("  TLS: {}", if self.server.tls_enabled { "enabled" } else { "disabled" });
        tracing::info!("Database:");
        tracing::info!("  Pool: {}-{} connections", self.database.pool_min, self.database.pool_max);
        tracing::info!("  SSL: {}", if self.database.ssl_enabled { "enabled" } else { "disabled" });
        tracing::info!("Redis:");
        tracing::info!("  Pool: {}-{} connections", self.redis.pool_min, self.redis.pool_max);
        tracing::info!("  TTL: {}s", self.redis.default_ttl_seconds);
        tracing::info!("S3:");
        tracing::info!("  Bucket: {}", self.s3.bucket);
        tracing::info!("  Region: {}", self.s3.region);
        tracing::info!("Security:");
        tracing::info!("  Auth: {}", if self.security.auth_enabled { "enabled" } else { "disabled" });
        tracing::info!("  Audit: {}", if self.security.audit_logging_enabled { "enabled" } else { "disabled" });
        tracing::info!("Features:");
        tracing::info!("  Caching: {}", if self.features.caching_enabled { "enabled" } else { "disabled" });
        tracing::info!("  Rate Limiting: {}", if self.features.rate_limiting_enabled { "enabled" } else { "disabled" });
        tracing::info!("  Circuit Breaker: {}", if self.features.circuit_breaker_enabled { "enabled" } else { "disabled" });
        tracing::info!("  Read-Only: {}", if self.features.read_only_mode { "YES" } else { "no" });
        tracing::info!("===========================================");
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerSettings {
                listen_address: default_listen_address(),
                http_port: default_http_port(),
                grpc_port: default_grpc_port(),
                shutdown_timeout_seconds: default_shutdown_timeout(),
                request_timeout_seconds: default_request_timeout(),
                tls_enabled: false,
                tls_cert_path: None,
                tls_key_path: None,
            },
            database: DatabaseConfig {
                url: "postgresql://postgres:postgres@localhost:5432/schema_registry".to_string(),
                pool_min: default_db_pool_min(),
                pool_max: default_db_pool_max(),
                connection_timeout_seconds: default_db_timeout(),
                statement_timeout_seconds: default_statement_timeout(),
                ssl_enabled: true,
                ssl_cert_path: None,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                pool_min: default_redis_pool_min(),
                pool_max: default_redis_pool_max(),
                connection_timeout_seconds: default_redis_timeout(),
                tls_enabled: false,
                key_prefix: default_redis_prefix(),
                default_ttl_seconds: default_cache_ttl(),
            },
            s3: S3Config {
                bucket: "schema-registry".to_string(),
                region: default_aws_region(),
                endpoint: None,
                path_style: false,
                access_key_id: None,
                secret_access_key: None,
            },
            security: SecurityConfig {
                auth_enabled: true,
                jwt_secret: None,
                jwt_expiration_seconds: default_jwt_expiration(),
                api_key_enabled: true,
                oauth_enabled: false,
                oauth_provider_url: None,
                oauth_client_id: None,
                oauth_client_secret: None,
                mtls_enabled: false,
                ca_cert_path: None,
                audit_logging_enabled: true,
            },
            observability: ObservabilityConfig {
                metrics_enabled: true,
                metrics_path: default_metrics_path(),
                tracing_enabled: true,
                otlp_endpoint: None,
                trace_sampling_rate: default_trace_sampling(),
                log_level: default_log_level(),
                log_format: default_log_format(),
                sentry_enabled: false,
                sentry_dsn: None,
            },
            features: FeatureFlags {
                caching_enabled: true,
                rate_limiting_enabled: true,
                cors_enabled: true,
                compression_enabled: true,
                read_only_mode: false,
                circuit_breaker_enabled: true,
                cache_warming_enabled: true,
            },
            performance: PerformanceConfig {
                rate_limit_rps: default_rate_limit_rps(),
                circuit_breaker_threshold_pct: default_circuit_breaker_threshold(),
                circuit_breaker_timeout_seconds: default_circuit_breaker_timeout(),
                max_request_body_bytes: default_max_body_size(),
                keepalive_seconds: default_keepalive(),
                worker_threads: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.server.http_port, 8080);
        assert_eq!(config.server.grpc_port, 9090);
        assert_eq!(config.database.pool_max, 50);
        assert_eq!(config.redis.pool_max, 25);
    }

    #[test]
    fn test_config_validation() {
        let config = ServerConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_pool_sizes() {
        let mut config = ServerConfig::default();
        config.database.pool_min = 100;
        config.database.pool_max = 50;

        let result = config.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("pool_min")));
    }

    #[test]
    fn test_invalid_trace_sampling() {
        let mut config = ServerConfig::default();
        config.observability.trace_sampling_rate = 1.5;

        let result = config.validate();
        assert!(result.is_err());
    }
}
