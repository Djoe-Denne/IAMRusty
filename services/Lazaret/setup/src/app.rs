//! Composition root: typed config, pool, empty registry, deny-all checker, workload identity.

use std::path::Path;
use std::sync::Arc;

use anyhow::Error;
use axum::Router;
use lazaret_application::{
    empty_command_registry, EmptyPluginLocator, GrantService, IdentityService, InvokeService,
    NamespacedDigestDnsPluginLocator, PluginEndpointLocator, StaticPluginLocator,
};
use lazaret_configuration::AppConfig;
use lazaret_domain::{
    AsyncKvStore, BindingGrantSnapshotPort, ConnectorRegistry, EnrollmentStore, IdentityError,
    SecretResolver,
};
use lazaret_http::{create_app_routes, create_prefixed_router, create_router};
use lazaret_infra::{
    build_identity_service, DeniedSecretResolver, HttpBindingGrantClient, KvPurgeEventConsumer,
    NamedConnectorProxy, PlatformInternalCa, PostgresKvStore, RedisKvStore,
    VaultHttpSecretResolver,
};
use readiness::{attach_ready, ComponentStatus, ReadinessProbe};
use rustycog::command::GenericCommandService;
use rustycog::config::{QueueConfig, ServerConfig};
use rustycog::db::DbConnectionPool;
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::permission::{InMemoryPermissionChecker, PermissionChecker};
use sea_orm::DatabaseConnection;

