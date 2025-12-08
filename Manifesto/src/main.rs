use anyhow::Result;
use manifesto_configuration::load_config;
use manifesto_setup::Application;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "manifesto=info,rustycog=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Manifesto service...");

    // Load application configuration
    let config = load_config()?;
    tracing::info!("Configuration loaded");

    // Server config is already available in config.server
    let server_config = config.server.clone();

    // Initialize application with all dependencies
    let app = Application::new(config).await?;

    // Start HTTP server
    app.run(server_config).await?;

    Ok(())
}
