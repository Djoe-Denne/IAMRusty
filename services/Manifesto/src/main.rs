use anyhow::Result;
use manifesto_setup::{load_config, setup_logging, Application};

#[tokio::main]
async fn main() -> Result<()> {
    // Load application configuration
    let config = load_config()?;
    setup_logging(&config);

    tracing::info!("Starting Manifesto service...");
    tracing::info!("Configuration loaded");

    // Server config is already available in config.server
    let server_config = config.server.clone();

    // Initialize application with all dependencies
    let app = Application::new(config).await?;

    // Start HTTP server
    app.run(server_config).await?;

    Ok(())
}
