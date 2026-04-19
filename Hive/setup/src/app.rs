use std::sync::Arc;

// Hive
use hive_application::{
    ExternalLinkUseCaseImpl, HiveCommandRegistryFactory, InvitationUseCaseImpl, MemberUseCaseImpl,
    OrganizationUseCaseImpl, SyncJobUseCaseImpl,
};
use hive_configuration::AppConfig;
use hive_domain::{
    service::{
        organization_service::OrganizationServiceImpl,
        member_service::MemberServiceImpl,
        invitation_service::InvitationServiceImpl,
        external_provider_service::ExternalProviderServiceImpl,
        sync_service::SyncServiceImpl,
        permission_service::ResourcePermissionFetcher,
        role_service::RoleServiceImpl,
    },
};
use hive_infra::{HiveErrorMapper,
    repository::{
        OrganizationRepositoryImpl,
        OrganizationReadRepositoryImpl,
        OrganizationWriteRepositoryImpl,
        OrganizationMemberRepositoryImpl,
        OrganizationMemberReadRepositoryImpl,
        OrganizationMemberWriteRepositoryImpl,
        OrganizationInvitationRepositoryImpl,
        OrganizationInvitationReadRepositoryImpl,
        OrganizationInvitationWriteRepositoryImpl,
        ExternalLinkRepositoryImpl,
        ExternalLinkReadRepositoryImpl,
        ExternalLinkWriteRepositoryImpl,
        ExternalProviderRepositoryImpl,
        ExternalProviderReadRepositoryImpl,
        ExternalProviderWriteRepositoryImpl,
        SyncJobRepositoryImpl,
        SyncJobReadRepositoryImpl,
        SyncJobWriteRepositoryImpl,
        ResourceRepositoryImpl,
        PermissionRepositoryImpl,
        RolePermissionRepositoryImpl,
        RolePermissionReadRepositoryImpl,
        RolePermissionWriteRepositoryImpl,
        MemberRoleRepositoryImpl,
        MemberRoleReadRepositoryImpl,
        MemberRoleWriteRepositoryImpl,
        ResourceReadRepositoryImpl,
        PermissionReadRepositoryImpl,
    },
    external_provider::external_provider_client::HttpExternalProviderClient,
};
use hive_http::create_app_routes;
use hive_migration::Migrator;

// Rustycog
use rustycog_command::GenericCommandService;
use rustycog_events::{ErrorMapper, MultiQueueEventPublisher, EventPublisher, create_multi_queue_event_publisher};
use rustycog_http::{AppState, UserIdExtractor};
use rustycog_db::DbConnectionPool;
use rustycog_core::error::DomainError;
use rustycog_permission::PermissionsFetcher;
use rustycog_config::ServerConfig;

// External
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use anyhow::Error;

// Use AppState from rustycog-http - no need to define our own

/// Application context for dependency injection
pub struct Application {
    pub config: AppConfig,
    pub state: AppState,
    pub organization_permission_fetcher: Arc<dyn PermissionsFetcher>,
    pub member_permission_fetcher: Arc<dyn PermissionsFetcher>,
    pub external_link_permission_fetcher: Arc<dyn PermissionsFetcher>,
}

impl Application {
    /// Create a new application instance with all dependencies
    pub async fn new(config: AppConfig) -> Result<Self, Error> {
        tracing::info!("Initializing Hive application...");

        // Setup database connection
        let db = setup_database(&config).await?;

        // Setup event publisher for Telegraph communication
        let event_publisher = create_multi_queue_event_publisher(&config.queue, None, Arc::new(HiveErrorMapper)).await?;

        // Setup use cases
        let (
            organization_usecase,
            member_usecase,
            invitation_usecase,
            external_link_usecase,
            sync_job_usecase,
            organization_permission_fetcher,
            member_permission_fetcher,
            external_link_permission_fetcher,
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

        // Create application state
        let state = AppState::new(command_service, user_id_extractor);

        tracing::info!("Hive application initialized successfully");

        Ok(Application { config, state, organization_permission_fetcher, member_permission_fetcher, external_link_permission_fetcher })
    }

    /// Start the HTTP server
    pub async fn run(self, server_config: ServerConfig) -> Result<(), Error> {
        tracing::info!("Starting Hive HTTP server...");

        create_app_routes(self.state, server_config, self.organization_permission_fetcher, self.member_permission_fetcher, self.external_link_permission_fetcher)
            .await
            .map_err(|e| anyhow::anyhow!("Server startup failed: {}", e))?;

        Ok(())
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
        Arc<dyn PermissionsFetcher>,
        Arc<dyn PermissionsFetcher>,
        Arc<dyn PermissionsFetcher>,
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
        organization_permission_fetcher,
        member_permission_fetcher,
        external_link_permission_fetcher,
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
        organization_permission_fetcher,
        member_permission_fetcher,
        external_link_permission_fetcher,
    ))
}

