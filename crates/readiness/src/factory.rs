//! Queue factories that classify and log rustycog no-op fallbacks.

use std::collections::HashSet;
use std::hash::BuildHasher;
use std::sync::Arc;

use rustycog::config::QueueConfig;
use rustycog::core::error::ServiceError;
use rustycog::events::{
    create_event_consumer_from_queue_config, create_event_publisher_from_queue_config,
    ConcreteEventConsumer, ConcreteEventPublisher, ErrorMapper, GenericEventPublisherAdapter,
    MultiQueueEventPublisher,
};

use crate::classify::{
    classify_consumer, classify_publisher, signal_queue_status, ComponentStatus, QueueRole,
};

/// Publisher returned by [`create_signaled_multi_queue_event_publisher`].
pub struct SignaledPublisher<TError> {
    /// Multi-queue adapter used by outbox / use cases.
    pub publisher: Arc<MultiQueueEventPublisher<TError>>,
    /// Factory outcome (disabled / live / degraded).
    pub status: ComponentStatus,
    /// Concrete transport used for `/ready` pings.
    pub transport: Arc<ConcreteEventPublisher>,
}

/// Consumer returned by [`create_signaled_event_consumer`].
pub struct SignaledConsumer {
    /// Concrete rustycog consumer.
    pub consumer: Arc<ConcreteEventConsumer>,
    /// Factory outcome (disabled / live / degraded).
    pub status: ComponentStatus,
}

/// Create a multi-queue publisher and emit an explicit signal if rustycog no-op'd.
///
/// # Errors
///
/// Returns the mapped error if the rustycog publisher factory fails.
pub async fn create_signaled_multi_queue_event_publisher<TError>(
    service: &'static str,
    config: &QueueConfig,
    queue_names: Option<Vec<String>>,
    error_mapper: Arc<dyn ErrorMapper<TError>>,
) -> Result<SignaledPublisher<TError>, TError> {
    let transport = create_event_publisher_from_queue_config(config)
        .await
        .map_err(|service_error| error_mapper.from_service_error(service_error))?;
    let status = classify_publisher(config, transport.as_ref());
    signal_queue_status(service, QueueRole::Publisher, &status);

    let queue_names = queue_names.map_or_else(
        || default_queue_names(config),
        |names| names.into_iter().collect(),
    );
    let adapted = GenericEventPublisherAdapter::<TError>::new(transport.clone(), error_mapper);
    let publisher = Arc::new(MultiQueueEventPublisher::new(vec![adapted], queue_names));

    Ok(SignaledPublisher {
        publisher,
        status,
        transport,
    })
}

/// Create a consumer and emit an explicit signal if rustycog no-op'd.
///
/// # Errors
///
/// Returns a [`ServiceError`] if the rustycog consumer factory fails.
pub async fn create_signaled_event_consumer(
    service: &'static str,
    config: &QueueConfig,
) -> Result<SignaledConsumer, ServiceError> {
    let consumer = create_event_consumer_from_queue_config(config).await?;
    let status = classify_consumer(config, consumer.as_ref());
    signal_queue_status(service, QueueRole::Consumer, &status);
    Ok(SignaledConsumer { consumer, status })
}

fn default_queue_names<S: BuildHasher + Default>(config: &QueueConfig) -> HashSet<String, S> {
    match config {
        QueueConfig::Disabled => HashSet::default(),
        QueueConfig::Sqs(sqs_config) => sqs_config.all_queue_names().into_iter().collect(),
        QueueConfig::Kafka(kafka_config) => {
            let mut queues = HashSet::default();
            queues.insert(kafka_config.user_events_topic.clone());
            queues
        }
    }
}
