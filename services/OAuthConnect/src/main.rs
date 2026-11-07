//! Standalone `OAuthConnect` entry point with SDK logging and owned HTTP startup.

use oauth_connect_configuration::{load_config, LoggingConfig};
use oauth_connect_setup::AppBuilder;
use rustycog::config::HasLoggingConfig;
use rustycog::core::error::DomainError;

// AppConfig deliberately remains unchanged. This local adapter only supplies the
// SDK logger trait, rather than adding another loader or tracing subscriber.
struct BinaryLogging(LoggingConfig);

impl HasLoggingConfig for BinaryLogging {
    fn logging_config(&self) -> &LoggingConfig {
        &self.0
    }

    fn set_logging_config(&mut self, config: LoggingConfig) {
        self.0 = config;
    }
}

/// Load strict configuration, initialize SDK logging once and enter setup.
///
/// # Errors
/// Returns a sanitized error for configuration or application startup failure.
/// No listener is started before the complete application has initialized.
#[tokio::main]
async fn main() -> Result<(), DomainError> {
    let config = load_config().map_err(|_| DomainError::InvalidInput {
        message: "Invalid OAuthConnect configuration".to_owned(),
    })?;
    rustycog::logger::setup_logging(&BinaryLogging(config.logging.clone()));
    AppBuilder::new(config).build().await?.run().await
}
