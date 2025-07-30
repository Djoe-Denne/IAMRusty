// Test utilities from rustycog-testing
pub use rustycog_testing::TestFixture;
pub use rustycog_testing::*;

// Migration crate import - use the correct crate name
use iammigration::{Migrator, MigratorTrait};

// IAM imports
use async_trait::async_trait;
use iam_configuration::{AppConfig, ServerConfig};
use iam_domain::error::DomainError;
use iam_infra::event_adapter::IAMErrorMapper;
use iam_setup::app::build_and_run;
use reqwest::Client;
use rustycog_events::adapter::{GenericEventPublisherAdapter, MultiQueueEventPublisher};
use std::collections::HashSet;
use std::sync::Arc;

pub struct IAMRustyTestDescriptor;

#[derive(Clone)]
pub struct IAMRustyTestDescriptorWithMockEvents {
    mock_event_publisher: Arc<MockEventPublisher>,
}

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for IAMRustyTestDescriptor {
    type Config = AppConfig;

    async fn build_app(
        &self,
        _config: AppConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_app(&self, config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        build_and_run(config, server_config, None).await
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

pub async fn setup_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>>
{
    let descriptor = Arc::new(IAMRustyTestDescriptor);
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let (server_url, client) =
        rustycog_testing::setup_test_server::<IAMRustyTestDescriptor, TestFixture>(descriptor)
            .await?;
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

    async fn build_app(
        &self,
        _config: AppConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_app(&self, _config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        let no_op_event_publisher = Arc::new(rustycog_events::ConcreteEventPublisher::NoOp(
            self.mock_event_publisher.clone(),
        ));
        let error_mapper = Arc::new(IAMErrorMapper);
        let multi_queue_event_publisher = MultiQueueEventPublisher::new(
            vec![GenericEventPublisherAdapter::<DomainError>::new(
                no_op_event_publisher,
                error_mapper,
            )],
            HashSet::new(),
        );
        build_and_run(
            _config,
            server_config,
            Some(Arc::new(multi_queue_event_publisher)),
        )
        .await?;
        Ok(())
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

pub async fn setup_test_server_with_mock_events(
) -> Result<(TestFixture, String, Client, Arc<MockEventPublisher>), Box<dyn std::error::Error>> {
    let descriptor = Arc::new(IAMRustyTestDescriptorWithMockEvents::new());
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let mock_event_publisher = descriptor.mock_event_publisher.clone();
    let (base_url, client) = rustycog_testing::setup_test_server::<
        IAMRustyTestDescriptorWithMockEvents,
        TestFixture,
    >(descriptor)
    .await?;
    Ok((fixture, base_url, client, mock_event_publisher))
}
