use std::sync::Arc;

use axum::Router;

// Hive
use hive_application::{
    ExternalLinkUseCaseImpl, HiveCommandRegistryFactory, InvitationUseCaseImpl, MemberUseCaseImpl,
    OrganizationUseCaseImpl, RoleUseCaseImpl, SyncJobUseCaseImpl,
};
use hive_configuration::AppConfig;
use hive_domain::service::{
    external_provider_service::ExternalProviderServiceImpl,
    invitation_service::InvitationServiceImpl, member_service::MemberServiceImpl,
    organization_service::OrganizationServiceImpl, role_service::RoleServiceImpl,
    sync_service::SyncServiceImpl,
};
use hive_http::{create_app_routes, create_router};
use hive_infra::{
    external_provider::external_provider_client::HttpExternalProviderClient,
    iam::{HttpIamOrganizationSignerClient, StaticCredential},
    repository::{
        ExternalLinkReadRepositoryImpl, ExternalLinkRepositoryImpl,
        ExternalLinkWriteRepositoryImpl, ExternalProviderReadRepositoryImpl,
        ExternalProviderRepositoryImpl, ExternalProviderWriteRepositoryImpl,
        MemberRoleReadRepositoryImpl, MemberRoleRepositoryImpl, MemberRoleWriteRepositoryImpl,
        OrganizationInvitationReadRepositoryImpl, OrganizationInvitationRepositoryImpl,
        OrganizationInvitationWriteRepositoryImpl, OrganizationMemberReadRepositoryImpl,
        OrganizationMemberRepositoryImpl, OrganizationMemberWriteRepositoryImpl,
        OrganizationReadRepositoryImpl, OrganizationRepositoryImpl,
        OrganizationWriteRepositoryImpl, PermissionReadRepositoryImpl, PermissionRepositoryImpl,
        ResourceReadRepositoryImpl, ResourceRepositoryImpl, RolePermissionReadRepositoryImpl,
        RolePermissionRepositoryImpl, RolePermissionWriteRepositoryImpl, SyncJobReadRepositoryImpl,
        SyncJobRepositoryImpl, SyncJobWriteRepositoryImpl,
    },
    HiveErrorMapper, HiveOutboxUnitOfWorkImpl,
};

// Rustycog
use readiness::{attach_ready, create_signaled_multi_queue_event_publisher, ReadinessProbe};
use rustycog::command::GenericCommandService;
use rustycog::config::ServerConfig;
use rustycog::core::error::DomainError;
use rustycog::db::DbConnectionPool;
use rustycog::events::EventPublisher;
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::outbox::{OutboxConfig, OutboxDispatcher, OutboxRecorder};
use rustycog::permission::{
    CachedPermissionChecker, MetricsPermissionChecker, OpenFgaPermissionChecker, PermissionChecker,
};
use std::time::Duration;

// External
use anyhow::Error;

type ApplicationUseCases = (
    Arc<dyn hive_application::OrganizationUseCase>,
    Arc<dyn hive_application::MemberUseCase>,
    Arc<dyn hive_application::InvitationUseCase>,
    Arc<dyn hive_application::ExternalLinkUseCase>,
    Arc<dyn hive_application::SyncJobUseCase>,
    Arc<dyn hive_application::RoleUseCase>,
    Arc<dyn hive_domain::port::service::IamOrganizationSignerClient>,
    Arc<dyn hive_domain::OrganizationRepository>,
);

type DomainServices = (
    Arc<dyn hive_domain::service::OrganizationService>,
    Arc<dyn hive_domain::service::MemberService>,
    Arc<dyn hive_domain::service::InvitationService>,
    Arc<dyn hive_domain::service::ExternalProviderService>,
    Arc<dyn hive_domain::service::RoleService>,
    Arc<dyn hive_domain::service::SyncService>,
);

type RepositoryBundle = (
    Arc<OrganizationRepositoryImpl>,
    Arc<OrganizationMemberRepositoryImpl>,
    Arc<OrganizationInvitationRepositoryImpl>,
    Arc<ExternalLinkRepositoryImpl>,
    Arc<ExternalProviderRepositoryImpl>,
    Arc<SyncJobRepositoryImpl>,
    Arc<ResourceRepositoryImpl>,
    Arc<PermissionRepositoryImpl>,
    Arc<RolePermissionRepositoryImpl>,
    Arc<MemberRoleRepositoryImpl>,
    Arc<HttpExternalProviderClient>,
    Arc<dyn hive_domain::port::service::IamOrganizationSignerClient>,
);

