//! Application setup for Telegraph

use tracing::{info, error};
use std::sync::Arc;
use telegraph_domain::{EmailService, SmsService, NotificationService, CommunicationService, TemplateService, EventProcessor};
use telegraph_infra::{
    communication::{EmailAdapter, SmsAdapter, NotificationAdapter, CompositeCommunicationService},
    event::EventConsumer,
    template::TeraTemplateService,
    event::processors::CompositeEventProcessor,
};
use telegraph_application::{
    usecase::{CommunicationUseCase, EventProcessingUseCase},
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
        let email_service: Arc<dyn EmailService> = Arc::new(
            EmailAdapter::new_default()
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
        
        // Create composite communication service
        let communication_service: Arc<dyn CommunicationService> = Arc::new(
            CompositeCommunicationService::new(
                email_service.clone(),
                sms_service.clone(),
                notification_service.clone(),
            )
        );
        
        // Create event processor (domain-level event processor)
        let domain_event_processor: Arc<dyn EventProcessor> = Arc::new(
            CompositeEventProcessor::with_all_processors(
                email_service.clone(),
                template_service.clone(),
                sms_service.clone(),
                notification_service.clone(),
            )
        );
        
        // Create use cases
        let _communication_usecase = Arc::new(CommunicationUseCase::new(
            communication_service.clone(),
        ));
        
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