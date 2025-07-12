use telegraph_configuration::load_config;
use telegraph_setup::{app, config};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let config = load_config()?;

    // Initialize logging
    config::setup_logging(&config.logging.level);
    info!(
        "Configuration loaded with log level: {}",
        config.logging.level
    );

    // Build and run the application
    app::AppBuilder::new(config)
        .build()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build application: {}", e))?
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))
} 