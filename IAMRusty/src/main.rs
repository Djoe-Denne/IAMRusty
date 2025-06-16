use configuration::load_config;
use setup::{app, config};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration first
    let config = load_config()?;

    // Initialize logging
    config::setup_logging(&config.logging.level);
    info!(
        "Configuration loaded with log level: {}",
        config.logging.level
    );

    // Create server configuration
    let server_config = config::ServerConfig {
        host: config.server.host.clone(),
        port: config.server.port,
        tls_enabled: config.server.tls_enabled,
        tls_cert_path: if config.server.tls_enabled {
            Some(config.server.tls_cert_path.clone())
        } else {
            None
        },
        tls_key_path: if config.server.tls_enabled {
            Some(config.server.tls_key_path.clone())
        } else {
            None
        },
        tls_port: if config.server.tls_enabled {
            Some(config.server.tls_port)
        } else {
            None
        },
    };

    // Build and run the application
    app::build_and_run(config, server_config).await
}
