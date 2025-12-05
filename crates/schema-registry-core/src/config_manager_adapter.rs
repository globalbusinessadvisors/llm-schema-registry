//! Config Manager Adapter
//!
//! Lightweight adapter module for consuming configuration and schema policies
//! from the LLM Config Manager. This module provides a thin, trait-based
//! interface for runtime integration without modifying core logic.
//!
//! # Architecture
//!
//! This adapter follows the "consumes-from" pattern where Schema Registry
//! ingests configuration state from Config Manager as an upstream system.
//!
//! # Integration Points
//!
//! 1. **Startup Configuration**: Load global settings at server initialization
//! 2. **Schema Policies**: Ingest validation rules and policy definitions
//! 3. **Runtime Refresh**: Optional hooks for live configuration updates

use llm_config_core::{ConfigManager, Environment, ConfigValue, Result as ConfigResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, debug};

// ============================================================================
// Configuration Traits
// ============================================================================

/// Trait for consuming configuration from an upstream config source
pub trait ConfigConsumer: Send + Sync {
    /// Load global configuration for the schema registry
    fn load_global_config(&self) -> Result<GlobalConfig, ConfigError>;

    /// Load schema validation policies
    fn load_schema_policies(&self) -> Result<SchemaPolicies, ConfigError>;

    /// Refresh configuration (for runtime updates)
    fn refresh(&self) -> Result<(), ConfigError>;
}

/// Trait for receiving configuration update notifications
pub trait ConfigUpdateListener: Send + Sync {
    /// Called when configuration is updated
    fn on_config_updated(&self, config: &GlobalConfig);

    /// Called when policies are updated
    fn on_policies_updated(&self, policies: &SchemaPolicies);
}

// ============================================================================
// Configuration Types
// ============================================================================

/// Global configuration for Schema Registry consumed from Config Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Storage configuration
    pub storage: StorageConfig,

    /// Validation configuration
    pub validation: ValidationConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            validation: ValidationConfig::default(),
            security: SecurityConfig::default(),
            metadata: HashMap::new(),
        }
    }
}

/// Server-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    pub host: String,

    /// Server port
    pub port: u16,

    /// Maximum request size in bytes
    pub max_request_size: usize,

    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_request_size: 10 * 1024 * 1024, // 10MB
            timeout_seconds: 30,
        }
    }
}

/// Storage-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database connection pool size
    pub pool_size: u32,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,

    /// Enable compression
    pub enable_compression: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            pool_size: 10,
            cache_ttl_seconds: 300,
            enable_compression: true,
        }
    }
}

/// Validation-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Maximum schema size in bytes
    pub max_schema_size: usize,

    /// Enable strict validation
    pub strict_mode: bool,

    /// Enable performance validation
    pub performance_checks: bool,

    /// Enable security validation
    pub security_checks: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_schema_size: 1024 * 1024, // 1MB
            strict_mode: false,
            performance_checks: true,
            security_checks: true,
        }
    }
}

/// Security-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub enable_auth: bool,

    /// Enable TLS
    pub enable_tls: bool,

    /// API rate limit (requests per second)
    pub rate_limit_rps: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_auth: false,
            enable_tls: false,
            rate_limit_rps: 100,
        }
    }
}

/// Schema validation policies consumed from Config Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaPolicies {
    /// Field naming policies
    pub field_naming: FieldNamingPolicy,

    /// Type restriction policies
    pub type_restrictions: Vec<String>,

    /// Required metadata fields
    pub required_metadata: Vec<String>,

    /// Custom validation rules
    pub custom_rules: Vec<CustomPolicyRule>,
}

impl Default for SchemaPolicies {
    fn default() -> Self {
        Self {
            field_naming: FieldNamingPolicy::default(),
            type_restrictions: Vec::new(),
            required_metadata: Vec::new(),
            custom_rules: Vec::new(),
        }
    }
}

/// Field naming policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldNamingPolicy {
    /// Naming convention: snake_case, camelCase, PascalCase
    pub convention: String,

    /// Enforce convention strictly
    pub enforce: bool,
}

impl Default for FieldNamingPolicy {
    fn default() -> Self {
        Self {
            convention: "snake_case".to_string(),
            enforce: false,
        }
    }
}

