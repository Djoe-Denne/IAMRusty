pub mod kafka;
pub mod no_op;

use configuration::KafkaConfig;
use domain::error::DomainError;
use domain::port::event_publisher::EventPublisher;
use std::sync::Arc;
use async_trait::async_trait;

pub use kafka::KafkaEventPublisher;
pub use no_op::*;

/// Concrete event publisher that can be either Kafka or NoOp
pub enum ConcreteEventPublisher {
    Kafka(KafkaEventPublisher),
    NoOp(NoOpEventPublisher),
}

#[async_trait]
impl EventPublisher for ConcreteEventPublisher {
    async fn publish(&self, event: domain::entity::events::DomainEvent) -> Result<(), DomainError> {
        match self {
            ConcreteEventPublisher::Kafka(kafka) => kafka.publish(event).await,
            ConcreteEventPublisher::NoOp(no_op) => no_op.publish(event).await,
        }
    }

    async fn publish_batch(&self, events: Vec<domain::entity::events::DomainEvent>) -> Result<(), DomainError> {
        match self {
            ConcreteEventPublisher::Kafka(kafka) => kafka.publish_batch(events).await,
            ConcreteEventPublisher::NoOp(no_op) => no_op.publish_batch(events).await,
        }
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        match self {
            ConcreteEventPublisher::Kafka(kafka) => kafka.health_check().await,
            ConcreteEventPublisher::NoOp(no_op) => no_op.health_check().await,
        }
    }
}

/// Factory function to create an event publisher based on configuration
pub fn create_event_publisher(config: &KafkaConfig) -> Result<Arc<ConcreteEventPublisher>, DomainError> {
    if config.enabled {
        match KafkaEventPublisher::new(config.clone()) {
            Ok(publisher) => {
                tracing::info!("Created Kafka event publisher");
                Ok(Arc::new(ConcreteEventPublisher::Kafka(publisher)))
            }
            Err(e) => {
                tracing::warn!("Failed to create Kafka event publisher, falling back to no-op: {}", e);
                // Fall back to no-op publisher if Kafka creation fails
                Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())))
            }
        }
    } else {
        tracing::info!("Kafka disabled, using no-op event publisher");
        Ok(Arc::new(ConcreteEventPublisher::NoOp(NoOpEventPublisher::new())))
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod test_consumer {
    use rdkafka::config::ClientConfig;
    use rdkafka::consumer::{Consumer, StreamConsumer};
    use rdkafka::message::Message;
    use std::time::Duration;
    use tracing::{debug, info, warn};
    use uuid::Uuid;

    /// Test Kafka consumer for verifying published events
    pub struct TestKafkaConsumer {
        consumer: StreamConsumer,
        topic: String,
    }

    impl TestKafkaConsumer {
        /// Create a new test consumer
        pub async fn new(brokers: &str, topic: &str) -> Result<Self, Box<dyn std::error::Error>> {
            let consumer: StreamConsumer = ClientConfig::new()
                .set("group.id", &format!("test-consumer-{}", Uuid::new_v4()))
                .set("bootstrap.servers", brokers)
                .set("enable.partition.eof", "false")
                .set("session.timeout.ms", "6000")
                .set("enable.auto.commit", "false")  // Disable auto commit for more control
                .set("auto.offset.reset", "earliest")
                .set("enable.auto.offset.store", "false")
                .set("api.version.request", "true")
                .set("fetch.wait.max.ms", "100")  // Reduce wait time for faster polling
                .create()?;

            consumer.subscribe(&[topic])?;
            debug!("Test consumer subscribed to topic: {}", topic);
            
            // Wait a moment for subscription to take effect
            tokio::time::sleep(Duration::from_millis(500)).await;

            Ok(Self {
                consumer,
                topic: topic.to_string(),
            })
        }

        /// Get all available messages from the topic
        pub async fn get_all_messages(&self, max_wait_secs: u64) -> Result<Vec<String>, Box<dyn std::error::Error>> {
            let mut messages = Vec::new();
            let timeout = Duration::from_secs(max_wait_secs);
            let start_time = std::time::Instant::now();

            info!("Starting to consume messages from topic: {} for up to {}s", self.topic, max_wait_secs);

            // Poll multiple times to ensure we get all messages
            let mut consecutive_timeouts = 0;
            const MAX_CONSECUTIVE_TIMEOUTS: u32 = 3;

            while start_time.elapsed() < timeout && consecutive_timeouts < MAX_CONSECUTIVE_TIMEOUTS {
                match tokio::time::timeout(Duration::from_millis(1000), self.consumer.recv()).await {
                    Ok(Ok(m)) => {
                        consecutive_timeouts = 0; // Reset timeout counter
                        
                        if let Some(payload) = m.payload() {
                            let message_str = String::from_utf8_lossy(payload).to_string();
                            debug!("Received message: {}", message_str);
                            messages.push(message_str);

                            // Manually commit the offset
                            if let Err(e) = self.consumer.commit_message(&m, rdkafka::consumer::CommitMode::Sync) {
                                warn!("Failed to commit message: {}", e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        debug!("Consumer error: {}", e);
                        consecutive_timeouts += 1;
                    }
                    Err(_) => {
                        // Timeout on recv() - this is expected when no more messages
                        debug!("Consumer timeout (expected when no more messages)");
                        consecutive_timeouts += 1;
                    }
                }

                // Small delay to prevent busy waiting
                tokio::time::sleep(Duration::from_millis(50)).await;
            }

            info!("Retrieved {} messages from topic {} in {:?}", messages.len(), self.topic, start_time.elapsed());
            Ok(messages)
        }

        /// Wait for a specific number of messages
        pub async fn wait_for_messages(&self, expected_count: usize, max_wait_secs: u64) -> Result<Vec<String>, Box<dyn std::error::Error>> {
            let start_time = std::time::Instant::now();
            let timeout = Duration::from_secs(max_wait_secs);

            while start_time.elapsed() < timeout {
                let messages = self.get_all_messages(2).await?;

                if messages.len() >= expected_count {
                    info!("Found {} messages (expected {})", messages.len(), expected_count);
                    return Ok(messages);
                }

                debug!("Found {} messages, waiting for {} (elapsed: {:?})", 
                       messages.len(), expected_count, start_time.elapsed());

                tokio::time::sleep(Duration::from_millis(500)).await;
            }

            let messages = self.get_all_messages(1).await?;
            if messages.len() >= expected_count {
                Ok(messages)
            } else {
                Err(format!("Timeout waiting for messages. Expected {}, found {} after {:?}", 
                           expected_count, messages.len(), timeout).into())
            }
        }
    }
} 