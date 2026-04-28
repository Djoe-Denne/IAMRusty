use std::sync::Arc;

use axum::Router;

// Manifesto
use manifesto_application::{
    ComponentUseCaseImpl, ManifestoCommandRegistryFactory, MemberUseCaseImpl, ProjectUseCaseImpl,
};
use manifesto_configuration::AppConfig;
use manifesto_domain::service::{ComponentServiceImpl, MemberServiceImpl, ProjectServiceImpl};
use manifesto_http_server::{create_app_routes, create_router};
use manifesto_infra::{
    adapters::ComponentServiceClient,
    processors::ComponentStatusProcessor,
    repository::{
        ComponentReadRepositoryImpl, ComponentRepositoryImpl, ComponentWriteRepositoryImpl,
        MemberReadRepositoryImpl, MemberRepositoryImpl, MemberWriteRepositoryImpl,
        PermissionReadRepositoryImpl, ProjectMemberRolePermissionReadRepositoryImpl,
        ProjectMemberRolePermissionRepositoryImpl, ProjectMemberRolePermissionWriteRepositoryImpl,
        ProjectReadRepositoryImpl, ProjectRepositoryImpl, ProjectWriteRepositoryImpl,
        ResourceReadRepositoryImpl, ResourceRepositoryImpl, ResourceWriteRepositoryImpl,
        RolePermissionReadRepositoryImpl, RolePermissionRepositoryImpl,
        RolePermissionWriteRepositoryImpl,
    },
    ApparatusEventConsumer, ManifestoErrorMapper, ProjectCreationUnitOfWorkImpl,
};

