use std::sync::Arc;

// Manifesto
use manifesto_application::{
    ComponentUseCaseImpl, ManifestoCommandRegistryFactory, MemberUseCaseImpl, ProjectUseCaseImpl,
};
use manifesto_configuration::AppConfig;
use manifesto_domain::service::{
    ComponentServiceImpl, MemberServiceImpl, ProjectServiceImpl, ProjectPermissionFetcher, MemberPermissionFetcher, ComponentPermissionFetcher, 
};
use manifesto_infra::{
    adapters::{ComponentServiceClient},
    repository::{
        ComponentReadRepositoryImpl, ComponentRepositoryImpl, ComponentWriteRepositoryImpl,
        MemberReadRepositoryImpl, MemberRepositoryImpl, MemberWriteRepositoryImpl,
        PermissionReadRepositoryImpl,
        ProjectMemberRolePermissionReadRepositoryImpl, ProjectMemberRolePermissionRepositoryImpl,
        ProjectMemberRolePermissionWriteRepositoryImpl,
        ProjectReadRepositoryImpl, ProjectRepositoryImpl, ProjectWriteRepositoryImpl,
        ResourceReadRepositoryImpl, ResourceRepositoryImpl, ResourceWriteRepositoryImpl,
        RolePermissionReadRepositoryImpl, RolePermissionRepositoryImpl, RolePermissionWriteRepositoryImpl,
    },
    ManifestoErrorMapper,
};
use manifesto_http_server::create_app_routes;

// Rustycog
use rustycog_command::GenericCommandService;
use rustycog_config::ServerConfig;
use rustycog_core::error::DomainError;
use rustycog_db::DbConnectionPool;
use rustycog_events::{create_multi_queue_event_publisher, EventPublisher};
use rustycog_http::{AppState, UserIdExtractor};
use rustycog_permission::PermissionsFetcher;

// External
use anyhow::Error;

/// Build and run the Manifesto application - used for tests
pub async fn build_and_run(
    config: AppConfig,
    server_config: ServerConfig,
    maybe_event_publisher: Option<Arc<dyn EventPublisher<DomainError>>>,
) -> Result<(), Error> {
    let app = Application::new_with_maybe_event_publisher(config, maybe_event_publisher).await?;
    app.run(server_config).await
}

/// Application context for dependency injection
pub struct Application {
    pub config: AppConfig,
    pub state: AppState,
    pub project_permission_fetcher: Arc<dyn PermissionsFetcher>,
    pub member_permission_fetcher: Arc<dyn PermissionsFetcher>,
    pub component_permission_fetcher: Arc<dyn PermissionsFetcher>,
}

impl Application {
    /// Create a new application instance with all dependencies
    pub async fn new(config: AppConfig) -> Result<Self, Error> {
        Self::new_with_maybe_event_publisher(config, None).await
    }

    /// Create a new application instance with an optional event publisher (for testing)
    pub async fn new_with_maybe_event_publisher(
        config: AppConfig,
        maybe_event_publisher: Option<Arc<dyn EventPublisher<DomainError>>>,
    ) -> Result<Self, Error> {
        tracing::info!("Initializing Manifesto application...");

        // Setup database connection
        let db = setup_database(&config).await?;

        // Setup event publisher for Telegraph communication
        let event_publisher = if let Some(ep) = maybe_event_publisher {
            ep
        } else {
            create_multi_queue_event_publisher(
                &config.queue,
                None,
                Arc::new(ManifestoErrorMapper),
            )
            .await?
        };

        // Setup use cases
        let (
            project_usecase,
            component_usecase,
            member_usecase,
            project_permission_fetcher,
            member_permission_fetcher,
            component_permission_fetcher,
        ) = setup_application(db, &config, event_publisher).await?;

        // Setup command registry
        let command_registry = ManifestoCommandRegistryFactory::create_manifesto_registry(
            project_usecase,
            component_usecase,
            member_usecase,
        );

        // Create command service
        let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));

        // Setup user ID extractor (for authentication)
        let user_id_extractor = UserIdExtractor::new();

        // Create application state
        let state = AppState::new(command_service, user_id_extractor);

        tracing::info!("Manifesto application initialized successfully");

        Ok(Application {
            config,
            state,
            project_permission_fetcher,
            member_permission_fetcher,
            component_permission_fetcher,
        })
    }

    /// Start the HTTP server
    pub async fn run(self, server_config: ServerConfig) -> Result<(), Error> {
        tracing::info!("Starting Manifesto HTTP server...");

        create_app_routes(self.state, server_config, self.project_permission_fetcher, self.member_permission_fetcher, self.component_permission_fetcher)
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
        Arc<dyn manifesto_application::ProjectUseCase>,
        Arc<dyn manifesto_application::ComponentUseCase>,
        Arc<dyn manifesto_application::MemberUseCase>,
        Arc<dyn PermissionsFetcher>,
        Arc<dyn PermissionsFetcher>,
        Arc<dyn PermissionsFetcher>,
    ),
    Error,
