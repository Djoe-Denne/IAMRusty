//! IAM service specific configuration
//!
//! This crate provides IAM-specific configuration structures including OAuth and JWT
//! configuration, while re-exporting core configuration utilities from rustycog-config.

// Re-export core configuration from rustycog-config
pub use rustycog_config::{
    ServerConfig, SetupServerConfig, DatabaseConfig, DatabaseCredentials, LoggingConfig,
    CommandConfig, CommandRetryConfig, KafkaConfig, setup_logging,
    clear_all_caches, ConfigError, load_config_with_cache, load_config_fresh, generate_default_config_toml
};

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, OnceLock};
use rustycog_config::{ConfigCache, ConfigLoader};
use tracing::debug;

/// OAuth configuration containing provider-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// GitHub OAuth configuration
    pub github: GitHubConfig,
    /// GitLab OAuth configuration
    pub gitlab: GitLabConfig,
}

/// GitHub OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// GitHub OAuth client ID
    pub client_id: String,
    /// GitHub OAuth client secret
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// GitHub authorization URL
    #[serde(default = "default_github_auth_url")]
    pub auth_url: String,
    /// GitHub token exchange URL
    #[serde(default = "default_github_token_url")]
    pub token_url: String,
    /// GitHub user info API URL
    #[serde(default = "default_github_user_url")]
    pub user_url: String,
}

/// GitLab OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabConfig {
    /// GitLab OAuth client ID
    pub client_id: String,
    /// GitLab OAuth client secret
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// GitLab authorization URL
    #[serde(default = "default_gitlab_auth_url")]
    pub auth_url: String,
    /// GitLab token exchange URL
    #[serde(default = "default_gitlab_token_url")]
    pub token_url: String,
    /// GitLab user info API URL
    #[serde(default = "default_gitlab_user_url")]
    pub user_url: String,
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT signing secret
    pub secret: String,
    /// Token expiration time in seconds
    #[serde(default = "default_jwt_expiration")]
    pub expiration_seconds: u64,
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// OAuth provider configurations
    pub oauth: OAuthConfig,
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Command configuration
    pub command: CommandConfig,
    /// Kafka configuration
    pub kafka: KafkaConfig,
}

// Default value functions
fn default_github_auth_url() -> String {
    "https://github.com/login/oauth/authorize".to_string()
}

fn default_github_token_url() -> String {
    "https://github.com/login/oauth/access_token".to_string()
}

fn default_github_user_url() -> String {
    "https://api.github.com/user".to_string()
}

fn default_gitlab_auth_url() -> String {
    "https://gitlab.com/oauth/authorize".to_string()
}

fn default_gitlab_token_url() -> String {
    "https://gitlab.com/oauth/token".to_string()
}

fn default_gitlab_user_url() -> String {
    "https://gitlab.com/api/v4/user".to_string()
}

fn default_jwt_expiration() -> u64 {
    3600 // 1 hour
}

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

// Type aliases for backward compatibility
pub type GithubConfig = GitHubConfig;
pub type GitlabConfig = GitLabConfig;

/// Global configuration cache
static CONFIG_CACHE: OnceLock<Arc<Mutex<Option<AppConfig>>>> = OnceLock::new();

/// Configuration cache implementation for AppConfig
pub struct AppConfigCache;

impl ConfigCache<AppConfig> for AppConfigCache {
    fn get_cached() -> Option<AppConfig> {
        let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
        let cached_config = cache.lock().unwrap();
        cached_config.clone()
    }
    
    fn set_cached(config: AppConfig) {
        let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
        let mut cached_config = cache.lock().unwrap();
        *cached_config = Some(config);
    }
    
    fn clear_cached() {
        let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
        let mut cached_config = cache.lock().unwrap();
        *cached_config = None;
        debug!("AppConfig cache cleared");
    }
}

/// Configuration loader implementation for AppConfig
impl ConfigLoader<AppConfig> for AppConfig {
    fn create_default() -> AppConfig {
        AppConfig {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            oauth: OAuthConfig {
                github: GitHubConfig {
                    client_id: "YOUR_GITHUB_CLIENT_ID".to_string(),
                    client_secret: "YOUR_GITHUB_CLIENT_SECRET".to_string(),
                    redirect_uri: "http://localhost:8080/auth/github/callback".to_string(),
                    auth_url: default_github_auth_url(),
                    token_url: default_github_token_url(),
                    user_url: default_github_user_url(),
                },
                gitlab: GitLabConfig {
                    client_id: "YOUR_GITLAB_CLIENT_ID".to_string(),
                    client_secret: "YOUR_GITLAB_CLIENT_SECRET".to_string(),
                    redirect_uri: "http://localhost:8080/auth/gitlab/callback".to_string(),
                    auth_url: default_gitlab_auth_url(),
                    token_url: default_gitlab_token_url(),
                    user_url: default_gitlab_user_url(),
                },
            },
            jwt: JwtConfig {
                secret: "your-256-bit-secret-key-change-this-in-production".to_string(),
                expiration_seconds: default_jwt_expiration(),
            },
            logging: LoggingConfig::default(),
            command: CommandConfig::default(),
            kafka: KafkaConfig::default(),
        }
    }
    
    fn config_prefix() -> &'static str {
        "IAM"
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::create_default()
    }
}

/// Load configuration from environment and config files
/// This function caches the configuration to ensure consistent behavior,
/// especially for random port generation in database configuration.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    load_config_with_cache::<AppConfig, AppConfigCache>()
}

/// Clear the configuration cache
pub fn clear_config_cache() {
    AppConfigCache::clear_cached();
}

/// Generate a default configuration file in TOML format
pub fn generate_default_config() -> Result<String, ConfigError> {
    generate_default_config_toml::<AppConfig>()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 0);
        assert_eq!(config.jwt.expiration_seconds, 3600);
    }
    
    #[test]
    fn test_config_cache() {
        // Clear any existing cache
        AppConfigCache::clear_cached();
        
        // Should return None initially
        assert!(AppConfigCache::get_cached().is_none());
        
        // Set a config
        let config = AppConfig::default();
        AppConfigCache::set_cached(config.clone());
        
        // Should return the cached config
        let cached = AppConfigCache::get_cached().unwrap();
        assert_eq!(cached.server.host, config.server.host);
        
        // Clear cache
        AppConfigCache::clear_cached();
        assert!(AppConfigCache::get_cached().is_none());
    }
    
    #[test]
    fn test_generate_default_config() {
        let toml_config = generate_default_config().expect("Should generate default config");
        assert!(toml_config.contains("[server]"));
        assert!(toml_config.contains("[database]"));
        assert!(toml_config.contains("[oauth.github]"));
        assert!(toml_config.contains("[jwt]"));
    }
    
    #[test]
    fn test_provider_config_conversion() {
        let github_config = GitHubConfig {
            client_id: "test_id".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
            auth_url: default_github_auth_url(),
            token_url: default_github_token_url(),
            user_url: default_github_user_url(),
        };
        
        let provider_config: ProviderConfig = (&github_config).into();
        assert_eq!(provider_config.client_id, "test_id");
        assert_eq!(provider_config.client_secret, "test_secret");
        assert_eq!(provider_config.redirect_uri, "http://localhost:8080/callback");
    }
} 