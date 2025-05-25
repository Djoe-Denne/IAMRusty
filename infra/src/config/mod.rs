//! Configuration module for the application
//! 
//! This module provides a flexible configuration system that supports:
//! - TOML configuration files
//! - Environment variables
//! - Hierarchical environment variables
//!
//! The configuration is loaded with the following precedence (highest to lowest):
//! 1. Environment variables
//! 2. `.env` file
//! 3. Config file specified by CONFIG_FILE environment variable
//! 4. Default config file (config.toml)

use std::path::Path;
use std::fs;
use std::env;
use dotenvy::dotenv;
use config::{Config, ConfigError, Environment, File, FileFormat};
use tracing::{info, debug, warn};

// Re-export all config types from application layer (single source of truth)
pub use application::config::{
    AppConfig, 
    ServerConfig, 
    DatabaseConfig, 
    JwtConfig, 
    OAuthConfig, 
    GitHubConfig, 
    GitLabConfig,
    LoggingConfig
};

// Type aliases for backward compatibility
pub type GithubConfig = GitHubConfig;
pub type GitlabConfig = GitLabConfig;

/// Generic provider configuration for conversion utilities
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Redirect URI
    pub redirect_uri: String,
}

impl From<&GitHubConfig> for ProviderConfig {
    fn from(config: &GitHubConfig) -> Self {
        ProviderConfig {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: config.redirect_uri.clone(),
        }
    }
}

impl From<&GitLabConfig> for ProviderConfig {
    fn from(config: &GitLabConfig) -> Self {
        ProviderConfig {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: config.redirect_uri.clone(),
        }
    }
}

/// Load configuration from environment and config files
pub fn load_config() -> Result<AppConfig, ConfigError> {
    // Load .env file if it exists
    let _ = dotenv().ok();
    
    // Get environment
    let env = env::var("RUN_ENV").unwrap_or_else(|_| "development".to_string());
    
    // Determine the configuration file to use
    let config_file = env::var("CONFIG_FILE").unwrap_or_else(|_| "config.toml".to_string());
    let config_path = Path::new(&config_file);
    
    info!("Loading configuration from {:?} (environment: {})", config_path, env);
    
    // Start building the configuration
    let mut cfg_builder = Config::builder();
    
    // Try loading from config directory (legacy format)
    let default_config_path = Path::new("config/default.toml");
    if default_config_path.exists() {
        debug!("Loading default configuration from {:?}", default_config_path);
        cfg_builder = cfg_builder.add_source(File::new("config/default.toml", FileFormat::Toml));
        
        // Add environment-specific configuration if it exists
        let env_config_path = format!("config/{}.toml", env);
        if Path::new(&env_config_path).exists() {
            debug!("Loading environment configuration from {}", env_config_path);
            cfg_builder = cfg_builder.add_source(File::new(&env_config_path, FileFormat::Toml));
        }
    } else if config_path.exists() {
        // Use specified config file
        debug!("Loading configuration from {:?}", config_path);
        cfg_builder = cfg_builder.add_source(File::from(config_path).format(FileFormat::Toml));
    } else {
        warn!("No configuration file found, using environment variables only");
    }
    
    // Add environment variables with IAM prefix
    cfg_builder = cfg_builder.add_source(
        Environment::with_prefix("IAM")
            .prefix_separator("_")
            .separator("__")
    );
    
    // Build and parse the configuration
    let config = cfg_builder.build()?;
    
    debug!("Configuration loaded successfully");
    
    // Deserialize into our configuration struct
    config.try_deserialize()
}

/// Generate a default configuration as a TOML string
pub fn generate_default_config() -> Result<String, ConfigError> {
    let default_config = AppConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            tls_enabled: false,
            tls_cert_path: "./certs/cert.pem".to_string(),
            tls_key_path: "./certs/key.pem".to_string(),
            tls_port: 8443,
        },
        oauth: OAuthConfig {
            github: GitHubConfig {
                client_id: "your_github_client_id".to_string(),
                client_secret: "your_github_client_secret".to_string(),
                redirect_uri: "http://localhost:8080/api/auth/github/callback".to_string(),
                auth_url: "https://github.com/login/oauth/authorize".to_string(),
                token_url: "https://github.com/login/oauth/access_token".to_string(),
                user_url: "https://api.github.com/user".to_string(),
            },
            gitlab: GitLabConfig {
                client_id: "your_gitlab_client_id".to_string(),
                client_secret: "your_gitlab_client_secret".to_string(),
                redirect_uri: "http://localhost:8080/api/auth/gitlab/callback".to_string(),
                auth_url: "https://gitlab.com/oauth/authorize".to_string(),
                token_url: "https://gitlab.com/oauth/token".to_string(),
                user_url: "https://gitlab.com/api/v4/user".to_string(),
            },
        },
        jwt: JwtConfig {
            secret: "your_secret_key_here_change_in_production".to_string(),
            expiration_seconds: 3600,
        },
        database: DatabaseConfig {
            url: "postgresql://username:password@localhost/iam_db".to_string(),
            read_replicas: vec![],
        },
        logging: LoggingConfig {
            level: "info".to_string(),
        },
    };
    
    toml::to_string_pretty(&default_config)
        .map_err(|e| ConfigError::Message(format!("Failed to serialize default config: {}", e)))
}

/// Write a default configuration file
pub fn write_default_config(path: &str) -> Result<(), ConfigError> {
    let config_content = generate_default_config()?;
    fs::write(path, config_content)
        .map_err(|e| ConfigError::Message(format!("Failed to write config file: {}", e)))?;
    
    info!("Default configuration written to {}", path);
    Ok(())
} 