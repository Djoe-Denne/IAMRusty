//! Atomic startup composition and SDK-owned HTTP execution for `OAuthConnect`.

pub mod app;
mod composition;
mod mixed;
mod secrets;
mod transport;

pub use app::{AppBuilder, Application};
pub use composition::{build_oauth_clients, ConfiguredOAuthClient};
pub use mixed::{build_clients, ConfiguredClient};
pub use rustycog::http::{HttpShutdownBudget, HttpShutdownReport, OwnedHttpServer};
pub use secrets::{resolve_secrets, ResolvedProviderSecrets};
pub use transport::build_http_client;
