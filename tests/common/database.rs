//! Test database utilities with testcontainers
//! 
//! This module provides a single PostgreSQL container for all tests with table truncation
//! between tests to ensure test isolation while maintaining performance.

use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use testcontainers::{GenericImage, ImageExt, ContainerAsync, runners::AsyncRunner};
use sea_orm::{Database, DatabaseConnection, DbErr, Statement, ConnectionTrait};
use infra::config::{AppConfig, DatabaseConfig};
use infra::db::DbConnectionPool;
use migration::{Migrator, MigratorTrait};
use tracing::{info, debug, warn};
use std::sync::atomic::{AtomicBool, Ordering};

/// Global test database container instance
static TEST_CONTAINER: OnceLock<Arc<Mutex<Option<Arc<TestDatabaseContainer>>>>> = OnceLock::new();

/// Flag to track if cleanup handler has been registered
static CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Test database container wrapper
pub struct TestDatabaseContainer {
    container: ContainerAsync<GenericImage>,
    pub database_url: String,
    pub port: u16,
}

impl TestDatabaseContainer {
    /// Stop and remove the container
    pub async fn cleanup(self) {
        info!("Stopping and removing test database container");
        if let Err(e) = self.container.stop().await {
            warn!("Failed to stop container: {}", e);
        } else {
            info!("Container stopped successfully");
        }
        if let Err(e) = self.container.rm().await {
            warn!("Failed to remove container: {}", e);
        } else {
            info!("Container removed successfully");
        }
        info!("Test database container cleanup completed");
    }
}

/// Test database fixture providing database connection and cleanup utilities
pub struct TestDatabase {
    pub pool: DbConnectionPool,
    pub connection: Arc<DatabaseConnection>,
    pub database_url: String,
}

impl TestDatabase {
    /// Get or create the global test database instance
    pub async fn new() -> Result<Self, DbErr> {
        let container = get_or_create_test_container().await?;
        let database_url = container.database_url.clone();
        
        // Create connection pool
        let pool = DbConnectionPool::new(&database_url, vec![]).await?;
        let connection = pool.get_write_connection();
        
        // Run migrations
        Self::run_migrations(&connection).await?;
        
        Ok(Self {
            pool,
            connection,
            database_url,
        })
    }
    
    /// Run database migrations
    async fn run_migrations(connection: &DatabaseConnection) -> Result<(), DbErr> {
        info!("Running database migrations for test database");
        Migrator::up(connection, None).await?;
        info!("Database migrations completed successfully");
        Ok(())
    }
    
    /// Truncate all tables to clean up between tests
    pub async fn truncate_all_tables(&self) -> Result<(), DbErr> {
        debug!("Truncating all tables for test cleanup");
        
        // Get all table names from the database
        let tables = self.get_all_table_names().await?;
        
        if tables.is_empty() {
            debug!("No tables found to truncate");
            return Ok(());
        }
        
        // Disable foreign key checks temporarily
        self.connection
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SET session_replication_role = replica;".to_string(),
            ))
            .await?;
        
        // Truncate all tables
        for table in &tables {
            let sql = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE;", table);
            self.connection
                .execute(Statement::from_string(
                    sea_orm::DatabaseBackend::Postgres,
                    sql,
                ))
                .await?;
        }
        
        // Re-enable foreign key checks
        self.connection
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SET session_replication_role = DEFAULT;".to_string(),
            ))
            .await?;
        
        debug!("Successfully truncated {} tables", tables.len());
        Ok(())
    }
    
    /// Get all table names from the current database
    async fn get_all_table_names(&self) -> Result<Vec<String>, DbErr> {
        let sql = r#"
            SELECT table_name 
            FROM information_schema.tables 
            WHERE table_schema = 'public' 
            AND table_type = 'BASE TABLE'
            AND table_name NOT LIKE 'seaql_%'
            ORDER BY table_name;
        "#;
        
        let result = self.connection
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql.to_string(),
            ))
            .await?;
        
        let tables: Vec<String> = result
            .into_iter()
            .map(|row| row.try_get::<String>("", "table_name"))
            .collect::<Result<Vec<_>, _>>()?;
        
        debug!("Found {} tables: {:?}", tables.len(), tables);
        Ok(tables)
    }
    
    /// Create a test configuration with the test database URL
    pub fn create_test_config(&self) -> AppConfig {
        let mut config = create_base_test_config();
        config.database.url = self.database_url.clone();
        config
    }
    
    /// Get the database connection for direct use
    pub fn get_connection(&self) -> Arc<DatabaseConnection> {
        self.connection.clone()
    }
    
    /// Get the connection pool
    pub fn get_pool(&self) -> &DbConnectionPool {
        &self.pool
    }
}