/// Custom policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPolicyRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// Pattern to match (regex)
    pub pattern: Option<String>,

    /// Whether this rule is mandatory
    pub mandatory: bool,
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during config consumption
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Config Manager error: {0}")]
    ConfigManager(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Configuration not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// ============================================================================
// Config Manager Adapter Implementation
// ============================================================================

/// Adapter for consuming configuration from LLM Config Manager
pub struct ConfigManagerAdapter {
    manager: Arc<ConfigManager>,
    environment: Environment,
    namespace: String,
}

impl ConfigManagerAdapter {
    /// Create a new adapter with the specified storage path
    ///
    /// # Arguments
    ///
    /// * `storage_path` - Path to the config storage directory
    /// * `environment` - The environment to use (Development, Production, etc.)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use schema_registry_core::config_manager_adapter::ConfigManagerAdapter;
    /// use llm_config_core::Environment;
    ///
    /// let adapter = ConfigManagerAdapter::new("./config", Environment::Development)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(storage_path: impl AsRef<Path>, environment: Environment) -> Result<Self, ConfigError> {
        let manager = ConfigManager::new(storage_path)
            .map_err(|e| ConfigError::ConfigManager(format!("{:?}", e)))?;

        info!("Initialized Config Manager adapter with environment: {:?}", environment);

        Ok(Self {
            manager: Arc::new(manager),
            environment,
            namespace: "schema-registry".to_string(),
        })
    }

    /// Create adapter with custom namespace
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Get the underlying config manager (for advanced usage)
    pub fn manager(&self) -> &Arc<ConfigManager> {
        &self.manager
    }

    /// Helper to get a config value from Config Manager
    fn get_config_value(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
        match self.manager.get_with_overrides(&self.namespace, key, self.environment.clone())? {
            Some(value) => Ok(Some(value)),
            None => {
                debug!("Config key '{}' not found, using default", key);
                Ok(None)
            }
        }
    }

    /// Parse config value as a specific type
    fn parse_value<T: for<'de> Deserialize<'de>>(&self, value: &ConfigValue) -> Result<T, ConfigError> {
        // ConfigValue should be serializable to JSON
        let json = serde_json::to_value(value)
            .map_err(|e| ConfigError::Serialization(e))?;

        serde_json::from_value(json)
            .map_err(|e| ConfigError::Serialization(e))
    }
}

impl ConfigConsumer for ConfigManagerAdapter {
    fn load_global_config(&self) -> Result<GlobalConfig, ConfigError> {
        info!("Loading global configuration from Config Manager");

        let mut config = GlobalConfig::default();

        // Attempt to load server config
        if let Ok(Some(value)) = self.get_config_value("server") {
            if let Ok(server_config) = self.parse_value::<ServerConfig>(&value) {
                config.server = server_config;
                debug!("Loaded server configuration from Config Manager");
            }
        }

        // Attempt to load storage config
        if let Ok(Some(value)) = self.get_config_value("storage") {
            if let Ok(storage_config) = self.parse_value::<StorageConfig>(&value) {
                config.storage = storage_config;
                debug!("Loaded storage configuration from Config Manager");
            }
        }

        // Attempt to load validation config
        if let Ok(Some(value)) = self.get_config_value("validation") {
            if let Ok(validation_config) = self.parse_value::<ValidationConfig>(&value) {
                config.validation = validation_config;
                debug!("Loaded validation configuration from Config Manager");
            }
        }

        // Attempt to load security config
        if let Ok(Some(value)) = self.get_config_value("security") {
            if let Ok(security_config) = self.parse_value::<SecurityConfig>(&value) {
                config.security = security_config;
                debug!("Loaded security configuration from Config Manager");
            }
        }

        info!("Global configuration loaded successfully");
        Ok(config)
    }

    fn load_schema_policies(&self) -> Result<SchemaPolicies, ConfigError> {
        info!("Loading schema policies from Config Manager");

        let mut policies = SchemaPolicies::default();

        // Attempt to load schema policies
        if let Ok(Some(value)) = self.get_config_value("policies/schema") {
            if let Ok(schema_policies) = self.parse_value::<SchemaPolicies>(&value) {
                policies = schema_policies;
                debug!("Loaded schema policies from Config Manager");
            }
        }

        // Load individual policy components if available
        if let Ok(Some(value)) = self.get_config_value("policies/field-naming") {
            if let Ok(field_naming) = self.parse_value::<FieldNamingPolicy>(&value) {
                policies.field_naming = field_naming;
                debug!("Loaded field naming policy from Config Manager");
            }
        }

        info!("Schema policies loaded successfully");
        Ok(policies)
    }

