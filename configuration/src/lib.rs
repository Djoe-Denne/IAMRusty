//! IAM service specific configuration
//!
//! This crate provides IAM-specific configuration structures including OAuth and JWT
//! configuration, while re-exporting core configuration utilities from rustycog-config.

// Re-export core configuration from rustycog-config
pub use rustycog_config::{
    ServerConfig, SetupServerConfig, DatabaseConfig, DatabaseCredentials, LoggingConfig,
    CommandConfig, CommandRetryConfig, KafkaConfig, setup_logging
};

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use std::env;
use std::sync::{Arc, Mutex, OnceLock};
use config::{Config, ConfigError, Environment, File, FileFormat};
use tracing::{info, debug, warn};

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret key
    pub secret: String,
    /// JWT token expiration in seconds
    pub expiration_seconds: u64,
}

/// GitHub OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    /// Authorization endpoint URL
    #[serde(default = "default_github_auth_url")]
    pub auth_url: String,
    /// Token exchange endpoint URL  
    #[serde(default = "default_github_token_url")]
    pub token_url: String,
    /// User info endpoint URL
    #[serde(default = "default_github_user_url")]
    pub user_url: String,
}

fn default_github_auth_url() -> String {
    "https://github.com/login/oauth/authorize".to_string()
}

fn default_github_token_url() -> String {
    "https://github.com/login/oauth/access_token".to_string()
}

fn default_github_user_url() -> String {
    "https://api.github.com/user".to_string()
}

/// GitLab OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    /// Authorization endpoint URL
    #[serde(default = "default_gitlab_auth_url")]
    pub auth_url: String,
    /// Token exchange endpoint URL  
    #[serde(default = "default_gitlab_token_url")]
    pub token_url: String,
    /// User info endpoint URL
    #[serde(default = "default_gitlab_user_url")]
    pub user_url: String,
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

/// OAuth configuration for all providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub github: GitHubConfig,
    pub gitlab: GitLabConfig,
}

/// IAM Application configuration combining all subsystem configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// OAuth configuration
    pub oauth: OAuthConfig,
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Command configuration
    #[serde(default)]
    pub command: CommandConfig,
    /// Kafka configuration
    #[serde(default)]
    pub kafka: KafkaConfig,
}

impl AppConfig {
    /// Create a new IAM AppConfig with IAM-specific Kafka defaults
    pub fn new_with_iam_defaults(
        server: ServerConfig,
        oauth: OAuthConfig,
        jwt: JwtConfig,
        database: DatabaseConfig,
    ) -> Self {
        let mut kafka = KafkaConfig::default();
        // Override with IAM-specific client ID
        kafka.client_id = "iam-service".to_string();
        
        Self {
            server,
            oauth,
            jwt,
            database,
            logging: LoggingConfig::default(),
            command: CommandConfig::default(),
            kafka,
        }
    }
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

/// Global configuration cache to ensure consistent random ports
static CONFIG_CACHE: OnceLock<Arc<Mutex<Option<AppConfig>>>> = OnceLock::new();

/// Load configuration from environment and config files
/// This function caches the configuration to ensure consistent behavior,
/// especially for random port generation in database configuration.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    let cache = CONFIG_CACHE.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut cached_config = cache.lock().unwrap();
    
    // Return cached config if available
    if let Some(ref config) = *cached_config {
        debug!("Returning cached configuration");
        return Ok(config.clone());
    }
    
    // Load fresh configuration
    let config = load_config_fresh()?;
    
    // Cache the configuration
    *cached_config = Some(config.clone());
    debug!("Configuration loaded and cached");
    
    Ok(config)
}

