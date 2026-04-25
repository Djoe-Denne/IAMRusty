use std::sync::Arc;

use axum::Router;

// Hive
use hive_application::{
    ExternalLinkUseCaseImpl, HiveCommandRegistryFactory, InvitationUseCaseImpl, MemberUseCaseImpl,
    OrganizationUseCaseImpl, SyncJobUseCaseImpl,
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
    HiveErrorMapper,
};

// Rustycog
use rustycog_command::GenericCommandService;
use rustycog_config::ServerConfig;
use rustycog_core::error::DomainError;
use rustycog_db::DbConnectionPool;
use rustycog_events::{create_multi_queue_event_publisher, EventPublisher};
use rustycog_http::{AppState, UserIdExtractor};
use rustycog_permission::{
    CachedPermissionChecker, MetricsPermissionChecker, OpenFgaPermissionChecker, PermissionChecker,
};
use std::time::Duration;

// External
use anyhow::Error;

/// Application context for dependency injection
pub struct Application {
    pub config: AppConfig,
    pub state: AppState,
}

impl Application {
    /// Create a new application instance with all dependencies
    pub async fn new(config: AppConfig) -> Result<Self, Error> {
        tracing::info!("Initializing Hive application...");

        // Setup database connection
        let db = setup_database(&config).await?;

        // Setup event publisher for Telegraph + sentinel-sync communication
        let event_publisher =
            create_multi_queue_event_publisher(&config.queue, None, Arc::new(HiveErrorMapper))
                .await?;

        // Setup use cases
        let (
            organization_usecase,
            member_usecase,
            invitation_usecase,
            external_link_usecase,
            sync_job_usecase,
        ) = setup_application(db, &config, event_publisher).await?;

        // Setup command registry
        let command_registry = HiveCommandRegistryFactory::create_hive_registry(
            organization_usecase,
            member_usecase,
            invitation_usecase,
            external_link_usecase,
            sync_job_usecase,
        );

        // Create command service
        let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));

        // Setup user ID extractor (for authentication)
        let user_id_extractor = UserIdExtractor::new(config.auth.clone())
            .map_err(|e| anyhow::anyhow!("Invalid auth configuration: {}", e))?;

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
                .map_err(|e| anyhow::anyhow!("Invalid OpenFGA configuration: {}", e))?,
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

        tracing::info!("Hive application initialized successfully");

        Ok(Application { config, state })
    }

    /// Start the HTTP server
    pub async fn run(self, server_config: ServerConfig) -> Result<(), Error> {
        tracing::info!("Starting Hive HTTP server...");

        create_app_routes(self.state, server_config)
            .await
            .map_err(|e| anyhow::anyhow!("Server startup failed: {}", e))?;

        Ok(())
    }

    pub fn router(&self) -> Router {
        create_router(self.state.clone())
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
async fn setup_application(
    db: DbConnectionPool,
    config: &AppConfig,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
) -> Result<
    (
        Arc<dyn hive_application::OrganizationUseCase>,
        Arc<dyn hive_application::MemberUseCase>,
        Arc<dyn hive_application::InvitationUseCase>,
        Arc<dyn hive_application::ExternalLinkUseCase>,
        Arc<dyn hive_application::SyncJobUseCase>,
    ),
    Error,
> {
    let (
        organization_service,
        member_service,
        invitation_service,
        external_provider_service,
        _role_service,
        sync_service,
    ) = setup_domain(db, config).await?;

    // Create organization use case
    let organization_usecase = Arc::new(OrganizationUseCaseImpl::new(
        organization_service.clone(),
        event_publisher.clone(),
    ));

    // Create member use case
    let member_usecase = Arc::new(MemberUseCaseImpl::new(
        member_service.clone(),
        organization_service.clone(),
        event_publisher.clone(),
    ));

    // Create invitation use case
    let invitation_usecase = Arc::new(InvitationUseCaseImpl::new(
        invitation_service.clone(),
        event_publisher.clone(),
    ));

    // Create external link use case
    let external_link_usecase = Arc::new(ExternalLinkUseCaseImpl::new(
        external_provider_service.clone(),
        event_publisher.clone(),
    ));

    // Create sync job use case
    let sync_job_usecase = Arc::new(SyncJobUseCaseImpl::new(
        sync_service.clone(),
        event_publisher,
    ));

    Ok((
        organization_usecase,
        member_usecase,
        invitation_usecase,
        external_link_usecase,
        sync_job_usecase,
    ))
}

async fn setup_domain(
    db: DbConnectionPool,
    config: &AppConfig,
) -> Result<
    (
        Arc<dyn hive_domain::service::OrganizationService>,
        Arc<dyn hive_domain::service::MemberService>,
        Arc<dyn hive_domain::service::InvitationService>,
        Arc<dyn hive_domain::service::ExternalProviderService>,
        Arc<dyn hive_domain::service::RoleService>,
        Arc<dyn hive_domain::service::SyncService>,
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
    ) = setup_infra(db, config).await?;

    let role_service = Arc::new(RoleServiceImpl::new(
        member_role_repo.clone(),
        resource_repo.clone(),
        permission_repo.clone(),
        role_permission_repo.clone(),
    ));

    let member_service = Arc::new(MemberServiceImpl::new(
        member_repo.clone(),
        organization_repo.clone(),
        role_service.clone(),
    ));

    let organization_service = Arc::new(OrganizationServiceImpl::new(
        organization_repo.clone(),
        member_service.clone(),
        role_service.clone(),
    ));

    let invitation_service = Arc::new(InvitationServiceImpl::new(
        invitation_repo.clone(),
        organization_service.clone(),
        member_service.clone(),
    ));

    let external_provider_service = Arc::new(ExternalProviderServiceImpl::new(
        organization_repo.clone(),
        external_link_repo.clone(),
        external_provider_repo.clone(),
        provider_client.clone(),
    ));

    let sync_service = Arc::new(SyncServiceImpl::new(
        sync_job_repo.clone(),
        external_link_repo.clone(),
        organization_repo.clone(),
        organization_service.clone(),
        invitation_service.clone(),
        provider_client,
    ));

    Ok((
        organization_service,
        member_service,
        invitation_service,
        external_provider_service,
        role_service,
        sync_service,
    ))
}

/// Setup repositories
async fn setup_infra(
    db: DbConnectionPool,
    config: &AppConfig,
) -> Result<
    (
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
    ),
    Error,
> {
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
    ))
}

/// Application builder for Hive
pub struct AppBuilder {
    config: AppConfig,
}

impl AppBuilder {
    /// Create a new app builder
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Build the Hive application
    pub async fn build(self) -> Result<Application, anyhow::Error> {
        Ok(Application::new(self.config).await?)
    }
}