/// IAM organization-signer port used by Hive (HTTP or InProcess).
pub type IamSignerClient = Arc<dyn hive_domain::port::service::IamOrganizationSignerClient>;

/// Host-injected outbound adapters for Hive (ADR-0104). `Default` = HTTP clients.
#[derive(Clone, Default)]
pub struct HiveOutboundOverrides {
    /// IAM organization-signer client. `None` → HTTP (ADR-0306).
    pub iam_organization_signer: Option<IamSignerClient>,
}

/// Application context for dependency injection
pub struct Application {
    pub config: AppConfig,
    pub state: AppState,
    pub outbox_dispatcher: Arc<OutboxDispatcher<DomainError>>,
    pub readiness: Arc<ReadinessProbe>,
}

impl Application {
    /// Create a new application instance with all dependencies.
    ///
    /// # Errors
    ///
    /// Returns an error if database, event publisher, auth, or `OpenFGA` setup fails.
    pub async fn new(config: AppConfig, overrides: HiveOutboundOverrides) -> Result<Self, Error> {
        tracing::info!("Initializing Hive application...");
        let iam_signer_client = overrides.iam_organization_signer;

        // Setup database connection
        let db = setup_database(&config).await?;
        let db_write = db.get_write_connection();

        // Setup event publisher for Telegraph + sentinel-sync communication
        let signaled = create_signaled_multi_queue_event_publisher(
            "hive",
            &config.queue,
            None,
            Arc::new(HiveErrorMapper),
        )
        .await?;
        let event_publisher = signaled.publisher;
        let publisher_status = signaled.status;
        let publisher_transport = signaled.transport;
        let outbox_dispatcher = Arc::new(OutboxDispatcher::new(
            db.clone(),
            event_publisher.clone(),
            OutboxConfig::default(),
        ));

        // Setup use cases
        let (
            organization_usecase,
            member_usecase,
            invitation_usecase,
            external_link_usecase,
            sync_job_usecase,
            role_usecase,
            iam_signer_client,
            organization_repo,
        ) = setup_application(db, &config, event_publisher, iam_signer_client)?;

        // Setup command registry
        let command_registry = HiveCommandRegistryFactory::create_hive_registry(
            organization_usecase,
            member_usecase,
            invitation_usecase,
            external_link_usecase,
            sync_job_usecase,
            role_usecase,
            iam_signer_client,
            organization_repo,
            &config.command,
        );

        // Create command service
        let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));

        // RS256 JWKS verifier (`[auth.jwt]` with jwks_url / allowed_algorithms).
        let user_id_extractor = UserIdExtractor::new(config.auth.clone())
            .map_err(|e| anyhow::anyhow!("Invalid auth configuration: {e}"))?;

        // Centralized permission checker (OpenFGA). Built once and shared
        // across every request through `AppState`. The checker chain is:
        //   MetricsPermissionChecker
        //     -> CachedPermissionChecker (short TTL LRU, optional)
        //       -> OpenFgaPermissionChecker (network)
        //
        // The cache is the production default (15s) but can be disabled at
        // test time by setting `openfga.cache_ttl_seconds = 0` so flows
        // that need to observe a freshly re-arranged decision (or a
        // wildcard subject from `optional_permission_middleware`) are not
        // masked by a stale cached entry.
        let raw_checker: Arc<dyn PermissionChecker> = Arc::new(
            OpenFgaPermissionChecker::new(config.openfga.clone())
                .map_err(|e| anyhow::anyhow!("Invalid OpenFGA configuration: {e}"))?,
        );
        let cache_ttl_seconds = config.openfga.cache_ttl_seconds.unwrap_or(15);
        let metered_inner: Arc<dyn PermissionChecker> = if cache_ttl_seconds == 0 {
            raw_checker
        } else {
            Arc::new(CachedPermissionChecker::new(
                raw_checker,
                Duration::from_secs(cache_ttl_seconds),
                10_000,
            ))
        };
        let permission_checker: Arc<dyn PermissionChecker> =
            Arc::new(MetricsPermissionChecker::new(metered_inner));

        // Create application state
        let state = AppState::new(command_service, user_id_extractor, permission_checker);
        let readiness = Arc::new(
            ReadinessProbe::new("hive")
                .with_database(db_write)
                .with_publisher(publisher_status, Some(publisher_transport)),
        );

        tracing::info!("Hive application initialized successfully");

        Ok(Self {
            config,
            state,
            outbox_dispatcher,
            readiness,
        })
    }

    /// Start the HTTP server.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP server or outbox dispatcher fails, or if a
    /// background task panics.
    pub async fn run(self, server_config: ServerConfig) -> Result<(), Error> {
        tracing::info!("Starting Hive HTTP server...");

        let mut server_handle = {
            let state = self.state.clone();
            let probe = self.readiness.clone();
            tokio::spawn(async move {
                create_app_routes(state, server_config, probe)
                    .await
                    .map_err(|e| anyhow::anyhow!("Server startup failed: {e}"))
            })
        };

        let mut background_tasks = self.start_background_tasks();
        let mut outbox_handle = background_tasks
            .pop()
            .ok_or_else(|| anyhow::anyhow!("Hive outbox dispatcher should always be configured"))?;

        let result: Result<(), Error> = tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Shutdown signal received; stopping Hive runtime");
                Ok(())
            }
            result = &mut outbox_handle => {
                match result {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(error)) => Err(error),
                    Err(error) => Err(anyhow::anyhow!("Hive outbox dispatcher task panicked: {error}")),
                }
            }
            result = &mut server_handle => {
                match result {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(error)) => Err(error),
                    Err(error) => Err(anyhow::anyhow!("Hive HTTP server task panicked: {error}")),
                }
            }
        };

        self.stop_background_tasks().await;
        if !outbox_handle.is_finished() {
            outbox_handle.abort();
        }
        if !server_handle.is_finished() {
            server_handle.abort();
        }
        let _ = outbox_handle.await;
        let _ = server_handle.await;

        result
    }

    pub fn router(&self) -> Router {
        attach_ready(create_router(self.state.clone()), self.readiness.clone())
    }

    #[must_use]
    pub fn readiness(&self) -> Arc<ReadinessProbe> {
        self.readiness.clone()
    }

    #[must_use]
    pub fn start_background_tasks(&self) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let dispatcher = self.outbox_dispatcher.clone();
        vec![tokio::spawn(async move {
            dispatcher
                .start()
                .await
                .map_err(|e| anyhow::anyhow!("Hive outbox dispatcher failed: {e}"))
        })]
    }

    pub async fn stop_background_tasks(&self) {
        if let Err(e) = self.outbox_dispatcher.stop().await {
            tracing::error!("Failed to stop Hive outbox dispatcher: {e}");
        }
    }
}

