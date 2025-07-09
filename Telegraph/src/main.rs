use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber;

use setup::{config::TelegraphConfig, AppBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup basic logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("Starting Telegraph Communication Service");
    
    // Create default configuration
    let config = TelegraphConfig::default();
    
    info!("Configuration loaded successfully");
    
    // Start the Telegraph service
    let app = AppBuilder::new(config).build().await?;
    
    info!("Telegraph service is ready to start!");
    
    // Run the service
    app.run().await?;
    
    Ok(())
} 