use infra::config::load_config;
use setup::{app, config};

pub async fn spawn_test_server() -> anyhow::Result<()> {
    // Use your real config loading logic
    let config = load_config().expect("failed to load test config");
    
    // Initialize logging for the test server
    //config::setup_logging(&config.logging.level);

    eprintln!("🚀 Starting test server with configuration:");
    eprintln!("   Server host: {}", config.server.host);
    eprintln!("   Server port: {}", config.server.port);
    eprintln!("   TLS enabled: {}", config.server.tls_enabled);
    eprintln!("   Database URL: {}", config.database.url());
    eprintln!("   Database actual port: {}", config.database.actual_port());

    // Create server configuration
    let server_config = config::ServerConfig {
        host: config.server.host.clone(),
        port: config.server.port,
        tls_enabled: config.server.tls_enabled,
        tls_cert_path: if config.server.tls_enabled { Some(config.server.tls_cert_path.clone()) } else { None },
        tls_key_path: if config.server.tls_enabled { Some(config.server.tls_key_path.clone()) } else { None },
        tls_port: if config.server.tls_enabled { Some(config.server.tls_port) } else { None },
    };

    eprintln!("🌐 Test server will listen on: http://{}:{}", server_config.host, server_config.port);

    // Build and run the application - this should run indefinitely
    eprintln!("🔄 Starting server...");
    app::build_and_run(config, server_config).await
}
