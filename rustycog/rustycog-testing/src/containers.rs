//! Test container utilities

use testcontainers::{Container, RunnableImage};
use testcontainers_modules::postgres::Postgres;

/// Test database container
pub struct TestDatabase {
    _container: Container<Postgres>,
}

impl TestDatabase {
    /// Create a new test database
    pub fn new() -> Self {
        let docker = testcontainers::clients::Cli::default();
        let container = docker.run(Postgres::default());
        
        Self {
            _container: container,
        }
    }
    
    /// Get the database URL
    pub fn url(&self) -> String {
        let port = self._container.get_host_port_ipv4(5432).unwrap_or(5432);
        format!("postgresql://postgres:postgres@localhost:{}/postgres", port)
    }
} 