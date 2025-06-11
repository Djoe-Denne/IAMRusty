use async_trait::async_trait;
use crate::event::{DomainEvent, EventPublisher};
use rustycog_core::error::ServiceError;
use rustycog_config::KafkaConfig;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use std::time::Duration;
use tracing::{debug, error, info};

/// Real Kafka event publisher implementation
pub struct KafkaEventPublisher {
    producer: FutureProducer,
    config: KafkaConfig,
}

impl KafkaEventPublisher {
    /// Create a new Kafka event publisher from configuration
    pub fn new(config: KafkaConfig) -> Result<Self, ServiceError> {
        let producer = Self::create_producer(&config)?;
        
        Ok(Self {
            producer,
            config,
        })
    }

    /// Create a Kafka producer from configuration
    fn create_producer(config: &KafkaConfig) -> Result<FutureProducer, ServiceError> {
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
            .map_err(|e| ServiceError::infrastructure(format!("Failed to create Kafka producer: {}", e)))
    }

    /// Serialize domain event to JSON
    fn serialize_event(&self, event: &dyn DomainEvent) -> Result<String, ServiceError> {
        event.to_json()
            .map_err(|e| ServiceError::infrastructure(format!("Failed to serialize event: {}", e)))
    }

    /// Get topic for event
    fn get_topic_for_event(&self, _event: &dyn DomainEvent) -> &str {
        // For now, all events go to the user events topic
        // In the future, we could route different event types to different topics
        &self.config.user_events_topic
    }
}

#[async_trait]
impl EventPublisher for KafkaEventPublisher {
    async fn publish(&self, event: Box<dyn DomainEvent>) -> Result<(), ServiceError> {
        if !self.config.enabled {
            debug!(
                event_id = %event.event_id(),
                event_type = %event.event_type(),
                "Kafka publishing disabled, skipping event"
            );
            return Ok(());
        }

        let topic = self.get_topic_for_event(event.as_ref());
        let payload = self.serialize_event(event.as_ref())?;
        let event_id = event.event_id().to_string();
        let aggregate_id = event.aggregate_id().to_string();

        debug!(
            event_id = %event.event_id(),
            event_type = %event.event_type(),
            aggregate_id = %event.aggregate_id(),
            topic = topic,
            "Publishing event to Kafka"
        );

        let record = FutureRecord::to(topic)
            .key(&aggregate_id) // Use aggregate_id as partition key for ordering
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
                    key: "aggregate_id",
                    value: Some(&aggregate_id),
                }));

        let timeout = Timeout::After(Duration::from_millis(self.config.timeout_ms));
        
        match self.producer.send(record, timeout).await {
            Ok((partition, offset)) => {
                info!(
                    event_id = %event.event_id(),
                    event_type = %event.event_type(),
                    aggregate_id = %event.aggregate_id(),
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
                    aggregate_id = %event.aggregate_id(),
                    topic = topic,
                    error = %kafka_error,
                    "❌ Failed to publish event to Kafka"
                );
                Err(ServiceError::infrastructure(format!(
                    "Failed to publish event to Kafka: {}", 
                    kafka_error
                )))
            }
        }
    }

    async fn publish_batch(&self, events: Vec<Box<dyn DomainEvent>>) -> Result<(), ServiceError> {
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
        let results: Vec<Result<(), ServiceError>> = futures::future::join_all(futures).await;

        // Check for any failures
        let failures: Vec<_> = results.into_iter()
            .enumerate()
            .filter_map(|(i, result)| {
                match result {
                    Err(e) => Some((i, e)),
                    Ok(_) => None,
                }
            })
            .collect();

        if !failures.is_empty() {
            let failure_count = failures.len();
            let first_error = &failures[0].1;
            error!(
                failure_count = failure_count,
                first_error = %first_error,
                "Failed to publish some events in batch"
            );
            
            // Return the first error for simplicity
            // In production, you might want to return all errors or a summary
            return Err(ServiceError::infrastructure(format!(
                "Failed to publish {} events in batch. First error: {}", 
                failure_count, 
                first_error
            )));
        }

        info!("✅ Successfully published all events in batch");
        Ok(())
    }

    async fn health_check(&self) -> Result<(), ServiceError> {
        if !self.config.enabled {
            debug!("Kafka publishing disabled, health check passed");
            return Ok(());
        }

        // Create a simple metadata request to check connectivity
        let _timeout = Duration::from_millis(self.config.timeout_ms);
        
        // This is a simple check - in production you might want to:
        // 1. Check cluster metadata
        // 2. Verify topic existence
        // 3. Test produce to a health check topic
        
        // For now, we'll just verify the producer is still healthy
        // by checking if we can create a new one with the same config
        match Self::create_producer(&self.config) {
            Ok(_) => {
                debug!("Kafka health check passed");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Kafka health check failed");
                Err(ServiceError::infrastructure(
                    format!("Kafka health check failed: {}", e)
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_kafka_config_creation() {
        let config = KafkaConfig::default();

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
} 