    fn refresh(&self) -> Result<(), ConfigError> {
        info!("Refreshing configuration from Config Manager");

        // In a production system, this would:
        // 1. Check for version changes in Config Manager
        // 2. Reload modified configurations
        // 3. Notify listeners of changes
        // 4. Apply new policies without restart

        // For now, we simply log the refresh attempt
        // The Config Manager supports version tracking and rollback
        // which enables safe runtime updates

        debug!("Configuration refresh completed");
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a default adapter for development environment
pub fn create_dev_adapter(storage_path: impl AsRef<Path>) -> Result<Arc<dyn ConfigConsumer>, ConfigError> {
    let adapter = ConfigManagerAdapter::new(storage_path, Environment::Development)?;
    Ok(Arc::new(adapter))
}

/// Create a default adapter for production environment
pub fn create_prod_adapter(storage_path: impl AsRef<Path>) -> Result<Arc<dyn ConfigConsumer>, ConfigError> {
    let adapter = ConfigManagerAdapter::new(storage_path, Environment::Production)?;
    Ok(Arc::new(adapter))
}

// ============================================================================
// Phase 2B: Schema Sources Configuration Adapter
// ============================================================================

/// Configuration for schema sources loaded from Config Manager
///
/// This defines external schema sources that can be consumed by the registry,
/// such as external registries, file systems, or cloud storage locations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSourcesConfig {
    /// List of configured schema sources
    pub sources: Vec<SchemaSource>,

    /// Default source to use when not specified
    pub default_source: Option<String>,

    /// Whether to enable source discovery
    pub enable_discovery: bool,
}

impl Default for SchemaSourcesConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            default_source: None,
            enable_discovery: false,
        }
    }
}

/// Individual schema source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSource {
    /// Unique identifier for this source
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Source type (file, http, s3, registry)
    pub source_type: SchemaSourceType,

    /// Connection/location URI
    pub uri: String,

    /// Authentication configuration (optional)
    pub auth: Option<SourceAuthConfig>,

    /// Polling interval for updates (in seconds, 0 = disabled)
    pub poll_interval_secs: u64,

    /// Priority for source resolution (lower = higher priority)
    pub priority: u32,

    /// Whether this source is enabled
    pub enabled: bool,
}

/// Type of schema source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaSourceType {
    /// Local file system
    File,
    /// HTTP/HTTPS endpoint
    Http,
    /// S3 bucket
    S3,
    /// External schema registry (Confluent, AWS Glue, etc.)
    Registry,
    /// Git repository
    Git,
}

/// Authentication configuration for schema sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAuthConfig {
    /// Authentication type
    pub auth_type: String,

    /// Credentials key reference (not the actual credentials)
    pub credentials_key: Option<String>,

    /// Additional auth parameters
    pub params: HashMap<String, String>,
}

// ============================================================================
// Phase 2B: Storage Paths Configuration Adapter
// ============================================================================

/// Configuration for schema storage paths loaded from Config Manager
///
/// This defines where schemas are stored, including primary, cache,
/// and archive locations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePathsConfig {
    /// Primary storage configuration
    pub primary: StoragePathEntry,

    /// Cache layer configuration (optional)
    pub cache: Option<StoragePathEntry>,

    /// Archive storage configuration (optional)
    pub archive: Option<StoragePathEntry>,

    /// Temporary/working directory
    pub temp_dir: Option<String>,

    /// Backup destination paths
    pub backup_paths: Vec<String>,

    /// Data migration paths
    pub migration_paths: MigrationPathsConfig,
}

impl Default for StoragePathsConfig {
    fn default() -> Self {
        Self {
            primary: StoragePathEntry::default(),
            cache: None,
            archive: None,
            temp_dir: None,
            backup_paths: Vec::new(),
            migration_paths: MigrationPathsConfig::default(),
        }
    }
}

/// Individual storage path entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePathEntry {
    /// Storage backend type
    pub backend: StorageBackendType,

    /// Connection string or path
    pub connection: String,

    /// Additional configuration parameters
    pub params: HashMap<String, String>,
}

impl Default for StoragePathEntry {
    fn default() -> Self {
        Self {
            backend: StorageBackendType::Postgres,
            connection: "postgresql://localhost:5432/schema_registry".to_string(),
            params: HashMap::new(),
        }
    }
}

