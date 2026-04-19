//! Configuration utilities

use manifesto_configuration::ManifestoConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub use rustycog_config::ServerConfig;

pub fn load_config() -> anyhow::Result<ManifestoConfig> {
    manifesto_configuration::load_config().map_err(Into::into)
}

/// Setup logging based on configuration
pub fn setup_logging(config: &ManifestoConfig) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}





