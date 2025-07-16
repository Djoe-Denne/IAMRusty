//! Common test utilities for Telegraph
//! 
//! Provides test infrastructure following rustycog-testing patterns
//! and Telegraph-specific test setup

use telegraph_configuration::{TelegraphConfig, load_config};
use std::sync::Arc;
use rustycog_events::{ConcreteEventPublisher, create_event_publisher_from_queue_config, create_sqs_event_publisher, EventPublisher, DomainEvent};
use rustycog_config::QueueConfig;
use rustycog_core::error::ServiceError;
use async_trait::async_trait;
use telegraph_infra::{
    communication::{MockEmailAdapter, SmsAdapter, NotificationAdapter},
    event::EventConsumer,
};
use telegraph_domain::{EmailService, SmsService, NotificationService};
use telegraph_infra::event::processors::{CommunicationEventProcessor, CompositeEventProcessor};

/// Custom test event publisher that routes events directly to the event processor
pub struct TestEventPublisher {
    pub event_processor: Arc<dyn CommunicationEventProcessor>,
}

impl TestEventPublisher {
    pub fn new(event_processor: Arc<dyn CommunicationEventProcessor>) -> Self {
        Self { event_processor }
    }
}

#[async_trait]
impl EventPublisher for TestEventPublisher {
    async fn publish(&self, event: Box<dyn DomainEvent>) -> Result<(), ServiceError> {
        // Convert the domain event to IAM event and process directly
        if let Ok(iam_event) = self.convert_to_iam_event(event.as_ref()) {
            self.event_processor.process_event(&iam_event).await
                .map_err(|e| ServiceError::infrastructure(format!("Event processing failed: {}", e)))?;
        }
        Ok(())
    }

    async fn publish_batch(&self, events: Vec<Box<dyn DomainEvent>>) -> Result<(), ServiceError> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    async fn health_check(&self) -> Result<(), ServiceError> {
        Ok(())
    }
}

impl TestEventPublisher {
    fn convert_to_iam_event(&self, event: &dyn DomainEvent) -> Result<iam_events::IamDomainEvent, ServiceError> {
        let event_json = event.to_json()?;
        serde_json::from_str(&event_json)
            .map_err(|e| ServiceError::infrastructure(format!("Failed to deserialize IAM event: {}", e)))
    }
}

/// Test fixture for Telegraph with real infrastructure
pub struct TelegraphTestFixture {
    config: TelegraphConfig,
    event_consumer: Arc<EventConsumer>,
    event_publisher: Arc<ConcreteEventPublisher>,
    test_event_publisher: Arc<TestEventPublisher>,
    email_service: Arc<dyn EmailService>,
    mock_email_service: Arc<MockEmailAdapter>,
    sms_service: Arc<dyn SmsService>,
    notification_service: Arc<dyn NotificationService>,
}

impl TelegraphTestFixture {
    pub fn new(
        config: TelegraphConfig,
        event_consumer: Arc<EventConsumer>,
        event_publisher: Arc<ConcreteEventPublisher>,
        test_event_publisher: Arc<TestEventPublisher>,
        email_service: Arc<dyn EmailService>,
        mock_email_service: Arc<MockEmailAdapter>,
        sms_service: Arc<dyn SmsService>,
        notification_service: Arc<dyn NotificationService>,
    ) -> Self {
        Self { 
            config,
            event_consumer,
            event_publisher,
            test_event_publisher,
            email_service,
            mock_email_service,
            sms_service,
            notification_service,
        }
    }
    
    /// Get the test event publisher that routes directly to the consumer
    pub fn test_event_publisher(&self) -> Arc<TestEventPublisher> {
        self.test_event_publisher.clone()
    }
    
    /// Get the real event publisher (for comparison/advanced testing)
    pub fn event_publisher(&self) -> Arc<ConcreteEventPublisher> {
        self.event_publisher.clone()
    }
    
    /// Get the event consumer for verification
    pub fn event_consumer(&self) -> Arc<EventConsumer> {
        self.event_consumer.clone()
    }
    
    /// Process an event directly through the event processor (for testing)
    pub async fn process_event_directly(&self, event: &iam_events::IamDomainEvent) -> Result<(), telegraph_domain::DomainError> {
        // Get access to the event processor from the test event publisher
        self.test_event_publisher.event_processor.process_event(event).await
    }
    
    /// Get the email service (will be MockEmailAdapter in tests)
    pub fn email_service(&self) -> Arc<dyn EmailService> {
        self.email_service.clone()
    }
    
    /// Get access to mock email adapter for verification
    pub fn mock_email_service(&self) -> Arc<MockEmailAdapter> {
        self.mock_email_service.clone()
    }
}

/// Setup Telegraph test environment with real infrastructure
pub async fn setup_telegraph_test_server() -> Result<(TelegraphTestFixture, Arc<dyn EventPublisher>), Box<dyn std::error::Error>> {
    let config = load_config()?;
    
    // Create real communication services for testing
    let mock_email_adapter = Arc::new(MockEmailAdapter::new());
    let email_service: Arc<dyn EmailService> = mock_email_adapter.clone();
    let sms_service: Arc<dyn SmsService> = Arc::new(SmsAdapter::new_default());
    let notification_service: Arc<dyn NotificationService> = Arc::new(NotificationAdapter::new_default());
    
    // Create event processor
    let event_processor = Arc::new(CompositeEventProcessor::with_all_processors(
        email_service.clone(),
        sms_service.clone(),
        notification_service.clone(),
    ));
    
    // Create event consumer using the same logic as app.rs
    let event_consumer = Arc::new(
        EventConsumer::new(config.clone()).await?
    );
    
    // Create test event publisher that routes directly to the event processor
    let test_event_publisher = Arc::new(TestEventPublisher::new(event_processor.clone()));
    
    // Create event publisher based on queue configuration
    let event_publisher = create_event_publisher_for_tests(&config.queue).await?;
    
    let fixture = TelegraphTestFixture::new(
        config,
        event_consumer,
        event_publisher,
        test_event_publisher.clone(),
        email_service,
        mock_email_adapter,
        sms_service,
        notification_service,
    );
    
    Ok((fixture, test_event_publisher))
}

/// Create event publisher for tests - handles async SQS creation
async fn create_event_publisher_for_tests(queue_config: &QueueConfig) -> Result<Arc<ConcreteEventPublisher>, Box<dyn std::error::Error>> {
    match queue_config {
        QueueConfig::Sqs(sqs_config) => {
            let publisher = create_sqs_event_publisher(sqs_config).await?;
            Ok(publisher)
        }
        _ => {
            // For Kafka and Disabled, use the sync function
            let publisher = create_event_publisher_from_queue_config(queue_config)?;
            Ok(publisher)
        }
    }
}