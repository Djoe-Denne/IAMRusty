// Test utilities from rustycog-testing
pub use rustycog_testing::*;
pub use rustycog_testing::TestFixture;

// Migration crate import - use the correct crate name
use iammigration::{Migrator, MigratorTrait};

// IAM imports
use iam_setup::app::{build, run};
use iam_infra::event_adapter::{MultiQueueEventPublisher, IAMEventPublisherAdapter, IAMEventAdapter, IAMErrorMapper};
use iam_configuration::{AppConfig, ServerConfig};
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
impl ServiceTestDescriptor<TestFixture> for IAMRustyTestDescriptor {
    type Config = AppConfig;

    async fn build_app(&self, config: AppConfig) -> anyhow::Result<()> {
        build(config, None).await
    }

    async fn run_app(&self, server_config: ServerConfig) -> anyhow::Result<()> {
        run(server_config).await
    }

    async fn run_migrations(&self, connection: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
        Migrator::up(connection, None).await?;
        Ok(())
    }

    fn has_db(&self) -> bool {
        true
    }

    fn has_sqs(&self) -> bool {
        false
    }
}

pub async fn setup_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>> {
    let descriptor = Arc::new(IAMRustyTestDescriptor);
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let (server_url, client) = rustycog_testing::setup_test_server::<IAMRustyTestDescriptor, TestFixture>(descriptor).await?;
    Ok((fixture, server_url, client))
}

impl IAMRustyTestDescriptorWithMockEvents {
    pub fn new() -> Self {
        Self {
            mock_event_publisher: Arc::new(MockEventPublisher::new()),
        }
    }
}

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for IAMRustyTestDescriptorWithMockEvents {
    type Config = AppConfig;

    async fn build_app(&self, config: AppConfig) -> anyhow::Result<()> {
        let no_op_event_publisher = Arc::new(rustycog_events::ConcreteEventPublisher::NoOp(self.mock_event_publisher.clone()));
        let error_mapper = Arc::new(IAMErrorMapper);
        let event_adapter = Arc::new(IAMEventAdapter);
        let multi_queue_event_publisher = MultiQueueEventPublisher::new(vec![IAMEventPublisherAdapter::new(no_op_event_publisher, error_mapper, event_adapter)], HashSet::new());
        build(config, Some(Arc::new(multi_queue_event_publisher))).await
    }

    async fn run_app(&self, server_config: ServerConfig) -> anyhow::Result<()> {
        run(server_config).await
    }

    async fn run_migrations(&self, connection: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
        Migrator::up(connection, None).await?;
        Ok(())
    }

    fn has_db(&self) -> bool {
        true
    }

    fn has_sqs(&self) -> bool {
        false // but maybe yes ?
    }
}

pub async fn setup_test_server_with_mock_events() -> Result<(TestFixture, String, Client, Arc<MockEventPublisher>), Box<dyn std::error::Error>> {
    let descriptor = Arc::new(IAMRustyTestDescriptorWithMockEvents::new());
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let mock_event_publisher = descriptor.mock_event_publisher.clone();
    let (base_url, client) = rustycog_testing::setup_test_server::<IAMRustyTestDescriptorWithMockEvents, TestFixture>(descriptor).await?;
    Ok((fixture, base_url, client, mock_event_publisher))
}