//! Minimal startup boundaries; no listener, registry or OIDC runtime is published.

mod composition;
mod secrets;
mod transport;

pub use composition::{build_oauth_clients, ConfiguredOAuthClient};
pub use secrets::{resolve_secrets, ResolvedProviderSecrets};
pub use transport::build_http_client;
