use configuration::load_config;
use setup::{app, config};
use tracing::debug;

pub async fn spawn_test_server() -> anyhow::Result<()> {
    // Use your real config loading logic
    let config = load_config().expect("failed to load test config");

    // Initialize logging for the test server
    if config.logging.level != "" {
        config::setup_logging(&config.logging.level);
    }

    debug!("🚀 Starting test server with configuration:");
    debug!("   Server host: {}", config.server.host);
    debug!("   Server port: {}", config.server.port);
    debug!("   TLS enabled: {}", config.server.tls_enabled);
    debug!("   Database URL: {}", config.database.url());
    debug!("   Database actual port: {}", config.database.actual_port());

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

    debug!(
        "🌐 Test server will listen on: http://{}:{}",
        server_config.host, server_config.port
    );

    // Build and run the application - this should run indefinitely
    debug!("🔄 Starting server...");
    app::build_and_run(config, server_config).await
}
