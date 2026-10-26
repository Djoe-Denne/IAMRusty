//! Application setup for Telegraph

use crate::runtime::{
    combine_results, flatten_join_result, shutdown_owned_tasks, wait_for_background_failure,
    OwnedTaskAbortGuard,
};
use axum::Router;
use readiness::{
    attach_ready, create_signaled_multi_queue_event_publisher, ComponentStatus, ReadinessProbe,
    SignaledPublisher,
};
use rustycog::command::GenericCommandService;
use rustycog::config::ServerConfig;
use rustycog::db::DbConnectionPool;
use rustycog::events::ConcreteEventPublisher;
use rustycog::http::{AppState, UserIdExtractor};
use rustycog::outbox::{OutboxConfig, OutboxDispatcher, OutboxRecorder};
use rustycog::permission::{
    CachedPermissionChecker, MetricsPermissionChecker, OpenFgaPermissionChecker, PermissionChecker,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use telegraph_application::{
    command::TelegraphCommandRegistryFactory,
    usecase::{EventProcessingUseCase, NotificationUseCaseImpl},
};
use telegraph_configuration::TelegraphConfig;
use telegraph_domain::{
    service::NotificationServiceImpl, CommunicationFactory, DomainError, EmailService,
    EventExtractor, EventProcessor, NotificationService, TemplateService,
};
use telegraph_http_server::{create_app_routes, create_router};
use telegraph_infra::{
    communication::EmailAdapter,
    event::processors::{CompositeEventProcessor, EventHandlerConfig},
    event::{EventConsumer, JsonEventExtractor, TelegraphErrorMapper},
    repository::{
        CombinedNotificationRepositoryImpl, NotificationReadRepositoryImpl,
        NotificationWriteRepositoryImpl,
    },
    template::TeraTemplateService,
};
use tracing::info;

/// Telegraph application context
pub struct TelegraphApp {
    state: AppState,
    event_consumer: Arc<EventConsumer>,
    readiness: Arc<ReadinessProbe>,
    dispatcher: Option<Arc<OutboxDispatcher<DomainError>>>,
    background_stop: tokio::sync::watch::Sender<bool>,
    background_started: AtomicBool,
}

impl TelegraphApp {
    /// Create a new Telegraph application
    ///
    /// # Errors
    ///
    /// Returns an error when the email adapter, database pool, template service,
    /// event consumer, auth extractor, or `OpenFGA` checker cannot be initialized,
    /// or when a configured event name is missing from `event_configs`.
    pub async fn new(config: TelegraphConfig) -> Result<Self, anyhow::Error> {
        info!("Starting Telegraph service and initializing components");

        let email_service = setup_email_service(&config)?;

        let db_pool = Box::pin(DbConnectionPool::new(&config.database)).await?;
        let db_write = db_pool.get_write_connection();
        info!(
            "Database connection pool initialized with {} read replicas",
            if config.database.read_replicas.is_empty() {
                0
            } else {
                config.database.read_replicas.len()
            }
        );

        let (notification_service, signaled_publisher) =
            Box::pin(setup_notification_service(&db_pool, &config)).await?;
        // Never give a no-op's successful return permission to mark durable rows
        // published. The factory status AND the concrete transport are required.
        let dispatcher = if matches!(signaled_publisher.status, ComponentStatus::Live { .. })
            && !matches!(
                signaled_publisher.transport.as_ref(),
                ConcreteEventPublisher::NoOp(_)
            ) {
            Some(Arc::new(OutboxDispatcher::new(
                db_pool.clone(),
                signaled_publisher.publisher.clone(),
                OutboxConfig::default(),
            )))
        } else {
            info!(status = ?signaled_publisher.status, "Telegraph outbox dispatch suspended; intentions remain durable");
            None
        };
        let communication_factory = setup_communication_factory(&config)?;
        let event_handler_config = EventHandlerConfig {
            event_mapping: setup_event_mapping(&config)?,
        };

        let domain_event_processor: Arc<dyn EventProcessor> =
            Arc::new(CompositeEventProcessor::with_all_processors(
                event_handler_config,
                email_service,
                communication_factory,
                notification_service.clone(),
            ));

        let event_processing_usecase =
            Arc::new(EventProcessingUseCase::new(domain_event_processor));
        let notification_usecase = Arc::new(NotificationUseCaseImpl::new(notification_service));

        let command_registry =
            Arc::new(TelegraphCommandRegistryFactory::create_telegraph_registry(
                event_processing_usecase,
                notification_usecase,
                &config.command,
            ));
        let command_service = Arc::new(GenericCommandService::new(command_registry));

        let event_consumer = Box::pin(EventConsumer::new(config.clone(), command_service.clone()))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create event consumer: {e}"))?;
        let consumer_status = event_consumer.transport_status().clone();
        let consumer_transport = event_consumer.inner();

        info!("Telegraph application initialized successfully");

        // Same HS256 secret as IAM `[jwt.secret]` — rustycog-http cannot
        // verify IAM JWKS/RS256 (rustycog-framework 0.1.1).
        let user_id_extractor = UserIdExtractor::new(config.auth.clone())
            .map_err(|e| anyhow::anyhow!("Invalid auth configuration: {e}"))?;
        let permission_checker = setup_permission_checker(&config)?;

        let state = AppState::new(command_service, user_id_extractor, permission_checker);
        let readiness = Arc::new(
            ReadinessProbe::new("telegraph")
                .with_database(db_write)
                .with_publisher(
                    signaled_publisher.status,
                    Some(signaled_publisher.transport),
                )
                .with_consumer(consumer_status, Some(consumer_transport)),
        );

        Ok(Self {
            event_consumer: Arc::new(event_consumer),
            state,
            readiness,
            dispatcher,
            background_stop: tokio::sync::watch::channel(false).0,
            background_started: AtomicBool::new(false),
        })
    }

    /// Start the Telegraph service
    ///
    /// # Errors
    ///
    /// Returns an error when an active background task exits unexpectedly, the
    /// HTTP server fails, or bounded cleanup fails.
    pub async fn run(&self, config: ServerConfig) -> Result<(), anyhow::Error> {
        let mut background_tasks = self.start_background_tasks();

        // Start axum server in a separate task
        info!("Starting HTTP server in parallel task");
        let mut server_handle = {
            let state = self.state.clone();
            let probe = self.readiness.clone();
            let config = config.clone();
            let mut stop = self.background_stop.subscribe();
            tokio::spawn(async move {
                tokio::select! {
                    result = create_app_routes(state, config, probe) => result,
                    () = wait_for_stop(&mut stop) => Ok(()),
                }
            })
        };
        let _abort_background_on_cancel = OwnedTaskAbortGuard::new(&background_tasks);
        let _abort_http_on_cancel = OwnedTaskAbortGuard::new(std::slice::from_ref(&server_handle));
        let (result, server_finished) = tokio::select! {
            signal = tokio::signal::ctrl_c() => (signal.map_err(Into::into), false),
            result = wait_for_background_failure(&mut background_tasks) => (result, false),
            result = &mut server_handle => (flatten_join_result("Telegraph HTTP", result), true),
        };
        if !server_finished {
            background_tasks.push(server_handle);
        }
        let cleanup =
            shutdown_owned_tasks(&mut background_tasks, self.stop_background_tasks()).await;
        info!("Telegraph service shut down complete");
        combine_results(result, cleanup)
    }

    pub fn router(&self) -> Router {
        attach_ready(create_router(self.state.clone()), self.readiness.clone())
    }

    #[must_use]
    pub fn readiness(&self) -> Arc<ReadinessProbe> {
        self.readiness.clone()
    }

    pub fn start_background_tasks(&self) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        // Rebuild the app to resume after shutdown: never reset stop beneath an
        // existing consumer/dispatcher or run two dispatchers on this writer.
        if self.background_started.swap(true, Ordering::SeqCst) {
            return vec![tokio::spawn(async {
                anyhow::bail!("Telegraph background tasks already started")
            })];
        }
        self.background_stop.send_replace(false);
        let mut tasks = Vec::new();
        if matches!(
            self.event_consumer.transport_status(),
            ComponentStatus::Live { .. }
        ) && !self.event_consumer.is_noop()
        {
            let consumer = self.event_consumer.clone();
            let stop = self.background_stop.subscribe();
            tasks.push(tokio::spawn(async move {
                consumer
                    .start()
                    .await
                    .map_err(|error| anyhow::anyhow!("Telegraph consumer failed: {error}"))?;
                if !*stop.borrow() {
                    anyhow::bail!("Active Telegraph consumer exited unexpectedly");
                }
                Ok(())
            }));
        }
        if let Some(dispatcher) = &self.dispatcher {
            let dispatcher = dispatcher.clone();
            let mut stop = self.background_stop.subscribe();
            tasks.push(tokio::spawn(async move {
                tokio::select! {
                    result = dispatcher.start() => {
                        result.map_err(|error| anyhow::anyhow!("Telegraph dispatcher failed: {error}"))?;
                        if !*stop.borrow() {
                            anyhow::bail!("Active Telegraph dispatcher exited unexpectedly");
                        }
                        Ok(())
                    }
                    () = wait_for_stop(&mut stop) => Ok(()),
                }
            }));
        }
        tasks
    }

    /// Request both stops; the runtime owner bounds and joins all tasks.
    ///
    /// # Errors
    /// Returns consumer or dispatcher stop failures without hiding either.
    pub async fn stop_background_tasks(&self) -> anyhow::Result<()> {
        self.background_stop.send_replace(true);
        let consumer = async {
            self.event_consumer
                .stop()
                .await
                .map_err(|error| anyhow::anyhow!("Telegraph consumer stop: {error}"))
        };
        let dispatcher = async {
            if let Some(dispatcher) = &self.dispatcher {
                dispatcher
                    .stop()
                    .await
                    .map_err(|error| anyhow::anyhow!("Telegraph dispatcher stop: {error}"))?;
            }
            Ok(())
        };
        let (consumer, dispatcher) = tokio::join!(consumer, dispatcher);
        combine_results(consumer, dispatcher)
    }
}

