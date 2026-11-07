//! Complete startup before admission, with caller-owned SDK HTTP execution.

use std::{
    fmt,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use idp_connect_contract::server::ConnectState;
use oauth_connect_configuration::{AppConfig, ServerConfig};
use oauth_connect_http::create_prefixed_router;
use readiness::ReadinessProbe;
use rustycog::command::{CommandRegistryBuilder, GenericCommandService};
use rustycog::core::error::DomainError;
use rustycog::http::{
    start_router_owned, AppState, HttpShutdownBudget, OwnedHttpServer, UserIdExtractor,
};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
use tokio::time::Instant;

use crate::build_clients;

/// Opaque, fully initialized application, created before admitting connections.
pub struct Application {
    router: axum::Router,
    server_config: ServerConfig,
}

impl fmt::Debug for Application {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Application([REDACTED])")
    }
}

impl Application {
    /// Prepare the whole mixed catalogue and HTTP state before any listener bind.
    ///
    /// # Errors
    /// Refuses any incomplete catalogue or invalid HTTP state without binding a
    /// listener. Errors never include secrets, remote data or private file paths.
    pub async fn new(config: AppConfig) -> Result<Self, DomainError> {
        let clients = build_clients(&config).await?;
        let commands = Arc::new(CommandRegistryBuilder::new().build());
        let command_service = Arc::new(GenericCommandService::new(commands));
        let extractor = hmac_only_user_id_extractor()?;
        let permissions: Arc<dyn PermissionChecker> = Arc::new(InMemoryPermissionChecker::new());
        let state = AppState::new(command_service, extractor, permissions);
        let entries = clients
            .into_iter()
            .map(|configured| {
                (
                    configured.id,
                    ConnectState {
                        client: configured.client,
                        hmac_key: configured.hmac_key,
                    },
                )
            })
            .collect();
        let probe = Arc::new(
            ReadinessProbe::new("oauth-connect-service")
                .with_liveness("catalogue", Arc::new(AtomicBool::new(true))),
        );
        let router = create_prefixed_router(state, entries, probe).map_err(|_| failed())?;
        Ok(Self {
            router,
            server_config: config.server,
        })
    }

    /// Start an SDK-owned listener, whose effective addresses support port zero.
    ///
    /// The implementation uses SDK owned startup; callers retain explicit
    /// shutdown/join ownership. Only successful binds may produce address logs.
    ///
    /// # Errors
    /// Refuses failed binds or TLS preparation with sanitized diagnostics.
    pub async fn start(&self) -> Result<OwnedHttpServer, DomainError> {
        let server = start_router_owned(self.router.clone(), self.server_config.clone())
            .await
            .map_err(|_| failed())?;
        for address in server.local_addrs() {
            tracing::info!("OAuthConnect listening on {address}");
        }
        Ok(server)
    }

    /// Run the service under signal supervision and join its owned HTTP resources.
    ///
    /// The lifecycle uses the existing five/six-second shutdown budget,
    /// not a test shutdown endpoint or a second HTTP runtime.
    ///
    /// # Errors
    /// Reports startup, signal, unexpected HTTP exit or incomplete/forced shutdown
    /// failures without exposing the SDK's private diagnostic data.
    pub async fn run(self) -> Result<(), DomainError> {
        let mut server = self.start().await?;
        let outcome = tokio::select! {
            signal = tokio::signal::ctrl_c() => signal.map_err(|_| failed()),
            _ = server.wait_for_server_exit() => Err(failed()),
        };
        let now = Instant::now();
        let budget =
            HttpShutdownBudget::new(now + Duration::from_secs(5), now + Duration::from_secs(6))
                .map_err(|_| failed())?;
        server.request_shutdown(budget);
        let report = server.join().await.map_err(|_| failed())?;
        if report.forced {
            return Err(failed());
        }
        outcome
    }
}

/// Typed configuration capture, preserving the existing setup builder pattern.
#[derive(Debug)]
pub struct AppBuilder {
    config: AppConfig,
}

impl AppBuilder {
    /// Capture configuration without validation, I/O or application construction.
    #[must_use]
    pub const fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Prepare an application through the same startup entry as direct callers.
    ///
    /// # Errors
    /// Returns the sanitized startup errors from `Application::new`.
    pub async fn build(self) -> Result<Application, DomainError> {
        Application::new(self.config).await
    }
}

// AppState requires this field, but this service has no Bearer routes. Empty
// inline trust accepts no JWT and acquires no keys or identities from elsewhere.
fn hmac_only_user_id_extractor() -> Result<UserIdExtractor, DomainError> {
    UserIdExtractor::from_inline_jwks(r#"{"keys":[]}"#, None).map_err(|_| failed())
}

fn failed() -> DomainError {
    DomainError::InvalidInput {
        message: "OAuthConnect service lifecycle failed".to_owned(),
    }
}
