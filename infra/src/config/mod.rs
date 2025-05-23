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
use serde::{Deserialize, Serialize};
use tracing::{info, debug, warn};

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Whether TLS/HTTPS is enabled
    #[serde(default)]
    pub tls_enabled: bool,
    /// Path to TLS certificate file
    #[serde(default = "default_cert_path")]
    pub tls_cert_path: String,
    /// Path to TLS private key file
    #[serde(default = "default_key_path")]
    pub tls_key_path: String,
    /// Port to use when TLS is enabled
    #[serde(default = "default_tls_port")]
    pub tls_port: u16,
}

fn default_cert_path() -> String {
    "./certs/cert.pem".to_string()
}

fn default_key_path() -> String {
    "./certs/key.pem".to_string()
}

fn default_tls_port() -> u16 {
    8443
}

/// Generic provider configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Redirect URI
    pub redirect_uri: String,
}

/// GitHub OAuth2 configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubConfig {
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Redirect URI
    pub redirect_uri: String,
}

impl From<&GithubConfig> for ProviderConfig {
    fn from(config: &GithubConfig) -> Self {
        ProviderConfig {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: config.redirect_uri.clone(),
        }
    }
}

/// GitLab OAuth2 configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitlabConfig {
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Redirect URI
    pub redirect_uri: String,
}

impl From<&GitlabConfig> for ProviderConfig {
    fn from(config: &GitlabConfig) -> Self {
        ProviderConfig {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: config.redirect_uri.clone(),
        }
    }
}

/// OAuth configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthConfig {
    /// GitHub OAuth2 configuration
    pub github: GithubConfig,
    /// GitLab OAuth2 configuration
    pub gitlab: GitlabConfig,
}

/// JWT configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    /// JWT secret key
    pub secret: String,
    /// JWT token expiration in seconds
    pub expiration_seconds: u64,
}

/// Database configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// PostgreSQL database URL
    pub url: String,
    /// Read replica database URLs
    #[serde(default)]
    pub read_replicas: Vec<String>,
}

/// Application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// OAuth configuration
    pub oauth: OAuthConfig,
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Database configuration
    pub database: DatabaseConfig,
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
        let env_config_filename = format!("config/{}.toml", env);
        let env_config_path = Path::new(&env_config_filename);
        if env_config_path.exists() {
            debug!("Loading environment configuration from {:?}", env_config_path);
            cfg_builder = cfg_builder.add_source(File::new(&env_config_filename, FileFormat::Toml));
        }
    }
    
    // Add new configuration file if it exists (higher precedence)
    if config_path.exists() {
        debug!("Loading configuration from file: {:?}", config_path);
        cfg_builder = cfg_builder.add_source(File::new(&config_file, FileFormat::Toml));
    } else if !default_config_path.exists() {
        // Only warn if neither config format is found
        warn!("Configuration file {:?} not found, using environment variables only", config_path);
    }
    
    // Add environment variables with flattened format:
    // APP_SERVER_HOST -> server.host
    cfg_builder = cfg_builder.add_source(
        Environment::with_prefix("APP")
            .separator("_")
            .prefix_separator("_")
    );
    
    // Also support legacy environment variable format for backward compatibility
    cfg_builder = cfg_builder.add_source(
        Environment::with_prefix("IAM")
            .separator("__")
    );
    
    // Map some specific environment variables for compatibility
    if let Ok(url) = env::var("DATABASE_URL") {
        cfg_builder = cfg_builder.set_override("database.url", url)?;
    }
    
    // Map GitHub OAuth2 variables
    if let Ok(val) = env::var("GITHUB_CLIENT_ID") {
        cfg_builder = cfg_builder.set_override("oauth.github.client_id", val)?;
    }
    if let Ok(val) = env::var("GITHUB_CLIENT_SECRET") {
        cfg_builder = cfg_builder.set_override("oauth.github.client_secret", val)?;
    }
    if let Ok(val) = env::var("GITHUB_REDIRECT_URL") {
        cfg_builder = cfg_builder.set_override("oauth.github.redirect_uri", val)?;
    }
    
    // Map GitLab OAuth2 variables
    if let Ok(val) = env::var("GITLAB_CLIENT_ID") {
        cfg_builder = cfg_builder.set_override("oauth.gitlab.client_id", val)?;
    }
    if let Ok(val) = env::var("GITLAB_CLIENT_SECRET") {
        cfg_builder = cfg_builder.set_override("oauth.gitlab.client_secret", val)?;
    }
    if let Ok(val) = env::var("GITLAB_REDIRECT_URL") {
        cfg_builder = cfg_builder.set_override("oauth.gitlab.redirect_uri", val)?;
    }
    
    // Map server variables
    if let Ok(val) = env::var("SERVER_HOST") {
        cfg_builder = cfg_builder.set_override("server.host", val)?;
    }
    if let Ok(val) = env::var("SERVER_PORT") {
        cfg_builder = cfg_builder.set_override("server.port", val)?;
    }
    
    // Build and convert to AppConfig
    let config = cfg_builder.build()?;
    let app_config: AppConfig = config.try_deserialize()?;
    
    Ok(app_config)
}

/// Generate a default configuration file
pub fn generate_default_config() -> Result<String, ConfigError> {
    let default_config = AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            tls_enabled: false,
            tls_cert_path: default_cert_path(),
            tls_key_path: default_key_path(),
            tls_port: default_tls_port(),
        },
        oauth: OAuthConfig {
            github: GithubConfig {
                client_id: "your-github-client-id".to_string(),
                client_secret: "your-github-client-secret".to_string(),
                redirect_uri: "http://localhost:8080/auth/github/callback".to_string(),
            },
            gitlab: GitlabConfig {
                client_id: "your-gitlab-client-id".to_string(),
                client_secret: "your-gitlab-client-secret".to_string(),
                redirect_uri: "http://localhost:8080/auth/gitlab/callback".to_string(),
            },
        },
        jwt: JwtConfig {
            secret: "your-jwt-secret-key-should-be-at-least-32-bytes".to_string(),
            expiration_seconds: 3600,
        },
        database: DatabaseConfig {
            url: "postgres://postgres:postgres@localhost:5432/iam".to_string(),
            read_replicas: vec![],
        },
    };
    
    // Serialize to TOML
    let toml = toml::to_string(&default_config)
        .map_err(|e| ConfigError::Message(format!("Failed to serialize default config: {}", e)))?;
    
    Ok(toml)
}

/// Write default configuration to file
pub fn write_default_config(path: &str) -> Result<(), ConfigError> {
    let toml = generate_default_config()?;
    fs::write(path, toml)
        .map_err(|e| ConfigError::Message(format!("Failed to write default config to {}: {}", path, e)))?;
    Ok(())
} 