/// Storage backend types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackendType {
    /// PostgreSQL database
    Postgres,
    /// Redis cache
    Redis,
    /// S3-compatible object storage
    S3,
    /// Local file system
    File,
}

/// Migration paths configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPathsConfig {
    /// Source path for migrations
    pub source: Option<String>,

    /// Destination path for migrations
    pub destination: Option<String>,

    /// Schema for migration scripts
    pub scripts_path: Option<String>,
}

impl Default for MigrationPathsConfig {
    fn default() -> Self {
        Self {
            source: None,
            destination: None,
            scripts_path: None,
        }
    }
}

// ============================================================================
// Phase 2B: Versioning Policies Configuration Adapter
// ============================================================================

/// Configuration for versioning policies loaded from Config Manager
///
/// This defines how schema versions are managed, including strategies,
/// retention, and compatibility requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersioningPoliciesConfig {
    /// Default versioning strategy
    pub default_strategy: VersioningStrategy,

    /// Version retention policy
    pub retention: VersionRetentionPolicy,

    /// Compatibility enforcement settings
    pub compatibility: CompatibilityEnforcementConfig,

    /// Prerelease version handling
    pub prerelease: PrereleaseConfig,

    /// Deprecation policy
    pub deprecation: DeprecationPolicy,
}

impl Default for VersioningPoliciesConfig {
    fn default() -> Self {
        Self {
            default_strategy: VersioningStrategy::Semantic,
            retention: VersionRetentionPolicy::default(),
            compatibility: CompatibilityEnforcementConfig::default(),
            prerelease: PrereleaseConfig::default(),
            deprecation: DeprecationPolicy::default(),
        }
    }
}

/// Versioning strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VersioningStrategy {
    /// Semantic versioning (MAJOR.MINOR.PATCH)
    Semantic,
    /// Auto-increment (simple integer versions)
    AutoIncrement,
    /// Timestamp-based versioning
    Timestamp,
    /// Hash-based versioning (content-addressable)
    ContentHash,
}

/// Version retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRetentionPolicy {
    /// Maximum number of versions to retain per schema
    pub max_versions: Option<u32>,

    /// Maximum age of versions to retain (in days)
    pub max_age_days: Option<u32>,

    /// Always retain the latest N versions regardless of age
    pub keep_latest: u32,

    /// Retain versions that are currently in use
    pub retain_in_use: bool,
}

impl Default for VersionRetentionPolicy {
    fn default() -> Self {
        Self {
            max_versions: None,
            max_age_days: None,
            keep_latest: 5,
            retain_in_use: true,
        }
    }
}

/// Compatibility enforcement configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityEnforcementConfig {
    /// Default compatibility mode
    pub default_mode: String,

    /// Enforce compatibility checks before registration
    pub enforce_on_register: bool,

    /// Allow compatibility mode overrides per schema
    pub allow_overrides: bool,

    /// Compatibility modes allowed for override
    pub allowed_modes: Vec<String>,
}

impl Default for CompatibilityEnforcementConfig {
    fn default() -> Self {
        Self {
            default_mode: "backward".to_string(),
            enforce_on_register: true,
            allow_overrides: false,
            allowed_modes: vec![
                "backward".to_string(),
                "forward".to_string(),
                "full".to_string(),
                "none".to_string(),
            ],
        }
    }
}

/// Prerelease version configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrereleaseConfig {
    /// Allow prerelease versions
    pub allow_prerelease: bool,

    /// Prerelease suffixes allowed (e.g., alpha, beta, rc)
    pub allowed_suffixes: Vec<String>,

    /// Auto-promote prereleases after duration (in days, 0 = disabled)
    pub auto_promote_days: u32,
}

impl Default for PrereleaseConfig {
    fn default() -> Self {
        Self {
            allow_prerelease: true,
            allowed_suffixes: vec![
                "alpha".to_string(),
                "beta".to_string(),
                "rc".to_string(),
            ],
            auto_promote_days: 0,
        }
    }
}

/// Deprecation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationPolicy {
    /// Deprecation notice period (in days)
    pub notice_period_days: u32,

    /// Allow immediate deprecation without notice
    pub allow_immediate: bool,

    /// Auto-archive deprecated versions after days
    pub auto_archive_days: Option<u32>,
}

