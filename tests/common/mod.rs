use std::sync::Arc;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::{ContainerAsync, runners::AsyncRunner};
use sea_orm::{Database, DatabaseConnection};
use sqlx::postgres::PgPoolOptions;
use tokio::time::{sleep, Duration};

pub mod fixtures;

/// Database test container setup
pub struct DatabaseContainer {
    pub connection: DatabaseConnection,
    pub database_url: String,
    _container: ContainerAsync<Postgres>,
}

impl DatabaseContainer {
    /// Create a new PostgreSQL test container
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Use the modern testcontainers API with AsyncRunner
        let postgres = Postgres::default();
        let container = postgres.start().await?;
        let host_port = container.get_host_port_ipv4(5432).await?;
        
        let database_url = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres", 
            host_port
        );
        
        // Wait for PostgreSQL to be ready
        let mut retries = 30;
        loop {
            match PgPoolOptions::new()
                .max_connections(1)
                .connect(&database_url)
                .await
            {
                Ok(_) => break,
                Err(_) if retries > 0 => {
                    retries -= 1;
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                Err(e) => return Err(Box::new(e)),
            }
        }
        
        // Connect with SeaORM
        let connection = Database::connect(&database_url).await?;
        
        // Run migrations (you may need to adjust this based on your migration setup)
        // migration::run_migrations(&connection).await?;
        
        Ok(Self {
            connection,
            database_url,
            _container: container,
        })
    }
    
    /// Clean database tables after each test
    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Clean up test data
        // You may need to adjust table names based on your schema
        // For now, we'll skip cleanup to avoid errors with non-existent tables
        // sqlx::query("TRUNCATE TABLE users, oauth_accounts, user_emails RESTART IDENTITY CASCADE")
        //     .execute(&sqlx::PgPool::connect(&self.database_url).await?)
        //     .await?;
        
        Ok(())
    }
}

/// Test application state builder
pub struct TestAppStateBuilder {
    pub database: Arc<DatabaseContainer>,
}

impl TestAppStateBuilder {
    pub fn new(database: Arc<DatabaseContainer>) -> Self {
        Self { database }
    }
    
    /// Build application state for testing
    /// This should match your actual AppState structure
    pub async fn build(self) -> Result<http_server::AppState, Box<dyn std::error::Error>> {
        // You'll need to implement this based on your actual AppState
        // This is a placeholder that shows the pattern
        todo!("Implement based on your actual AppState structure")
    }
} 