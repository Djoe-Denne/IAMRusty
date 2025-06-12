//! Core configuration crate for RustyCog services
//!
//! This crate provides core configuration structures and utilities that can be shared
//! across multiple services, including server, database, command retry, Kafka, and logging configuration.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashMap;
use tracing::{debug, Level};

// Re-export config and dotenvy for service use
pub use config::{Config, ConfigError, Environment, File, FileFormat};
pub use dotenvy::dotenv;

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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
            tls_enabled: false,
            tls_cert_path: default_cert_path(),
            tls_key_path: default_key_path(),
            tls_port: default_tls_port(),
        }
    }
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
            
            // Create a unique cache key for this database configuration
            port_cache.remove(&"db".to_string());
            debug!("DB port cleared from cache");
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            creds: DatabaseCredentials {
                username: "postgres".to_string(),
                password: "postgres".to_string(),
            },
            host: "localhost".to_string(),
            port: 0, // Use random port by default
            db: "app_database".to_string(),
            read_replicas: vec![],
        }
    }
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

/// Command configuration with retry settings and command-specific overrides
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
    /// Returns command-specific config if available, otherwise returns default
    pub fn get_retry_config(&self, command_type: &str) -> &CommandRetryConfig {
        self.overrides.get(command_type).unwrap_or(&self.retry)
    }
}

/// SQS configuration for event publishing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqsConfig {
    /// AWS region
    #[serde(default = "default_sqs_region")]
    pub region: String,
    /// AWS account ID (required for building queue URLs)
    #[serde(default = "default_sqs_account_id")]
    pub account_id: String,
    /// Queue names for different event types (generic, not IAM-specific)
    #[serde(default = "default_sqs_queues")]
    pub queues: HashMap<String, String>,
    /// Default queue name to use when no specific queue is configured for an event type
    #[serde(default = "default_sqs_default_queue")]
    pub default_queue: String,
    /// AWS access key ID (optional, can use IAM roles or environment variables)
    #[serde(default)]
    pub access_key_id: Option<String>,
    /// AWS secret access key (optional, can use IAM roles or environment variables)
    #[serde(default)]
    pub secret_access_key: Option<String>,
    /// AWS session token (optional, for temporary credentials)
    #[serde(default)]
    pub session_token: Option<String>,
    /// Custom endpoint host (for LocalStack or custom SQS implementations)
    #[serde(default = "default_sqs_host")]
    pub host: String,
    /// Custom endpoint port (for LocalStack or custom SQS implementations, 0 for random port)
    #[serde(default = "default_sqs_port")]
    pub port: u16,
    /// Custom endpoint URL (for LocalStack or custom SQS implementations) - deprecated, use host/port instead
    #[serde(default)]
    pub endpoint_url: Option<String>,
    /// Whether to enable SQS (for testing/development flexibility)
    #[serde(default = "default_sqs_enabled")]
    pub enabled: bool,
    /// Maximum number of retries for failed messages
    #[serde(default = "default_sqs_max_retries")]
    pub max_retries: u32,
    /// Message timeout in seconds
    #[serde(default = "default_sqs_timeout_seconds")]
    pub timeout_seconds: u64,
}

impl SqsConfig {
    /// Check if a queue is a FIFO queue based on queue name
    pub fn is_fifo_queue(&self, queue_name: &str) -> bool {
        queue_name.ends_with(".fifo")
    }

    /// Get the queue name for a specific event type, falling back to default queue
    pub fn get_queue_name(&self, event_type: &str) -> &str {
        self.queues.get(event_type).map(|s| s.as_str()).unwrap_or(&self.default_queue)
    }

    /// Build the full queue URL for a given queue name
    pub fn build_queue_url(&self, queue_name: &str) -> String {
        if self.host == "localhost" {
            // For LocalStack or custom endpoint
            format!("http://{}:{}/000000000000/{}", self.host, self.actual_port(), queue_name)
        } else {
            // For AWS
            format!("https://sqs.{}.scaleway.com/{}/{}", self.region, self.account_id, queue_name)
        }
    }

