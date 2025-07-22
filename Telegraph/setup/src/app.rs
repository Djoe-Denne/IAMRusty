//! Application setup for Telegraph

use std::collections::HashMap;
use tracing::{info, error};
use std::sync::Arc;
use telegraph_domain::{EmailService, SmsService, NotificationService, CommunicationService, TemplateService, EventProcessor, EventExtractor, CommunicationFactory};
use telegraph_infra::{
    communication::{EmailAdapter, SmsAdapter, NotificationAdapter, CompositeCommunicationService},
    event::{EventConsumer, JsonEventExtractor},
    template::TeraTemplateService,
    event::processors::{CompositeEventProcessor, EventHandlerConfig},
};
use telegraph_application::{
    usecase::EventProcessingUseCase,
    command::TelegraphCommandRegistryFactory,
};
use rustycog_command::GenericCommandService;

use telegraph_configuration::TelegraphConfig;

/// Telegraph application context
pub struct TelegraphApp {
    config: TelegraphConfig,
}

impl TelegraphApp {
    /// Create a new Telegraph application
    pub fn new(config: TelegraphConfig) -> Self {
        info!("Initializing Telegraph application");
        
        Self {
            config,
        }
    }
    
    /// Start the Telegraph service
    pub async fn run(&self) -> Result<(), anyhow::Error> {
        info!("Starting Telegraph service and initializing components");
        
        // Create communication adapters
        let email_config = telegraph_infra::communication::EmailConfig {
            smtp_host: self.config.communication.email.smtp.host.clone(),
            smtp_port: self.config.communication.email.smtp.port,
            smtp_username: self.config.communication.email.smtp.username.clone().unwrap_or_default(),
            smtp_password: self.config.communication.email.smtp.password.clone().unwrap_or_default(),
            from_email: self.config.communication.email.from_address.clone(),
            from_name: self.config.communication.email.from_name.clone(),
            use_tls: self.config.communication.email.smtp.use_tls,
        };
        
        let email_service: Arc<dyn EmailService> = Arc::new(
            EmailAdapter::new(email_config)
                .map_err(|e| anyhow::anyhow!("Failed to create email adapter: {}", e))?
        );
        
        let sms_service: Arc<dyn SmsService> = Arc::new(
            SmsAdapter::new_default()
        );
        
        let notification_service: Arc<dyn NotificationService> = Arc::new(
            NotificationAdapter::new_default()
        );
        
        // Create template service
        let template_service: Arc<dyn TemplateService> = Arc::new(
            TeraTemplateService::new(self.config.communication.template.clone())
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
        
        // Create composite communication service
        let communication_service: Arc<dyn CommunicationService> = Arc::new(
            CompositeCommunicationService::new(
                email_service.clone(),
                sms_service.clone(),
                notification_service.clone(),
            )
        );
        
        let mut event_mapping = HashMap::new();
        let queues_config = self.config.queues.clone();
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
        let event_consumer = Arc::new(
            EventConsumer::new(self.config.clone(), command_service.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create event consumer: {}", e))?
        );
        
        info!("✅ Telegraph application initialized successfully");
        
        // Start event consumer
        info!("🚀 Starting event consumer");
        event_consumer.start().await
            .map_err(|e| anyhow::anyhow!("Failed to start event consumer: {}", e))?;
        
        info!("✅ Telegraph service started successfully and is processing events");
        
        // Wait for shutdown signal
        tokio::signal::ctrl_c().await?;
        
        info!("Shutdown signal received, stopping Telegraph service");
        
        // Stop event consumer
        if let Err(e) = event_consumer.stop().await {
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
        Ok(TelegraphApp::new(self.config))
    }
} 