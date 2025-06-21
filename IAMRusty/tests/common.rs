// Re-export everything from rustycog-testing for backward compatibility
pub use rustycog_testing::*;

// Specific re-exports for commonly used functions
pub use rustycog_testing::{
    create_test_client, 
    TestFixture,
    TestKafkaFixture,
    TestSqsFixture,
    MockEventPublisher,
    ServiceTestDescriptor,
}; 

use setup::app::{build_app_state_with_event_publisher, build_and_run};
use infra::event_adapter::{MultiQueueEventPublisher, IAMEventPublisherAdapter, IAMEventAdapter, IAMErrorMapper};
use configuration::{AppConfig, ServerConfig};
use std::sync::Arc;
use reqwest::Client;
use async_trait::async_trait;
use std::collections::HashSet;

pub struct IAMRustyTestDescriptor;

#[derive(Clone)]
pub struct IAMRustyTestDescriptorWithMockEvents {
    mock_event_publisher: Arc<MockEventPublisher>,
}

#[async_trait]
impl ServiceTestDescriptor for IAMRustyTestDescriptor {
    type Config = AppConfig;

    async fn run_app(&self, config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        build_and_run(config, server_config, None).await
    }
}

pub async fn setup_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>> {
    rustycog_testing::setup_test_server::<IAMRustyTestDescriptor>(Arc::new(IAMRustyTestDescriptor)).await
}

impl IAMRustyTestDescriptorWithMockEvents {
    pub fn new() -> Self {
        Self {
            mock_event_publisher: Arc::new(MockEventPublisher::new()),
        }
    }
}

#[async_trait]
impl ServiceTestDescriptor for IAMRustyTestDescriptorWithMockEvents {
    type Config = AppConfig;

    async fn run_app(&self, config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        let no_op_event_publisher = Arc::new(rustycog_events::ConcreteEventPublisher::NoOp(self.mock_event_publisher.clone()));
        let error_mapper = Arc::new(IAMErrorMapper);
        let event_adapter = Arc::new(IAMEventAdapter);
        let multi_queue_event_publisher = MultiQueueEventPublisher::new(vec![IAMEventPublisherAdapter::new(no_op_event_publisher, error_mapper, event_adapter)], HashSet::new());
        build_and_run(config, server_config, Some(Arc::new(multi_queue_event_publisher))).await
    }
}

pub async fn setup_test_server_with_mock_events() -> Result<(TestFixture, String, Client, Arc<MockEventPublisher>), Box<dyn std::error::Error>> {
    let descriptor = IAMRustyTestDescriptorWithMockEvents::new();
    let mock_event_publisher = descriptor.mock_event_publisher.clone();
    let (fixture, base_url, client) = rustycog_testing::setup_test_server::<IAMRustyTestDescriptorWithMockEvents>(Arc::new(descriptor)).await?;
    Ok((fixture, base_url, client, mock_event_publisher))
}