/// Get or create the global test container
async fn get_or_create_test_container() -> Result<Arc<TestDatabaseContainer>, DbErr> {
    let container_mutex = TEST_CONTAINER.get_or_init(|| {
        Arc::new(Mutex::new(None))
    });
    
    let mut container_guard = container_mutex.lock().await;
    
    if let Some(ref container) = *container_guard {
        return Ok(container.clone());
    }
    
    info!("Creating new PostgreSQL test container");
    
    // Create PostgreSQL container using GenericImage with a static name
    let postgres_image = GenericImage::new("postgres", "15-alpine")
        .with_env_var("POSTGRES_DB", "iam_test")
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_container_name("iam-test-db"); // Static name for easy cleanup
    
    let container = postgres_image
        .start()
        .await
        .map_err(|e| DbErr::Custom(format!("Failed to start container: {}", e)))?;
    
    let port = container.get_host_port_ipv4(5432).await
        .map_err(|e| DbErr::Custom(format!("Failed to get container port: {}", e)))?;
    let database_url = format!(
        "postgres://postgres:postgres@localhost:{}/iam_test",
        port
    );
    
    info!("Test database container started on port {}", port);
    info!("Database URL: {}", database_url);
    
    // Wait for database to be ready
    wait_for_database(&database_url).await?;
    
    let test_container = Arc::new(TestDatabaseContainer {
        container,
        database_url,
        port,
    });
    
    *container_guard = Some(test_container.clone());
    
    // Register cleanup handler on first container creation
    register_cleanup_handler().await;
    
    Ok(test_container)
}

