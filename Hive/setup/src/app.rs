use std::{fs::Permissions, sync::Arc};

use hive_application::{
    ExternalLinkUseCaseImpl, HiveCommandRegistryFactory, InvitationUseCaseImpl, MemberUseCaseImpl,
    OrganizationUseCaseImpl, SyncJobUseCaseImpl,
};
use hive_configuration::AppConfig;
use hive_infra::{HiveErrorMapper,
    repository::{
        OrganizationRepositoryImpl,
        OrganizationMemberRepositoryImpl,
        OrganizationInvitationRepositoryImpl,
        ExternalLinkRepositoryImpl,
        ExternalProviderRepositoryImpl,
        SyncJobRepositoryImpl,
        ResourceRepositoryImpl,
        PermissionRepositoryImpl,
        RolePermissionRepositoryImpl,
        MemberRoleRepositoryImpl,
    },
    external_provider::external_provider_client::HttpExternalProviderClient,
};
use hive_http::create_app_routes;
use hive_migration::Migrator;
use rustycog_command::GenericCommandService;
use rustycog_events::{ErrorMapper, MultiQueueEventPublisher, EventPublisher, create_multi_queue_event_publisher};
use rustycog_http::{AppState, UserIdExtractor};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use anyhow::Error;
use hive_domain::{error::DomainError,
    service::{
        organization_service::OrganizationServiceImpl,
        member_service::MemberServiceImpl,
        invitation_service::InvitationServiceImpl,
        external_provider_service::ExternalProviderServiceImpl,
        role_service::RoleServiceImpl,
        sync_service::SyncServiceImpl,
    },
};
// Use AppState from rustycog-http - no need to define our own

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

        // Run migrations if enabled
        if config.database.run_migrations {
            run_migrations(&db).await?;
        }

        // Setup event publisher for Telegraph communication
        let event_publisher = create_multi_queue_event_publisher(&config.queue, None, Arc::new(HiveErrorMapper)).await?;

        // Setup use cases
        let (
            organization_usecase,
            member_usecase,
            invitation_usecase,
            external_link_usecase,
            sync_job_usecase,
        ) = setup_use_cases(db.clone(), event_publisher).await?;

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
        let user_id_extractor = UserIdExtractor::new();

        // Create application state
        let state = AppState::new(command_service, user_id_extractor);

        tracing::info!("Hive application initialized successfully");

        Ok(Application { config, state })
    }

    /// Start the HTTP server
    pub async fn serve(self) -> Result<(), Error> {
        tracing::info!("Starting Hive HTTP server...");

        create_app_routes(self.state, self.config.server)
            .await
            .map_err(|e| Error::ServerError {
                message: format!("Server startup failed: {}", e),
            })?;

        Ok(())
    }

    /// Get server address for testing
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.config.server.host, self.config.server.port)
    }
}

/// Setup database connection
async fn setup_database(config: &AppConfig) -> Result<DatabaseConnection, Error> {
    tracing::info!("Connecting to database: {}", config.database.url);

    let mut opt = sea_orm::ConnectOptions::new(&config.database.url);
    opt.max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .connect_timeout(std::time::Duration::from_secs(
            config.database.connection_timeout_seconds,
        ))
        .idle_timeout(std::time::Duration::from_secs(
            config.database.idle_timeout_seconds,
        ))
        .max_lifetime(std::time::Duration::from_secs(
            config.database.max_lifetime_seconds,
        ));

    let db = Database::connect(opt)
        .await
        .map_err(|e| Error::DatabaseError {
            message: e.to_string(),
        })?;

    tracing::info!("Database connection established");
    Ok(db)
}

/// Run database migrations
async fn run_migrations(db: &DatabaseConnection) -> Result<(), Error> {
    tracing::info!("Running database migrations...");

    Migrator::up(db, None)
        .await
        .map_err(|e| Error::MigrationError {
            message: e.to_string(),
        })?;

    tracing::info!("Database migrations completed");
    Ok(())
}