async fn setup_domain(db: DbConnectionPool, config: &AppConfig) -> Result<(
    Arc<dyn hive_domain::service::OrganizationService>,
    Arc<dyn hive_domain::service::MemberService>,
    Arc<dyn hive_domain::service::InvitationService>,
    Arc<dyn hive_domain::service::ExternalProviderService>,
    Arc<dyn hive_domain::service::RoleService>,
    Arc<dyn hive_domain::service::SyncService>,
    Arc<dyn PermissionsFetcher>,
    Arc<dyn PermissionsFetcher>,
    Arc<dyn PermissionsFetcher>,
), Error> {

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

    let organization_permission_fetcher = ResourcePermissionFetcher::new(organization_service.clone(), member_service.clone(), member_role_repo.clone(), vec!["organization".to_string()]);
    let member_permission_fetcher = ResourcePermissionFetcher::new(organization_service.clone(), member_service.clone(), member_role_repo.clone(), vec!["organization".to_string(), "member".to_string()]);
    let external_link_permission_fetcher = ResourcePermissionFetcher::new(organization_service.clone(), member_service.clone(), member_role_repo.clone(), vec!["organization".to_string(), "external_link".to_string()]);

    Ok((
        organization_service,
        member_service,
        invitation_service,
        external_provider_service,
        role_service,
        sync_service,
        Arc::new(organization_permission_fetcher),
        Arc::new(member_permission_fetcher),
        Arc::new(external_link_permission_fetcher),
    ))
}

/// Setup repositories
async fn setup_infra(db: DbConnectionPool, config: &AppConfig) -> Result<(
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
), Error> {
    tracing::info!("Setting up repositories...");

    let organization_read_repo = OrganizationReadRepositoryImpl::new(db.get_read_connection());
    let organization_write_repo = OrganizationWriteRepositoryImpl::new(db.get_write_connection());
    let organization_repo = OrganizationRepositoryImpl::new(
        Arc::new(organization_read_repo),
        Arc::new(organization_write_repo),
    );
    let organization_member_read_repo = OrganizationMemberReadRepositoryImpl::new(db.get_read_connection());
    let organization_member_write_repo = OrganizationMemberWriteRepositoryImpl::new(db.get_write_connection());
    let member_repo = OrganizationMemberRepositoryImpl::new(
        Arc::new(organization_member_read_repo),
        Arc::new(organization_member_write_repo),
    );

    let invitation_read_repo = OrganizationInvitationReadRepositoryImpl::new(db.get_read_connection());
    let invitation_write_repo = OrganizationInvitationWriteRepositoryImpl::new(db.get_write_connection());
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

    let external_provider_read_repo = ExternalProviderReadRepositoryImpl::new(db.get_read_connection());
    let external_provider_write_repo = ExternalProviderWriteRepositoryImpl::new(db.get_write_connection());
    let external_provider_repo = ExternalProviderRepositoryImpl::new(
        Arc::new(external_provider_read_repo),
        Arc::new(external_provider_write_repo),
    );

    let sync_job_read_repo = SyncJobReadRepositoryImpl::new(db.get_read_connection());
    let sync_job_write_repo = SyncJobWriteRepositoryImpl::new(db.get_write_connection());
    let sync_job_repo = SyncJobRepositoryImpl::new(
        Arc::new(sync_job_read_repo),
        Arc::new(sync_job_write_repo),
    );

    let resource_read_repo = ResourceReadRepositoryImpl::new(db.get_read_connection());
    let resource_repo = ResourceRepositoryImpl::new(Arc::new(resource_read_repo));

    let permission_read_repo = PermissionReadRepositoryImpl::new(db.get_read_connection());
    let permission_repo = PermissionRepositoryImpl::new(Arc::new(permission_read_repo));

    let role_permission_read_repo = RolePermissionReadRepositoryImpl::new(db.get_read_connection());
    let role_permission_write_repo = RolePermissionWriteRepositoryImpl::new(db.get_write_connection());
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
        config.external_provider_service.timeout_seconds.clone(),
        config.external_provider_service.max_retries.clone(),
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
