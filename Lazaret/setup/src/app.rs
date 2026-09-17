//! Composition root: typed config, pool, empty registry, deny-all checker, workload identity.

use std::sync::Arc;

use anyhow::Error;
use axum::Router;
use lazaret_application::{empty_command_registry, GrantService, IdentityService, InvokeService};
use lazaret_configuration::AppConfig;
use lazaret_domain::{AsyncKvStore, BindingGrantSnapshotPort, ConnectorRegistry, SecretResolver};
use lazaret_http::{create_app_routes, create_router};
use lazaret_infra::{
    build_identity_service, DeniedSecretResolver, HttpBindingGrantClient, KvPurgeEventConsumer,
    NamedConnectorProxy, PostgresKvStore, RedisKvStore, VaultHttpSecretResolver,
};
use readiness::{attach_ready, ComponentStatus, ReadinessProbe};
use rustycog::command::GenericCommandService;
use rustycog::config::{QueueConfig, ServerConfig};
use rustycog::db::DbConnectionPool;
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
use sea_orm::DatabaseConnection;

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
    kv_event_consumer: Option<Arc<KvPurgeEventConsumer>>,
}

impl Application {
    /// Wire pool, empty command registry, JWT extractor, deny-all checker, identity service.
    ///
    /// # Errors
    ///
    /// Returns an error if database, auth, identity, KV, or queue setup fails.
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
        let kv = build_platform_kv(&config, kv_conn)?;
        let secrets = build_secret_resolver(&config)?;
        let connectors = build_named_connectors(&config)?;
        let kv_event_consumer = maybe_kv_event_consumer(&config.queue, kv.clone()).await?;
        let invoke = Arc::new(InvokeService::new(
            identity.clone(),
            grant_service.clone(),
            kv,
            secrets,
            connectors,
        ));
        let readiness = build_readiness(db_write, kv_event_consumer.as_ref());

        tracing::info!("Lazaret application initialized successfully");

        Ok(Self {
            config,
            state,
            readiness,
            identity,
            grant_service,
            invoke,
            kv_event_consumer,
        })
    }

    /// Start the HTTP server (prefixed router), plus the KV consumer when the queue is live.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP server or a background task fails.
    pub async fn run(self, server_config: ServerConfig) -> Result<(), Error> {
        let background_tasks = self.start_background_tasks();
        if background_tasks.is_empty() {
            tracing::info!("Starting Lazaret HTTP server...");
            return create_app_routes(
                self.state,
                server_config,
                self.readiness,
                self.identity,
                self.invoke,
            )
            .await;
        }

        run_http_with_background(self, server_config, background_tasks).await
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

    /// KV purge consumer when `queue.enabled` and the factory is not a no-op.
    #[must_use]
    pub fn start_background_tasks(&self) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let Some(consumer) = self.kv_event_consumer.clone() else {
            return Vec::new();
        };
        if consumer.is_noop() {
            return Vec::new();
        }
        let handler = consumer.handler();
        vec![tokio::spawn(async move {
            consumer
                .start(handler)
                .await
                .map_err(|e| anyhow::anyhow!("Lazaret KV event consumer failed: {e}"))
        })]
    }

    /// Stop the KV purge consumer when it was wired.
    pub async fn stop_background_tasks(&self) {
        if let Some(consumer) = &self.kv_event_consumer {
            if let Err(e) = consumer.stop().await {
                tracing::error!("Failed to stop Lazaret KV event consumer: {e}");
            }
        }
    }
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

fn build_platform_kv(
    config: &AppConfig,
    kv_conn: DatabaseConnection,
) -> Result<Arc<dyn AsyncKvStore>, Error> {
    if config.kv.backend.eq_ignore_ascii_case("redis") {
        Ok(Arc::new(
            RedisKvStore::connect(&config.redis.url())
                .map_err(|e| anyhow::anyhow!("Redis KV: {e}"))?,
        ))
    } else {
        Ok(Arc::new(PostgresKvStore::new(kv_conn)))
    }
}

fn build_secret_resolver(config: &AppConfig) -> Result<Arc<dyn SecretResolver>, Error> {
    if config.vault.base_url.is_empty() {
        Ok(Arc::new(DeniedSecretResolver))
    } else {
        Ok(Arc::new(
            VaultHttpSecretResolver::new(
                config.vault.base_url.clone(),
                config.vault.token.clone(),
                config.vault.mount.clone(),
            )
            .map_err(|e| anyhow::anyhow!("Vault resolver: {e}"))?,
        ))
    }
}