// Rustycog
use rustycog_command::GenericCommandService;
use rustycog_config::ServerConfig;
use rustycog_core::error::DomainError;
use rustycog_db::DbConnectionPool;
use rustycog_events::{create_multi_queue_event_publisher, EventPublisher};
use rustycog_http::{AppState, UserIdExtractor};
use rustycog_outbox::{OutboxConfig, OutboxDispatcher, OutboxRecorder};
use rustycog_permission::{
    CachedPermissionChecker, MetricsPermissionChecker, OpenFgaPermissionChecker, PermissionChecker,
};
use std::time::Duration;

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
    pub apparatus_event_consumer: Option<Arc<ApparatusEventConsumer>>,
    pub outbox_dispatcher: Arc<OutboxDispatcher<DomainError>>,
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

        // Setup event publisher for Telegraph + sentinel-sync communication
        let event_publisher = if let Some(ep) = maybe_event_publisher {
            ep
        } else {
            create_multi_queue_event_publisher(&config.queue, None, Arc::new(ManifestoErrorMapper))
                .await?
        };

        // Setup use cases
        let outbox_dispatcher = Arc::new(OutboxDispatcher::new(
            db.clone(),
            event_publisher.clone(),
            OutboxConfig::default(),
        ));
        let (project_usecase, component_usecase, member_usecase, apparatus_event_consumer) =
            setup_application(db, &config, event_publisher).await?;

        // Setup command registry
        let command_registry = ManifestoCommandRegistryFactory::create_manifesto_registry(
            project_usecase,
            component_usecase,
            member_usecase,
            config.command.clone(),
        );

        // Create command service
        let command_service = Arc::new(GenericCommandService::new(Arc::new(command_registry)));

        // Setup user ID extractor (for authentication)
        let user_id_extractor = UserIdExtractor::new(config.auth.clone())
            .map_err(|e| anyhow::anyhow!("Invalid auth configuration: {}", e))?;

        // Centralized permission checker (OpenFGA) with structured metrics in
        // front and an optional short-TTL cache. The cache is the production
        // default (15s) but can be disabled at test time by setting
        // `openfga.cache_ttl_seconds = 0` so flows that revoke a permission
        // mid-request observe the new decision instead of the cached one.
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

        tracing::info!("Manifesto application initialized successfully");

        Ok(Self {
            config,
            state,
            apparatus_event_consumer,
            outbox_dispatcher,
        })
    }

    /// Start the HTTP server
    pub async fn run(self, server_config: ServerConfig) -> Result<(), Error> {
        let mut server_handle = {
            let state = self.state.clone();
            let server_config = server_config.clone();

            tokio::spawn(async move {
                create_app_routes(state, server_config)
                    .await
                    .map_err(|e| anyhow::anyhow!("HTTP server failed: {}", e))
            })
        };

        let background_tasks = self.start_background_tasks();
        if !background_tasks.is_empty() {
            let mut background_handle = tokio::spawn(async move {
                let mut join_set = tokio::task::JoinSet::new();

                for task in background_tasks {
                    join_set.spawn(async move {
                        task.await.map_err(|error| {
                            anyhow::anyhow!("Manifesto background task panicked: {}", error)
                        })?
                    });
                }

                while let Some(result) = join_set.join_next().await {
                    result.map_err(|error| {
                        anyhow::anyhow!("Manifesto background task monitor panicked: {}", error)
                    })??;
                }

                Ok::<(), Error>(())
            });
            tracing::info!("Starting Manifesto HTTP server and background tasks");

            let shutdown_result: Result<(), Error> = tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Shutdown signal received; stopping Manifesto runtime");
                    Ok(())
                }
                result = &mut background_handle => {
                    match result {
                        Ok(Ok(())) => {
                            tracing::info!("Manifesto background tasks completed");
                            Ok(())
                        }
                        Ok(Err(error)) => Err(error),
                        Err(error) => Err(anyhow::anyhow!("Manifesto background supervisor task panicked: {}", error)),
                    }
                }
                result = &mut server_handle => {
                    match result {
                        Ok(Ok(())) => {
                            tracing::info!("HTTP server completed");
                            Ok(())
                        }
                        Ok(Err(error)) => Err(error),
                        Err(error) => Err(anyhow::anyhow!("HTTP server task panicked: {}", error)),
                    }
                }
            };

            self.stop_background_tasks().await;
            if !background_handle.is_finished() {
                background_handle.abort();
            }
            if !server_handle.is_finished() {
                server_handle.abort();
            }

            let _ = background_handle.await;
            let _ = server_handle.await;

            return shutdown_result;
        }

        tracing::info!("Starting Manifesto HTTP server without apparatus queue consumer");

        let shutdown_result: Result<(), Error> = tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Shutdown signal received; stopping Manifesto HTTP server");
                Ok(())
            }
            result = &mut server_handle => {
                match result {
                    Ok(Ok(())) => {
                        tracing::info!("HTTP server completed");
                        Ok(())
                    }
                    Ok(Err(error)) => Err(error),
                    Err(error) => Err(anyhow::anyhow!("HTTP server task panicked: {}", error)),
                }
            }
        };

        if !server_handle.is_finished() {
            server_handle.abort();
        }
        let _ = server_handle.await;

        shutdown_result
    }

    pub fn router(&self) -> Router {
        create_router(self.state.clone())
    }

    #[must_use]
    pub fn start_background_tasks(&self) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let mut tasks = Vec::new();

        if let Some(consumer) = self.apparatus_event_consumer.clone() {
            tasks.push(tokio::spawn(async move {
                consumer
                    .start()
                    .await
                    .map_err(|e| anyhow::anyhow!("Manifesto apparatus consumer failed: {}", e))
            }));
        }

        let dispatcher = self.outbox_dispatcher.clone();
        tasks.push(tokio::spawn(async move {
            dispatcher
                .start()
                .await
                .map_err(|e| anyhow::anyhow!("Manifesto outbox dispatcher failed: {}", e))
        }));

        tasks
    }

    pub async fn stop_background_tasks(&self) {
        if let Err(e) = self.outbox_dispatcher.stop().await {
            tracing::error!("Failed to stop Manifesto outbox dispatcher: {}", e);
        }

        if let Some(consumer) = self.apparatus_event_consumer.clone() {
            if let Err(e) = consumer.stop().await {
                tracing::error!("Failed to stop Manifesto apparatus consumer: {}", e);
            }
        }
    }
}

/// Setup database connection
async fn setup_database(config: &AppConfig) -> Result<DbConnectionPool, Error> {
    tracing::info!("Connecting to database");

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
        Option<Arc<ApparatusEventConsumer>>,
    ),
    Error,
