//! Event consumer: purge KV and revoke enrollment on Manifesto `component_removed`.

use std::sync::Arc;

use apparatus_contracts::BindingId;
use async_trait::async_trait;
use lazaret_application::purge_binding_namespace;
use lazaret_domain::{AsyncKvStore, EnrollmentStore};
use manifesto_events::ManifestoDomainEvent;
use readiness::{create_signaled_event_consumer, ComponentStatus};
use rustycog::config::QueueConfig;
use rustycog::core::error::ServiceError;
use rustycog::events::{
    ConcreteEventConsumer, EventConsumer as RustycogEventConsumer, EventHandler,
};
use tracing::{error, info, warn};

/// Signaled rustycog consumer for `lazaret-kv-events`.
pub struct KvPurgeEventConsumer {
    inner_consumer: Arc<ConcreteEventConsumer>,
    kv: Arc<dyn AsyncKvStore>,
    enrollments: Arc<dyn EnrollmentStore>,
    transport_status: ComponentStatus,
}

impl KvPurgeEventConsumer {
    /// Create a consumer from queue configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceError`] if the rustycog consumer factory fails.
    pub async fn new(
        queue_config: &QueueConfig,
        kv: Arc<dyn AsyncKvStore>,
        enrollments: Arc<dyn EnrollmentStore>,
    ) -> Result<Self, ServiceError> {
        let signaled = create_signaled_event_consumer("lazaret", queue_config).await?;
        Ok(Self {
            inner_consumer: signaled.consumer,
            kv,
            enrollments,
            transport_status: signaled.status,
        })
    }

    /// Factory outcome for `/ready` (disabled / live / degraded no-op).
    #[must_use]
    pub const fn transport_status(&self) -> &ComponentStatus {
        &self.transport_status
    }

    /// Underlying rustycog consumer, used by `/ready` transport pings.
    #[must_use]
    pub fn inner(&self) -> Arc<ConcreteEventConsumer> {
        self.inner_consumer.clone()
    }

    /// Whether the underlying queue consumer is a no-op placeholder.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        matches!(self.inner_consumer.as_ref(), ConcreteEventConsumer::NoOp(_))
    }

    /// Start consuming events from queues.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceError`] if the underlying consumer fails to start.
    pub async fn start(&self, handler: KvPurgeEventHandler) -> Result<(), ServiceError> {
        info!("Starting Lazaret KV purge event consumer");
        self.inner_consumer.start(handler).await
    }

    /// Stop the event consumer.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceError`] if the underlying consumer fails to stop.
    pub async fn stop(&self) -> Result<(), ServiceError> {
        info!("Stopping Lazaret KV purge event consumer");
        self.inner_consumer.stop().await
    }

    /// Health check for the event consumer.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceError`] if the underlying consumer health check fails.
    pub async fn health_check(&self) -> Result<(), ServiceError> {
        self.inner_consumer.health_check().await
    }

    /// Handler bound to the same KV store and enrollment registry this consumer was wired with.
    #[must_use]
    pub fn handler(&self) -> KvPurgeEventHandler {
        KvPurgeEventHandler::new(self.kv.clone(), self.enrollments.clone())
    }
}

/// Purges one binding namespace and revokes its enrollment; ignores other Manifesto event types.
pub struct KvPurgeEventHandler {
    kv: Arc<dyn AsyncKvStore>,
    enrollments: Arc<dyn EnrollmentStore>,
}

impl KvPurgeEventHandler {
    /// Construct a handler over the platform KV store and enrollment registry.
    #[must_use]
    pub const fn new(kv: Arc<dyn AsyncKvStore>, enrollments: Arc<dyn EnrollmentStore>) -> Self {
        Self { kv, enrollments }
    }

    async fn process_manifesto_event(
        &self,
        event: ManifestoDomainEvent,
    ) -> Result<(), ServiceError> {
        if let ManifestoDomainEvent::ComponentRemoved(removed) = event {
            self.enrollments
                .revoke_binding(removed.component_id)
                .await
                .map_err(|error| {
                    error!(error = %error, "enrollment revoke failed");
                    ServiceError::infrastructure(format!("enrollment revoke failed: {error}"))
                })?;
            let binding = match removed.component_id.to_string().parse::<BindingId>() {
                Ok(binding) => binding,
                Err(error) => {
                    warn!(
                        component_id = %removed.component_id,
                        error = %error,
                        "Ignoring component_removed with unparseable BindingId"
                    );
                    return Ok(());
                }
            };
            purge_binding_namespace(self.kv.as_ref(), binding)
                .await
                .map_err(|error| {
                    error!(error = %error, "KV purge failed");
                    ServiceError::infrastructure(format!("KV purge failed: {error}"))
                })?;
        }
        Ok(())
    }
}

#[async_trait]
impl EventHandler for KvPurgeEventHandler {
    async fn handle_event(
        &self,
        event: Box<dyn rustycog::events::DomainEvent>,
    ) -> Result<(), ServiceError> {
        let event_id = event.event_id();
        let event_type = event.event_type().to_string();

        info!(
            event_id = %event_id,
            event_type = %event_type,
            "Lazaret received event from queue"
        );

        let event_json = event.to_json().map_err(|e| {
            error!("Failed to serialize event: {e}");
            ServiceError::infrastructure(format!("Failed to serialize event: {e}"))
        })?;

        let manifesto_event: ManifestoDomainEvent =
            serde_json::from_str(&event_json).map_err(|e| {
                error!("Failed to parse manifesto event: {e}");
                ServiceError::infrastructure(format!("Failed to parse event: {e}"))
            })?;

        self.process_manifesto_event(manifesto_event).await?;

        info!(event_id = %event_id, "Event processed successfully");
        Ok(())
    }

    fn supports_event_type(&self, event_type: &str) -> bool {
        matches!(event_type, "component_removed" | "ComponentRemoved")
    }
}