fn build_named_connectors(config: &AppConfig) -> Result<Arc<NamedConnectorProxy>, Error> {
    let entries: Vec<(String, String)> = config
        .connectors
        .iter()
        .map(|entry| (entry.name.clone(), entry.url.clone()))
        .collect();
    NamedConnectorProxy::new(ConnectorRegistry::from_entries(&entries))
        .map(Arc::new)
        .map_err(|e| anyhow::anyhow!("Connector proxy: {e}"))
}

/// Wire the dedicated KV-events consumer when the queue is enabled and live.
///
/// # Errors
///
/// Returns an error if the rustycog consumer factory fails.
async fn maybe_kv_event_consumer(
    queue: &QueueConfig,
    kv: Arc<dyn AsyncKvStore>,
) -> Result<Option<Arc<KvPurgeEventConsumer>>, Error> {
    if !queue.is_enabled() {
        return Ok(None);
    }
    let consumer = KvPurgeEventConsumer::new(queue, kv)
        .await
        .map_err(|e| anyhow::anyhow!("KV event consumer: {e}"))?;
    // Keep no-op so `/ready` can surface `Degraded` (factory_fallback_noop), like Manifesto.
    Ok(Some(Arc::new(consumer)))
}

fn build_readiness(
    db_write: Arc<DatabaseConnection>,
    consumer: Option<&Arc<KvPurgeEventConsumer>>,
) -> Arc<ReadinessProbe> {
    let mut probe = ReadinessProbe::new("lazaret")
        .with_database(db_write)
        .with_publisher(ComponentStatus::Disabled, None);
    match consumer {
        Some(consumer) if consumer.is_noop() => {
            probe = probe.with_consumer(consumer.transport_status().clone(), None);
        }
        Some(consumer) => {
            probe =
                probe.with_consumer(consumer.transport_status().clone(), Some(consumer.inner()));
        }
        None => {
            probe = probe.with_consumer(ComponentStatus::Disabled, None);
        }
    }
    Arc::new(probe)
}

/// Serve HTTP alongside background tasks (standalone `queue.enabled = true`).
///
/// # Errors
///
/// Returns an error if the HTTP server or a background task fails.
async fn run_http_with_background(
    app: Application,
    server_config: ServerConfig,
    background_tasks: Vec<tokio::task::JoinHandle<anyhow::Result<()>>>,
) -> Result<(), Error> {
    let mut server_handle = {
        let state = app.state.clone();
        let readiness = app.readiness.clone();
        let identity = app.identity.clone();
        let invoke = app.invoke.clone();
        let server_config = server_config.clone();
        tokio::spawn(async move {
            create_app_routes(state, server_config, readiness, identity, invoke)
                .await
                .map_err(|e| anyhow::anyhow!("HTTP server failed: {e}"))
        })
    };

    let mut background_handle = tokio::spawn(async move {
        let mut join_set = tokio::task::JoinSet::new();
        for task in background_tasks {
            join_set.spawn(async move {
                task.await
                    .map_err(|error| anyhow::anyhow!("Lazaret background task panicked: {error}"))?
            });
        }
        while let Some(result) = join_set.join_next().await {
            result.map_err(|error| {
                anyhow::anyhow!("Lazaret background task monitor panicked: {error}")
            })??;
        }
        Ok::<(), Error>(())
    });

    tracing::info!("Starting Lazaret HTTP server and background tasks");
    let shutdown_result: Result<(), Error> = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received; stopping Lazaret runtime");
            Ok(())
        }
        result = &mut background_handle => {
            match result {
                Ok(Ok(())) => {
                    tracing::info!("Lazaret background tasks completed");
                    Ok(())
                }
                Ok(Err(error)) => Err(error),
                Err(error) => Err(anyhow::anyhow!("Lazaret background supervisor task panicked: {error}")),
            }
        }
        result = &mut server_handle => {
            match result {
                Ok(Ok(())) => {
                    tracing::info!("HTTP server completed");
                    Ok(())
                }
                Ok(Err(error)) => Err(error),
                Err(error) => Err(anyhow::anyhow!("HTTP server task panicked: {error}")),
            }
        }
    };

    app.stop_background_tasks().await;
    if !background_handle.is_finished() {
        background_handle.abort();
    }
    if !server_handle.is_finished() {
        server_handle.abort();
    }
    let _ = background_handle.await;
    let _ = server_handle.await;
    shutdown_result
}