> {
    let (project_service, component_service, member_service, permission_service) =
        setup_domain(db.clone(), config).await?;
    let project_creation_uow = Arc::new(ProjectCreationUnitOfWorkImpl::new(
        db,
        OutboxRecorder::new(),
    ));

    let project_usecase = Arc::new(ProjectUseCaseImpl::new_with_project_creation_uow(
        project_service.clone(),
        component_service.clone(),
        member_service.clone(),
        permission_service.clone(),
        event_publisher.clone(),
        config.service.business.clone(),
        project_creation_uow,
    ));

    let component_usecase = Arc::new(ComponentUseCaseImpl::new(
        component_service.clone(),
        project_service.clone(),
        permission_service.clone(),
        event_publisher.clone(),
        config.service.business.clone(),
    ));

    let member_usecase = Arc::new(MemberUseCaseImpl::new(
        member_service.clone(),
        project_service.clone(),
        permission_service.clone(),
        event_publisher.clone(),
        config.service.business.clone(),
    ));

    let apparatus_event_consumer = {
        let component_status_processor =
            Arc::new(ComponentStatusProcessor::new(component_service.clone()));
        let consumer = ApparatusEventConsumer::new(&config.queue, component_status_processor)
            .await
            .map_err(|error| {
                anyhow::anyhow!("Failed to create apparatus event consumer: {}", error)
            })?;

        if consumer.is_noop() {
            tracing::info!(
                "Apparatus event consumer is disabled or unavailable; Manifesto will run without a background queue consumer"
            );
            None
        } else {
            Some(Arc::new(consumer))
        }
    };

    Ok((
        project_usecase,
        component_usecase,
        member_usecase,
        apparatus_event_consumer,
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
    ),
    Error,
> {
    let (
        project_repo,
        component_repo,
        member_repo,
        permission_repo,
        resource_repo,
        role_permission_repo,
        member_role_permission_repo,
    ) = setup_repositories(db.clone()).await?;

    let component_service_adapter = Arc::new(ComponentServiceClient::new(
        config.service.component_service.base_url.clone(),
        config.service.component_service.api_key.clone(),
        config.service.component_service.timeout_seconds,
    )?);

    let permission_service: Arc<dyn manifesto_domain::service::PermissionService> =
        Arc::new(manifesto_domain::service::PermissionServiceImpl::new(
            permission_repo,
            resource_repo,
            role_permission_repo,
            member_role_permission_repo,
        ));

    let project_service = Arc::new(ProjectServiceImpl::new(
        project_repo,
        component_repo.clone(),
    ));

    let component_service = Arc::new(ComponentServiceImpl::new(
        component_repo,
        component_service_adapter,
        permission_service.clone(),
    ));

    let member_service = Arc::new(MemberServiceImpl::new(member_repo));

    Ok((
        project_service,
        component_service,
        member_service,
        permission_service,
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
    let project_read_repo = Arc::new(ProjectReadRepositoryImpl::new(db.get_read_connection()));
    let project_write_repo = Arc::new(ProjectWriteRepositoryImpl::new(db.get_write_connection()));
    let project_repo = Arc::new(ProjectRepositoryImpl::new(
        project_read_repo,
        project_write_repo,
    ));

    let component_read_repo = Arc::new(ComponentReadRepositoryImpl::new(db.get_read_connection()));
    let component_write_repo =
        Arc::new(ComponentWriteRepositoryImpl::new(db.get_write_connection()));
    let component_repo = Arc::new(ComponentRepositoryImpl::new(
        component_read_repo,
        component_write_repo,
    ));

    let permission_repo = Arc::new(PermissionReadRepositoryImpl::new(db.get_read_connection()));

    let resource_read_repo = Arc::new(ResourceReadRepositoryImpl::new(db.get_read_connection()));
    let resource_write_repo = Arc::new(ResourceWriteRepositoryImpl::new(db.get_write_connection()));
    let resource_repo = Arc::new(ResourceRepositoryImpl::new(
        resource_read_repo,
        resource_write_repo,
    ));

    let role_permission_read_repo = Arc::new(RolePermissionReadRepositoryImpl::new(
        db.get_read_connection(),
    ));
    let role_permission_write_repo = Arc::new(RolePermissionWriteRepositoryImpl::new(
        db.get_write_connection(),
    ));
    let role_permission_repo = Arc::new(RolePermissionRepositoryImpl::new(
        role_permission_read_repo.clone(),
        role_permission_write_repo,
    ));

    let pmrp_read_repo = Arc::new(ProjectMemberRolePermissionReadRepositoryImpl::new(
        db.get_read_connection(),
        role_permission_read_repo.clone(),
    ));
    let pmrp_write_repo = Arc::new(ProjectMemberRolePermissionWriteRepositoryImpl::new(
        db.get_write_connection(),
        role_permission_read_repo,
    ));
    let pmrp_repo = Arc::new(ProjectMemberRolePermissionRepositoryImpl::new(
        pmrp_read_repo.clone(),
        pmrp_write_repo,
    ));

    let member_read_repo = Arc::new(MemberReadRepositoryImpl::new(
        db.get_read_connection(),
        pmrp_read_repo.clone(),
    ));
    let member_write_repo = Arc::new(MemberWriteRepositoryImpl::new(
        db.get_write_connection(),
        pmrp_read_repo,
    ));
    let member_repo = Arc::new(MemberRepositoryImpl::new(
        member_read_repo,
        member_write_repo,
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
