use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub use rustycog_config::ServerConfig;

/// Setup logging based on the specified level
pub fn setup_logging(level: &str) {
    let log_level = match level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{{SERVICE_NAME}}_service={}", level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Logging initialized at level: {}", level);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_logging() {
        // Test that setup_logging doesn't panic with valid levels
        setup_logging("info");
        setup_logging("debug");
        setup_logging("warn");
        setup_logging("error");
        setup_logging("trace");
        
        // Test invalid level defaults to info
        setup_logging("invalid");
    }
} 