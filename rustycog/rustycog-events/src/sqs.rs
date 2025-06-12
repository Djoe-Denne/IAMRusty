use async_trait::async_trait;
use crate::event::{DomainEvent, EventPublisher};
use rustycog_core::error::ServiceError;
use rustycog_config::SqsConfig;
use aws_sdk_sqs::{Client, Config};
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use tracing::{debug, error, info};
use serde_json::json;

/// SQS event publisher implementation
pub struct SqsEventPublisher {
    client: Client,
    config: SqsConfig,
}

impl SqsEventPublisher {
    /// Create a new SQS event publisher from configuration
    pub async fn new(config: SqsConfig) -> Result<Self, ServiceError> {
        let client = Self::create_client(&config).await?;
        
        Ok(Self {
            client,
            config,
        })
    }

    /// Create an SQS client from configuration
    async fn create_client(config: &SqsConfig) -> Result<Client, ServiceError> {
        let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest());

        // Set region
        aws_config_builder = aws_config_builder.region(Region::new(config.region.clone()));

        // Set endpoint if using localstack or custom endpoint (now using host/port configuration)
        if let Some(endpoint_url) = config.endpoint_url() {
            aws_config_builder = aws_config_builder.endpoint_url(endpoint_url);
        }

        // Set credentials if provided
        if let (Some(ref access_key), Some(ref secret_key)) = (&config.access_key_id, &config.secret_access_key) {
            let credentials = Credentials::new(
                access_key,
                secret_key,
                config.session_token.clone(),
                None,
                "rustycog-events",
            );
            aws_config_builder = aws_config_builder.credentials_provider(credentials);
        }

        let aws_config = aws_config_builder.load().await;
        let sqs_config = Config::from(&aws_config);
        let client = Client::from_conf(sqs_config);

        Ok(client)
    }

    /// Serialize domain event to SQS message body
    fn serialize_event(&self, event: &dyn DomainEvent) -> Result<String, ServiceError> {
        let message_body = json!({
            "event_id": event.event_id(),
            "event_type": event.event_type(),
            "aggregate_id": event.aggregate_id(),
            "occurred_at": event.occurred_at(),
            "version": event.version(),
            "data": event.to_json()?,
            "metadata": event.metadata()
        });

        serde_json::to_string(&message_body)
            .map_err(|e| ServiceError::infrastructure(format!("Failed to serialize event for SQS: {}", e)))
    }

    /// Get queue URL for event
    fn get_queue_url_for_event(&self, event: &dyn DomainEvent) -> String {
        // Use the new generic queue configuration
        self.config.get_queue_url(event.event_type())
    }

    /// Create message attributes for the event
    fn create_message_attributes(&self, event: &dyn DomainEvent) -> std::collections::HashMap<String, aws_sdk_sqs::types::MessageAttributeValue> {
        let mut attributes = std::collections::HashMap::new();
        
        attributes.insert(
            "event_id".to_string(),
            aws_sdk_sqs::types::MessageAttributeValue::builder()
                .data_type("String")
                .string_value(event.event_id().to_string())
                .build()
                .unwrap(),
        );
        
        attributes.insert(
            "event_type".to_string(),
            aws_sdk_sqs::types::MessageAttributeValue::builder()
                .data_type("String")
                .string_value(event.event_type())
                .build()
                .unwrap(),
        );
        
        attributes.insert(
            "aggregate_id".to_string(),
            aws_sdk_sqs::types::MessageAttributeValue::builder()
                .data_type("String")
                .string_value(event.aggregate_id().to_string())
                .build()
                .unwrap(),
        );

        attributes.insert(
            "source".to_string(),
            aws_sdk_sqs::types::MessageAttributeValue::builder()
                .data_type("String")
                .string_value("rustycog-events")
                .build()
                .unwrap(),
        );

        attributes
    }
}

#[async_trait]
impl EventPublisher for SqsEventPublisher {
    async fn publish(&self, event: Box<dyn DomainEvent>) -> Result<(), ServiceError> {
        if !self.config.enabled {
            debug!(
                event_id = %event.event_id(),
                event_type = %event.event_type(),
                "SQS publishing disabled, skipping event"
            );
            return Ok(());
        }

        let queue_url = self.get_queue_url_for_event(event.as_ref());
        let message_body = self.serialize_event(event.as_ref())?;
        let message_attributes = self.create_message_attributes(event.as_ref());

        debug!(
            event_id = %event.event_id(),
            event_type = %event.event_type(),
            aggregate_id = %event.aggregate_id(),
            queue_url = queue_url,
            "Publishing event to SQS"
        );

        let mut send_request = self.client
            .send_message()
            .queue_url(&queue_url)
            .message_body(message_body)
            .set_message_attributes(Some(message_attributes));

        // Use aggregate_id as message group ID for FIFO queues
        let queue_name = self.config.get_queue_name(event.event_type());
        if self.config.is_fifo_queue(queue_name) {
            send_request = send_request
                .message_group_id(event.aggregate_id().to_string())
                .message_deduplication_id(event.event_id().to_string());
        }

        match send_request.send().await {
            Ok(response) => {
                info!(
                    event_id = %event.event_id(),
                    event_type = %event.event_type(),
                    aggregate_id = %event.aggregate_id(),
                    message_id = response.message_id().unwrap_or("unknown"),
                    queue_url = queue_url,
                    "✅ Event successfully published to SQS"
                );
                Ok(())
            }
            Err(aws_error) => {
                error!(
                    event_id = %event.event_id(),
                    event_type = %event.event_type(),
                    aggregate_id = %event.aggregate_id(),
                    queue_url = queue_url,
                    error = %aws_error,
                    "❌ Failed to publish event to SQS"
                );
                Err(ServiceError::infrastructure(format!(
                    "Failed to publish event to SQS: {}", 
                    aws_error
                )))
            }
        }
    }

