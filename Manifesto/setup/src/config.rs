//! Configuration utilities

use manifesto_configuration::ManifestoConfig;

pub use manifesto_configuration::setup_logging;
pub use rustycog::config::ServerConfig;

pub fn load_config() -> anyhow::Result<ManifestoConfig> {
    manifesto_configuration::load_config().map_err(Into::into)
}