/// Setup database connection
async fn setup_database(config: &AppConfig) -> Result<DbConnectionPool, Error> {
    tracing::info!("Connecting to database");

    // Setup database connection pool
    let db_pool = DbConnectionPool::new(&config.database).await?;
    tracing::info!(
        "Database connection pool initialized with {} read replicas",
        if config.database.read_replicas.is_empty() {
            0
        } else {
            config.database.read_replicas.len()
        }
    );
    tracing::info!("Database connection established");
    Ok(db_pool)
}

/// Setup use cases with their dependencies
fn setup_application(
    db: DbConnectionPool,
    config: &AppConfig,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
    iam_signer_client: Option<IamSignerClient>,
) -> Result<ApplicationUseCases, Error> {
    let (
        organization_service,
        member_service,
        invitation_service,
        external_provider_service,
        role_service,
        sync_service,
        iam_signer_client,
        organization_repo,
    ) = setup_domain(&db, config, iam_signer_client)?;

    let outbox_unit_of_work = Arc::new(HiveOutboxUnitOfWorkImpl::new(db, OutboxRecorder));

    // Create organization use case
    let organization_usecase = Arc::new(OrganizationUseCaseImpl::new_with_outbox_unit_of_work(
        organization_service.clone(),
        member_service.clone(),
        event_publisher.clone(),
        outbox_unit_of_work.clone(),
    ));

    // Create member use case
    let member_usecase = Arc::new(MemberUseCaseImpl::new_with_outbox_unit_of_work(
        member_service.clone(),
        organization_service.clone(),
        event_publisher.clone(),
        outbox_unit_of_work.clone(),
    ));

    // Create invitation use case
    let invitation_usecase = Arc::new(InvitationUseCaseImpl::new_with_outbox_unit_of_work(
        invitation_service.clone(),
        event_publisher.clone(),
        outbox_unit_of_work.clone(),
    ));

    // Create external link use case
    let external_link_usecase = Arc::new(ExternalLinkUseCaseImpl::new_with_outbox_unit_of_work(
        external_provider_service.clone(),
        event_publisher.clone(),
        outbox_unit_of_work.clone(),
    ));

    // Create sync job use case
    let sync_job_usecase = Arc::new(SyncJobUseCaseImpl::new_with_outbox_unit_of_work(
        sync_service.clone(),
        event_publisher,
        outbox_unit_of_work,
    ));

    let role_usecase = Arc::new(RoleUseCaseImpl::new(role_service));

    Ok((
        organization_usecase,
        member_usecase,
        invitation_usecase,
        external_link_usecase,
        sync_job_usecase,
        role_usecase,
        iam_signer_client,
        organization_repo,
    ))
}

