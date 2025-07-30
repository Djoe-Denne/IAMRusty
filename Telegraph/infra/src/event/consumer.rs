//! Event consumer for processing IAM domain events

use async_trait::async_trait;
use iam_events::{DomainEvent, IamDomainEvent};
use rustycog_command::{CommandContext, GenericCommandService};
use rustycog_core::error::ServiceError;
use rustycog_events::{
    create_event_consumer_from_queue_config, ConcreteEventConsumer,
    EventConsumer as RustycogEventConsumer, EventHandler,
};
use serde_json;
use std::sync::Arc;
use telegraph_application::command::ProcessEventCommand;
use telegraph_configuration::TelegraphConfig;
use tracing::{debug, error, info};

/// Telegraph event consumer using rustycog-events
pub struct EventConsumer {
    inner_consumer: Arc<ConcreteEventConsumer>,
    config: TelegraphConfig,
    command_service: Arc<GenericCommandService>,
}

impl EventConsumer {
    /// Create a new event consumer from configuration with command service
    pub async fn new(
        config: TelegraphConfig,
        command_service: Arc<GenericCommandService>,
    ) -> Result<Self, telegraph_domain::DomainError> {
        let inner_consumer = create_event_consumer_from_queue_config(&config.queue)
            .await
            .map_err(|e| {
                telegraph_domain::DomainError::InfrastructureError(format!(
                    "Failed to create event consumer: {}",
                    e
                ))
            })?;

        Ok(Self {
            inner_consumer,
            config,
            command_service,
        })
    }

    /// Start consuming events from queues
    pub async fn start(&self) -> Result<(), telegraph_domain::DomainError> {
        info!("Starting Telegraph event consumer with command service");

        // Log the configured queues and events for debugging
        for (queue_name, queue_config) in &self.config.queues {
            info!(
                queue_name = %queue_name,
                events = ?queue_config.events,
                "📋 Queue configuration loaded"
            );
        }

        // Create a handler that uses command service
        let handler = TelegraphEventHandler::new(self.config.clone(), self.command_service.clone());

        self.inner_consumer.start(handler).await.map_err(|e| {
            telegraph_domain::DomainError::InfrastructureError(format!(
                "Event consumer error: {}",
                e
            ))
        })?;

        Ok(())
    }

    /// Stop the event consumer
    pub async fn stop(&self) -> Result<(), telegraph_domain::DomainError> {
        info!("Stopping Telegraph event consumer");

        self.inner_consumer.stop().await.map_err(|e| {
            telegraph_domain::DomainError::InfrastructureError(format!(
                "Failed to stop event consumer: {}",
                e
            ))
        })?;

        Ok(())
    }

    /// Health check for the event consumer
    pub async fn health_check(&self) -> Result<(), telegraph_domain::DomainError> {
        self.inner_consumer.health_check().await.map_err(|e| {
            telegraph_domain::DomainError::InfrastructureError(format!(
                "Event consumer health check failed: {}",
                e
            ))
        })?;

        Ok(())
    }
}

/// Telegraph event handler that uses command service for processing
pub struct TelegraphEventHandler {
    config: TelegraphConfig,
    command_service: Arc<GenericCommandService>,
}

impl TelegraphEventHandler {
    /// Create a new Telegraph event handler
    pub fn new(config: TelegraphConfig, command_service: Arc<GenericCommandService>) -> Self {
        Self {
            config,
            command_service,
        }
    }
}

#[async_trait]
impl EventHandler for TelegraphEventHandler {
    async fn handle_event(
        &self,
        event: Box<dyn rustycog_events::DomainEvent>,
    ) -> Result<(), ServiceError> {
        let event_id = event.event_id();

        info!(
            event_id = %event_id,
            "🎯 Telegraph received event from queue!"
        );

        // Create process event command
        let command = ProcessEventCommand::new(event.into());
        let context = CommandContext::new();

        // Execute command through command service
        match self.command_service.execute(command, context).await {
            Ok(()) => {
                info!(
                    event_id = %event_id,
                    "Event processed successfully via command service"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    event_id = %event_id,
                    error = %e,
                    "Failed to process event via command service"
                );
                Err(ServiceError::infrastructure(format!(
                    "Command execution failed: {}",
                    e
                )))
            }
        }
    }

    fn supports_event_type(&self, event_type: &str) -> bool {
        // Check if any configured queue supports this event type
        let mut supports = false;
        let mut supporting_queues = Vec::new();

        for (queue_name, queue_config) in &self.config.queues {
            if queue_config.events.contains(&event_type.to_string()) {
                supports = true;
                supporting_queues.push(queue_name.clone());
            }
        }

        if supports {
            info!(
                event_type = event_type,
                supporting_queues = ?supporting_queues,
                "✅ Event type supported by configuration"
            );
        } else {
            // Log discarded event - base data at INFO level
            info!(
                event_type = event_type,
                configured_queues = ?self.config.queues.keys().collect::<Vec<_>>(),
                "❌ Event type not supported by any configured queue - discarding"
            );

            // Log full configuration details at DEBUG level
            debug!(
                event_type = event_type,
                full_queue_config = ?self.config.queues,
                "🔍 Full queue configuration for discarded event"
            );
        }

        supports
    }
}
