//! Common test utilities for Telegraph
//! 
//! Provides test infrastructure following rustycog-testing patterns
//! and Telegraph-specific test setup

use telegraph_configuration::{TelegraphConfig, load_config, setup_logging};
use telegraph_setup::app::AppBuilder;
use rustycog_config::ServerConfig;
use std::sync::Arc;
use rustycog_events::{ConcreteEventPublisher, create_event_publisher_from_queue_config, create_sqs_event_publisher, EventPublisher, DomainEvent};
use rustycog_config::QueueConfig;
use rustycog_core::error::ServiceError;
use async_trait::async_trait;
use rustycog_testing::*;
use telegraphmigration::{Migrator, MigratorTrait};
use sea_orm::DatabaseConnection;
use reqwest::Client;

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::*;
use telegraph_infra::{
    communication::{MockEmailAdapter, SmsAdapter, NotificationAdapter},
    event::EventConsumer,
    repository::{NotificationReadRepository, NotificationWriteRepository, CombinedNotificationRepository},
};
use telegraph_domain::{EmailService, SmsService, NotificationService, TemplateService};
use telegraph_infra::{
    event::processors::{CommunicationEventProcessor, CompositeEventProcessor, DatabaseNotificationProcessor},
    template::TeraTemplateService,
};

/// Telegraph test descriptor following rustycog-testing patterns
pub struct TelegraphTestDescriptor;

#[async_trait]
impl ServiceTestDescriptor for TelegraphTestDescriptor {
    type Config = TelegraphConfig;

    async fn run_app(&self, config: TelegraphConfig, server_config: ServerConfig) -> anyhow::Result<()> {    
        AppBuilder::new(config)
            .build()
            .await?
            .run()
            .await?;
        Ok(())
    }

    async fn run_migrations(&self, connection: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
        Migrator::up(connection, None).await?;
        Ok(())
    }
}

/// Setup Telegraph test server with database support
pub async fn setup_test_server() -> Result<(TestFixture, String, Client, Arc<ConcreteEventPublisher>), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let event_publisher = create_event_publisher_for_tests(&config.queue).await?;

    let (fixture, base_url, client) = rustycog_testing::setup_test_server::<TelegraphTestDescriptor>(Arc::new(TelegraphTestDescriptor)).await?;
    Ok((fixture, base_url, client, event_publisher))
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