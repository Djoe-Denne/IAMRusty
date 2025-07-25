//! Application setup for Telegraph

use std::collections::HashMap;
use tracing::{info, error};
use std::sync::Arc;
use telegraph_domain::{EmailService, NotificationService, TemplateService, EventProcessor, EventExtractor, CommunicationFactory};
use telegraph_infra::{
    event::{EventConsumer, JsonEventExtractor},
    communication::EmailAdapter,
    template::TeraTemplateService,
    event::processors::{CompositeEventProcessor, EventHandlerConfig},
    repository::{NotificationReadRepositoryImpl, NotificationWriteRepositoryImpl, CombinedNotificationRepositoryImpl},
};
use telegraph_application::{
    usecase::EventProcessingUseCase,
    command::TelegraphCommandRegistryFactory,
};
use rustycog_command::GenericCommandService;
use rustycog_db::DbConnectionPool;
use telegraph_configuration::TelegraphConfig;

/// Telegraph application context
pub struct TelegraphApp {
    config: TelegraphConfig,
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
            smtp_username: config.communication.email.smtp.username.clone().unwrap_or_default(),
            smtp_password: config.communication.email.smtp.password.clone().unwrap_or_default(),
            from_email: config.communication.email.from_address.clone(),
            from_name: config.communication.email.from_name.clone(),
            use_tls: config.communication.email.smtp.use_tls,
        };
        println!("Email config: {:?}", email_config);
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

        let notification_read_repo = NotificationReadRepositoryImpl::new(db_pool.get_read_connection());
        let notification_write_repo = NotificationWriteRepositoryImpl::new(db_pool.get_write_connection());
        let notification_repo = CombinedNotificationRepositoryImpl::new(Arc::new(notification_read_repo), Arc::new(notification_write_repo));
        let notification_service: Arc<NotificationService> = Arc::new(NotificationService::new(Arc::new(notification_repo)));
        
        // Create template service
        let template_service: Arc<dyn TemplateService> = Arc::new(
            TeraTemplateService::new(config.communication.template.clone())
                .map_err(|e| anyhow::anyhow!("Failed to create template service: {}", e))?
        );
        
        // Create event extractor for JSON processing
        let event_extractor: Arc<dyn EventExtractor> = Arc::new(
            JsonEventExtractor::new()
        );
        
        // Create communication factory (using hardcoded path for now - should be configurable)
        let descriptor_dir = std::path::PathBuf::from("resources/communication_descriptor");
        let communication_factory = Arc::new(
            CommunicationFactory::new(
                template_service.clone(),
                event_extractor,
                descriptor_dir,
            )
        );
        
        let mut event_mapping = HashMap::new();
        let queues_config = config.queues.clone();
        for (_, event_config) in queues_config {
            for event_name in event_config.events {
                info!("Adding event mapping for event: {} with modes: {:?}", event_name, event_config.event_configs.get(&event_name).unwrap().modes);
                event_mapping.insert(event_name.clone(), event_config.event_configs.get(&event_name).unwrap().modes.clone());
            }
        }
        let event_handler_config = EventHandlerConfig {
            event_mapping,
        };
        // Create event processor (domain-level event processor)
        let domain_event_processor: Arc<dyn EventProcessor> = Arc::new(
            CompositeEventProcessor::with_all_processors(
                event_handler_config,
                email_service.clone(),
                communication_factory.clone(),
                notification_service.clone(),
            )
        );
        
        // Create use cases
        let event_processing_usecase = Arc::new(EventProcessingUseCase::new(
            domain_event_processor,
        ));
        
        // Create command registry and service
        let command_registry = Arc::new(
            TelegraphCommandRegistryFactory::create_telegraph_registry(
                event_processing_usecase.clone(),
            )
        );
        
        let command_service = Arc::new(
            GenericCommandService::new(command_registry)
        );
        
        // Create event consumer with command service
        let event_consumer = EventConsumer::new(config.clone(), command_service.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create event consumer: {}", e))?;
        
        info!("✅ Telegraph application initialized successfully");
        
        
        Ok(Self {
            config,
            event_consumer: Arc::new(event_consumer),
        })
    }
    
    /// Start the Telegraph service
    pub async fn run(&self) -> Result<(), anyhow::Error> {
        // Start event consumer in a separate task
        info!("🚀 Starting event consumer in parallel task");
        let consumer_handle = {
            let event_consumer = self.event_consumer.clone();
            tokio::spawn(async move {
                if let Err(e) = event_consumer.start().await {
                    error!("Event consumer failed: {}", e);
                    return Err(e);
                }
                Ok(())
            })
        };
        
        info!("✅ Telegraph service started successfully and is processing events");
        
        // Wait for shutdown signal
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received, stopping Telegraph service");
            }
            result = consumer_handle => {
                match result {
                    Ok(Ok(())) => {
                        info!("Event consumer completed successfully");
                    }
                    Ok(Err(e)) => {
                        error!("Event consumer failed: {}", e);
                        return Err(anyhow::anyhow!("Event consumer failed: {}", e));
                    }
                    Err(e) => {
                        error!("Event consumer task panicked: {}", e);
                        return Err(anyhow::anyhow!("Event consumer task panicked: {}", e));
                    }
                }
            }
        }
        
        // Stop event consumer gracefully
        if let Err(e) = self.event_consumer.stop().await {
            error!("Failed to stop event consumer: {}", e);
        }
        
        info!("✅ Telegraph service shut down complete");
        Ok(())
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