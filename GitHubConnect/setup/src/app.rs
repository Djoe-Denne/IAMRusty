//! Composition root: empty command registry, deny-all checker, GitHub HMAC router.

use std::sync::Arc;

use anyhow::Error;
use axum::Router;
use github_connect_configuration::AppConfig;
use github_connect_http::{create_app_routes, create_prefixed_router, create_router};
use github_connect_infra::GitHubConnectClient;
use idp_connect_contract::hmac::HmacKey;
use idp_connect_contract::server::ConnectState;
use readiness::ReadinessProbe;
use rustycog::command::{CommandRegistry, CommandRegistryBuilder, GenericCommandService};
use rustycog::config::ServerConfig;
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};

/// Mirror IAM `HttpIdpConnector` (`IAMRusty/infra/src/auth/http_connector.rs`).
const MIN_HMAC_SECRET_LEN: usize = 16;

/// Application context for standalone serving.
pub struct Application {
    /// Loaded configuration.
    pub config: AppConfig,
    /// HTTP application state (extractor constructed, unused on `/v1/*`).
    pub state: AppState,
    /// HMAC + federated client for the contract router.
    pub connect: ConnectState,
    /// Readiness probe (`GET /ready`).
    pub readiness: Arc<ReadinessProbe>,
}

/// Build an empty command registry (no handlers this slice).
#[must_use]
pub fn empty_command_registry() -> CommandRegistry {
    CommandRegistryBuilder::new().build()
}

impl Application {
    /// Wire empty registry, JWT extractor, deny-all checker, GitHub vendor client.
    ///
    /// # Errors
    ///
    /// Returns an error if `hmac_secret` is empty, whitespace-only, shorter
    /// than 16 bytes after trim, auth config is invalid, or GitHub vendor
    /// URLs cannot be parsed.
    pub fn new(config: AppConfig) -> Result<Self, Error> {
        tracing::info!("Initializing GitHub Connect application...");

        let hmac_secret = config.github.hmac_secret.trim();
        if hmac_secret.is_empty() {
            return Err(anyhow::anyhow!("hmac_secret must not be empty"));
        }
        if hmac_secret.len() < MIN_HMAC_SECRET_LEN {
            return Err(anyhow::anyhow!(
                "hmac_secret must be at least {MIN_HMAC_SECRET_LEN} bytes"
            ));
        }
        let hmac_key = HmacKey::new(hmac_secret.as_bytes());

        let command_registry = empty_command_registry();
        let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));

        // rustycog AppState requires UserIdExtractor; connector authn is HMAC not bearer.
        let user_id_extractor = UserIdExtractor::new(config.auth.clone())
            .map_err(|e| anyhow::anyhow!("Invalid auth configuration: {e}"))?;

        let permission_checker: Arc<dyn PermissionChecker> =
            Arc::new(InMemoryPermissionChecker::new());

        let state = AppState::new(command_service, user_id_extractor, permission_checker);
        let client = Arc::new(GitHubConnectClient::from_config(&config.github)?);
        let connect = ConnectState { hmac_key, client };
        let readiness = Arc::new(ReadinessProbe::new("github-connect-service"));

        tracing::info!("GitHub Connect application initialized successfully");

        Ok(Self {
            config,
            state,
            connect,
            readiness,
        })
    }

    /// Start the HTTP server (prefixed router).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP server fails.
    pub async fn run(self, server_config: ServerConfig) -> Result<(), Error> {
        tracing::info!("Starting GitHub Connect HTTP server...");
        create_app_routes(self.state, self.connect, server_config, self.readiness).await
    }

    /// Unprefixed router (health + `/v1/*`). Not nested into the monolith in v1.
    pub fn router(&self) -> Router {
        create_router(self.state.clone(), self.connect.clone())
    }

    /// Router nested under `SERVICE_PREFIX` for standalone and integration tests.
    pub fn prefixed_router(&self) -> Router {
        create_prefixed_router(
            self.state.clone(),
            self.connect.clone(),
            self.readiness.clone(),
        )
    }

    /// Shared readiness probe.
    #[must_use]
    pub fn readiness(&self) -> Arc<ReadinessProbe> {
        Arc::clone(&self.readiness)
    }

    /// No background tasks this slice.
    #[must_use]
    pub const fn start_background_tasks(&self) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        Vec::new()
    }

    /// No background tasks this slice. Signature kept for the rustycog service shape.
    pub const fn stop_background_tasks(&self) {}
}

/// Builder for [`Application`].
pub struct AppBuilder {
    config: AppConfig,
}

impl AppBuilder {
    /// Capture typed config for later build.
    #[must_use]
    pub const fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Build the application.
    ///
    /// # Errors
    ///
    /// Returns an error if application initialization fails.
    pub fn build(self) -> Result<Application, Error> {
        Application::new(self.config)
    }
}
