use sea_orm::{Database, DatabaseConnection, DbErr};
use testcontainers::{clients::Cli, images::postgres::Postgres, Container};
use uuid::Uuid;

use {{SERVICE_NAME}}_migration::{Migrator, MigratorTrait};

/// Database test fixture that manages a PostgreSQL test database
pub struct DatabaseFixture<'a> {
    _container: Container<'a, Postgres>,
    pub connection: DatabaseConnection,
    pub database_url: String,
}

impl<'a> DatabaseFixture<'a> {
    /// Create a new database fixture with a fresh PostgreSQL instance
    pub async fn new(docker: &'a Cli) -> Result<Self, DbErr> {
        // Start PostgreSQL container
        let postgres_image = Postgres::default()
            .with_db_name("test_db")
            .with_user("test_user")
            .with_password("test_password");
        
        let container = docker.run(postgres_image);
        let host_port = container.get_host_port_ipv4(5432);
        
        // Build connection URL
        let database_url = format!(
            "postgres://test_user:test_password@localhost:{}/test_db",
            host_port
        );
        
        // Wait for database to be ready and connect
        let connection = Database::connect(&database_url).await?;
        
        // Run migrations
        Migrator::up(&connection, None).await?;
        
        Ok(Self {
            _container: container,
            connection,
            database_url,
        })
    }
    
    /// Get a database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }
    
    /// Clean all tables (for test isolation)
    pub async fn clean_database(&self) -> Result<(), DbErr> {
        // In a real implementation, you would truncate all tables
        // For now, we'll just run a simple query
        use sea_orm::Statement;
        
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "TRUNCATE TABLE example_entities RESTART IDENTITY CASCADE;".to_string(),
        );
        
        self.connection.execute(stmt).await?;
        Ok(())
    }
}

/// In-memory database fixture for faster tests (using SQLite)
pub struct InMemoryDatabaseFixture {
    pub connection: DatabaseConnection,
}

impl InMemoryDatabaseFixture {
    /// Create a new in-memory database fixture
    pub async fn new() -> Result<Self, DbErr> {
        let database_url = format!("sqlite::memory:{}", Uuid::new_v4());
        let connection = Database::connect(&database_url).await?;
        
        // Run migrations
        Migrator::up(&connection, None).await?;
        
        Ok(Self { connection })
    }
    
    /// Get a database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_database_fixture() {
        let fixture = InMemoryDatabaseFixture::new().await.unwrap();
        
        // Test that we can get a connection
        let connection = fixture.connection();
        assert!(connection.ping().await.is_ok());
    }
} 