/// Host-injected outbound adapters for Lazaret (ADR-0104). `Default` = HTTP clients.
#[derive(Clone, Default)]
pub struct LazaretOutboundOverrides {
    /// Manifesto binding-grant snapshots. `None` → HTTP.
    pub binding_grant_snapshots: Option<Arc<dyn BindingGrantSnapshotPort>>,
}

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
    pub async fn new(
        config: AppConfig,
        overrides: LazaretOutboundOverrides,
    ) -> Result<Self, Error> {
        tracing::info!("Initializing Lazaret application...");
        let injected_snapshots = overrides.binding_grant_snapshots;

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
        let (snapshots, _) = resolve_binding_grant_snapshots(&config, injected_snapshots)?;
        let (identity, ca) =
            build_identity_service(&config.identity, snapshots.clone(), kv_conn.clone())
                .map_err(|e| anyhow::anyhow!("Invalid identity configuration: {e}"))?;
        ensure_boot_tls(&ca, &config.server)
            .map_err(|e| anyhow::anyhow!("Invalid TLS material: {e}"))?;
        let grant_service = Arc::new(GrantService::new(snapshots));
        let kv = build_platform_kv(&config, kv_conn)?;
        let secrets = build_secret_resolver(&config)?;
        let connectors = build_named_connectors(&config)?;
        let kv_event_consumer =
            maybe_kv_event_consumer(&config.queue, kv.clone(), identity.enrollment_store()).await?;
        let invoke = Arc::new(InvokeService::new_with_locator(
            identity.clone(),
            grant_service.clone(),
            kv,
            secrets,
            connectors,
            plugin_endpoint_locator(&config)?,
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

    /// Router nested under `SERVICE_PREFIX` for standalone/IT only.
    ///
    /// The monolith must call [`Self::router`] then nest the prefix once.
    /// Nesting this router again yields `/lazaret/lazaret` (tests assert 404).
    pub fn prefixed_router(&self) -> Router {
        create_prefixed_router(
            self.state.clone(),
            self.readiness.clone(),
            self.identity.clone(),
            self.invoke.clone(),
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

/// Resolve Manifesto binding-grant snapshots (ADR-0104).
///
/// Without injection → HTTP adapter. With setter → injected capability (InProcess).
///
/// Returns `(client, used_injected)` so unit tests can prove the transport choice.
pub(crate) fn resolve_binding_grant_snapshots(
    config: &AppConfig,
    injected: Option<Arc<dyn BindingGrantSnapshotPort>>,
) -> Result<(Arc<dyn BindingGrantSnapshotPort>, bool), Error> {
    if let Some(snapshots) = injected {
        return Ok((snapshots, true));
    }
    let http = Arc::new(
        HttpBindingGrantClient::from_config(&config.manifesto_service)
            .map_err(|e| anyhow::anyhow!("Invalid Manifesto service configuration: {e}"))?,
    ) as Arc<dyn BindingGrantSnapshotPort>;
    Ok((http, false))
}

/// Application builder for Lazaret.
pub struct AppBuilder {
    config: AppConfig,
    overrides: LazaretOutboundOverrides,
}

impl AppBuilder {
    /// Create a new app builder (default outbound transport = HTTP).
    #[must_use]
    pub const fn new(config: AppConfig) -> Self {
        Self {
            config,
            overrides: LazaretOutboundOverrides {
                binding_grant_snapshots: None,
            },
        }
    }

    /// Replace the outbound-overrides bag (monolith host API, ADR-0104).
    #[must_use]
    pub fn with_outbound(mut self, bag: LazaretOutboundOverrides) -> Self {
        self.overrides = bag;
        self
    }

    /// Inject an InProcess (or test) binding-grant snapshot port.
    ///
    /// Sugar over [`Self::with_outbound`]: writes `binding_grant_snapshots`.
    #[must_use]
    pub fn with_binding_grant_snapshots(
        mut self,
        snapshots: Arc<dyn BindingGrantSnapshotPort>,
    ) -> Self {
        self.overrides.binding_grant_snapshots = Some(snapshots);
        self
    }

    /// Build the application.
    ///
    /// # Errors
    ///
    /// Returns an error if application initialization fails.
    pub async fn build(self) -> Result<Application, anyhow::Error> {
        Application::new(self.config, self.overrides).await
    }
}

fn ensure_boot_tls(ca: &PlatformInternalCa, server: &ServerConfig) -> Result<(), IdentityError> {
    if !server.tls_enabled {
        return Ok(());
    }
    ca.ensure_server_leaf(
        Path::new(&server.tls_cert_path),
        Path::new(&server.tls_key_path),
    )?;
    if !server.tls_client_ca_path.trim().is_empty() {
        ca.write_trust_anchor(Path::new(&server.tls_client_ca_path))?;
    }
    Ok(())
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

fn plugin_endpoint_locator(config: &AppConfig) -> Result<Arc<dyn PluginEndpointLocator>, Error> {
    if config.plugin_hop.use_dns_formula {
        config.plugin_hop.validate().map_err(anyhow::Error::msg)?;
        return Ok(Arc::new(NamespacedDigestDnsPluginLocator::try_new(
            config.plugin_hop.namespace.clone(),
        )?));
    }
    let url = config.plugin_hop.endpoint_url.trim();
    if url.is_empty() {
        Ok(Arc::new(EmptyPluginLocator))
    } else {
        Ok(Arc::new(StaticPluginLocator::new(url.to_owned())))
    }
}

/// Wire the dedicated KV-events consumer when the queue is enabled and live.
///
/// # Errors
///
/// Returns an error if the rustycog consumer factory fails.
async fn maybe_kv_event_consumer(
    queue: &QueueConfig,
    kv: Arc<dyn AsyncKvStore>,
    enrollments: Arc<dyn EnrollmentStore>,
) -> Result<Option<Arc<KvPurgeEventConsumer>>, Error> {
    if !queue.is_enabled() {
        return Ok(None);
    }
    let consumer = KvPurgeEventConsumer::new(queue, kv, enrollments)
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

#[cfg(test)]
mod resolve_binding_grant_tests {
    use super::*;
    use async_trait::async_trait;
    use lazaret_domain::{BindingGrantSnapshot, GrantFetchError};
    use uuid::Uuid;

    struct CapturingGrantPort;

    #[async_trait]
    impl BindingGrantSnapshotPort for CapturingGrantPort {
        async fn fetch(
            &self,
            _project_id: Uuid,
            _component_id: Uuid,
            _principal: Option<Uuid>,
        ) -> Result<BindingGrantSnapshot, GrantFetchError> {
            unreachable!("resolver test must not call fetch")
        }
    }

    fn minimal_config() -> AppConfig {
        AppConfig::default()
    }

    #[test]
    fn without_injection_builds_http_client() {
        let config = minimal_config();
        let (client, used_injected) =
            resolve_binding_grant_snapshots(&config, None).expect("http client builds");
        assert!(!used_injected, "default transport must be HTTP");
        let injected: Arc<dyn BindingGrantSnapshotPort> = Arc::new(CapturingGrantPort);
        let (resolved, injected_flag) =
            resolve_binding_grant_snapshots(&config, Some(injected.clone())).expect("inject");
        assert!(injected_flag);
        assert!(Arc::ptr_eq(&resolved, &injected));
        assert!(!Arc::ptr_eq(&client, &resolved));
    }

    #[test]
    fn with_injection_uses_provided_client_not_http() {
        let config = minimal_config();
        let injected: Arc<dyn BindingGrantSnapshotPort> = Arc::new(CapturingGrantPort);
        let (resolved, used_injected) =
            resolve_binding_grant_snapshots(&config, Some(injected.clone())).expect("inject");
        assert!(used_injected);
        assert!(
            Arc::ptr_eq(&resolved, &injected),
            "injected client must be the one threaded into the bundle"
        );
    }
}
