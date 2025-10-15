//! Application builder and runner

use anyhow::Result;
use manifesto_configuration::ManifestoConfig;
use rustycog_server::{Server, ServerConfig};

/// Application state
pub struct App {
    config: ManifestoConfig,
}

/// Application builder
pub struct AppBuilder {
    config: ManifestoConfig,
}

impl AppBuilder {
    /// Create a new application builder
    pub fn new(config: ManifestoConfig) -> Self {
        Self { config }
    }

    /// Build the application
    pub async fn build(self) -> Result<App> {
        // TODO: Initialize database connection pool
        // TODO: Initialize repositories
        // TODO: Initialize application services
        
        Ok(App {
            config: self.config,
        })
    }
}

impl App {
    /// Run the application
    pub async fn run(self, server_config: ServerConfig) -> Result<()> {
        tracing::info!("Starting Manifesto service...");
        
        // Create the HTTP router
        let router = manifesto_http_server::create_router();
        
        // Create and run the server
        let server = Server::new(server_config, router);
        server.run().await?;
        
        Ok(())
    }
}