> {
    let (
        project_service,
        component_service,
        member_service,
        permission_service,
        project_permission_fetcher,
        member_permission_fetcher,
        component_permission_fetcher,
    ) = setup_domain(db, config).await?;

    // Create project use case
    let project_usecase = Arc::new(ProjectUseCaseImpl::new(
        project_service.clone(),
        component_service.clone(),
        member_service.clone(),
        event_publisher.clone(),
    ));

    // Create component use case
    let component_usecase = Arc::new(ComponentUseCaseImpl::new(
        component_service.clone(),
        project_service.clone(),
        permission_service.clone(),
        event_publisher.clone(),
    ));

    // Create member use case
    let member_usecase = Arc::new(MemberUseCaseImpl::new(
        member_service.clone(),
        project_service.clone(),
        permission_service.clone(),
        event_publisher.clone(),
    ));

    Ok((
        project_usecase,
        component_usecase,
        member_usecase,
        project_permission_fetcher,
        member_permission_fetcher,
        component_permission_fetcher,
    ))
}

async fn setup_domain(
    db: DbConnectionPool,
    config: &AppConfig,
) -> Result<
    (
        Arc<dyn manifesto_domain::service::ProjectService>,
        Arc<dyn manifesto_domain::service::ComponentService>,
        Arc<dyn manifesto_domain::service::MemberService>,
        Arc<dyn manifesto_domain::service::PermissionService>,
        Arc<dyn PermissionsFetcher>,
        Arc<dyn PermissionsFetcher>,
        Arc<dyn PermissionsFetcher>,
    ),
    Error,
> {
    let (project_repo, component_repo, member_repo, permission_repo, resource_repo, role_permission_repo, member_role_permission_repo) = setup_repositories(db.clone()).await?;

    // Setup component service adapter (external HTTP client)
    let component_service_adapter = Arc::new(ComponentServiceClient::new(
        config.service.component_service.base_url.clone(),
        30, // timeout_seconds
    ));

    // Create permission service first (needed by component service and member use case)
    let permission_service: Arc<dyn manifesto_domain::service::PermissionService> = Arc::new(
        manifesto_domain::service::PermissionServiceImpl::new(
            permission_repo,
            resource_repo,
            role_permission_repo,
            member_role_permission_repo,
        ),
    );

    // Create domain services
    let project_service = Arc::new(ProjectServiceImpl::new(
        project_repo.clone(),
        component_repo.clone(),
    ));

    let component_service = Arc::new(ComponentServiceImpl::new(
        component_repo.clone(),
        component_service_adapter,
        permission_service.clone(),
    ));

    let member_service = Arc::new(MemberServiceImpl::new(member_repo.clone()));

    // Create permission fetcher for HTTP middleware
    let project_permission_fetcher = Arc::new(ProjectPermissionFetcher::new(
        project_service.clone(),
        member_service.clone(),
    ));

    let member_permission_fetcher = Arc::new(MemberPermissionFetcher::new(
        project_service.clone(),
        member_service.clone(),
    ));

    let component_permission_fetcher = Arc::new(ComponentPermissionFetcher::new(
        project_service.clone(),
        member_service.clone(),
    ));

    Ok((
        project_service,
        component_service,
        member_service,
        permission_service,
        project_permission_fetcher,
        member_permission_fetcher,
        component_permission_fetcher,
    ))
}