fn setup_domain(
    db: &DbConnectionPool,
    config: &AppConfig,
    iam_signer_client: Option<IamSignerClient>,
) -> Result<
    (
        Arc<dyn hive_domain::service::OrganizationService>,
        Arc<dyn hive_domain::service::MemberService>,
        Arc<dyn hive_domain::service::InvitationService>,
        Arc<dyn hive_domain::service::ExternalProviderService>,
        Arc<dyn hive_domain::service::RoleService>,
        Arc<dyn hive_domain::service::SyncService>,
        Arc<dyn hive_domain::port::service::IamOrganizationSignerClient>,
        Arc<dyn hive_domain::OrganizationRepository>,
    ),
    Error,
> {
    let (
        organization_repo,
        member_repo,
        invitation_repo,
        external_link_repo,
        external_provider_repo,
        sync_job_repo,
        resource_repo,
        permission_repo,
        role_permission_repo,
        member_role_repo,
        provider_client,
        iam_signer_client,
    ) = setup_infra(db, config, iam_signer_client)?;

    let role_service = Arc::new(RoleServiceImpl::new(
        member_role_repo,
        resource_repo,
        permission_repo,
        role_permission_repo,
    ));

    let member_service = Arc::new(MemberServiceImpl::new(
        member_repo,
        organization_repo.clone(),
        role_service.clone(),
    ));

    let organization_service = Arc::new(OrganizationServiceImpl::new(
        organization_repo.clone(),
        member_service.clone(),
        role_service.clone(),
    ));

    let invitation_service = Arc::new(InvitationServiceImpl::new(
        invitation_repo,
        organization_service.clone(),
        member_service.clone(),
    ));

    let external_provider_service = Arc::new(ExternalProviderServiceImpl::new(
        organization_repo.clone(),
        external_link_repo.clone(),
        external_provider_repo,
        provider_client.clone(),
    ));

    let sync_service = Arc::new(SyncServiceImpl::new(
        sync_job_repo,
        external_link_repo,
        organization_repo.clone(),
        organization_service.clone(),
        invitation_service.clone(),
        provider_client,
    ));

    let organization_repo_dyn: Arc<dyn hive_domain::OrganizationRepository> = organization_repo;
    let iam_signer_dyn: Arc<dyn hive_domain::port::service::IamOrganizationSignerClient> =
        iam_signer_client;

    Ok((
        organization_service,
        member_service,
        invitation_service,
        external_provider_service,
        role_service,
        sync_service,
        iam_signer_dyn,
        organization_repo_dyn,
    ))
}

/// Resolve IAM organization-signer client (ADR-0306).
///
/// Without injection → HTTP adapter. With setter → injected capability (InProcess).
///
/// Returns `(client, used_injected)` so unit tests can prove the transport choice.
pub(crate) fn resolve_iam_signer_client(
    config: &AppConfig,
    injected: Option<IamSignerClient>,
) -> Result<(IamSignerClient, bool), Error> {
    if let Some(client) = injected {
        return Ok((client, true));
    }
    let iam_workload = Arc::new(StaticCredential::from_pair(
        "iam-internal-token",
        config.iam_service.api_key.clone(),
    )?) as Arc<dyn hive_domain::port::service::WorkloadIdentity>;
    let http = Arc::new(HttpIamOrganizationSignerClient::with_workload(
        &config.iam_service.base_url,
        "iam-internal-token",
        iam_workload,
        config.iam_service.timeout_seconds,
    )?) as IamSignerClient;
    Ok((http, false))
}

