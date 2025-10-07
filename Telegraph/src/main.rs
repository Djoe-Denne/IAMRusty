use telegraph_configuration::load_config;
use telegraph_setup::{app, config};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let config = load_config()?;

    // Initialize logging
    config::setup_logging(&config);
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
            config.server.tls_cert_path.clone()
        } else {
            "".to_string()
        },
        tls_key_path: if config.server.tls_enabled {
            config.server.tls_key_path.clone()
        } else {
            "".to_string()
        },
        tls_port: if config.server.tls_enabled {
            config.server.tls_port
        } else {
            0
        },
    };

    // Build and run the application
    app::AppBuilder::new(config)
        .build()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build application: {}", e))?
        .run(server_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))
}
