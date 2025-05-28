//! Configuration crate for the IAM service
//!
//! This crate provides all configuration structures and utilities for the application,
//! including server, database, OAuth, JWT, and logging configuration.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashMap;
use tracing::Level;

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

impl ServerConfig {
    /// Convert to setup ServerConfig format (for backward compatibility)
    pub fn to_setup_config(&self) -> SetupServerConfig {
        SetupServerConfig {
            host: self.host.clone(),
            port: self.port,
            tls_enabled: self.tls_enabled,
            tls_cert_path: if self.tls_enabled { Some(self.tls_cert_path.clone()) } else { None },
            tls_key_path: if self.tls_enabled { Some(self.tls_key_path.clone()) } else { None },
            tls_port: if self.tls_enabled { Some(self.tls_port) } else { None },
        }
    }
}

/// Setup server configuration (for backward compatibility with setup module)
#[derive(Debug, Clone)]
pub struct SetupServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub tls_port: Option<u16>,
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

/// Command retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRetryConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Base delay between retries in milliseconds
    #[serde(default = "default_base_delay_ms")]
    pub base_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
    /// Whether to use jitter
    #[serde(default = "default_use_jitter")]
    pub use_jitter: bool,
}

fn default_max_attempts() -> u32 {
    3
}

fn default_base_delay_ms() -> u64 {
    100
}

fn default_max_delay_ms() -> u64 {
    30000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_use_jitter() -> bool {
    true
}

impl Default for CommandRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            base_delay_ms: default_base_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
            use_jitter: default_use_jitter(),
        }
    }
}

/// Command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    /// Default retry configuration for all commands
    #[serde(default)]
    pub retry: CommandRetryConfig,
    /// Command-specific retry configurations
    #[serde(default)]
    pub overrides: HashMap<String, CommandRetryConfig>,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            retry: CommandRetryConfig::default(),
            overrides: HashMap::new(),
        }
    }
}

impl CommandConfig {
    /// Get retry configuration for a specific command
    /// Returns command-specific configuration if available, otherwise returns default
    pub fn get_retry_config(&self, command_type: &str) -> &CommandRetryConfig {
        self.overrides.get(command_type).unwrap_or(&self.retry)
    }
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
    /// Command configuration
    #[serde(default)]
    pub command: CommandConfig,
}

