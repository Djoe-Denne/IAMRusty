use hive_configuration::{ConfigError, ServiceConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize application configuration
pub fn init_config() -> Result<ServiceConfig, ConfigError> {
    let config = ServiceConfig::load()?;
    
    // Initialize logging based on configuration
    init_logging(&config);
    
    tracing::info!(
        service = %config.service.name,
        version = %config.service.version,
        environment = %config.service.environment,
        "Configuration loaded successfully"
    );
    
    Ok(config)
}

/// Initialize logging configuration
pub fn init_logging(config: &ServiceConfig) {
    let level = match config.logging.level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    
    if config.logging.structured {
        // JSON structured logging
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_max_level(level)
            )
            .init();
    } else {
        // Human-readable logging
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .with_max_level(level)
            )
            .init();
    }
} 