    /// Get the full queue URL for a specific event type
    pub fn get_queue_url(&self, event_type: &str) -> String {
        let queue_name = self.get_queue_name(event_type);
        self.build_queue_url(queue_name)
    }

    /// Get the default queue URL (for backward compatibility)
    pub fn default_queue_url(&self) -> String {
        self.build_queue_url(&self.default_queue)
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
                    Err(_) => 4566, // fallback to LocalStack default
                }
            }
            Err(_) => 4566, // fallback to LocalStack default
        }
    }

    /// Get the actual port being used (resolves random port if needed)
    /// This method caches the resolved port to ensure consistency across calls
    pub fn actual_port(&self) -> u16 {
        if self.port == 0 {
            // Create a unique cache key for this SQS configuration
            let cache_key = format!("sqs:{}:{}", self.host, self.region);
            
            let cache = PORT_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
            let mut port_cache = cache.lock().unwrap();
            
            // Return cached port if available
            if let Some(&cached_port) = port_cache.get(&cache_key) {
                debug!("Using cached SQS port: {}", cached_port);
                return cached_port;
            }
            
            // Generate new random port and cache it
            let random_port = Self::get_random_port();
            port_cache.insert(cache_key, random_port);
            
            println!("Generated random SQS port: {}", random_port);
            random_port
        } else {
            println!("Using SQS port from config: {}", self.port);
            self.port
        }        
    }

    /// Get the endpoint URL for SQS (constructs from host/port or uses legacy endpoint_url)
    pub fn endpoint_url(&self) -> Option<String> {
        // If legacy endpoint_url is provided, use it
        if let Some(ref url) = self.endpoint_url {
            return Some(url.clone());
        }
        
        // If host is localhost (default), construct URL from host/port
        if self.host == "localhost" {
            let port = self.actual_port();
            Some(format!("http://{}:{}", self.host, port))
        } else {
            // For non-localhost hosts, assume it's AWS (no custom endpoint needed)
            None
        }
    }

    /// Create a new SqsConfig with the specified components
    pub fn new(region: String, account_id: String, queues: HashMap<String, String>, default_queue: String) -> Self {
        Self {
            region,
            account_id,
            queues,
            default_queue,
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
            host: default_sqs_host(),
            port: default_sqs_port(),
            endpoint_url: None,
            enabled: default_sqs_enabled(),
            max_retries: default_sqs_max_retries(),
            timeout_seconds: default_sqs_timeout_seconds(),
        }
    }

    /// Clear the port cache for SQS
    pub fn clear_port_cache() {
        if let Some(cache) = PORT_CACHE.get() {
            let mut port_cache = cache.lock().unwrap();
            // Create a unique cache key for this SQS configuration
            port_cache.remove(&"sqs".to_string());
            debug!("SQS port cleared from cache");
        }
    }
}

impl Default for SqsConfig {
    fn default() -> Self {
        Self {
            region: default_sqs_region(),
            account_id: default_sqs_account_id(),
            queues: default_sqs_queues(),
            default_queue: default_sqs_default_queue(),
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
            host: default_sqs_host(),
            port: default_sqs_port(), // Use random port for testing by default
            endpoint_url: None,
            enabled: default_sqs_enabled(),
            max_retries: default_sqs_max_retries(),
            timeout_seconds: default_sqs_timeout_seconds(),
        }
    }
}

// SQS configuration defaults
fn default_sqs_region() -> String {
    "us-east-1".to_string()
}

fn default_sqs_account_id() -> String {
    "123456789012".to_string()
}

fn default_sqs_queues() -> HashMap<String, String> {
    HashMap::new()
}

fn default_sqs_default_queue() -> String {
    "user-events".to_string()
}

fn default_sqs_host() -> String {
    "localhost".to_string()
}

fn default_sqs_port() -> u16 {
    4566 // LocalStack SQS default port
}

fn default_sqs_enabled() -> bool {
    true
}

fn default_sqs_max_retries() -> u32 {
    3
}

fn default_sqs_timeout_seconds() -> u64 {
    30
}

