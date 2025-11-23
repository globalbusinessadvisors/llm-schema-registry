//! Configuration management for the CLI

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::{CliError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub registry_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub retry_attempts: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            registry_url: "http://localhost:8080".to_string(),
            api_key: None,
            timeout_seconds: 30,
            retry_attempts: 3,
        }
    }
}

pub fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| CliError::ConfigError("Could not determine config directory".to_string()))?;

    let schema_config_dir = config_dir.join("schema-registry");
    Ok(schema_config_dir.join("config.yaml"))
}

pub fn load_config(path: Option<&str>) -> Result<Config> {
    let config_file = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        match config_path() {
            Ok(p) if p.exists() => p,
            _ => return Ok(Config::default()),
        }
    };

    let contents = fs::read_to_string(&config_file)
        .map_err(|e| CliError::ConfigError(format!("Failed to read config file: {}", e)))?;

    serde_yaml::from_str(&contents)
        .map_err(|e| CliError::ConfigError(format!("Failed to parse config: {}", e)))
}

pub fn init_config(url: &str, force: bool) -> Result<()> {
    let config_file = config_path()?;

    if config_file.exists() && !force {
        return Err(CliError::ConfigError(
            format!("Config file already exists at {}. Use --force to overwrite.", config_file.display())
        ));
    }

    // Create parent directory if it doesn't exist
    if let Some(parent) = config_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| CliError::ConfigError(format!("Failed to create config directory: {}", e)))?;
    }

    let config = Config {
        registry_url: url.to_string(),
        ..Config::default()
    };

    let yaml = serde_yaml::to_string(&config)
        .map_err(|e| CliError::ConfigError(format!("Failed to serialize config: {}", e)))?;

    fs::write(&config_file, yaml)
        .map_err(|e| CliError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}
