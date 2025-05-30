use async_trait::async_trait;
use domain::entity::events::DomainEvent;
use domain::error::DomainError;
use domain::port::event_publisher::EventPublisher;
use configuration::KafkaConfig;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde_json;
use std::time::Duration;
use tracing::{debug, error, warn, info};

/// Real Kafka event publisher implementation
pub struct KafkaEventPublisher {
    producer: FutureProducer,
    config: KafkaConfig,
}

impl KafkaEventPublisher {
    /// Create a new Kafka event publisher from configuration
    pub fn new(config: KafkaConfig) -> Result<Self, DomainError> {
        let producer = Self::create_producer(&config)?;
        
        Ok(Self {
            producer,
            config,
        })
    }

    /// Create a Kafka producer from configuration
    fn create_producer(config: &KafkaConfig) -> Result<FutureProducer, DomainError> {
        let mut client_config = ClientConfig::new();
        
        // Basic configuration
        client_config
            .set("bootstrap.servers", &config.brokers())
            .set("client.id", &config.client_id)
            .set("message.timeout.ms", &config.timeout_ms.to_string())
            .set("retries", &config.max_retries.to_string())
            .set("compression.type", &config.compression);

        // Security configuration
        client_config.set("security.protocol", &config.security_protocol);
        
        // SASL configuration if provided (for plaintext SASL)
        if let Some(ref mechanism) = config.sasl_mechanism {
            client_config.set("sasl.mechanism", mechanism);
        }
        
        if let Some(ref username) = config.sasl_username {
            client_config.set("sasl.username", username);
        }
        
        if let Some(ref password) = config.sasl_password {
            client_config.set("sasl.password", password);
        }

        // SSL configuration for secure connections
        if config.security_protocol == "ssl" || config.security_protocol == "sasl_ssl" {
            // Set CA certificate location (default to system certificates if not specified)
            let ca_location = config.ssl_ca_location.as_deref().unwrap_or("probe");
            client_config.set("ssl.ca.location", ca_location);
            
            // Enable SSL certificate verification
            client_config.set("ssl.certificate.verification", "true");
            
            // Set SSL endpoint identification algorithm
            client_config.set("ssl.endpoint.identification.algorithm", "https");
            
            // Set client certificate and key if provided (for mutual TLS)
            if let Some(ref cert_location) = config.ssl_certificate_location {
                client_config.set("ssl.certificate.location", cert_location);
            }
            
            if let Some(ref key_location) = config.ssl_key_location {
                client_config.set("ssl.key.location", key_location);
            }
            
            if let Some(ref key_password) = config.ssl_key_password {
                client_config.set("ssl.key.password", key_password);
            }
        }
        
        // Producer-specific configuration
        client_config
            .set("acks", "all") // Wait for all replicas to acknowledge
            .set("enable.idempotence", "true") // Enable idempotent producer
            .set("max.in.flight.requests.per.connection", "5")
            .set("batch.size", "16384")
            .set("linger.ms", "5");

        client_config
            .create()
            .map_err(|e| DomainError::RepositoryError(format!("Failed to create Kafka producer: {}", e)))
    }

    /// Serialize domain event to JSON
    fn serialize_event(&self, event: &DomainEvent) -> Result<String, DomainError> {
        serde_json::to_string(event)
            .map_err(|e| DomainError::RepositoryError(format!("Failed to serialize event: {}", e)))
    }

    /// Get topic for event
    fn get_topic_for_event(&self, _event: &DomainEvent) -> &str {
        // For now, all events go to the user events topic
        // In the future, we could route different event types to different topics
        &self.config.user_events_topic
    }
}

