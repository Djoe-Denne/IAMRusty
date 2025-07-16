use async_trait::async_trait;
use crate::event::{DomainEvent, EventPublisher};
use crate::{EventConsumer, EventHandler};
use rustycog_core::error::ServiceError;
use rustycog_config::SqsConfig;
use aws_sdk_sqs::{Client, Config};
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use tracing::{debug, error, info, warn};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::time::{Duration, sleep};

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
        // Get the event JSON and parse it back to a Value so it's properly structured in the data field
        let event_json_str = event.to_json()?;
        let event_data: serde_json::Value = serde_json::from_str(&event_json_str)
            .map_err(|e| ServiceError::infrastructure(format!("Failed to parse event JSON: {}", e)))?;

        let message_body = json!({
            "event_id": event.event_id(),
            "event_type": event.event_type(),
            "aggregate_id": event.aggregate_id(),
            "occurred_at": event.occurred_at(),
            "version": event.version(),
            "data": event_data,
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

/// SQS event consumer implementation
pub struct SqsEventConsumer {
    client: Client,
    config: SqsConfig,
    should_stop: Arc<std::sync::atomic::AtomicBool>,
}

impl SqsEventConsumer {
    /// Create a new SQS event consumer from configuration
    pub async fn new(config: SqsConfig) -> Result<Self, ServiceError> {
        let client = SqsEventPublisher::create_client(&config).await?;
        
        Ok(Self {
            client,
            config,
            should_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Parse message body into a domain event
    fn parse_message_body(&self, body: &str) -> Result<Box<dyn DomainEvent>, ServiceError> {
        let message: Value = serde_json::from_str(body)
            .map_err(|e| ServiceError::infrastructure(format!("Failed to parse SQS message: {}", e)))?;

        // Extract basic event information
        let event_id = message.get("event_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::infrastructure("Missing event_id in SQS message".to_string()))?;

        let event_type = message.get("event_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::infrastructure("Missing event_type in SQS message".to_string()))?;

        let aggregate_id = message.get("aggregate_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::infrastructure("Missing aggregate_id in SQS message".to_string()))?;

        let occurred_at = message.get("occurred_at")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::infrastructure("Missing occurred_at in SQS message".to_string()))?;

        let version = message.get("version")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;

        let data = message.get("data")
            .ok_or_else(|| ServiceError::infrastructure("Missing data in SQS message".to_string()))?;

        let metadata = message.get("metadata")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Create a generic domain event from the parsed data
        let event = GenericDomainEvent {
            event_id: event_id.to_string(),
            event_type: event_type.to_string(),
            aggregate_id: aggregate_id.to_string(),
            occurred_at: occurred_at.to_string(),
            version,
            data: data.clone(),
            metadata,
        };

        Ok(Box::new(event))
    }

    /// Poll for messages and handle them
    async fn poll_and_handle_messages<H>(&self, handler: &H) -> Result<(), ServiceError>
    where
        H: EventHandler + Send + Sync,
    {
        let queue_url = self.config.default_queue_url();
        
        match self.client
            .receive_message()
            .queue_url(&queue_url)
            .max_number_of_messages(10) // SQS max
            .wait_time_seconds(20) // Long polling
            .send()
            .await
        {
            Ok(response) => {
                if let Some(messages) = response.messages {
                    for message in messages {
                        if let Some(body) = message.body() {
                            match self.parse_message_body(body) {
                                Ok(event) => {
                                    if handler.supports_event_type(&event.event_type()) {
                                        if let Err(e) = handler.handle_event(event).await {
                                            warn!(
                                                error = %e,
                                                message_id = message.message_id().unwrap_or("unknown"),
                                                "Failed to handle message"
                                            );
                                            continue;
                                        }
                                    } else {
                                        debug!(
                                            event_type = event.event_type(),
                                            "Handler doesn't support event type, skipping"
                                        );
                                    }

                                    // Delete the message after successful processing
                                    if let Some(receipt_handle) = message.receipt_handle() {
                                        if let Err(e) = self.client
                                            .delete_message()
                                            .queue_url(&queue_url)
                                            .receipt_handle(receipt_handle)
                                            .send()
                                            .await
                                        {
                                            warn!(
                                                error = %e,
                                                message_id = message.message_id().unwrap_or("unknown"),
                                                "Failed to delete processed message"
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        error = %e,
                                        message_id = message.message_id().unwrap_or("unknown"),
                                        "Failed to parse message body"
                                    );
                                }
                            }
                        }
                    }
                } else {
                    debug!("No messages received from SQS");
                }
            }
            Err(e) => {
                error!(
                    error = %e,
                    queue_url = queue_url,
                    "Failed to receive messages from SQS"
                );
                return Err(ServiceError::infrastructure(format!("SQS receive error: {}", e)));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl EventConsumer for SqsEventConsumer {
    async fn start<H>(&self, handler: H) -> Result<(), ServiceError>
    where
        H: EventHandler + Send + Sync + 'static,
    {
        if !self.config.enabled {
            info!("SQS consumer disabled, not starting");
            return Ok(());
        }

        info!("Starting SQS event consumer");
        self.should_stop.store(false, std::sync::atomic::Ordering::SeqCst);

        let handler = Arc::new(handler);
        
        while !self.should_stop.load(std::sync::atomic::Ordering::SeqCst) {
            if let Err(e) = self.poll_and_handle_messages(handler.as_ref()).await {
                error!(error = %e, "Error polling SQS messages");
                // Sleep for a bit before retrying to avoid tight loop on errors
                sleep(Duration::from_secs(5)).await;
            }
        }

        info!("SQS event consumer stopped");
        Ok(())
    }

    async fn stop(&self) -> Result<(), ServiceError> {
        info!("Stopping SQS event consumer");
        self.should_stop.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn health_check(&self) -> Result<(), ServiceError> {
        if !self.config.enabled {
            return Ok(());
        }

        debug!("Performing SQS consumer health check");

        // Try to get queue attributes as a health check
        match self.client
            .get_queue_attributes()
            .queue_url(&self.config.default_queue_url())
            .attribute_names(aws_sdk_sqs::types::QueueAttributeName::ApproximateNumberOfMessages)
            .send()
            .await 
        {
            Ok(_) => {
                debug!("✅ SQS consumer health check passed");
                Ok(())
            }
            Err(aws_error) => {
                error!(
                    queue_url = %self.config.default_queue_url(),
                    error = %aws_error,
                    "❌ SQS consumer health check failed"
                );
                Err(ServiceError::infrastructure(format!(
                    "SQS consumer health check failed: {}", 
                    aws_error
                )))
            }
        }
    }
}

/// Generic domain event implementation for parsing SQS messages
#[derive(Debug, Clone)]
struct GenericDomainEvent {
    event_id: String,
    event_type: String,
    aggregate_id: String,
    occurred_at: String,
    version: i32,
    data: Value,
    metadata: serde_json::Map<String, Value>,
}

impl DomainEvent for GenericDomainEvent {
    fn event_id(&self) -> uuid::Uuid {
        uuid::Uuid::parse_str(&self.event_id).unwrap_or_else(|_| uuid::Uuid::new_v4())
    }

    fn event_type(&self) -> &str {
        &self.event_type
    }

    fn aggregate_id(&self) -> uuid::Uuid {
        uuid::Uuid::parse_str(&self.aggregate_id).unwrap_or_else(|_| uuid::Uuid::new_v4())
    }

    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(&self.occurred_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now())
    }

    fn version(&self) -> u32 {
        self.version as u32
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(&self.data)
            .map_err(|e| ServiceError::infrastructure(format!("Failed to serialize event data: {}", e)))
    }

    fn metadata(&self) -> std::collections::HashMap<String, String> {
        self.metadata
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect()
    }
} 