/// Wait for the database to be ready for connections
async fn wait_for_database(database_url: &str) -> Result<(), DbErr> {
    use tokio::time::{sleep, Duration, timeout};
    
    info!("Waiting for database to be ready...");
    
    let max_attempts = 30;
    let mut attempts = 0;
    
    while attempts < max_attempts {
        match timeout(Duration::from_secs(2), Database::connect(database_url)).await {
            Ok(Ok(conn)) => {
                // Test the connection with a simple query
                match conn.ping().await {
                    Ok(_) => {
                        info!("Database is ready after {} attempts", attempts + 1);
                        return Ok(());
                    }
                    Err(e) => {
                        debug!("Database ping failed: {}", e);
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("Database connection failed: {}", e);
            }
            Err(_) => {
                debug!("Database connection timed out");
            }
        }
        
        attempts += 1;
        if attempts < max_attempts {
            debug!("Retrying database connection in 1 second... (attempt {}/{})", attempts, max_attempts);
            sleep(Duration::from_secs(1)).await;
        }
    }
    
    Err(DbErr::Custom(format!(
        "Database failed to become ready after {} attempts",
        max_attempts
    )))
}

/// Register cleanup handler to stop container when process exits
async fn register_cleanup_handler() {
    // Only register once
    if CLEANUP_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    
    info!("Registering test database container cleanup handler");
    
    // Register cleanup for Ctrl+C and other signals
    let _ = ctrlc::set_handler(move || {
        info!("Received termination signal, cleaning up test database container");
        
        // Use direct docker command to cleanup the specific container
        use std::process::Command;
        let _ = Command::new("docker").args(&["stop", "iam-test-db"]).output();
        let _ = Command::new("docker").args(&["rm", "iam-test-db"]).output();
        
        std::process::exit(0);
    });
    
    // Register cleanup for normal process termination
    extern "C" fn cleanup_on_exit() {
        eprintln!("Process exiting, attempting to cleanup test database container...");
        // Note: We can't do async cleanup here, but the container will be cleaned up
        // by Docker eventually. This is just for logging.
    }
    
    unsafe {
        libc::atexit(cleanup_on_exit);
    }
}

/// Create a base test configuration
fn create_base_test_config() -> AppConfig {
    AppConfig {
        server: infra::config::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            tls_enabled: false,
            tls_cert_path: "./certs/cert.pem".to_string(),
            tls_key_path: "./certs/key.pem".to_string(),
            tls_port: 8443,
        },
        database: DatabaseConfig {
            url: "postgres://postgres:postgres@localhost:5432/iam_test".to_string(), // Will be overridden
            read_replicas: vec![],
        },
        oauth: infra::config::OAuthConfig {
            github: infra::config::GitHubConfig {
                client_id: "test_github_client_id".to_string(),
                client_secret: "test_github_client_secret".to_string(),
                redirect_uri: "http://localhost:8080/api/auth/github/callback".to_string(),
                auth_url: "http://localhost:3000/login/oauth/authorize".to_string(),
                token_url: "http://localhost:3000/login/oauth/access_token".to_string(),
                user_url: "http://localhost:3000/user".to_string(),
            },
            gitlab: infra::config::GitLabConfig {
                client_id: "test_gitlab_client_id".to_string(),
                client_secret: "test_gitlab_client_secret".to_string(),
                redirect_uri: "http://localhost:8080/api/auth/gitlab/callback".to_string(),
                auth_url: "http://localhost:3000/oauth/authorize".to_string(),
                token_url: "http://localhost:3000/oauth/token".to_string(),
                user_url: "http://localhost:3000/api/v4/user".to_string(),
            },
        },
        jwt: infra::config::JwtConfig {
            secret: "test_jwt_secret_for_testing_only_must_be_at_least_32_bytes_long".to_string(),
            expiration_seconds: 3600,
        },
        logging: infra::config::LoggingConfig {
            level: "debug".to_string(),
        },
    }
}

/// Test fixture that automatically cleans up after each test
pub struct TestFixture {
    pub database: TestDatabase,
}

impl TestFixture {
    /// Create a new test fixture with database cleanup
    pub async fn new() -> Result<Self, DbErr> {
        let database = TestDatabase::new().await?;
        
        // Clean up any existing data
        database.truncate_all_tables().await?;
        
        Ok(Self { database })
    }
    
    /// Get the test configuration
    pub fn config(&self) -> AppConfig {
        self.database.create_test_config()
    }
    
    /// Get the database connection
    pub fn db(&self) -> Arc<DatabaseConnection> {
        self.database.get_connection()
    }
    
    /// Get the database pool
    pub fn pool(&self) -> &DbConnectionPool {
        self.database.get_pool()
    }
    
    /// Manual cleanup (automatically called on drop)
    pub async fn cleanup(&self) -> Result<(), DbErr> {
        self.database.truncate_all_tables().await
    }
    
    /// Cleanup the global test container (stops and removes it)
    pub async fn cleanup_container() -> Result<(), DbErr> {
        let container_mutex = TEST_CONTAINER.get();
        if let Some(container_mutex) = container_mutex {
            let mut container_guard = container_mutex.lock().await;
            if let Some(container_arc) = container_guard.take() {
                info!("Manually cleaning up test database container");
                // Try to unwrap the Arc to get ownership
                match Arc::try_unwrap(container_arc) {
                    Ok(container) => {
                        container.cleanup().await;
                        info!("Test database container cleanup completed");
                    }
                    Err(_) => {
                        warn!("Could not cleanup container: still has references");
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        // Schedule cleanup in a blocking context
        // Note: This is best effort cleanup. The main cleanup happens in TestFixture::new()
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            let database = self.database.connection.clone();
            rt.spawn(async move {
                if let Err(e) = truncate_tables_best_effort(&database).await {
                    warn!("Failed to cleanup test database on drop: {}", e);
                }
            });
        }
    }
}

/// Best effort table truncation for cleanup
async fn truncate_tables_best_effort(connection: &DatabaseConnection) -> Result<(), DbErr> {
    // Simple truncation without detailed error handling
    let tables = ["refresh_tokens", "provider_tokens", "user_emails", "users"];
    
    // Disable foreign key checks
    let _ = connection
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SET session_replication_role = replica;".to_string(),
        ))
        .await;
    
    // Truncate known tables
    for table in &tables {
        let sql = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE;", table);
        let _ = connection
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql,
            ))
            .await;
    }
    
    // Re-enable foreign key checks
    let _ = connection
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SET session_replication_role = DEFAULT;".to_string(),
        ))
        .await;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    
    #[tokio::test]
    #[serial]
    async fn test_database_container_creation() {
        let db = TestDatabase::new().await.expect("Failed to create test database");
        assert!(!db.database_url.is_empty());
        assert!(db.database_url.contains("localhost"));
    }
    
    #[tokio::test]
    #[serial]
    async fn test_table_truncation() {
        let fixture = TestFixture::new().await.expect("Failed to create test fixture");
        
        // Test that truncation works without errors
        fixture.cleanup().await.expect("Failed to truncate tables");
    }
    
    #[tokio::test]
    #[serial]
    async fn test_config_generation() {
        let fixture = TestFixture::new().await.expect("Failed to create test fixture");
        let config = fixture.config();
        
        assert!(config.database.url.contains("localhost"));
        assert_eq!(config.jwt.secret, "test_jwt_secret_for_testing_only_must_be_at_least_32_bytes_long");
    }
} 