//! Application setup for Telegraph

use axum::Router;
use rustycog_command::GenericCommandService;
use rustycog_config::ServerConfig;
use rustycog_db::DbConnectionPool;
use rustycog_http::{AppState, UserIdExtractor};
use rustycog_permission::{
    CachedPermissionChecker, MetricsPermissionChecker, OpenFgaPermissionChecker, PermissionChecker,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use telegraph_application::{
    command::TelegraphCommandRegistryFactory,
    usecase::{EventProcessingUseCase, NotificationUseCaseImpl},
};
use telegraph_configuration::TelegraphConfig;
use telegraph_domain::{
    service::NotificationServiceImpl, CommunicationFactory, EmailService, EventExtractor,
    EventProcessor, TemplateService,
};
use telegraph_http_server::{create_app_routes, create_router};
use telegraph_infra::{
    communication::EmailAdapter,
    event::processors::{CompositeEventProcessor, EventHandlerConfig},
    event::{EventConsumer, JsonEventExtractor},
    repository::{
        CombinedNotificationRepositoryImpl, NotificationReadRepositoryImpl,
        NotificationWriteRepositoryImpl,
    },
    template::TeraTemplateService,
};
use tracing::{error, info};

/// Telegraph application context
pub struct TelegraphApp {
    config: TelegraphConfig,
    state: AppState,
    event_consumer: Arc<EventConsumer>,
}

impl TelegraphApp {
    /// Create a new Telegraph application
    pub async fn new(config: TelegraphConfig) -> Result<Self, anyhow::Error> {
        info!("Starting Telegraph service and initializing components");

        // Create communication adapters
        let email_config = telegraph_infra::communication::EmailConfig {
            smtp_host: config.communication.email.smtp.host.clone(),
            smtp_port: config.communication.email.smtp.port,
            smtp_username: config
                .communication
                .email
                .smtp
                .username
                .clone()
                .unwrap_or_default(),
            smtp_password: config
                .communication
                .email
                .smtp
                .password
                .clone()
                .unwrap_or_default(),
            from_email: config.communication.email.from_address.clone(),
            from_name: config.communication.email.from_name.clone(),
            use_tls: config.communication.email.smtp.use_tls,
        };
        let email_adapter = EmailAdapter::new(email_config)
            .map_err(|e| anyhow::anyhow!("Failed to create email adapter: {}", e))?;

        info!("Email adapter created");
        let email_service: Arc<EmailService> = Arc::new(EmailService::new(Arc::new(email_adapter)));
        info!("Email service created");

        // Setup database connection pool
        let db_pool = DbConnectionPool::new(&config.database).await?;
        info!(
            "Database connection pool initialized with {} read replicas",
            if config.database.read_replicas.is_empty() {
                0
            } else {
                config.database.read_replicas.len()
            }
        );

        let notification_read_repo =
            NotificationReadRepositoryImpl::new(db_pool.get_read_connection());
        let notification_write_repo =
            NotificationWriteRepositoryImpl::new(db_pool.get_write_connection());
        let notification_repo = CombinedNotificationRepositoryImpl::new(
            Arc::new(notification_read_repo),
            Arc::new(notification_write_repo),
        );
        let notification_service =
            Arc::new(NotificationServiceImpl::new(Arc::new(notification_repo)));

        // Create template service
        let template_service: Arc<dyn TemplateService> = Arc::new(
            TeraTemplateService::new(config.communication.template.clone())
                .map_err(|e| anyhow::anyhow!("Failed to create template service: {}", e))?,
        );

        // Create event extractor for JSON processing
        let event_extractor: Arc<dyn EventExtractor> = Arc::new(JsonEventExtractor::new());

        // Create communication factory (using hardcoded path for now - should be configurable)
        let descriptor_dir = std::path::PathBuf::from("resources/communication_descriptor");
        let communication_factory = Arc::new(CommunicationFactory::new(
            template_service.clone(),
            event_extractor,
            descriptor_dir,
        ));

        let mut event_mapping = HashMap::new();
        let queues_config = config.queues.clone();
        for (_, event_config) in queues_config {
            for event_name in event_config.events {
                info!(
                    "Adding event mapping for event: {} with modes: {:?}",
                    event_name,
                    event_config.event_configs.get(&event_name).unwrap().modes
                );
                event_mapping.insert(
                    event_name.clone(),
                    event_config
                        .event_configs
                        .get(&event_name)
                        .unwrap()
                        .modes
                        .clone(),
                );
            }
        }
        let event_handler_config = EventHandlerConfig { event_mapping };
        // Create event processor (domain-level event processor)
        let domain_event_processor: Arc<dyn EventProcessor> =
            Arc::new(CompositeEventProcessor::with_all_processors(
                event_handler_config,
                email_service.clone(),
                communication_factory.clone(),
                notification_service.clone(),
            ));

        // Create use cases
        let event_processing_usecase =
            Arc::new(EventProcessingUseCase::new(domain_event_processor));

        // Create notification use case
        let notification_usecase =
            Arc::new(NotificationUseCaseImpl::new(notification_service.clone()));

        // Create command registry and service
        let command_registry =
            Arc::new(TelegraphCommandRegistryFactory::create_telegraph_registry(
                event_processing_usecase.clone(),
                notification_usecase,
            ));

        let command_service = Arc::new(GenericCommandService::new(command_registry));

        // Create event consumer with command service
        let event_consumer = EventConsumer::new(config.clone(), command_service.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create event consumer: {}", e))?;

        info!("Telegraph application initialized successfully");

        let user_id_extractor = UserIdExtractor::new(config.auth.clone())
            .map_err(|e| anyhow::anyhow!("Invalid auth configuration: {}", e))?;

        // Centralized permission checker (OpenFGA) with structured metrics
        // in front and an optional short-TTL cache. The cache is the
        // production default (15s) but can be disabled at test time by
        // setting `openfga.cache_ttl_seconds = 0` so flows that re-arrange
        // mock decisions mid-request observe the new decision instead of
        // a stale cached entry.
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

        let state = AppState::new(command_service, user_id_extractor, permission_checker);

        Ok(Self {
            config,
            event_consumer: Arc::new(event_consumer),
            state,
        })
    }

    /// Start the Telegraph service
    pub async fn run(&self, config: ServerConfig) -> Result<(), anyhow::Error> {
        let mut background_tasks = self.start_background_tasks();
        let Some(mut consumer_handle) = background_tasks.pop() else {
            return Err(anyhow::anyhow!(
                "Telegraph event consumer task was not started"
            ));
        };

        // Start axum server in a separate task
        info!("Starting HTTP server in parallel task");
        let mut server_handle = {
            let state = self.state.clone();
            let config = config.clone();
            tokio::spawn(async move {
                if let Err(e) = create_app_routes(state, config).await {
                    error!("HTTP server failed: {}", e);
                    return Err(e);
                }
                Ok(())
            })
        };

        info!(
            "Telegraph service started successfully - both event consumer and HTTP server are running"
        );

        // Wait for shutdown signal or any service to complete/fail
        let shutdown_result: Result<(), anyhow::Error> = tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received, stopping Telegraph service");
                Ok(())
            }
            result = &mut consumer_handle => {
                match result {
                    Ok(Ok(())) => {
                        info!("Event consumer completed successfully");
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        error!("Event consumer failed: {}", e);
                        Err(anyhow::anyhow!("Event consumer failed: {}", e))
                    }
                    Err(e) => {
                        error!("Event consumer task panicked: {}", e);
                        Err(anyhow::anyhow!("Event consumer task panicked: {}", e))
                    }
                }
            }
            result = &mut server_handle => {
                match result {
                    Ok(Ok(())) => {
                        info!("HTTP server completed successfully");
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        error!("HTTP server failed: {}", e);
                        Err(anyhow::anyhow!("HTTP server failed: {}", e))
                    }
                    Err(e) => {
                        error!("HTTP server task panicked: {}", e);
                        Err(anyhow::anyhow!("HTTP server task panicked: {}", e))
                    }
                }
            }
        };

        // Stop event consumer gracefully
        self.stop_background_tasks().await;
        if !consumer_handle.is_finished() {
            consumer_handle.abort();
        }
        if !server_handle.is_finished() {
            server_handle.abort();
        }

        info!("Telegraph service shut down complete");
        shutdown_result
    }

    pub fn router(&self) -> Router {
        create_router(self.state.clone())
    }

    pub fn start_background_tasks(&self) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        info!("Starting event consumer in parallel task");
        let event_consumer = self.event_consumer.clone();

        vec![tokio::spawn(async move {
            event_consumer
                .start()
                .await
                .map_err(|e| anyhow::anyhow!("Telegraph event consumer failed: {}", e))
        })]
    }

    pub async fn stop_background_tasks(&self) {
        if let Err(e) = self.event_consumer.stop().await {
            error!("Failed to stop event consumer: {}", e);
        }
    }
}

/// Application builder for Telegraph
pub struct AppBuilder {
    config: TelegraphConfig,
}

impl AppBuilder {
    /// Create a new app builder
    pub fn new(config: TelegraphConfig) -> Self {
        Self { config }
    }

    /// Build the Telegraph application
    pub async fn build(self) -> Result<TelegraphApp, anyhow::Error> {
        Ok(TelegraphApp::new(self.config).await?)
    }
}