async fn setup_repositories(
    db: DbConnectionPool,
) -> Result<
    (
        Arc<ProjectRepositoryImpl>,
        Arc<ComponentRepositoryImpl>,
        Arc<MemberRepositoryImpl>,
        Arc<PermissionReadRepositoryImpl>,
        Arc<ResourceRepositoryImpl>,
        Arc<RolePermissionRepositoryImpl>,
        Arc<ProjectMemberRolePermissionRepositoryImpl>,
    ),
    Error,
> {
    // Project repository
    let project_read_repo = Arc::new(ProjectReadRepositoryImpl::new(db.get_read_connection()));
    let project_write_repo = Arc::new(ProjectWriteRepositoryImpl::new(db.get_write_connection()));
    let project_repo = Arc::new(ProjectRepositoryImpl::new(
        project_read_repo.clone(),
        project_write_repo.clone(),
    ));

    // Component repository
    let component_read_repo = Arc::new(ComponentReadRepositoryImpl::new(db.get_read_connection()));
    let component_write_repo = Arc::new(ComponentWriteRepositoryImpl::new(db.get_write_connection()));
    let component_repo = Arc::new(ComponentRepositoryImpl::new(
        component_read_repo.clone(),
        component_write_repo.clone(),
    ));

    // Permission repository (read-only)
    let permission_repo = Arc::new(PermissionReadRepositoryImpl::new(db.get_read_connection()));

    // Resource repository
    let resource_read_repo = Arc::new(ResourceReadRepositoryImpl::new(db.get_read_connection()));
    let resource_write_repo = Arc::new(ResourceWriteRepositoryImpl::new(db.get_write_connection()));
    let resource_repo = Arc::new(ResourceRepositoryImpl::new(
        resource_read_repo.clone(),
        resource_write_repo.clone(),
    ));

    // Role permission repository (needed by PMRP repos)
    let role_permission_read_repo = Arc::new(RolePermissionReadRepositoryImpl::new(db.get_read_connection()));
    let role_permission_write_repo = Arc::new(RolePermissionWriteRepositoryImpl::new(db.get_write_connection()));
    let role_permission_repo = Arc::new(RolePermissionRepositoryImpl::new(
        role_permission_read_repo.clone(),
        role_permission_write_repo.clone(),
    ));

    // Project member role permission repository (needed by member repos)
    let pmrp_read_repo = Arc::new(ProjectMemberRolePermissionReadRepositoryImpl::new(
        db.get_read_connection(),
        role_permission_read_repo.clone(),
    ));
    let pmrp_write_repo = Arc::new(ProjectMemberRolePermissionWriteRepositoryImpl::new(
        db.get_write_connection(),
        role_permission_read_repo.clone(),
    ));
    let pmrp_repo = Arc::new(ProjectMemberRolePermissionRepositoryImpl::new(
        pmrp_read_repo.clone(),
        pmrp_write_repo.clone(),
    ));

    // Member repository (needs pmrp_read_repo)
    let member_read_repo = Arc::new(MemberReadRepositoryImpl::new(
        db.get_read_connection(),
        pmrp_read_repo.clone(),
    ));
    let member_write_repo = Arc::new(MemberWriteRepositoryImpl::new(
        db.get_write_connection(),
        pmrp_read_repo.clone(),
    ));
    let member_repo = Arc::new(MemberRepositoryImpl::new(
        member_read_repo.clone(),
        member_write_repo.clone(),
    ));

    Ok((
        project_repo,
        component_repo,
        member_repo,
        permission_repo,
        resource_repo,
        role_permission_repo,
        pmrp_repo,
    ))
}