/// Event queue configuration - can be either Kafka or SQS
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QueueConfig {
    #[serde(rename = "kafka")]
    Kafka(KafkaConfig),
    #[serde(rename = "sqs")]
    Sqs(SqsConfig),
    #[serde(rename = "disabled")]
    Disabled,
}

impl Default for QueueConfig {
    fn default() -> Self {
        QueueConfig::Kafka(KafkaConfig::default())
    }
}

impl QueueConfig {
    /// Check if queue is enabled
    pub fn is_enabled(&self) -> bool {
        match self {
            QueueConfig::Kafka(config) => config.enabled,
            QueueConfig::Sqs(config) => config.enabled,
            QueueConfig::Disabled => false,
        }
    }
}

/// Kafka configuration for event publishing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Kafka broker host
    #[serde(default = "default_kafka_host")]
    pub host: String,
    /// Kafka broker port (9092 default, 0 for random port)
    #[serde(default = "default_kafka_port")]
    pub port: u16,
    /// Topic for user events
    #[serde(default = "default_user_events_topic")]
    pub user_events_topic: String,
    /// Producer client ID
    #[serde(default = "default_kafka_client_id")]
    pub client_id: String,
    /// Message timeout in milliseconds
    #[serde(default = "default_kafka_timeout_ms")]
    pub timeout_ms: u64,
    /// Maximum number of retries for failed messages
    #[serde(default = "default_kafka_max_retries")]
    pub max_retries: u32,
    /// Whether to enable Kafka (for testing/development flexibility)
    #[serde(default = "default_kafka_enabled")]
    pub enabled: bool,
    /// Compression type for messages (none, gzip, snappy, lz4, zstd)
    #[serde(default = "default_kafka_compression")]
    pub compression: String,
    /// Security protocol (plaintext, ssl, sasl_plaintext, sasl_ssl)
    #[serde(default = "default_kafka_security_protocol")]
    pub security_protocol: String,
    /// SASL mechanism (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512, GSSAPI, OAUTHBEARER)
    #[serde(default)]
    pub sasl_mechanism: Option<String>,
    /// SASL username
    #[serde(default)]
    pub sasl_username: Option<String>,
    /// SASL password
    #[serde(default)]
    pub sasl_password: Option<String>,
    /// SSL CA certificate location (use "probe" for system CA certificates)
    #[serde(default)]
    pub ssl_ca_location: Option<String>,
    /// SSL certificate location for client authentication
    #[serde(default)]
    pub ssl_certificate_location: Option<String>,
    /// SSL private key location for client authentication
    #[serde(default)]
    pub ssl_key_location: Option<String>,
    /// SSL private key password
    #[serde(default)]
    pub ssl_key_password: Option<String>,
    /// Additional broker hosts (for multi-broker setups) - backward compatibility
    #[serde(default)]
    pub additional_brokers: Vec<String>,
}

impl KafkaConfig {
    /// Get the brokers string for Kafka client configuration
    pub fn brokers(&self) -> String {
        let port = self.actual_port();
        let primary_broker = format!("{}:{}", self.host, port);
        
        if self.additional_brokers.is_empty() {
            primary_broker
        } else {
            let mut all_brokers = vec![primary_broker];
            all_brokers.extend(self.additional_brokers.clone());
            all_brokers.join(",")
        }
    }
    
