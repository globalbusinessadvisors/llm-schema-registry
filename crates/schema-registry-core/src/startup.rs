//! Server Startup Integration
//!
//! Provides startup utilities for initializing Schema Registry with
//! Config Manager integration. This module demonstrates the Phase 2B
//! runtime "consumes-from" pattern without modifying core logic.
//!
//! # Phase 2B Enhancements
//!
//! This module now supports loading additional configuration domains:
//! - Schema sources (external registries, file systems, cloud storage)
//! - Storage paths (primary, cache, archive configurations)
//! - Versioning policies (strategies, retention, compatibility)
//! - Validation settings (comprehensive validation configuration)

use crate::config_manager_adapter::{
    ConfigConsumer, ConfigConsumerExt, ConfigManagerAdapter, GlobalConfig, SchemaPolicies, ConfigError,
    SchemaSourcesConfig, StoragePathsConfig, VersioningPoliciesConfig, ValidationSettingsConfig,
};
use llm_config_core::Environment;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, debug};

/// Startup configuration for Schema Registry
#[derive(Debug, Clone)]
pub struct StartupConfig {
    /// Path to config storage
    pub config_storage_path: PathBuf,

    /// Environment (Development, Staging, Production)
    pub environment: Environment,

    /// Whether to fail if config loading fails
    pub require_config: bool,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            config_storage_path: PathBuf::from("./config"),
            environment: Environment::Development,
            require_config: false,
        }
    }
}

/// Startup context containing loaded configuration and policies
///
/// This struct contains all configuration loaded from Config Manager,
/// including the Phase 2B extensions for schema sources, storage paths,
/// versioning policies, and validation settings.
#[derive(Clone)]
pub struct StartupContext {
    /// Global configuration loaded from Config Manager
    pub global_config: GlobalConfig,

    /// Schema validation policies
    pub schema_policies: SchemaPolicies,

    /// Config adapter for runtime refresh
    pub config_adapter: Option<Arc<dyn ConfigConsumer>>,

    // Phase 2B: Additional configuration domains

    /// Schema sources configuration (Phase 2B)
    pub schema_sources: SchemaSourcesConfig,

    /// Storage paths configuration (Phase 2B)
    pub storage_paths: StoragePathsConfig,

    /// Versioning policies configuration (Phase 2B)
    pub versioning_policies: VersioningPoliciesConfig,

    /// Validation settings configuration (Phase 2B)
    pub validation_settings: ValidationSettingsConfig,
}

impl Default for StartupContext {
    fn default() -> Self {
        Self {
            global_config: GlobalConfig::default(),
            schema_policies: SchemaPolicies::default(),
            config_adapter: None,
            schema_sources: SchemaSourcesConfig::default(),
            storage_paths: StoragePathsConfig::default(),
            versioning_policies: VersioningPoliciesConfig::default(),
            validation_settings: ValidationSettingsConfig::default(),
        }
    }
}

impl StartupContext {
    /// Refresh configuration from Config Manager
    pub fn refresh(&self) -> Result<(), ConfigError> {
        if let Some(adapter) = &self.config_adapter {
            adapter.refresh()?;
            info!("Configuration refreshed successfully");
        }
        Ok(())
    }
}