async fn wait_for_stop(stop: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *stop.borrow_and_update() {
            return;
        }
        if stop.changed().await.is_err() {
            return;
        }
    }
}

fn setup_email_service(config: &TelegraphConfig) -> Result<Arc<EmailService>, anyhow::Error> {
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
        .map_err(|e| anyhow::anyhow!("Failed to create email adapter: {e}"))?;

    info!("Email adapter created");
    let email_service = Arc::new(EmailService::new(Arc::new(email_adapter)));
    info!("Email service created");
    Ok(email_service)
}

async fn setup_notification_service(
    db_pool: &DbConnectionPool,
    config: &TelegraphConfig,
) -> Result<(Arc<dyn NotificationService>, SignaledPublisher<DomainError>), anyhow::Error> {
    let notification_read_repo = NotificationReadRepositoryImpl::new(db_pool.get_read_connection());
    let notification_write_repo =
        NotificationWriteRepositoryImpl::new(db_pool.get_write_connection(), OutboxRecorder::new());
    let notification_repo = CombinedNotificationRepositoryImpl::new(
        Arc::new(notification_read_repo),
        Arc::new(notification_write_repo),
    );
    let signaled_publisher = create_signaled_multi_queue_event_publisher(
        "telegraph",
        &config.queue,
        None,
        Arc::new(TelegraphErrorMapper),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to create event publisher: {e}"))?;
    let notification_service = Arc::new(NotificationServiceImpl::new(Arc::new(notification_repo)));
    Ok((notification_service, signaled_publisher))
}

fn setup_communication_factory(
    config: &TelegraphConfig,
) -> Result<Arc<CommunicationFactory>, anyhow::Error> {
    let template_service: Arc<dyn TemplateService> = Arc::new(
        TeraTemplateService::new(config.communication.template.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create template service: {e}"))?,
    );
    let event_extractor: Arc<dyn EventExtractor> = Arc::new(JsonEventExtractor::new());
    let descriptor_dir = std::path::PathBuf::from(&config.communication.template.descriptor_dir);
    Ok(Arc::new(CommunicationFactory::new(
        template_service,
        event_extractor,
        descriptor_dir,
    )))
}

/// Build event-name → communication-modes from queue config.
///
/// # Errors
///
/// Returns an error if a configured event name is missing from `event_configs`.
fn setup_event_mapping(
    config: &TelegraphConfig,
) -> Result<HashMap<String, Vec<String>>, anyhow::Error> {
    let mut event_mapping = HashMap::new();
    for event_config in config.queues.values() {
        for event_name in &event_config.events {
            let modes = event_config
                .event_configs
                .get(event_name)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "configured event name `{event_name}` missing from event_configs"
                    )
                })?
                .modes
                .clone();
            info!("Adding event mapping for event: {event_name} with modes: {modes:?}");
            event_mapping.insert(event_name.clone(), modes);
        }
    }
    Ok(event_mapping)
}

fn setup_permission_checker(
    config: &TelegraphConfig,
) -> Result<Arc<dyn PermissionChecker>, anyhow::Error> {
    // Centralized permission checker (OpenFGA) with structured metrics
    // in front and an optional short-TTL cache. The cache is the
    // production default (15s) but can be disabled at test time by
    // setting `openfga.cache_ttl_seconds = 0` so flows that re-arrange
    // mock decisions mid-request observe the new decision instead of
    // a stale cached entry.
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
    Ok(Arc::new(MetricsPermissionChecker::new(metered_inner)))
}

/// Application builder for Telegraph
pub struct AppBuilder {
    config: TelegraphConfig,
}

impl AppBuilder {
    /// Create a new app builder
    #[must_use]
    pub const fn new(config: TelegraphConfig) -> Self {
        Self { config }
    }

    /// Build the Telegraph application
    ///
    /// # Errors
    ///
    /// Returns an error when [`TelegraphApp::new`] fails.
    pub async fn build(self) -> Result<TelegraphApp, anyhow::Error> {
        Box::pin(TelegraphApp::new(self.config)).await
    }
}
