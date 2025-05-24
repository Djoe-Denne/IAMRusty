use std::env;

/// Test configuration for consistent behavior across environments
pub struct TestConfig {
    /// Whether to run with Docker (true) or local PostgreSQL (false)
    pub use_docker: bool,
    /// Database connection timeout in seconds
    pub db_timeout: u64,
    /// Number of retries for database connection
    pub db_retries: u32,
    /// Whether to enable verbose logging in tests
    pub verbose_logging: bool,
    /// Maximum test concurrency level
    pub max_concurrency: usize,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            use_docker: env::var("TEST_USE_DOCKER").unwrap_or_else(|_| "true".to_string()) == "true",
            db_timeout: env::var("TEST_DB_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            db_retries: env::var("TEST_DB_RETRIES")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            verbose_logging: env::var("TEST_VERBOSE")
                .unwrap_or_else(|_| "false".to_string()) == "true",
            max_concurrency: env::var("TEST_MAX_CONCURRENCY")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .unwrap_or(4),
        }
    }
}

impl TestConfig {
    /// Get the test configuration
    pub fn get() -> Self {
        Self::default()
    }
    
    /// Initialize test logging based on configuration
    pub fn init_logging(&self) {
        if self.verbose_logging && env::var("RUST_LOG").is_err() {
            env::set_var("RUST_LOG", "debug");
        }
        
        // Initialize tracing subscriber if not already initialized
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }
}

/// CI/CD optimization settings
pub struct CiConfig;

impl CiConfig {
    /// Check if running in CI environment
    pub fn is_ci() -> bool {
        env::var("CI").is_ok() || 
        env::var("GITHUB_ACTIONS").is_ok() || 
        env::var("GITLAB_CI").is_ok()
    }
    
    /// Get optimized settings for CI
    pub fn get_ci_config() -> TestConfig {
        if Self::is_ci() {
            TestConfig {
                use_docker: true,
                db_timeout: 60, // Longer timeout for CI
                db_retries: 50, // More retries for CI
                verbose_logging: true, // Enable logging in CI
                max_concurrency: 2, // Reduce concurrency in CI
            }
        } else {
            TestConfig::default()
        }
    }
}

/// Test environment setup
pub struct TestEnvironment;

impl TestEnvironment {
    /// Setup test environment with optimal settings
    pub fn setup() -> TestConfig {
        let config = if CiConfig::is_ci() {
            CiConfig::get_ci_config()
        } else {
            TestConfig::get()
        };
        
        config.init_logging();
        
        // Set test-specific environment variables
        env::set_var("RUST_TEST_THREADS", config.max_concurrency.to_string());
        
        config
    }
} 