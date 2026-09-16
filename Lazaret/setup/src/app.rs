//! Composition root: typed config, pool, empty registry, deny-all checker, workload identity.

use std::sync::Arc;

use anyhow::Error;
use axum::Router;
use lazaret_application::{empty_command_registry, GrantService, IdentityService, InvokeService};
use lazaret_configuration::AppConfig;
use lazaret_domain::{AsyncKvStore, BindingGrantSnapshotPort, ConnectorRegistry, SecretResolver};
use lazaret_http::{create_app_routes, create_router};
use lazaret_infra::{
    build_identity_service, DeniedSecretResolver, HttpBindingGrantClient, NamedConnectorProxy,
    PostgresKvStore, RedisKvStore, VaultHttpSecretResolver,
};
use readiness::{attach_ready, ComponentStatus, ReadinessProbe};
use rustycog::command::GenericCommandService;
use rustycog::config::ServerConfig;
use rustycog::db::DbConnectionPool;
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};

/// Application context for standalone and monolith embedding.
pub struct Application {
    /// Loaded configuration.
    pub config: AppConfig,
    /// HTTP application state.
    pub state: AppState,
    /// Readiness probe (`GET /ready`).
    pub readiness: Arc<ReadinessProbe>,
    /// Workload identity (enroll/session).
    pub identity: Arc<IdentityService>,
    /// Live Manifesto grant consult (no HTTP call until [`GrantService::authorize`]).
    pub grant_service: Arc<GrantService>,
    invoke: Arc<InvokeService>,
}

impl Application {
    /// Wire pool, empty command registry, JWT extractor, deny-all checker, identity service.
    ///
    /// # Errors
    ///
    /// Returns an error if database, auth, or identity setup fails.
    pub async fn new(config: AppConfig) -> Result<Self, Error> {
        tracing::info!("Initializing Lazaret application...");

        let db = DbConnectionPool::new(&config.database).await?;
        let db_write = db.get_write_connection();
        let kv_conn = db_write.as_ref().clone();

        let command_registry = empty_command_registry();
        let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));

        let user_id_extractor = UserIdExtractor::new(config.auth.clone())
            .map_err(|e| anyhow::anyhow!("Invalid auth configuration: {e}"))?;

        // No OpenFGA type this slice. Deny-all checker satisfies `AppState::new`.
        let permission_checker: Arc<dyn PermissionChecker> =
            Arc::new(InMemoryPermissionChecker::new());

        let state = AppState::new(command_service, user_id_extractor, permission_checker);
        let snapshots: Arc<dyn BindingGrantSnapshotPort> = Arc::new(
            HttpBindingGrantClient::from_config(&config.manifesto_service)
                .map_err(|e| anyhow::anyhow!("Invalid Manifesto service configuration: {e}"))?,
        );
        let identity = build_identity_service(&config.identity, snapshots.clone())
            .map_err(|e| anyhow::anyhow!("Invalid identity configuration: {e}"))?;
        let grant_service = Arc::new(GrantService::new(snapshots));
        let kv: Arc<dyn AsyncKvStore> = if config.kv.backend.eq_ignore_ascii_case("redis") {
            Arc::new(
                RedisKvStore::connect(&config.redis.url())
                    .map_err(|e| anyhow::anyhow!("Redis KV: {e}"))?,
            )
        } else {
            Arc::new(PostgresKvStore::new(kv_conn))
        };
        let secrets: Arc<dyn SecretResolver> = if config.vault.base_url.is_empty() {
            Arc::new(DeniedSecretResolver)
        } else {
            Arc::new(
                VaultHttpSecretResolver::new(
                    config.vault.base_url.clone(),
                    config.vault.token.clone(),
                    config.vault.mount.clone(),
                )
                .map_err(|e| anyhow::anyhow!("Vault resolver: {e}"))?,
            )
        };
        let entries: Vec<(String, String)> = config
            .connectors
            .iter()
            .map(|entry| (entry.name.clone(), entry.url.clone()))
            .collect();
        let connectors = Arc::new(
            NamedConnectorProxy::new(ConnectorRegistry::from_entries(&entries))
                .map_err(|e| anyhow::anyhow!("Connector proxy: {e}"))?,
        );
        let invoke = Arc::new(InvokeService::new(
            identity.clone(),
            grant_service.clone(),
            kv,
            secrets,
            connectors,
        ));
        let readiness = Arc::new(
            ReadinessProbe::new("lazaret")
                .with_database(db_write)
                .with_publisher(ComponentStatus::Disabled, None),
        );

        tracing::info!("Lazaret application initialized successfully");

        Ok(Self {
            config,
            state,
            readiness,
            identity,
            grant_service,
            invoke,
        })
    }

    /// Start the HTTP server (prefixed router).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP server fails.
    pub async fn run(self, server_config: ServerConfig) -> Result<(), Error> {
        tracing::info!("Starting Lazaret HTTP server...");
        create_app_routes(
            self.state,
            server_config,
            self.readiness,
            self.identity,
            self.invoke,
        )
        .await
    }

    /// Unprefixed router for monolith `.nest`.
    pub fn router(&self) -> Router {
        attach_ready(
            create_router(
                self.state.clone(),
                self.identity.clone(),
                self.invoke.clone(),
            ),
            self.readiness.clone(),
        )
    }

    /// Shared readiness probe.
    #[must_use]
    pub fn readiness(&self) -> Arc<ReadinessProbe> {
        self.readiness.clone()
    }

    /// No background consumers this slice.
    #[must_use]
    pub const fn start_background_tasks(&self) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        Vec::new()
    }

    /// No-op stop (no background tasks).
    #[allow(clippy::unused_async)]
    pub async fn stop_background_tasks(&self) {}
}

/// Application builder for Lazaret.
pub struct AppBuilder {
    config: AppConfig,
}

impl AppBuilder {
    /// Create a new app builder.
    #[must_use]
    pub const fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Build the application.
    ///
    /// # Errors
    ///
    /// Returns an error if application initialization fails.
    pub async fn build(self) -> Result<Application, anyhow::Error> {
        Application::new(self.config).await
    }
}
