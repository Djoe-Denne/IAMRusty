use lazaret_configuration::load_config;
use lazaret_setup::{app, config};
use tracing::info;

/// Start the Lazaret capability-boundary service.
///
/// # Errors
///
/// Returns an error if configuration cannot be loaded or the application fails to build or run.
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let config = load_config()?;

    config::setup_logging(&config);
    info!(
        "Configuration loaded with log level: {}",
        config.logging.level
    );

    let server_config = config::ServerConfig {
        host: config.server.host.clone(),
        port: config.server.port,
        tls_enabled: config.server.tls_enabled,
        tls_cert_path: if config.server.tls_enabled {
            config.server.tls_cert_path.clone()
        } else {
            String::new()
        },
        tls_key_path: if config.server.tls_enabled {
            config.server.tls_key_path.clone()
        } else {
            String::new()
        },
        tls_client_ca_path: config.server.tls_client_ca_path.clone(),
        tls_port: if config.server.tls_enabled {
            config.server.tls_port
        } else {
            0
        },
    };

    app::AppBuilder::new(config)
        .build()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build application: {e}"))?
        .run(server_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run application: {e}"))
}