/// Initialize Schema Registry with Config Manager integration
///
/// This function performs Phase 2B runtime integration:
/// 1. Initializes Config Manager adapter
/// 2. Loads global configuration
/// 3. Ingests schema validation policies
/// 4. Prepares optional runtime refresh hooks
///
/// # Arguments
///
/// * `config` - Startup configuration
///
/// # Returns
///
/// A `StartupContext` containing loaded configuration and policies
///
/// # Example
///
/// ```no_run
/// use schema_registry_core::startup::{initialize_with_config_manager, StartupConfig};
/// use llm_config_core::Environment;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = StartupConfig {
///     config_storage_path: "./config".into(),
///     environment: Environment::Production,
///     require_config: false,
/// };
///
/// let context = initialize_with_config_manager(config).await?;
/// println!("Loaded config with max schema size: {}", context.global_config.validation.max_schema_size);
/// # Ok(())
/// # }
/// ```
pub async fn initialize_with_config_manager(
    config: StartupConfig,
) -> Result<StartupContext, ConfigError> {
    info!("Initializing Schema Registry with Config Manager integration");
    info!("Environment: {:?}, Config path: {:?}", config.environment, config.config_storage_path);

    // Create Config Manager adapter
    let adapter = match ConfigManagerAdapter::new(&config.config_storage_path, config.environment) {
        Ok(adapter) => {
            info!("Config Manager adapter initialized successfully");
            adapter
        }
        Err(e) => {
            if config.require_config {
                return Err(e);
            } else {
                warn!("Failed to initialize Config Manager, using defaults: {}", e);
                return Ok(StartupContext::default());
            }
        }
    };

    // Load global configuration
    let global_config = match adapter.load_global_config() {
        Ok(config) => {
            info!("Global configuration loaded from Config Manager");
            config
        }
        Err(e) => {
            if config.require_config {
                return Err(e);
            } else {
                warn!("Failed to load global config, using defaults: {}", e);
                GlobalConfig::default()
            }
        }
    };

    // Load schema validation policies
    let schema_policies = match adapter.load_schema_policies() {
        Ok(policies) => {
            info!("Schema policies loaded from Config Manager");
            policies
        }
        Err(e) => {
            if config.require_config {
                return Err(e);
            } else {
                warn!("Failed to load schema policies, using defaults: {}", e);
                SchemaPolicies::default()
            }
        }
    };

    // Phase 2B: Load schema sources configuration
    let schema_sources = match adapter.load_schema_sources() {
        Ok(sources) => {
            info!("Schema sources configuration loaded ({} sources)", sources.sources.len());
            sources
        }
        Err(e) => {
            debug!("Failed to load schema sources, using defaults: {}", e);
            SchemaSourcesConfig::default()
        }
    };

    // Phase 2B: Load storage paths configuration
    let storage_paths = match adapter.load_storage_paths() {
        Ok(paths) => {
            info!("Storage paths configuration loaded (primary: {:?})", paths.primary.backend);
            paths
        }
        Err(e) => {
            debug!("Failed to load storage paths, using defaults: {}", e);
            StoragePathsConfig::default()
        }
    };

    // Phase 2B: Load versioning policies configuration
    let versioning_policies = match adapter.load_versioning_policies() {
        Ok(policies) => {
            info!("Versioning policies loaded (strategy: {:?})", policies.default_strategy);
            policies
        }
        Err(e) => {
            debug!("Failed to load versioning policies, using defaults: {}", e);
            VersioningPoliciesConfig::default()
        }
    };

    // Phase 2B: Load validation settings configuration
    let validation_settings = match adapter.load_validation_settings() {
        Ok(settings) => {
            info!("Validation settings loaded (LLM validation: {})", settings.llm.enabled);
            settings
        }
        Err(e) => {
            debug!("Failed to load validation settings, using defaults: {}", e);
            ValidationSettingsConfig::default()
        }
    };

    info!("Schema Registry initialization complete (Phase 2B)");
    info!("Server will listen on {}:{}", global_config.server.host, global_config.server.port);
    info!("Validation: max_schema_size={} bytes, strict_mode={}",
          global_config.validation.max_schema_size,
          global_config.validation.strict_mode);
    info!("Phase 2B: schema_sources={}, versioning={:?}, llm_validation={}",
          schema_sources.sources.len(),
          versioning_policies.default_strategy,
          validation_settings.llm.enabled);

    Ok(StartupContext {
        global_config,
        schema_policies,
        config_adapter: Some(Arc::new(adapter)),
        schema_sources,
        storage_paths,
        versioning_policies,
        validation_settings,
    })
}

/// Quick initialization for development
pub async fn initialize_dev() -> Result<StartupContext, ConfigError> {
    initialize_with_config_manager(StartupConfig {
        environment: Environment::Development,
        require_config: false,
        ..Default::default()
    })
    .await
}

/// Quick initialization for production
pub async fn initialize_prod(config_path: PathBuf) -> Result<StartupContext, ConfigError> {
    initialize_with_config_manager(StartupConfig {
        config_storage_path: config_path,
        environment: Environment::Production,
        require_config: true,
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager_adapter::{VersioningStrategy, StorageBackendType};

    #[tokio::test]
    async fn test_startup_with_defaults() {
        let config = StartupConfig::default();
        assert_eq!(config.environment, Environment::Development);
        assert!(!config.require_config);
    }

    #[tokio::test]
    async fn test_startup_context_default() {
        let context = StartupContext::default();
        assert_eq!(context.global_config.server.port, 8080);
        assert_eq!(context.schema_policies.field_naming.convention, "snake_case");
    }

    #[tokio::test]
    async fn test_startup_context_phase_2b_defaults() {
        let context = StartupContext::default();

        // Verify Phase 2B schema sources defaults
        assert!(context.schema_sources.sources.is_empty());
        assert!(!context.schema_sources.enable_discovery);

        // Verify Phase 2B storage paths defaults
        assert_eq!(context.storage_paths.primary.backend, StorageBackendType::Postgres);
        assert!(context.storage_paths.cache.is_none());

        // Verify Phase 2B versioning policies defaults
        assert_eq!(context.versioning_policies.default_strategy, VersioningStrategy::Semantic);
        assert!(context.versioning_policies.compatibility.enforce_on_register);

        // Verify Phase 2B validation settings defaults
        assert!(context.validation_settings.llm.enabled);
        assert!(context.validation_settings.custom_rules.enabled);
    }

    #[test]
    fn test_startup_config_builder() {
        let config = StartupConfig {
            config_storage_path: PathBuf::from("/custom/path"),
            environment: Environment::Production,
            require_config: true,
        };

        assert_eq!(config.config_storage_path, PathBuf::from("/custom/path"));
        assert_eq!(config.environment, Environment::Production);
        assert!(config.require_config);
    }
}
