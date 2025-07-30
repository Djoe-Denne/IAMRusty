use std::sync::Arc;

use hive_application::{
    ExternalLinkUseCaseImpl, HiveCommandRegistryFactory, InvitationUseCaseImpl, MemberUseCaseImpl,
    OrganizationUseCaseImpl, SyncJobUseCaseImpl,
};
use hive_configuration::ServiceConfig;
use hive_http::create_app_routes;
use hive_infra::{
    create_event_publisher_with_queue_config, ConfluenceProvider, GitHubProvider, GitLabProvider,
    TelegraphEventPublisher,
};
use hive_migration::Migrator;
use rustycog_command::GenericCommandService;
use rustycog_events::{ErrorMapper, MultiQueueEventPublisher};
use rustycog_http::{AppState, UserIdExtractor};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use thiserror::Error;

/// Application setup errors
#[derive(Debug, Error)]
pub enum SetupError {
    #[error("Database connection failed: {message}")]
    DatabaseError { message: String },

    #[error("Migration failed: {message}")]
    MigrationError { message: String },

    #[error("HTTP server setup failed: {message}")]
    ServerError { message: String },

    #[error("Event publisher setup failed: {message}")]
    EventPublisherError { message: String },

    #[error("Configuration error: {0}")]
    Config(#[from] hive_configuration::ConfigError),
}

// Use AppState from rustycog-http - no need to define our own

/// Application context for dependency injection
pub struct Application {
    pub config: ServiceConfig,
    pub state: AppState,
}

impl Application {
    /// Create a new application instance with all dependencies
    pub async fn new(config: ServiceConfig) -> Result<Self, SetupError> {
        tracing::info!("Initializing Hive application...");

        // Setup database connection
        let db = setup_database(&config).await?;

        // Run migrations if enabled
        if config.database.run_migrations {
            run_migrations(&db).await?;
        }

        // Setup event publisher for Telegraph communication
        let event_publisher = setup_event_publisher(&config).await?;

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
        let user_id_extractor = UserIdExtractor::new(config.auth.jwt_secret.clone());

        // Create application state
        let state = AppState::new(command_service, user_id_extractor);

        tracing::info!("Hive application initialized successfully");

        Ok(Application { config, state })
    }

    /// Start the HTTP server
    pub async fn serve(self) -> Result<(), SetupError> {
        tracing::info!("Starting Hive HTTP server...");

        create_app_routes(self.state, self.config.server)
            .await
            .map_err(|e| SetupError::ServerError {
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
async fn setup_database(config: &ServiceConfig) -> Result<DatabaseConnection, SetupError> {
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
        .map_err(|e| SetupError::DatabaseError {
            message: e.to_string(),
        })?;

    tracing::info!("Database connection established");
    Ok(db)
}

/// Run database migrations
async fn run_migrations(db: &DatabaseConnection) -> Result<(), SetupError> {
    tracing::info!("Running database migrations...");

    Migrator::up(db, None)
        .await
        .map_err(|e| SetupError::MigrationError {
            message: e.to_string(),
        })?;

    tracing::info!("Database migrations completed");
    Ok(())
}

/// Setup event publisher for Telegraph communication
async fn setup_event_publisher(
    config: &ServiceConfig,
) -> Result<Arc<TelegraphEventPublisher>, SetupError> {
    tracing::info!("Setting up event publisher for Telegraph service...");

    if !config.external_services.events.enabled {
        tracing::warn!("Event publishing is disabled - notifications will not be sent");
    }

    let concrete_publisher = create_event_publisher_with_queue_config(config.queue_config())
        .await
        .map_err(|e| SetupError::EventPublisherError {
            message: format!("Failed to create event publisher: {}", e),
        })?;

    let telegraph_publisher = Arc::new(TelegraphEventPublisher::new(concrete_publisher));

    tracing::info!("Event publisher configured for Telegraph service");
    Ok(telegraph_publisher)
}

/// Setup use cases with their dependencies
async fn setup_use_cases(
    db: DatabaseConnection,
    event_publisher: Arc<TelegraphEventPublisher>,
) -> Result<
    (
        Arc<dyn hive_application::OrganizationUseCase>,
        Arc<dyn hive_application::MemberUseCase>,
        Arc<dyn hive_application::InvitationUseCase>,
        Arc<dyn hive_application::ExternalLinkUseCase>,
        Arc<dyn hive_application::SyncJobUseCase>,
    ),
    SetupError,
> {
    // Create error mapper for domain errors
    let error_mapper = Arc::new(DomainErrorMapper);

    // Create MultiQueueEventPublisher with domain error mapping
    let multi_queue_publisher = Arc::new(MultiQueueEventPublisher::new(
        vec![rustycog_events::GenericEventPublisherAdapter::new(
            event_publisher.inner(),
            error_mapper,
        )],
        std::collections::HashSet::new(),
    ));

    // TODO: Setup repositories when infrastructure layer is ready
    // For now, create placeholder use cases

    // Create organization use case
    let organization_usecase = Arc::new(OrganizationUseCaseImpl::new(
        todo!("organization_service"),
        todo!("organization_repository"),
        multi_queue_publisher.clone(),
    ));

    // Create member use case
    let member_usecase = Arc::new(MemberUseCaseImpl::new(
        todo!("member_service"),
        todo!("member_repository"),
        todo!("organization_repository"),
        multi_queue_publisher.clone(),
    ));

    // Create invitation use case
    let invitation_usecase = Arc::new(InvitationUseCaseImpl::new(
        todo!("invitation_repository"),
        multi_queue_publisher.clone(),
    ));

    // Create external link use case
    let external_link_usecase = Arc::new(ExternalLinkUseCaseImpl::new(
        todo!("external_link_repository"),
        multi_queue_publisher.clone(),
    ));

    // Create sync job use case
    let sync_job_usecase = Arc::new(SyncJobUseCaseImpl::new(
        todo!("sync_job_repository"),
        multi_queue_publisher,
    ));

    Ok((
        organization_usecase,
        member_usecase,
        invitation_usecase,
        external_link_usecase,
        sync_job_usecase,
    ))
}

/// Error mapper for converting domain errors to service errors
struct DomainErrorMapper;

impl ErrorMapper<hive_domain::DomainError> for DomainErrorMapper {
    fn from_service_error(
        &self,
        error: rustycog_core::error::ServiceError,
    ) -> hive_domain::DomainError {
        hive_domain::DomainError::Internal {
            message: error.to_string(),
        }
    }
}

/// Setup external providers
fn setup_external_providers(
    config: &ServiceConfig,
) -> (GitHubProvider, GitLabProvider, ConfluenceProvider) {
    tracing::info!("Setting up external providers...");

    let github = GitHubProvider::new();
    let gitlab = GitLabProvider::new();
    let confluence = ConfluenceProvider::new();

    tracing::info!(
        github_enabled = config.features.github_integration,
        gitlab_enabled = config.features.gitlab_integration,
        confluence_enabled = config.features.confluence_integration,
        "External providers configured"
    );

    (github, gitlab, confluence)
}

/// Setup repositories
async fn setup_repositories(_db: &DatabaseConnection) -> Result<(), SetupError> {
    tracing::info!("Setting up repositories...");

    // TODO: Initialize repository implementations
    // let organization_repo = PostgresOrganizationRepository::new(db.clone());
    // let member_repo = PostgresOrganizationMemberRepository::new(db.clone());
    // ... etc

    tracing::info!("Repositories initialized");
    Ok(())
}
