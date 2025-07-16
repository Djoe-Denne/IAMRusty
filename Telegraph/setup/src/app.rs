//! Application setup for Telegraph

use tracing::{info, error};
use std::sync::Arc;
use telegraph_domain::{EmailService, SmsService, NotificationService, CommunicationService};
use telegraph_infra::{
    communication::{EmailAdapter, SmsAdapter, NotificationAdapter, CompositeCommunicationService},
    event::EventConsumer,
};
use telegraph_application::usecase::CommunicationUseCase;

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
        
        // Create composite communication service
        let communication_service: Arc<dyn CommunicationService> = Arc::new(
            CompositeCommunicationService::new(
                email_service.clone(),
                sms_service.clone(),
                notification_service.clone(),
            )
        );
        
        // Create event processor
        let event_processor = Arc::new(
            telegraph_infra::event::processors::CompositeEventProcessor::with_all_processors(
                email_service.clone(),
                sms_service.clone(),
                notification_service.clone(),
            )
        );
        
        // Create event consumer
        let event_consumer = Arc::new(
            EventConsumer::new(self.config.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create event consumer: {}", e))?
        );
        
        // Create use cases
        let _communication_usecase = Arc::new(CommunicationUseCase::new(
            communication_service.clone(),
        ));
        
        info!("✅ Telegraph application initialized successfully");
        
        // Start event consumer
        info!("🚀 Starting event consumer");
        event_consumer.start(event_processor).await
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