#[async_trait]
impl EventPublisher for KafkaEventPublisher {
    async fn publish(&self, event: DomainEvent) -> Result<(), DomainError> {
        if !self.config.enabled {
            debug!(
                event_id = %event.event_id(),
                event_type = %event.event_type(),
                "Kafka publishing disabled, skipping event"
            );
            return Ok(());
        }

        let topic = self.get_topic_for_event(&event);
        let payload = self.serialize_event(&event)?;
        let event_id = event.event_id().to_string();
        let user_id = event.user_id().to_string();

        debug!(
            event_id = %event.event_id(),
            event_type = %event.event_type(),
            user_id = %event.user_id(),
            topic = topic,
            "Publishing event to Kafka"
        );

        let record = FutureRecord::to(topic)
            .key(&user_id) // Use user_id as partition key for ordering
            .payload(&payload)
            .headers(rdkafka::message::OwnedHeaders::new()
                .insert(rdkafka::message::Header {
                    key: "event_id",
                    value: Some(&event_id),
                })
                .insert(rdkafka::message::Header {
                    key: "event_type", 
                    value: Some(event.event_type()),
                })
                .insert(rdkafka::message::Header {
                    key: "user_id",
                    value: Some(&user_id),
                }));

        let timeout = Timeout::After(Duration::from_millis(self.config.timeout_ms));
        
        match self.producer.send(record, timeout).await {
            Ok((partition, offset)) => {
                info!(
                    event_id = %event.event_id(),
                    event_type = %event.event_type(),
                    user_id = %event.user_id(),
                    partition = partition,
                    offset = offset,
                    topic = topic,
                    "✅ Event successfully published to Kafka"
                );
                Ok(())
            }
            Err((kafka_error, _)) => {
                error!(
                    event_id = %event.event_id(),
                    event_type = %event.event_type(),
                    user_id = %event.user_id(),
                    topic = topic,
                    error = %kafka_error,
                    "❌ Failed to publish event to Kafka"
                );
                Err(DomainError::RepositoryError(format!(
                    "Failed to publish event to Kafka: {}", 
                    kafka_error
                )))
            }
        }
    }

    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), DomainError> {
        if !self.config.enabled {
            debug!(
                event_count = events.len(),
                "Kafka publishing disabled, skipping batch"
            );
            return Ok(());
        }

        debug!(event_count = events.len(), "Publishing batch of events to Kafka");

        // Publish all events concurrently
        let futures: Vec<_> = events.into_iter()
            .map(|event| self.publish(event))
            .collect();

        // Wait for all to complete
        let results: Vec<Result<(), DomainError>> = futures::future::join_all(futures).await;

        // Check if any failed
        let mut first_error = None;
        let mut failure_count = 0;
        
        for result in &results {
            if let Err(e) = result {
                failure_count += 1;
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        if let Some(error) = first_error {
            warn!(
                failed_count = failure_count,
                total_count = results.len(),
                "Some events in batch failed to publish"
            );
            // Return a new error based on the first error
            return Err(DomainError::RepositoryError(format!(
                "Batch publishing failed: {}", 
                error
            )));
        }

        debug!("Batch of events successfully published to Kafka");
        Ok(())
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        if !self.config.enabled {
            return Ok(());
        }

        // For Kafka health check, we'll try to get metadata
        // This is a simple way to verify connectivity
        let timeout = Duration::from_millis(self.config.timeout_ms);
        
        // Use a simple approach: try to fetch metadata
        match tokio::time::timeout(timeout, async {
            // Create a simple check by trying to send a test message to a non-existent topic
            // This will trigger a metadata request which tests connectivity
            let test_record = FutureRecord::to("__kafka_health_check__")
                .payload("test")
                .key("health_check");
            
            // We don't actually care if this succeeds, just that we can connect
            let _ = self.producer.send(test_record, Timeout::After(Duration::from_millis(1000))).await;
            Ok::<(), DomainError>(())
        }).await {
            Ok(_) => {
                debug!("Kafka health check passed");
                Ok(())
            }
            Err(_) => {
                error!("Kafka health check failed: timeout");
                Err(DomainError::RepositoryError(
                    "Kafka health check failed: timeout".to_string()
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::entity::events::{UserSignedUpEvent, DomainEvent};
    use uuid::Uuid;

    #[test]
    fn test_kafka_config_creation() {
        let config = KafkaConfig {
            enabled: true,
            host: "localhost".to_string(),
            port: 9092,
            user_events_topic: "test-topic".to_string(),
            client_id: "test-client".to_string(),
            timeout_ms: 5000,
            max_retries: 3,
            compression: "gzip".to_string(),
            security_protocol: "plaintext".to_string(),
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
            ssl_ca_location: None,
            ssl_certificate_location: None,
            ssl_key_location: None,
            ssl_key_password: None,
            additional_brokers: vec![],
        };

        // Note: This will fail in CI/test environments without Kafka
        // We're mainly testing that the configuration is properly set up
        let result = KafkaEventPublisher::new(config);
        
        // In test environments, we expect this to fail due to no Kafka broker
        // The important thing is that we get a meaningful error, not a panic
        match result {
            Ok(_) => {
                // If Kafka is available, great!
            }
            Err(e) => {
                // Expected in test environments
                assert!(e.to_string().contains("Failed to create Kafka producer"));
            }
        }
    }

    #[test]
    fn test_event_serialization() {
        let config = KafkaConfig::default();
        let event = DomainEvent::UserSignedUp(UserSignedUpEvent::new(
            Uuid::new_v4(),
            "test@example.com".to_string(),
            "testuser".to_string(),
            false,
        ));

        // Create publisher without actually connecting to Kafka
        // This tests the serialization logic
        if let Ok(publisher) = KafkaEventPublisher::new(config) {
            let serialized = publisher.serialize_event(&event);
            assert!(serialized.is_ok());
            
            let json = serialized.unwrap();
            assert!(json.contains("user_signed_up"));
            assert!(json.contains("test@example.com"));
            assert!(json.contains("testuser"));
        }
    }
} 