//! Typed configuration for GitLab Connect.

use rustycog::config::{HasLoggingConfig, HasServerConfig};
use serde::{Deserialize, Serialize};

pub use rustycog::config::{
    load_config_fresh, AuthConfig, ConfigError, ConfigLoader, JwtAuthConfig, LoggingConfig,
    ServerConfig,
};
pub use rustycog::logger::setup_logging;

/// GitLab vendor + S2S HMAC settings.
#[derive(Clone, Serialize, Deserialize)]
pub struct GitLabConfig {
    /// OAuth application client id (not a secret).
    #[serde(default)]
    pub client_id: String,
    /// OAuth application client secret. Never log this value.
    #[serde(default)]
    pub client_secret: String,
    /// Authorization endpoint.
    #[serde(default = "default_auth_url")]
    pub auth_url: String,
    /// Token endpoint.
    #[serde(default = "default_token_url")]
    pub token_url: String,
    /// User profile endpoint.
    #[serde(default = "default_user_url")]
    pub user_url: String,
    /// Shared HMAC secret for IAM ↔ connector. Empty is rejected at boot.
    #[serde(default)]
    pub hmac_secret: String,
    /// Exact-match allowlist of OAuth `redirect_uri` values.
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

impl Default for GitLabConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            auth_url: default_auth_url(),
            token_url: default_token_url(),
            user_url: default_user_url(),
            hmac_secret: String::new(),
            redirect_uris: Vec::new(),
        }
    }
}

impl std::fmt::Debug for GitLabConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitLabConfig")
            .field("client_id", &self.client_id)
            .field("client_secret", &"***")
            .field("auth_url", &self.auth_url)
            .field("token_url", &self.token_url)
            .field("user_url", &self.user_url)
            .field("hmac_secret", &"***")
            .field("redirect_uris", &self.redirect_uris)
            .finish()
    }
}

fn default_auth_url() -> String {
    "https://gitlab.com/oauth/authorize".to_owned()
}

fn default_token_url() -> String {
    "https://gitlab.com/oauth/token".to_owned()
}

fn default_user_url() -> String {
    "https://gitlab.com/api/v4/user".to_owned()
}

/// Main application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// HTTP server bind (TLS like other platform services).
    #[serde(default)]
    pub server: ServerConfig,
    /// JWT consumer for rustycog `UserIdExtractor` construction only.
    #[serde(default)]
    pub auth: AuthConfig,
    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,
    /// GitLab vendor + HMAC.
    #[serde(default)]
    pub gitlab: GitLabConfig,
}

impl ConfigLoader<Self> for AppConfig {
    fn create_default() -> Self {
        Self::default()
    }

    fn config_prefix() -> &'static str {
        "GITLAB_CONNECT"
    }
}

impl HasServerConfig for AppConfig {
    fn server_config(&self) -> &ServerConfig {
        &self.server
    }

    fn set_server_config(&mut self, config: ServerConfig) {
        self.server = config;
    }
}

impl HasLoggingConfig for AppConfig {
    fn logging_config(&self) -> &LoggingConfig {
        &self.logging
    }

    fn set_logging_config(&mut self, config: LoggingConfig) {
        self.logging = config;
    }
}

/// Load configuration from environment and config files.
///
/// # Errors
///
/// Returns [`ConfigError`] if the configuration cannot be loaded.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    load_config_fresh::<AppConfig>()
}

#[cfg(test)]
mod tests {
    use super::GitLabConfig;

    #[test]
    fn debug_hides_client_secret_and_hmac_secret() {
        let config = GitLabConfig {
            client_id: "id".to_owned(),
            client_secret: "super-secret".to_owned(),
            hmac_secret: "hmac-secret".to_owned(),
            ..GitLabConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("super-secret"));
        assert!(!debug.contains("hmac-secret"));
        assert!(debug.contains("***"));
    }
}