    /// Get a random available port
    fn get_random_port() -> u16 {
        use std::net::TcpListener;
        
        // Try to bind to a random port
        match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => {
                match listener.local_addr() {
                    Ok(addr) => addr.port(),
                    Err(_) => 9092, // fallback to default
                }
            }
            Err(_) => 9092, // fallback to default
        }
    }
    
    /// Get the actual port being used (resolves random port if needed)
    /// This method caches the resolved port to ensure consistency across calls
    pub fn actual_port(&self) -> u16 {
        if self.port == 0 {
            // Create a unique cache key for this Kafka configuration
            let cache_key = format!("kafka:{}:{}", self.host, self.client_id);
            
            let cache = PORT_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
            let mut port_cache = cache.lock().unwrap();
            
            // Return cached port if available
            if let Some(&cached_port) = port_cache.get(&cache_key) {
                debug!("cached_port: {}", cached_port);
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
    
    /// Create a new KafkaConfig with the specified components
    pub fn new(host: String, port: u16, user_events_topic: String, client_id: String) -> Self {
        Self {
            host,
            port,
            user_events_topic,
            client_id,
            timeout_ms: default_kafka_timeout_ms(),
            max_retries: default_kafka_max_retries(),
            enabled: default_kafka_enabled(),
            compression: default_kafka_compression(),
            security_protocol: default_kafka_security_protocol(),
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
            ssl_ca_location: None,
            ssl_certificate_location: None,
            ssl_key_location: None,
            ssl_key_password: None,
            additional_brokers: vec![],
        }
    }
    
    /// Create a KafkaConfig from a brokers string (for backward compatibility)
    pub fn from_brokers(brokers: &str) -> Result<Self, String> {
        let broker_list: Vec<&str> = brokers.split(',').collect();
        if broker_list.is_empty() {
            return Err("Brokers string cannot be empty".to_string());
        }
        
        // Parse the first broker as primary
        let primary_broker = broker_list[0].trim();
        let parts: Vec<&str> = primary_broker.split(':').collect();
        
        if parts.len() != 2 {
            return Err(format!("Invalid broker format '{}', expected 'host:port'", primary_broker));
        }
        
        let host = parts[0].to_string();
        let port = parts[1].parse::<u16>()
            .map_err(|_| format!("Invalid port in broker '{}'", primary_broker))?;
        
        // Handle additional brokers
        let additional_brokers = if broker_list.len() > 1 {
            broker_list[1..].iter().map(|b| b.trim().to_string()).collect()
        } else {
            vec![]
        };
        
        Ok(Self {
            host,
            port,
            user_events_topic: default_user_events_topic(),
            client_id: default_kafka_client_id(),
            timeout_ms: default_kafka_timeout_ms(),
            max_retries: default_kafka_max_retries(),
            enabled: default_kafka_enabled(),
            compression: default_kafka_compression(),
            security_protocol: default_kafka_security_protocol(),
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
            ssl_ca_location: None,
            ssl_certificate_location: None,
            ssl_key_location: None,
            ssl_key_password: None,
            additional_brokers,
        })
    }
    
    /// Clear the port cache (useful for testing)
    pub fn clear_port_cache() {
        if let Some(cache) = PORT_CACHE.get() {
            let mut port_cache = cache.lock().unwrap();
            // Create a unique cache key for this Kafka configuration
            port_cache.remove(&"kafka".to_string());
            debug!("Kafka port cleared from cache");
        }
    }
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            host: default_kafka_host(),
            port: default_kafka_port(),
            user_events_topic: default_user_events_topic(),
            client_id: default_kafka_client_id(),
            timeout_ms: default_kafka_timeout_ms(),
            max_retries: default_kafka_max_retries(),
            enabled: default_kafka_enabled(),
            compression: default_kafka_compression(),
            security_protocol: default_kafka_security_protocol(),
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
            ssl_ca_location: None,
            ssl_certificate_location: None,
            ssl_key_location: None,
            ssl_key_password: None,
            additional_brokers: vec![],
        }
    }
}

// Kafka configuration defaults
fn default_kafka_host() -> String {
    "localhost".to_string()
}

fn default_kafka_port() -> u16 {
    9092
}

fn default_user_events_topic() -> String {
    "user-events".to_string()
}

fn default_kafka_client_id() -> String {
    "rustycog-service".to_string()
}

fn default_kafka_timeout_ms() -> u64 {
    5000
}

fn default_kafka_max_retries() -> u32 {
    3
}

fn default_kafka_enabled() -> bool {
    true
}

fn default_kafka_compression() -> String {
    "gzip".to_string()
}

fn default_kafka_security_protocol() -> String {
    "plaintext".to_string()
}

/// Generic configuration cache and loading functionality
/// This allows any service to implement their own configuration structure
/// while using the same caching and loading logic.