/// Setup repositories
fn setup_infra(
    db: &DbConnectionPool,
    config: &AppConfig,
    iam_signer_override: Option<IamSignerClient>,
) -> Result<RepositoryBundle, Error> {
    tracing::info!("Setting up repositories...");

    let organization_read_repo = OrganizationReadRepositoryImpl::new(db.get_read_connection());
    let organization_write_repo = OrganizationWriteRepositoryImpl::new(db.get_write_connection());
    let organization_repo = OrganizationRepositoryImpl::new(
        Arc::new(organization_read_repo),
        Arc::new(organization_write_repo),
    );
    let organization_member_read_repo =
        OrganizationMemberReadRepositoryImpl::new(db.get_read_connection());
    let organization_member_write_repo =
        OrganizationMemberWriteRepositoryImpl::new(db.get_write_connection());
    let member_repo = OrganizationMemberRepositoryImpl::new(
        Arc::new(organization_member_read_repo),
        Arc::new(organization_member_write_repo),
    );

    let invitation_read_repo =
        OrganizationInvitationReadRepositoryImpl::new(db.get_read_connection());
    let invitation_write_repo =
        OrganizationInvitationWriteRepositoryImpl::new(db.get_write_connection());
    let invitation_repo = OrganizationInvitationRepositoryImpl::new(
        Arc::new(invitation_read_repo),
        Arc::new(invitation_write_repo),
    );

    let external_link_read_repo = ExternalLinkReadRepositoryImpl::new(db.get_read_connection());
    let external_link_write_repo = ExternalLinkWriteRepositoryImpl::new(db.get_write_connection());
    let external_link_repo = ExternalLinkRepositoryImpl::new(
        Arc::new(external_link_read_repo),
        Arc::new(external_link_write_repo),
    );

    let external_provider_read_repo =
        ExternalProviderReadRepositoryImpl::new(db.get_read_connection());
    let external_provider_write_repo =
        ExternalProviderWriteRepositoryImpl::new(db.get_write_connection());
    let external_provider_repo = ExternalProviderRepositoryImpl::new(
        Arc::new(external_provider_read_repo),
        Arc::new(external_provider_write_repo),
    );

    let sync_job_read_repo = SyncJobReadRepositoryImpl::new(db.get_read_connection());
    let sync_job_write_repo = SyncJobWriteRepositoryImpl::new(db.get_write_connection());
    let sync_job_repo =
        SyncJobRepositoryImpl::new(Arc::new(sync_job_read_repo), Arc::new(sync_job_write_repo));

    let resource_read_repo = ResourceReadRepositoryImpl::new(db.get_read_connection());
    let resource_repo = ResourceRepositoryImpl::new(Arc::new(resource_read_repo));

    let permission_read_repo = PermissionReadRepositoryImpl::new(db.get_read_connection());
    let permission_repo = PermissionRepositoryImpl::new(Arc::new(permission_read_repo));

    let role_permission_read_repo = RolePermissionReadRepositoryImpl::new(db.get_read_connection());
    let role_permission_write_repo =
        RolePermissionWriteRepositoryImpl::new(db.get_write_connection());
    let role_permission_repo = RolePermissionRepositoryImpl::new(
        Arc::new(role_permission_read_repo),
        Arc::new(role_permission_write_repo),
    );

    let member_role_read_repo = MemberRoleReadRepositoryImpl::new(db.get_read_connection());
    let member_role_write_repo = MemberRoleWriteRepositoryImpl::new(db.get_write_connection());
    let member_role_repo = MemberRoleRepositoryImpl::new(
        Arc::new(member_role_read_repo),
        Arc::new(member_role_write_repo),
    );

    let provider_client = HttpExternalProviderClient::new(
        config.external_provider_service.base_url.clone(),
        config.external_provider_service.api_key.clone(),
        config.external_provider_service.timeout_seconds,
        config.external_provider_service.max_retries,
    )?;

    let (iam_signer_client, _) = resolve_iam_signer_client(config, iam_signer_override)?;

    tracing::info!("Repositories initialized");
    Ok((
        Arc::new(organization_repo),
        Arc::new(member_repo),
        Arc::new(invitation_repo),
        Arc::new(external_link_repo),
        Arc::new(external_provider_repo),
        Arc::new(sync_job_repo),
        Arc::new(resource_repo),
        Arc::new(permission_repo),
        Arc::new(role_permission_repo),
        Arc::new(member_role_repo),
        Arc::new(provider_client),
        iam_signer_client,
    ))
}