    async fn publish_batch(&self, events: Vec<Box<dyn DomainEvent>>) -> Result<(), ServiceError> {
        if !self.config.enabled {
            debug!(
                event_count = events.len(),
                "SQS publishing disabled, skipping batch"
            );
            return Ok(());
        }

        if events.is_empty() {
            return Ok(());
        }

        debug!(event_count = events.len(), "Publishing batch of events to SQS");

        // SQS supports batch sending up to 10 messages at a time
        let batch_size = 10;
        let mut all_successful = true;
        let mut first_error = None;

        for chunk in events.chunks(batch_size) {
            let queue_url = self.get_queue_url_for_event(chunk[0].as_ref());
            let mut entries = Vec::new();

            for (idx, event) in chunk.iter().enumerate() {
                let message_body = match self.serialize_event(event.as_ref()) {
                    Ok(body) => body,
                    Err(e) => {
                        all_successful = false;
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                        continue;
                    }
                };

                let message_attributes = self.create_message_attributes(event.as_ref());
                let entry_id = format!("entry_{}", idx);

                let mut entry = aws_sdk_sqs::types::SendMessageBatchRequestEntry::builder()
                    .id(entry_id)
                    .message_body(message_body)
                    .set_message_attributes(Some(message_attributes));

                // For FIFO queues
                let queue_name = self.config.get_queue_name(event.event_type());
                if self.config.is_fifo_queue(queue_name) {
                    entry = entry
                        .message_group_id(event.aggregate_id().to_string())
                        .message_deduplication_id(event.event_id().to_string());
                }

                if let Ok(built_entry) = entry.build() {
                    entries.push(built_entry);
                }
            }

            if !entries.is_empty() {
                match self.client
                    .send_message_batch()
                    .queue_url(&queue_url)
                    .set_entries(Some(entries))
                    .send()
                    .await
                {
                    Ok(response) => {
                        let failed = response.failed();
                        if !failed.is_empty() {
                            all_successful = false;
                            error!(
                                failed_count = failed.len(),
                                queue_url = queue_url,
                                "Some messages failed to send in batch"
                            );
                            if first_error.is_none() {
                                let first_failed = &failed[0];
                                first_error = Some(ServiceError::infrastructure(format!(
                                    "SQS batch send failed: {} - {}", 
                                    first_failed.code(),
                                    first_failed.message().unwrap_or("no message")
                                )));
                            }
                        }
                        
                        let successful = response.successful();
                        info!(
                            successful_count = successful.len(),
                            queue_url = queue_url,
                            "✅ Messages successfully sent in batch"
                        );
                    }
                    Err(aws_error) => {
                        all_successful = false;
                        error!(
                            queue_url = queue_url,
                            error = %aws_error,
                            "❌ Failed to send message batch to SQS"
                        );
                        if first_error.is_none() {
                            first_error = Some(ServiceError::infrastructure(format!(
                                "Failed to send batch to SQS: {}", 
                                aws_error
                            )));
                        }
                    }
                }
            }
        }

        if all_successful {
            info!("✅ Successfully published all events in batch");
            Ok(())
        } else {
            Err(first_error.unwrap_or_else(|| {
                ServiceError::infrastructure("Some events failed to publish in batch".to_string())
            }))
        }
    }

    async fn health_check(&self) -> Result<(), ServiceError> {
        if !self.config.enabled {
            return Ok(());
        }

        debug!("Performing SQS health check");

        // Try to get queue attributes as a health check
        match self.client
            .get_queue_attributes()
            .queue_url(&self.config.default_queue_url())
            .attribute_names(aws_sdk_sqs::types::QueueAttributeName::ApproximateNumberOfMessages)
            .send()
            .await 
        {
            Ok(_) => {
                debug!("✅ SQS health check passed");
                Ok(())
            }
            Err(aws_error) => {
                error!(
                    queue_url = %self.config.default_queue_url(),
                    error = %aws_error,
                    "❌ SQS health check failed"
                );
                Err(ServiceError::infrastructure(format!(
                    "SQS health check failed: {}", 
                    aws_error
                )))
            }
        }
    }
} 