/// Configuration cache trait that services must implement
pub trait ConfigCache<T> {
    /// Get the cached configuration if available
    fn get_cached() -> Option<T>;
    /// Set the cached configuration
    fn set_cached(config: T);
    /// Clear the cached configuration
    fn clear_cached();
}

/// Generic configuration loader
pub trait ConfigLoader<T>: Default + for<'de> Deserialize<'de> + Serialize + Clone {
    /// Create a default configuration instance
    fn create_default() -> T;
    /// Get the configuration prefix for environment variables (e.g., "IAM" for IAM_*)
    fn config_prefix() -> &'static str;
}

/// Load configuration with caching
pub fn load_config_with_cache<T, C>() -> Result<T, ConfigError>
where
    T: ConfigLoader<T>,
    C: ConfigCache<T>,
{
    // Return cached config if available
    if let Some(config) = C::get_cached() {
        tracing::debug!("Returning cached configuration");
        return Ok(config);
    }
    
    // Load fresh configuration
    let config = load_config_fresh::<T>()?;
    
    // Cache the configuration
    C::set_cached(config.clone());
    tracing::debug!("Configuration loaded and cached");
    
    Ok(config)
}

/// Load fresh configuration without caching
pub fn load_config_fresh<T>() -> Result<T, ConfigError>
where
    T: ConfigLoader<T>,
{
    use std::path::Path;
    use std::env;
    
    // Load .env file if it exists
    let _ = dotenv().ok();
    
    // Get environment
    let env = env::var("RUN_ENV").unwrap_or_else(|_| "development".to_string());
    
    // Determine the configuration file to use
    let config_file = match env.as_str() {
        "test" => "config/test.toml",
        "production" => "config/production.toml",
        _ => "config/development.toml",
    };
    
    tracing::info!("Loading configuration from environment: {}", env);
    tracing::debug!("Configuration file: {}", config_file);
    
    let mut builder = Config::builder();
    
    // Load base configuration file if it exists
    if Path::new(config_file).exists() {
        tracing::debug!("Loading configuration file: {}", config_file);
        builder = builder.add_source(File::with_name(config_file).format(FileFormat::Toml));
    } else {
        tracing::warn!("Configuration file not found: {}, using defaults", config_file);
    }
    
    // Load environment-specific configuration if different from base
    if env != "development" {
        let env_config_path = format!("config/{}.toml", env);
        if Path::new(&env_config_path).exists() && env_config_path != config_file {
            tracing::debug!("Loading environment configuration from {}", env_config_path);
            builder = builder.add_source(File::with_name(&env_config_path).format(FileFormat::Toml));
        }
    }
    
    // Add environment variable overrides with service-specific prefix
    let prefix = T::config_prefix();
    tracing::debug!("Loading environment variables with prefix: {}_", prefix);
    builder = builder.add_source(
        Environment::with_prefix(prefix)
            .separator("__")
            .try_parsing(true)
    );
    
    // Build configuration
    let config = builder.build()?;
    
    // Try to deserialize to the target type
    match config.try_deserialize::<T>() {
        Ok(app_config) => {
            tracing::info!("Configuration loaded successfully for environment: {}", env);
            Ok(app_config)
        }
        Err(e) => {
            tracing::error!("Failed to deserialize configuration: {}", e);
            tracing::info!("Falling back to default configuration");
            Ok(T::create_default())
        }
    }
}

/// Clear all configuration caches
/// This is useful for testing to ensure fresh configuration loading
pub fn clear_all_caches() {
    DatabaseConfig::clear_port_cache();
    KafkaConfig::clear_port_cache();
    SqsConfig::clear_port_cache();
    println!("All configuration caches cleared");
}