/// Application builder for Hive
pub struct AppBuilder {
    config: AppConfig,
    overrides: HiveOutboundOverrides,
}

impl AppBuilder {
    /// Create a new app builder (default outbound transport = HTTP).
    #[must_use]
    pub const fn new(config: AppConfig) -> Self {
        Self {
            config,
            overrides: HiveOutboundOverrides {
                iam_organization_signer: None,
            },
        }
    }

    /// Replace the outbound-overrides bag (monolith host API, ADR-0104).
    #[must_use]
    pub fn with_outbound(mut self, bag: HiveOutboundOverrides) -> Self {
        self.overrides = bag;
        self
    }

    /// Inject an InProcess (or test) IAM organization-signer client (ADR-0306).
    ///
    /// Sugar over [`Self::with_outbound`]: writes `iam_organization_signer`.
    #[must_use]
    pub fn with_iam_organization_signer_client(mut self, client: IamSignerClient) -> Self {
        self.overrides.iam_organization_signer = Some(client);
        self
    }

    /// Build the Hive application.
    ///
    /// # Errors
    ///
    /// Returns an error if application initialization fails.
    pub async fn build(self) -> Result<Application, anyhow::Error> {
        Application::new(self.config, self.overrides).await
    }
}

#[cfg(test)]
mod resolve_iam_signer_tests {
    use super::*;
    use async_trait::async_trait;
    use hive_domain::port::service::{
        ConfigureOrganizationSignerRequest, IamOrganizationSignerClient, OrganizationSignerResponse,
    };
    use uuid::Uuid;

    struct CapturingIamSigner;

    #[async_trait]
    impl IamOrganizationSignerClient for CapturingIamSigner {
        async fn configure_organization_signer(
            &self,
            _org_id: Uuid,
            _request: &ConfigureOrganizationSignerRequest,
        ) -> Result<OrganizationSignerResponse, DomainError> {
            unreachable!("resolver test must not call configure")
        }

        async fn test_organization_signer(
            &self,
            _org_id: Uuid,
        ) -> Result<OrganizationSignerResponse, DomainError> {
            unreachable!("resolver test must not call test")
        }

        async fn rotate_organization_signer(
            &self,
            _org_id: Uuid,
        ) -> Result<OrganizationSignerResponse, DomainError> {
            unreachable!("resolver test must not call rotate")
        }

        async fn disable_organization_signer(
            &self,
            _org_id: Uuid,
        ) -> Result<OrganizationSignerResponse, DomainError> {
            unreachable!("resolver test must not call disable")
        }
    }

    fn minimal_iam_config() -> AppConfig {
        let mut config = AppConfig::default();
        config.iam_service.base_url = "http://127.0.0.1:9".into();
        config.iam_service.api_key = "test-internal-token".into();
        config.iam_service.timeout_seconds = 1;
        config
    }

    #[test]
    fn without_injection_builds_http_client() {
        let config = minimal_iam_config();
        let (client, used_injected) =
            resolve_iam_signer_client(&config, None).expect("http client builds");
        assert!(!used_injected, "default transport must be HTTP");
        // Prove we did not keep a Capturing stub: a second resolve with injection
        // yields a different Arc than this default client.
        let injected: IamSignerClient = Arc::new(CapturingIamSigner);
        let (resolved, injected_flag) =
            resolve_iam_signer_client(&config, Some(injected.clone())).expect("inject");
        assert!(injected_flag);
        assert!(Arc::ptr_eq(&resolved, &injected));
        assert!(!Arc::ptr_eq(&client, &resolved));
    }

    #[test]
    fn with_injection_uses_provided_client_not_http() {
        let config = minimal_iam_config();
        let injected: IamSignerClient = Arc::new(CapturingIamSigner);
        let (resolved, used_injected) =
            resolve_iam_signer_client(&config, Some(injected.clone())).expect("inject");
        assert!(used_injected);
        assert!(
            Arc::ptr_eq(&resolved, &injected),
            "injected client must be the one threaded into the bundle"
        );
    }
}
