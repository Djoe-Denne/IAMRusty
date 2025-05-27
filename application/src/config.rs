use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashMap;

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

/// Database credentials configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseCredentials {
    /// Database username
    pub username: String,
    /// Database password
    pub password: String,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database credentials
    pub creds: DatabaseCredentials,
    /// Database host
    pub host: String,
    /// Database port (5432 default, 0 for random port)
    #[serde(default = "default_db_port")]
    pub port: u16,
    /// Database name
    pub db: String,
    /// Read replica database URLs (still using full URLs for flexibility)
    #[serde(default)]
    pub read_replicas: Vec<String>,
}

fn default_db_port() -> u16 {
    5432
}

/// Global cache for resolved random ports to ensure consistency
static PORT_CACHE: OnceLock<Arc<Mutex<HashMap<String, u16>>>> = OnceLock::new();

impl DatabaseConfig {
    /// Construct the primary database URL from components
    pub fn url(&self) -> String {
        let port = self.actual_port();
        
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.creds.username,
            self.creds.password,
            self.host,
            port,
            self.db
        )
    }
    
    /// Get a random available port
    fn get_random_port() -> u16 {
        use std::net::{TcpListener, SocketAddr};
        
        // Try to bind to a random port
        match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => {
                match listener.local_addr() {
                    Ok(SocketAddr::V4(addr)) => addr.port(),
                    Ok(SocketAddr::V6(addr)) => addr.port(),
                    Err(_) => 5432, // fallback to default
                }
            }
            Err(_) => 5432, // fallback to default
        }
    }
    
    /// Get the actual port being used (resolves random port if needed)
    /// This method caches the resolved port to ensure consistency across calls
    pub fn actual_port(&self) -> u16 {
        if self.port == 0 {
            // Create a unique cache key for this database configuration
            let cache_key = format!("{}:{}:{}", self.host, self.db, self.creds.username);
            
            let cache = PORT_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
            let mut port_cache = cache.lock().unwrap();
            
            // Return cached port if available
            if let Some(&cached_port) = port_cache.get(&cache_key) {
                return cached_port;
            }
            
            // Generate new random port and cache it
            let random_port = Self::get_random_port();
            port_cache.insert(cache_key, random_port);
            random_port
        } else {
            self.port
        }
    }
    
    /// Create a new DatabaseConfig with the specified components
    pub fn new(username: String, password: String, host: String, port: u16, db: String) -> Self {
        Self {
            creds: DatabaseCredentials { username, password },
            host,
            port,
            db,
            read_replicas: vec![],
        }
    }
    
    /// Create a DatabaseConfig from a URL (for backward compatibility)
    pub fn from_url(url: &str) -> Result<Self, String> {
        use url::Url;
        
        let parsed = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
        
        if parsed.scheme() != "postgres" && parsed.scheme() != "postgresql" {
            return Err("URL must use postgres:// or postgresql:// scheme".to_string());
        }
        
        let username = parsed.username().to_string();
        let password = parsed.password().unwrap_or("").to_string();
        let host = parsed.host_str().unwrap_or("localhost").to_string();
        let port = parsed.port().unwrap_or(5432);
        let db = parsed.path().trim_start_matches('/').to_string();
        
        if db.is_empty() {
            return Err("Database name is required in URL path".to_string());
        }
        
        Ok(Self::new(username, password, host, port, db))
    }
    
    /// Clear the port cache (useful for testing)
    pub fn clear_port_cache() {
        if let Some(cache) = PORT_CACHE.get() {
            let mut port_cache = cache.lock().unwrap();
            port_cache.clear();
        }
    }
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