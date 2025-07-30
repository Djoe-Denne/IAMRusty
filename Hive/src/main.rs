use hive_setup::{init_config, Application};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize configuration and logging
    let config = init_config()?;

    tracing::info!("Starting Hive organization management service...");

    // Create and initialize application
    let app = Application::new(config).await?;

    tracing::info!(
        "Hive service is ready! Listening on {}",
        app.server_address()
    );

    // Start the HTTP server
    app.serve().await?;

    Ok(())
}