/// Internal function to load fresh configuration without caching
fn load_config_fresh() -> Result<AppConfig, ConfigError> {
    // Load .env file if it exists
    let _ = dotenvy::dotenv().ok();
    
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

/// Clear the configuration cache (useful for testing)
pub fn clear_config_cache() {
    if let Some(cache) = CONFIG_CACHE.get() {
        let mut cached_config = cache.lock().unwrap();
        *cached_config = None;
        debug!("Configuration cache cleared");
    }
}

/// Clear all configuration caches (useful for testing)
pub fn clear_all_caches() {
    clear_config_cache();
    DatabaseConfig::clear_port_cache();
    KafkaConfig::clear_port_cache();
    debug!("All configuration caches cleared");
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
        database: DatabaseConfig::new(
            "postgres".to_string(),
            "postgres".to_string(),
            "localhost".to_string(),
            5432,
            "iam".to_string(),
        ),
        oauth: OAuthConfig {
            github: GitHubConfig {
                client_id: "YOUR_GITHUB_CLIENT_ID".to_string(),
                client_secret: "YOUR_GITHUB_CLIENT_SECRET".to_string(),
                redirect_uri: "http://localhost:8080/auth/github/callback".to_string(),
                auth_url: "https://github.com/login/oauth/authorize".to_string(),
                token_url: "https://github.com/login/oauth/access_token".to_string(),
                user_url: "https://api.github.com/user".to_string(),
            },
            gitlab: GitLabConfig {
                client_id: "YOUR_GITLAB_CLIENT_ID".to_string(),
                client_secret: "YOUR_GITLAB_CLIENT_SECRET".to_string(),
                redirect_uri: "http://localhost:8080/auth/gitlab/callback".to_string(),
                auth_url: "https://gitlab.com/oauth/authorize".to_string(),
                token_url: "https://gitlab.com/oauth/token".to_string(),
                user_url: "https://gitlab.com/api/v4/user".to_string(),
            },
        },
        jwt: JwtConfig {
            secret: "your-256-bit-secret-key-change-this-in-production".to_string(),
            expiration_seconds: 3600,
        },
        logging: LoggingConfig::default(),
        command: CommandConfig::default(),
        kafka: KafkaConfig::default(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::*;
    use rstest::*;
    use serde_json;

    #[fixture]
    fn sample_github_config() -> GitHubConfig {
        GitHubConfig {
            client_id: "github_client_id".to_string(),
            client_secret: "github_client_secret".to_string(),
            redirect_uri: "http://localhost:8080/auth/github/callback".to_string(),
            auth_url: default_github_auth_url(),
            token_url: default_github_token_url(),
            user_url: default_github_user_url(),
        }
    }

    #[fixture]
    fn sample_gitlab_config() -> GitLabConfig {
        GitLabConfig {
            client_id: "gitlab_client_id".to_string(),
            client_secret: "gitlab_client_secret".to_string(),
            redirect_uri: "http://localhost:8080/auth/gitlab/callback".to_string(),
            auth_url: default_gitlab_auth_url(),
            token_url: default_gitlab_token_url(),
            user_url: default_gitlab_user_url(),
        }
    }

    #[fixture]
    fn sample_jwt_config() -> JwtConfig {
        JwtConfig {
            secret: "test_secret_key".to_string(),
            expiration_seconds: 3600,
        }
    }

    #[fixture]
    fn sample_server_config() -> ServerConfig {
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            tls_enabled: false,
            tls_cert_path: "./certs/cert.pem".to_string(),
            tls_key_path: "./certs/key.pem".to_string(),
            tls_port: 8443,
        }
    }

    #[fixture]
    fn sample_database_config() -> DatabaseConfig {
        DatabaseConfig::new(
            "testuser".to_string(),
            "testpass".to_string(),
            "localhost".to_string(),
            5432,
            "testdb".to_string(),
        )
    }

    mod oauth_configs {
        use super::*;

        #[rstest]
        #[test]
        fn github_config_has_correct_defaults(sample_github_config: GitHubConfig) {
            assert_eq!(sample_github_config.auth_url, "https://github.com/login/oauth/authorize");
            assert_eq!(sample_github_config.token_url, "https://github.com/login/oauth/access_token");
            assert_eq!(sample_github_config.user_url, "https://api.github.com/user");
        }

        #[rstest]
        #[test]
        fn gitlab_config_has_correct_defaults(sample_gitlab_config: GitLabConfig) {
            assert_eq!(sample_gitlab_config.auth_url, "https://gitlab.com/oauth/authorize");
            assert_eq!(sample_gitlab_config.token_url, "https://gitlab.com/oauth/token");
            assert_eq!(sample_gitlab_config.user_url, "https://gitlab.com/api/v4/user");
        }

        #[rstest]
        #[test]
        fn oauth_config_contains_both_providers(
            sample_github_config: GitHubConfig,
            sample_gitlab_config: GitLabConfig
        ) {
            let oauth_config = OAuthConfig {
                github: sample_github_config.clone(),
                gitlab: sample_gitlab_config.clone(),
            };

            assert_eq!(oauth_config.github.client_id, sample_github_config.client_id);
            assert_eq!(oauth_config.gitlab.client_id, sample_gitlab_config.client_id);
        }

        #[test]
        fn github_config_serialization_works() {
            let config = GitHubConfig {
                client_id: "test_id".to_string(),
                client_secret: "test_secret".to_string(),
                redirect_uri: "http://localhost/callback".to_string(),
                auth_url: default_github_auth_url(),
                token_url: default_github_token_url(),
                user_url: default_github_user_url(),
            };

            let json = assert_ok!(serde_json::to_string(&config));
            let deserialized: GitHubConfig = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.client_id, config.client_id);
            assert_eq!(deserialized.auth_url, config.auth_url);
        }
    }

    mod jwt_config {
        use super::*;

        #[rstest]
        #[test]
        fn jwt_config_stores_secret_and_expiration(sample_jwt_config: JwtConfig) {
            assert_eq!(sample_jwt_config.secret, "test_secret_key");
            assert_eq!(sample_jwt_config.expiration_seconds, 3600);
        }

        #[test]
        fn jwt_config_handles_different_expiration_times() {
            let config = JwtConfig {
                secret: "secret".to_string(),
                expiration_seconds: 7200, // 2 hours
            };

            assert_eq!(config.expiration_seconds, 7200);
        }

        #[rstest]
        #[test]
        fn jwt_config_serialization_preserves_data(sample_jwt_config: JwtConfig) {
            let json = assert_ok!(serde_json::to_string(&sample_jwt_config));
            let deserialized: JwtConfig = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.secret, sample_jwt_config.secret);
            assert_eq!(deserialized.expiration_seconds, sample_jwt_config.expiration_seconds);
        }
    }

    mod app_config {
        use super::*;

        #[test]
        fn app_config_combines_all_components() {
            let app_config = AppConfig {
                server: sample_server_config(),
                oauth: OAuthConfig {
                    github: sample_github_config(),
                    gitlab: sample_gitlab_config(),
                },
                jwt: sample_jwt_config(),
                database: sample_database_config(),
                logging: LoggingConfig::default(),
                command: CommandConfig::default(),
                kafka: KafkaConfig::default(),
            };

            assert_eq!(app_config.server.host, "127.0.0.1");
            assert_eq!(app_config.oauth.github.client_id, "github_client_id");
            assert_eq!(app_config.jwt.secret, "test_secret_key");
            assert_eq!(app_config.database.db, "testdb");
            assert_eq!(app_config.logging.level, "info");
        }

        #[test]
        fn app_config_serialization_preserves_structure() {
            let app_config = AppConfig {
                server: sample_server_config(),
                oauth: OAuthConfig {
                    github: sample_github_config(),
                    gitlab: sample_gitlab_config(),
                },
                jwt: sample_jwt_config(),
                database: sample_database_config(),
                logging: LoggingConfig::default(),
                command: CommandConfig::default(),
                kafka: KafkaConfig::default(),
            };

            let json = assert_ok!(serde_json::to_string(&app_config));
            let deserialized: AppConfig = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.server.host, app_config.server.host);
            assert_eq!(deserialized.oauth.github.client_id, app_config.oauth.github.client_id);
            assert_eq!(deserialized.jwt.secret, app_config.jwt.secret);
        }

        #[test]
        fn new_with_iam_defaults_sets_correct_kafka_client_id() {
            let app_config = AppConfig::new_with_iam_defaults(
                sample_server_config(),
                OAuthConfig {
                    github: sample_github_config(),
                    gitlab: sample_gitlab_config(),
                },
                sample_jwt_config(),
                sample_database_config(),
            );

            assert_eq!(app_config.kafka.client_id, "iam-service");
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn oauth_configs_handle_custom_endpoints() {
            let github_config = GitHubConfig {
                client_id: "id".to_string(),
                client_secret: "secret".to_string(),
                redirect_uri: "uri".to_string(),
                auth_url: "https://custom.github.com/oauth/authorize".to_string(),
                token_url: "https://custom.github.com/oauth/token".to_string(),
                user_url: "https://custom.github.com/api/user".to_string(),
            };

            assert!(github_config.auth_url.contains("custom.github.com"));
        }
    }
} 