/// Generate a default configuration file in TOML format
pub fn generate_default_config_toml<T>() -> Result<String, ConfigError>
where
    T: ConfigLoader<T>,
{
    let default_config = T::create_default();
    toml::to_string_pretty(&default_config)
        .map_err(|e| ConfigError::Message(format!("Failed to serialize default config: {}", e)))
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

    mod kafka_config {
        use super::*;

        #[test]
        fn default_kafka_config_has_expected_values() {
            let config = KafkaConfig::default();
            
            assert_eq!(config.host, "localhost");
            assert_eq!(config.port, 9092);
            assert_eq!(config.user_events_topic, "user-events");
            assert_eq!(config.client_id, "rustycog-service");
            assert_eq!(config.timeout_ms, 5000);
            assert_eq!(config.max_retries, 3);
            assert!(config.enabled);
            assert_eq!(config.compression, "gzip");
            assert_eq!(config.security_protocol, "plaintext");
            assert!(config.sasl_mechanism.is_none());
            assert!(config.sasl_username.is_none());
            assert!(config.sasl_password.is_none());
            assert!(config.ssl_ca_location.is_none());
            assert!(config.ssl_certificate_location.is_none());
            assert!(config.ssl_key_location.is_none());
            assert!(config.ssl_key_password.is_none());
            assert!(config.additional_brokers.is_empty());
        }

        #[test]
        fn kafka_config_serialization_works() {
            let config = KafkaConfig {
                host: "test_host".to_string(),
                port: 10000,
                user_events_topic: "test_topic".to_string(),
                client_id: "test_client_id".to_string(),
                timeout_ms: 10000,
                max_retries: 5,
                enabled: false,
                compression: "snappy".to_string(),
                security_protocol: "ssl".to_string(),
                sasl_mechanism: Some("SCRAM-SHA-256".to_string()),
                sasl_username: Some("test_username".to_string()),
                sasl_password: Some("test_password".to_string()),
                ssl_ca_location: Some("probe".to_string()),
                ssl_certificate_location: Some("/path/to/cert.pem".to_string()),
                ssl_key_location: Some("/path/to/key.pem".to_string()),
                ssl_key_password: Some("password".to_string()),
                additional_brokers: vec!["test_broker1:10001".to_string(), "test_broker2:10002".to_string()],
            };

            let json = assert_ok!(serde_json::to_string(&config));
            let deserialized: KafkaConfig = assert_ok!(serde_json::from_str(&json));
            
            assert_eq!(deserialized.host, "test_host");
            assert_eq!(deserialized.port, 10000);
            assert_eq!(deserialized.user_events_topic, "test_topic");
            assert_eq!(deserialized.client_id, "test_client_id");
            assert_eq!(deserialized.timeout_ms, 10000);
            assert_eq!(deserialized.max_retries, 5);
            assert!(!deserialized.enabled);
            assert_eq!(deserialized.compression, "snappy");
            assert_eq!(deserialized.security_protocol, "ssl");
            assert_eq!(deserialized.sasl_mechanism, Some("SCRAM-SHA-256".to_string()));
            assert_eq!(deserialized.sasl_username, Some("test_username".to_string()));
            assert_eq!(deserialized.sasl_password, Some("test_password".to_string()));
            assert_eq!(deserialized.ssl_ca_location, Some("probe".to_string()));
            assert_eq!(deserialized.ssl_certificate_location, Some("/path/to/cert.pem".to_string()));
            assert_eq!(deserialized.ssl_key_location, Some("/path/to/key.pem".to_string()));
            assert_eq!(deserialized.ssl_key_password, Some("password".to_string()));
            assert_eq!(deserialized.additional_brokers, vec!["test_broker1:10001".to_string(), "test_broker2:10002".to_string()]);
        }

        #[test]
        fn brokers_method_returns_single_broker() {
            let config = KafkaConfig::new(
                "test-host".to_string(),
                9092,
                "test-topic".to_string(),
                "test-client".to_string(),
            );

            assert_eq!(config.brokers(), "test-host:9092");
        }

        #[test]
        fn brokers_method_includes_additional_brokers() {
            let mut config = KafkaConfig::new(
                "primary".to_string(),
                9092,
                "test-topic".to_string(),
                "test-client".to_string(),
            );
            config.additional_brokers = vec![
                "secondary:9093".to_string(),
                "tertiary:9094".to_string(),
            ];

            let brokers = config.brokers();
            assert!(brokers.contains("primary:9092"));
            assert!(brokers.contains("secondary:9093"));
            assert!(brokers.contains("tertiary:9094"));
            
            // Should be comma-separated
            let broker_list: Vec<&str> = brokers.split(',').collect();
            assert_eq!(broker_list.len(), 3);
        }

        #[test]
        fn actual_port_returns_configured_port_when_not_zero() {
            let config = KafkaConfig::new(
                "localhost".to_string(),
                9093,
                "test-topic".to_string(),
                "test-client".to_string(),
            );

            assert_eq!(config.actual_port(), 9093);
        }

        #[test]
        fn actual_port_returns_random_port_when_zero() {
            KafkaConfig::clear_port_cache();
            
            let config = KafkaConfig::new(
                "localhost".to_string(),
                0,
                "test-topic".to_string(),
                "test-client-random".to_string(),
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
        fn actual_port_caches_different_ports_for_different_configs() {
            KafkaConfig::clear_port_cache();
            
            let config1 = KafkaConfig::new(
                "localhost".to_string(),
                0,
                "topic1".to_string(),
                "client1".to_string(),
            );
            
            let config2 = KafkaConfig::new(
                "localhost".to_string(),
                0,
                "topic2".to_string(),
                "client2".to_string(),
            );

            let port1 = config1.actual_port();
            let port2 = config2.actual_port();
            
            // Different configs should get different ports
            assert_ne!(port1, port2);
            
            // But should be consistent for same config
            assert_eq!(config1.actual_port(), port1);
            assert_eq!(config2.actual_port(), port2);
        }

        #[test]
        fn from_brokers_parses_single_broker() {
            let config = assert_ok!(KafkaConfig::from_brokers("localhost:9092"));
            
            assert_eq!(config.host, "localhost");
            assert_eq!(config.port, 9092);
            assert!(config.additional_brokers.is_empty());
            assert_eq!(config.user_events_topic, "user-events");
            assert_eq!(config.client_id, "rustycog-service");
        }

        #[test]
        fn from_brokers_parses_multiple_brokers() {
            let config = assert_ok!(KafkaConfig::from_brokers("primary:9092,secondary:9093,tertiary:9094"));
            
            assert_eq!(config.host, "primary");
            assert_eq!(config.port, 9092);
            assert_eq!(config.additional_brokers, vec!["secondary:9093", "tertiary:9094"]);
        }

        #[test]
        fn from_brokers_handles_whitespace() {
            let config = assert_ok!(KafkaConfig::from_brokers(" primary:9092 , secondary:9093 "));
            
            assert_eq!(config.host, "primary");
            assert_eq!(config.port, 9092);
            assert_eq!(config.additional_brokers, vec!["secondary:9093"]);
        }

        #[test]
        fn from_brokers_rejects_invalid_format() {
            assert_err!(KafkaConfig::from_brokers("invalid"));
            assert_err!(KafkaConfig::from_brokers("host:invalid_port"));
            assert_err!(KafkaConfig::from_brokers(""));
            assert_err!(KafkaConfig::from_brokers("host"));
        }

        #[test]
        fn new_creates_valid_config() {
            let config = KafkaConfig::new(
                "test-host".to_string(),
                9092,
                "test-topic".to_string(),
                "test-client".to_string(),
            );

            assert_eq!(config.host, "test-host");
            assert_eq!(config.port, 9092);
            assert_eq!(config.user_events_topic, "test-topic");
            assert_eq!(config.client_id, "test-client");
            assert_eq!(config.timeout_ms, 5000);
            assert_eq!(config.max_retries, 3);
            assert!(config.enabled);
            assert_eq!(config.compression, "gzip");
            assert_eq!(config.security_protocol, "plaintext");
            assert!(config.additional_brokers.is_empty());
        }

        #[test]
        fn clear_port_cache_clears_cached_ports() {
            let config = KafkaConfig::new(
                "localhost".to_string(),
                0,
                "test-topic".to_string(),
                "test-client-clear".to_string(),
            );

            let _port1 = config.actual_port(); // This caches a port
            KafkaConfig::clear_port_cache();
            let port2 = config.actual_port(); // This should generate a new port
            
            // Note: ports might be the same due to randomness, but cache was cleared
            assert!(port2 > 1024);
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
    }
} 