/// Setup logging based on configuration
pub fn setup_logging(log_level_str: &str) {
    let log_level = match log_level_str.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    
    // Use try_init() to avoid panicking if subscriber is already initialized
    // This is especially important during testing where setup_logging might be called multiple times
    let _ = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level_str))
        )
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::*;
    use rstest::*;
    use serde_json;

    // Test fixtures
    #[fixture]
    fn sample_database_creds() -> DatabaseCredentials {
        DatabaseCredentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        }
    }

    #[fixture]
    fn sample_database_config() -> DatabaseConfig {
        DatabaseConfig {
            creds: DatabaseCredentials {
                username: "testuser".to_string(),
                password: "testpass".to_string(),
            },
            host: "localhost".to_string(),
            port: 5432,
            db: "testdb".to_string(),
            read_replicas: vec![],
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
    fn sample_command_retry_config() -> CommandRetryConfig {
        CommandRetryConfig {
            max_attempts: 5,
            base_delay_ms: 200,
            max_delay_ms: 10000,
            backoff_multiplier: 1.5,
            use_jitter: false,
        }
    }

    #[fixture]
    fn sample_command_config() -> CommandConfig {
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("test_command".to_string(), CommandRetryConfig {
            max_attempts: 2,
            base_delay_ms: 50,
            max_delay_ms: 5000,
            backoff_multiplier: 1.2,
            use_jitter: false,
        });
        
        CommandConfig {
            retry: sample_command_retry_config(),
            overrides,
        }
    }

    mod database_credentials {
        use super::*;

        #[rstest]
        #[test]
        fn new_creates_valid_credentials(sample_database_creds: DatabaseCredentials) {
            assert_eq!(sample_database_creds.username, "testuser");
            assert_eq!(sample_database_creds.password, "testpass");
        }

        #[rstest]
        #[test]
        fn serialization_roundtrip(sample_database_creds: DatabaseCredentials) {
            let json = assert_ok!(serde_json::to_string(&sample_database_creds));
            let deserialized: DatabaseCredentials = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.username, sample_database_creds.username);
            assert_eq!(deserialized.password, sample_database_creds.password);
        }

        #[test]
        fn clone_creates_independent_copy() {
            let original = DatabaseCredentials {
                username: "original".to_string(),
                password: "pass".to_string(),
            };
            
            let cloned = original.clone();
            assert_eq!(cloned.username, original.username);
            assert_eq!(cloned.password, original.password);
        }
    }

    mod database_config {
        use super::*;

        #[rstest]
        #[test]
        fn new_creates_valid_config() {
            let config = DatabaseConfig::new(
                "user".to_string(),
                "pass".to_string(),
                "host".to_string(),
                5432,
                "db".to_string(),
            );

            assert_eq!(config.creds.username, "user");
            assert_eq!(config.creds.password, "pass");
            assert_eq!(config.host, "host");
            assert_eq!(config.port, 5432);
            assert_eq!(config.db, "db");
            assert!(config.read_replicas.is_empty());
        }

        #[rstest]
        #[test]
        fn url_builds_correct_postgres_url(sample_database_config: DatabaseConfig) {
            let url = sample_database_config.url();
            
            assert_eq!(url, "postgres://testuser:testpass@localhost:5432/testdb");
        }

        #[rstest]
        #[test]
        fn actual_port_returns_configured_port_when_not_zero() {
            let config = DatabaseConfig::new(
                "user".to_string(),
                "pass".to_string(),
                "localhost".to_string(),
                5433,
                "db".to_string(),
            );

            assert_eq!(config.actual_port(), 5433);
        }

        #[test]
        fn actual_port_returns_random_port_when_zero() {
            DatabaseConfig::clear_port_cache();
            
            let config = DatabaseConfig::new(
                "user".to_string(),
                "pass".to_string(),
                "localhost".to_string(),
                0,
                "db".to_string(),
            );

            let port1 = config.actual_port();
            let port2 = config.actual_port();
            
            // Should be consistent (cached)
            assert_eq!(port1, port2);
            // Should be a valid port number
            assert!(port1 > 1024);
            assert!(port1 <= 65535);
        }

        #[test]
        fn actual_port_caches_random_ports() {
            DatabaseConfig::clear_port_cache();
            
            let config1 = DatabaseConfig::new(
                "user1".to_string(),
                "pass".to_string(),
                "localhost".to_string(),
                0,
                "db1".to_string(),
            );
            
            let config2 = DatabaseConfig::new(
                "user2".to_string(),
                "pass".to_string(),
                "localhost".to_string(),
                0,
                "db2".to_string(),
            );

            let port1 = config1.actual_port();
            let port2 = config2.actual_port();
            
            // Different configs should get different ports
            assert_ne!(port1, port2);
            
            // But should be consistent for same config
            assert_eq!(config1.actual_port(), port1);
            assert_eq!(config2.actual_port(), port2);
        }

        #[rstest]
        #[case("postgres://user:pass@localhost:5432/testdb")]
        #[case("postgresql://user:pass@localhost:5432/testdb")]
        #[case("postgres://user:pass@localhost/testdb")]
        #[case("postgres://user@localhost:5432/testdb")]
        #[test]
        fn from_url_parses_valid_urls(#[case] url: &str) {
            let result = DatabaseConfig::from_url(url);
            assert_ok!(&result);
            
            let config = result.unwrap();
            assert_eq!(config.creds.username, "user");
            assert_eq!(config.host, "localhost");
            assert_eq!(config.db, "testdb");
        }

        #[rstest]
        #[case("http://user:pass@localhost:5432/testdb", "URL must use postgres")]
        #[case("postgres://user:pass@localhost:5432/", "Database name is required")]
        #[case("not-a-url", "Invalid URL")]
        #[test]
        fn from_url_rejects_invalid_urls(#[case] url: &str, #[case] expected_error: &str) {
            let result = DatabaseConfig::from_url(url);
            assert_err!(&result);
            
            let error = result.unwrap_err();
            assert!(error.contains(expected_error));
        }

        #[test]
        fn from_url_handles_missing_password() {
            let result = DatabaseConfig::from_url("postgres://user@localhost:5432/testdb");
            assert_ok!(&result);
            
            let config = result.unwrap();
            assert_eq!(config.creds.password, "");
        }

        #[test]
        fn from_url_uses_default_port_when_missing() {
            let result = DatabaseConfig::from_url("postgres://user:pass@localhost/testdb");
            assert_ok!(&result);
            
            let config = result.unwrap();
            assert_eq!(config.port, 5432);
        }

        #[test]
        fn clear_port_cache_clears_cached_ports() {
            let config = DatabaseConfig::new(
                "user".to_string(),
                "pass".to_string(),
                "localhost".to_string(),
                0,
                "db".to_string(),
            );

            let _port1 = config.actual_port(); // This caches a port
            DatabaseConfig::clear_port_cache();
            let port2 = config.actual_port(); // This should generate a new port
            
            // Note: ports might be the same due to randomness, but cache was cleared
            assert!(port2 > 1024);
        }

        #[rstest]
        #[test]
        fn serialization_preserves_all_fields(sample_database_config: DatabaseConfig) {
            let json = assert_ok!(serde_json::to_string(&sample_database_config));
            let deserialized: DatabaseConfig = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.creds.username, sample_database_config.creds.username);
            assert_eq!(deserialized.host, sample_database_config.host);
            assert_eq!(deserialized.port, sample_database_config.port);
            assert_eq!(deserialized.db, sample_database_config.db);
        }
    }

    mod server_config {
        use super::*;

        #[rstest]
        #[test]
        fn to_setup_config_without_tls(sample_server_config: ServerConfig) {
            let setup_config = sample_server_config.to_setup_config();
            
            assert_eq!(setup_config.host, sample_server_config.host);
            assert_eq!(setup_config.port, sample_server_config.port);
            assert!(!setup_config.tls_enabled);
            assert!(setup_config.tls_cert_path.is_none());
            assert!(setup_config.tls_key_path.is_none());
            assert!(setup_config.tls_port.is_none());
        }

        #[test]
        fn to_setup_config_with_tls() {
            let config = ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                tls_enabled: true,
                tls_cert_path: "/path/to/cert.pem".to_string(),
                tls_key_path: "/path/to/key.pem".to_string(),
                tls_port: 8443,
            };

            let setup_config = config.to_setup_config();
            
            assert!(setup_config.tls_enabled);
            assert_eq!(setup_config.tls_cert_path, Some("/path/to/cert.pem".to_string()));
            assert_eq!(setup_config.tls_key_path, Some("/path/to/key.pem".to_string()));
            assert_eq!(setup_config.tls_port, Some(8443));
        }

        #[test]
        fn default_values_applied_correctly() {
            let config = ServerConfig {
                host: "localhost".to_string(),
                port: 3000,
                tls_enabled: false,
                tls_cert_path: default_cert_path(),
                tls_key_path: default_key_path(),
                tls_port: default_tls_port(),
            };

            assert_eq!(config.tls_cert_path, "./certs/cert.pem");
            assert_eq!(config.tls_key_path, "./certs/key.pem");
            assert_eq!(config.tls_port, 8443);
        }

        #[rstest]
        #[test]
        fn serialization_includes_default_values(sample_server_config: ServerConfig) {
            let json = assert_ok!(serde_json::to_string(&sample_server_config));
            
            // Should include all fields even with defaults
            assert!(json.contains("tls_enabled"));
            assert!(json.contains("tls_cert_path"));
            assert!(json.contains("tls_key_path"));
            assert!(json.contains("tls_port"));
        }
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

    mod logging_config {
        use super::*;

        #[test]
        fn default_logging_config_uses_info_level() {
            let config = LoggingConfig::default();
            assert_eq!(config.level, "info");
        }

        #[rstest]
        #[case("debug")]
        #[case("info")]
        #[case("warn")]
        #[case("error")]
        #[test]
        fn logging_config_accepts_valid_levels(#[case] level: &str) {
            let config = LoggingConfig {
                level: level.to_string(),
            };
            assert_eq!(config.level, level);
        }

        #[test]
        fn logging_config_serialization_works() {
            let config = LoggingConfig {
                level: "debug".to_string(),
            };

            let json = assert_ok!(serde_json::to_string(&config));
            let deserialized: LoggingConfig = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.level, config.level);
        }
    }

    mod command_retry_config {
        use super::*;

        #[test]
        fn default_command_retry_config_has_expected_values() {
            let config = CommandRetryConfig::default();
            
            assert_eq!(config.max_attempts, 3);
            assert_eq!(config.base_delay_ms, 100);
            assert_eq!(config.max_delay_ms, 30000);
            assert_eq!(config.backoff_multiplier, 2.0);
            assert!(config.use_jitter);
        }

        #[rstest]
        #[test]
        fn command_retry_config_stores_custom_values(sample_command_retry_config: CommandRetryConfig) {
            assert_eq!(sample_command_retry_config.max_attempts, 5);
            assert_eq!(sample_command_retry_config.base_delay_ms, 200);
            assert_eq!(sample_command_retry_config.max_delay_ms, 10000);
            assert_eq!(sample_command_retry_config.backoff_multiplier, 1.5);
            assert!(!sample_command_retry_config.use_jitter);
        }

        #[test]
        fn command_retry_config_serialization_works() {
            let config = CommandRetryConfig {
                max_attempts: 3,
                base_delay_ms: 150,
                max_delay_ms: 20000,
                backoff_multiplier: 2.5,
                use_jitter: true,
            };

            let json = assert_ok!(serde_json::to_string(&config));
            let deserialized: CommandRetryConfig = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.max_attempts, config.max_attempts);
            assert_eq!(deserialized.base_delay_ms, config.base_delay_ms);
            assert_eq!(deserialized.max_delay_ms, config.max_delay_ms);
            assert_eq!(deserialized.backoff_multiplier, config.backoff_multiplier);
            assert_eq!(deserialized.use_jitter, config.use_jitter);
        }

        #[test]
        fn command_retry_config_serde_defaults_work() {
            // Test that serde defaults are applied when fields are missing
            let json = r#"{}"#;
            let config: CommandRetryConfig = assert_ok!(serde_json::from_str(json));
            
            assert_eq!(config.max_attempts, 3);
            assert_eq!(config.base_delay_ms, 100);
            assert_eq!(config.max_delay_ms, 30000);
            assert_eq!(config.backoff_multiplier, 2.0);
            assert!(config.use_jitter);
        }
    }

    mod command_config {
        use super::*;

        #[test]
        fn default_command_config_has_expected_values() {
            let config = CommandConfig::default();
            
            assert_eq!(config.retry.max_attempts, 3);
            assert!(config.overrides.is_empty());
        }

        #[rstest]
        #[test]
        fn command_config_stores_retry_and_overrides(sample_command_config: CommandConfig) {
            assert_eq!(sample_command_config.retry.max_attempts, 5);
            assert_eq!(sample_command_config.overrides.len(), 1);
            assert!(sample_command_config.overrides.contains_key("test_command"));
        }

        #[rstest]
        #[test]
        fn get_retry_config_returns_override_when_available(sample_command_config: CommandConfig) {
            let config = sample_command_config.get_retry_config("test_command");
            
            assert_eq!(config.max_attempts, 2);
            assert_eq!(config.base_delay_ms, 50);
            assert_eq!(config.backoff_multiplier, 1.2);
        }

        #[rstest]
        #[test]
        fn get_retry_config_returns_default_when_no_override(sample_command_config: CommandConfig) {
            let config = sample_command_config.get_retry_config("unknown_command");
            
            assert_eq!(config.max_attempts, 5);
            assert_eq!(config.base_delay_ms, 200);
            assert_eq!(config.backoff_multiplier, 1.5);
        }

        #[test]
        fn command_config_serialization_preserves_structure() {
            let mut overrides = std::collections::HashMap::new();
            overrides.insert("cmd1".to_string(), CommandRetryConfig {
                max_attempts: 1,
                base_delay_ms: 25,
                max_delay_ms: 2500,
                backoff_multiplier: 1.1,
                use_jitter: false,
            });

            let config = CommandConfig {
                retry: CommandRetryConfig::default(),
                overrides,
            };

            let json = assert_ok!(serde_json::to_string(&config));
            let deserialized: CommandConfig = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.retry.max_attempts, config.retry.max_attempts);
            assert_eq!(deserialized.overrides.len(), 1);
            assert!(deserialized.overrides.contains_key("cmd1"));
            
            let override_config = deserialized.overrides.get("cmd1").unwrap();
            assert_eq!(override_config.max_attempts, 1);
            assert_eq!(override_config.base_delay_ms, 25);
        }

        #[test]
        fn command_config_serde_defaults_work() {
            // Test that serde defaults are applied when fields are missing
            let json = r#"{}"#;
            let config: CommandConfig = assert_ok!(serde_json::from_str(json));
            
            assert_eq!(config.retry.max_attempts, 3);
            assert!(config.overrides.is_empty());
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
                command: sample_command_config(),
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
                command: sample_command_config(),
            };

            let json = assert_ok!(serde_json::to_string(&app_config));
            let deserialized: AppConfig = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.server.host, app_config.server.host);
            assert_eq!(deserialized.oauth.github.client_id, app_config.oauth.github.client_id);
            assert_eq!(deserialized.jwt.secret, app_config.jwt.secret);
        }
    }

    mod setup_logging {
        use super::*;

        #[test]
        fn setup_logging_handles_valid_levels() {
            // Test just ensures the function doesn't panic for valid levels
            // We can't test actual logging setup because global subscriber can only be set once
            
            // These calls should not panic
            let levels = ["trace", "debug", "info", "warn", "error"];
            for level in levels.iter() {
                // We test the level matching logic without calling the actual setup
                let parsed_level = match level.to_lowercase().as_str() {
                    "trace" => Level::TRACE,
                    "debug" => Level::DEBUG,
                    "info" => Level::INFO,
                    "warn" => Level::WARN,
                    "error" => Level::ERROR,
                    _ => Level::INFO,
                };
                
                // Just verify the level parsing works
                assert!(matches!(parsed_level, Level::TRACE | Level::DEBUG | Level::INFO | Level::WARN | Level::ERROR));
            }
        }

        #[test]
        fn setup_logging_handles_invalid_level() {
            // Test that invalid levels default to INFO
            let parsed_level = match "invalid".to_lowercase().as_str() {
                "trace" => Level::TRACE,
                "debug" => Level::DEBUG,
                "info" => Level::INFO,
                "warn" => Level::WARN,
                "error" => Level::ERROR,
                _ => Level::INFO,
            };
            
            assert!(matches!(parsed_level, Level::INFO));
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn database_config_handles_empty_strings() {
            let config = DatabaseConfig::new(
                "".to_string(),
                "".to_string(),
                "".to_string(),
                0,
                "".to_string(),
            );

            // Should build URL even with empty strings (though not practical)
            let url = config.url();
            assert!(url.starts_with("postgres://"));
        }

        #[test]
        fn database_config_handles_special_characters_in_password() {
            let config = DatabaseConfig::new(
                "user".to_string(),
                "p@ss:w0rd!".to_string(),
                "localhost".to_string(),
                5432,
                "db".to_string(),
            );

            let url = config.url();
            assert!(url.contains("p@ss:w0rd!"));
        }

        #[test]
        fn server_config_handles_ipv6_address() {
            let config = ServerConfig {
                host: "::1".to_string(),
                port: 8080,
                tls_enabled: false,
                tls_cert_path: default_cert_path(),
                tls_key_path: default_key_path(),
                tls_port: default_tls_port(),
            };

            assert_eq!(config.host, "::1");
        }

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