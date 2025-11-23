//! Error handling for the CLI

use colored::Colorize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("{0}")]
    Other(String),
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        CliError::SerializationError(e.to_string())
    }
}

impl From<serde_yaml::Error> for CliError {
    fn from(e: serde_yaml::Error) -> Self {
        CliError::SerializationError(e.to_string())
    }
}

impl From<anyhow::Error> for CliError {
    fn from(e: anyhow::Error) -> Self {
        CliError::Other(e.to_string())
    }
}

pub fn print_error(error: &CliError) {
    eprintln!("{} {}", "Error:".red().bold(), error);

    // Print additional context for certain error types
    match error {
        CliError::ConfigError(_) => {
            eprintln!("\n{}", "Hint:".yellow().bold());
            eprintln!("  Run 'schema-cli init --url <URL>' to initialize configuration");
        }
        CliError::ApiError(_) => {
            eprintln!("\n{}", "Hint:".yellow().bold());
            eprintln!("  Check that the registry URL is correct and the server is running");
        }
        _ => {}
    }
}
