//! Configuration utilities

use manifesto_configuration::ManifestoConfig;

pub use manifesto_configuration::setup_logging;
pub use rustycog::config::ServerConfig;

/// Load Manifesto configuration from files and environment.
///
/// # Errors
///
/// Returns an error if configuration cannot be loaded or parsed.
pub fn load_config() -> anyhow::Result<ManifestoConfig> {
    manifesto_configuration::load_config().map_err(Into::into)
}
