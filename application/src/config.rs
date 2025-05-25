use serde::{Deserialize, Serialize};

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL database URL
    pub url: String,
    /// Read replica database URLs
    #[serde(default)]
    pub read_replicas: Vec<String>,
}

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

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

/// OAuth configuration for all providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub github: GitHubConfig,
    pub gitlab: GitLabConfig,
}

/// Complete application configuration
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
} 