/// Setup use cases with their dependencies
async fn setup_use_cases(
    db: DatabaseConnection,
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
   
    // Create organization use case
    let organization_usecase = Arc::new(OrganizationUseCaseImpl::new(
        todo!("organization_service"),
        todo!("organization_repository"),
        event_publisher.clone(),
    ));

    // Create member use case
    let member_usecase = Arc::new(MemberUseCaseImpl::new(
        todo!("member_service"),
        todo!("member_repository"),
        todo!("organization_repository"),
        event_publisher.clone(),
    ));

    // Create invitation use case
    let invitation_usecase = Arc::new(InvitationUseCaseImpl::new(
        todo!("invitation_repository"),
        event_publisher.clone(),
    ));

    // Create external link use case
    let external_link_usecase = Arc::new(ExternalLinkUseCaseImpl::new(
        todo!("external_link_repository"),
        event_publisher.clone(),
    ));

    // Create sync job use case
    let sync_job_usecase = Arc::new(SyncJobUseCaseImpl::new(
        todo!("sync_job_repository"),
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

async fn setup_services(db: Arc<DatabaseConnection>, config: &AppConfig) -> Result<(
    Arc<hive_domain::service::OrganizationServiceImpl>,
    Arc<hive_domain::service::MemberServiceImpl>,
    Arc<hive_domain::service::InvitationServiceImpl>,
    Arc<hive_domain::service::ExternalProviderServiceImpl>,
    Arc<hive_domain::service::RoleServiceImpl>,
    Arc<hive_domain::service::SyncServiceImpl>,
), Error> {

    let (
        organization_repo,
        member_repo,
        invitation_repo,
        external_link_repo,
        external_provider_repo,
        sync_job_repo,
        permission_repo,
        resource_repo,
        role_permission_repo,
        member_role_repo,
    ) = setup_repositories(db).await?;


    let role_engine = Arc::new(RoleEngineImpl::new(
        permission_repo,
        resource_repo,
    ));

    let provider_client =HttpExternalProviderClient::new(
        config.external_provider_service.base_url,
        config.external_provider_service.api_key,
        config.external_provider_service.timeout_seconds,
        config.external_provider_service.max_retries,
    )?;

    let role_service = Arc::new(RoleServiceImpl::new(
        member_repo,
        organization_repo,
        member_role_repo,
        resource_repo,
        permission_repo,
        role_engine,
        role_permission_repo,
    ));

    let member_service = Arc::new(MemberServiceImpl::new(
        member_repo,
        organization_repo,
        role_service,
    ));

    let organization_service = Arc::new(OrganizationServiceImpl::new(
        organization_repo,
        member_service,
        role_service,
    ));

    let invitation_service = Arc::new(InvitationServiceImpl::new(
        invitation_repo,
        organization_service,
        role_service,
        member_service,
    ));

    let external_provider_service = Arc::new(ExternalProviderServiceImpl::new(
        organization_repo,
        external_link_repo,
        external_provider_repo,
        role_service,
        provider_client,
    ));

    let sync_service = Arc::new(SyncServiceImpl::new(
        sync_job_repo,
        external_link_repo,
        organization_repo,
        role_service,
        organization_service,
        invitation_service,
        provider_client,
    ));
}

/// Setup repositories
async fn setup_repositories(db: Arc<DatabaseConnection>) -> Result<(
    OrganizationRepositoryImpl,
    OrganizationMemberRepositoryImpl,
    OrganizationInvitationRepositoryImpl,
    ExternalLinkRepositoryImpl,
    ExternalProviderRepositoryImpl,
    SyncJobRepositoryImpl, 
    ResourceRepositoryImpl,
    PermissionRepositoryImpl,
    RolePermissionRepositoryImpl,
    MemberRoleRepositoryImpl,
), Error> {
    tracing::info!("Setting up repositories...");

    let organization_repo = OrganizationRepositoryImpl::new(db.clone());
    let member_repo = OrganizationMemberRepositoryImpl::new(db.clone());
    let invitation_repo = OrganizationInvitationRepositoryImpl::new(db.clone());
    let external_link_repo = ExternalLinkRepositoryImpl::new(db.clone());
    let external_provider_repo = ExternalProviderRepositoryImpl::new(db.clone());
    let sync_job_repo = SyncJobRepositoryImpl::new(db.clone());
    let resource_repo = ResourceRepositoryImpl::new(db.clone());
    let permissions_repo = PermissionRepositoryImpl::new(db.clone());
    let role_permission_repo = RolePermissionRepositoryImpl::new(db.clone());
    let member_role_repo = MemberRoleRepositoryImpl::new(db.clone());

    tracing::info!("Repositories initialized");
    Ok((
        organization_repo,
        member_repo,
        invitation_repo,
        external_link_repo,
        external_provider_repo,
        sync_job_repo,
        resource_repo,
        permissions_repo,
        role_permission_repo,
        member_role_repo,
    ))
}