impl Default for DeprecationPolicy {
    fn default() -> Self {
        Self {
            notice_period_days: 30,
            allow_immediate: false,
            auto_archive_days: Some(90),
        }
    }
}

// ============================================================================
// Phase 2B: Validation Settings Configuration Adapter
// ============================================================================

/// Comprehensive validation settings loaded from Config Manager
///
/// This extends the basic ValidationConfig with additional settings
/// for LLM-specific validation, custom rules, and advanced options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSettingsConfig {
    /// Core validation settings
    pub core: ValidationConfig,

    /// LLM-specific validation settings
    pub llm: LlmValidationSettings,

    /// Custom rule configuration
    pub custom_rules: CustomRulesConfig,

    /// Performance thresholds
    pub performance: PerformanceThresholds,

    /// Result filtering and reporting
    pub reporting: ValidationReportingConfig,
}

impl Default for ValidationSettingsConfig {
    fn default() -> Self {
        Self {
            core: ValidationConfig::default(),
            llm: LlmValidationSettings::default(),
            custom_rules: CustomRulesConfig::default(),
            performance: PerformanceThresholds::default(),
            reporting: ValidationReportingConfig::default(),
        }
    }
}

/// LLM-specific validation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmValidationSettings {
    /// Enable LLM-specific validation rules
    pub enabled: bool,

    /// Require descriptions for all fields
    pub require_descriptions: bool,

    /// Require examples for schemas
    pub require_examples: bool,

    /// Minimum description length
    pub min_description_length: usize,

    /// Maximum token count estimate for schemas
    pub max_token_estimate: Option<usize>,

    /// Validate field names for LLM friendliness
    pub validate_field_names: bool,
}

impl Default for LlmValidationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            require_descriptions: false,
            require_examples: false,
            min_description_length: 10,
            max_token_estimate: None,
            validate_field_names: true,
        }
    }
}

/// Custom validation rules configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRulesConfig {
    /// Enable custom rule execution
    pub enabled: bool,

    /// Path to custom rule definitions
    pub rules_path: Option<String>,

    /// Inline rule definitions
    pub inline_rules: Vec<CustomPolicyRule>,

    /// Maximum execution time per rule (in milliseconds)
    pub max_execution_ms: u64,
}

impl Default for CustomRulesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules_path: None,
            inline_rules: Vec::new(),
            max_execution_ms: 1000,
        }
    }
}

/// Performance validation thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Maximum nesting depth
    pub max_depth: usize,

    /// Maximum number of properties/fields
    pub max_properties: usize,

    /// Maximum array items constraint
    pub max_array_items: Option<usize>,

    /// Maximum regex complexity score
    pub max_regex_complexity: usize,

    /// Warn on potential performance issues
    pub warn_on_issues: bool,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_depth: 50,
            max_properties: 500,
            max_array_items: Some(10000),
            max_regex_complexity: 100,
            warn_on_issues: true,
        }
    }
}

/// Validation reporting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReportingConfig {
    /// Include warnings in results
    pub include_warnings: bool,

    /// Include info-level messages
    pub include_info: bool,

    /// Maximum issues to report
    pub max_issues: usize,

    /// Group issues by category
    pub group_by_category: bool,

    /// Include suggested fixes
    pub include_suggestions: bool,
}

impl Default for ValidationReportingConfig {
    fn default() -> Self {
        Self {
            include_warnings: true,
            include_info: false,
            max_issues: 100,
            group_by_category: true,
            include_suggestions: true,
        }
    }
}

// ============================================================================
// Phase 2B: Extended Config Consumer Trait
// ============================================================================

/// Extended trait for consuming Phase 2B configuration from Config Manager
///
/// This trait extends the base ConfigConsumer with additional methods
/// for loading schema sources, storage paths, versioning policies,
/// and comprehensive validation settings.
pub trait ConfigConsumerExt: ConfigConsumer {
    /// Load schema sources configuration
    fn load_schema_sources(&self) -> Result<SchemaSourcesConfig, ConfigError>;

    /// Load storage paths configuration
    fn load_storage_paths(&self) -> Result<StoragePathsConfig, ConfigError>;

    /// Load versioning policies configuration
    fn load_versioning_policies(&self) -> Result<VersioningPoliciesConfig, ConfigError>;

    /// Load comprehensive validation settings
    fn load_validation_settings(&self) -> Result<ValidationSettingsConfig, ConfigError>;
}

impl ConfigConsumerExt for ConfigManagerAdapter {
    fn load_schema_sources(&self) -> Result<SchemaSourcesConfig, ConfigError> {
        info!("Loading schema sources configuration from Config Manager");

        if let Ok(Some(value)) = self.get_config_value("schema-sources") {
            if let Ok(config) = self.parse_value::<SchemaSourcesConfig>(&value) {
                debug!("Loaded schema sources configuration from Config Manager");
                return Ok(config);
            }
        }

        debug!("Using default schema sources configuration");
        Ok(SchemaSourcesConfig::default())
    }

    fn load_storage_paths(&self) -> Result<StoragePathsConfig, ConfigError> {
        info!("Loading storage paths configuration from Config Manager");

        if let Ok(Some(value)) = self.get_config_value("storage-paths") {
            if let Ok(config) = self.parse_value::<StoragePathsConfig>(&value) {
                debug!("Loaded storage paths configuration from Config Manager");
                return Ok(config);
            }
        }

        debug!("Using default storage paths configuration");
        Ok(StoragePathsConfig::default())
    }

    fn load_versioning_policies(&self) -> Result<VersioningPoliciesConfig, ConfigError> {
        info!("Loading versioning policies configuration from Config Manager");

        if let Ok(Some(value)) = self.get_config_value("versioning-policies") {
            if let Ok(config) = self.parse_value::<VersioningPoliciesConfig>(&value) {
                debug!("Loaded versioning policies configuration from Config Manager");
                return Ok(config);
            }
        }

        debug!("Using default versioning policies configuration");
        Ok(VersioningPoliciesConfig::default())
    }

    fn load_validation_settings(&self) -> Result<ValidationSettingsConfig, ConfigError> {
        info!("Loading validation settings configuration from Config Manager");

        if let Ok(Some(value)) = self.get_config_value("validation-settings") {
            if let Ok(config) = self.parse_value::<ValidationSettingsConfig>(&value) {
                debug!("Loaded validation settings configuration from Config Manager");
                return Ok(config);
            }
        }

        debug!("Using default validation settings configuration");
        Ok(ValidationSettingsConfig::default())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configs() {
        let global = GlobalConfig::default();
        assert_eq!(global.server.port, 8080);
        assert_eq!(global.storage.pool_size, 10);
        assert!(global.validation.performance_checks);

        let policies = SchemaPolicies::default();
        assert_eq!(policies.field_naming.convention, "snake_case");
    }

    #[test]
    fn test_server_config_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_request_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_validation_config_defaults() {
        let config = ValidationConfig::default();
        assert_eq!(config.max_schema_size, 1024 * 1024);
        assert!(config.performance_checks);
        assert!(config.security_checks);
    }

    #[test]
    fn test_schema_sources_config_defaults() {
        let config = SchemaSourcesConfig::default();
        assert!(config.sources.is_empty());
        assert!(!config.enable_discovery);
    }

    #[test]
    fn test_storage_paths_config_defaults() {
        let config = StoragePathsConfig::default();
        assert_eq!(config.primary.backend, StorageBackendType::Postgres);
        assert!(config.cache.is_none());
        assert!(config.archive.is_none());
    }

    #[test]
    fn test_versioning_policies_config_defaults() {
        let config = VersioningPoliciesConfig::default();
        assert_eq!(config.default_strategy, VersioningStrategy::Semantic);
        assert!(config.compatibility.enforce_on_register);
        assert_eq!(config.retention.keep_latest, 5);
    }

    #[test]
    fn test_validation_settings_config_defaults() {
        let config = ValidationSettingsConfig::default();
        assert!(config.llm.enabled);
        assert!(config.custom_rules.enabled);
        assert!(config.reporting.include_warnings);
    }

    #[test]
    fn test_schema_source_type_serialization() {
        let source_type = SchemaSourceType::Http;
        let json = serde_json::to_string(&source_type).unwrap();
        assert_eq!(json, "\"http\"");

        let parsed: SchemaSourceType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SchemaSourceType::Http);
    }

    #[test]
    fn test_versioning_strategy_serialization() {
        let strategy = VersioningStrategy::Semantic;
        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(json, "\"semantic\"");

        let parsed: VersioningStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, VersioningStrategy::Semantic);
    }
}
