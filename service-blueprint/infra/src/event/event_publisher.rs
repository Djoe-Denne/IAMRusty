use async_trait::async_trait;
use rustycog_events::{Event, EventPublisher};
use serde_json::Value;

use {{SERVICE_NAME}}_domain::DomainError;

/// Dummy event publisher for testing and development
pub struct DummyEventPublisher;

impl DummyEventPublisher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DummyEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventPublisher for DummyEventPublisher {
    type Error = DomainError;

    async fn publish(&self, event: &Event) -> Result<(), Self::Error> {
        tracing::info!(
            event_type = event.event_type(),
            event_id = %event.id(),
            "Dummy event published: {:?}",
            event
        );
        Ok(())
    }

    async fn publish_batch(&self, events: &[Event]) -> Result<(), Self::Error> {
        tracing::info!(
            event_count = events.len(),
            "Dummy batch events published"
        );
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }
}

/// SQS event publisher implementation
pub struct SqsEventPublisher {
    // SQS client would go here
    queue_url: String,
}

impl SqsEventPublisher {
    pub async fn new(queue_url: String) -> Result<Self, DomainError> {
        // Implementation would create SQS client
        Ok(Self { queue_url })
    }
}

#[async_trait]
impl EventPublisher for SqsEventPublisher {
    type Error = DomainError;

    async fn publish(&self, event: &Event) -> Result<(), Self::Error> {
        // Convert event to JSON
        let message_body = serde_json::to_string(event)
            .map_err(|e| DomainError::internal(&format!("Failed to serialize event: {}", e)))?;

        tracing::debug!(
            queue_url = %self.queue_url,
            event_type = event.event_type(),
            event_id = %event.id(),
            "Publishing event to SQS"
        );

        // Implementation would send to SQS
        // For now, just log
        tracing::info!(
            queue_url = %self.queue_url,
            message_body = %message_body,
            "SQS event published (mock)"
        );

        Ok(())
    }

    async fn publish_batch(&self, events: &[Event]) -> Result<(), Self::Error> {
        tracing::debug!(
            queue_url = %self.queue_url,
            event_count = events.len(),
            "Publishing batch events to SQS"
        );

        // Implementation would use SQS batch send
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }
}

/// Example domain events
pub mod events {
    use chrono::{DateTime, Utc};
    use rustycog_events::Event;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    /// Event published when an entity is created
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EntityCreatedEvent {
        pub entity_id: Uuid,
        pub entity_name: String,
        pub created_by: Option<Uuid>,
        pub created_at: DateTime<Utc>,
    }

    impl Event for EntityCreatedEvent {
        fn event_type(&self) -> &'static str {
            "{{SERVICE_NAME}}.entity.created"
        }

        fn id(&self) -> Uuid {
            self.entity_id
        }

        fn occurred_at(&self) -> DateTime<Utc> {
            self.created_at
        }

        fn metadata(&self) -> std::collections::HashMap<String, String> {
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("entity_name".to_string(), self.entity_name.clone());
            if let Some(created_by) = self.created_by {
                metadata.insert("created_by".to_string(), created_by.to_string());
            }
            metadata
        }
    }

    /// Event published when an entity is updated
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EntityUpdatedEvent {
        pub entity_id: Uuid,
        pub entity_name: String,
        pub updated_by: Option<Uuid>,
        pub updated_at: DateTime<Utc>,
        pub changes: Vec<String>, // List of changed fields
    }

    impl Event for EntityUpdatedEvent {
        fn event_type(&self) -> &'static str {
            "{{SERVICE_NAME}}.entity.updated"
        }

        fn id(&self) -> Uuid {
            self.entity_id
        }

        fn occurred_at(&self) -> DateTime<Utc> {
            self.updated_at
        }

        fn metadata(&self) -> std::collections::HashMap<String, String> {
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("entity_name".to_string(), self.entity_name.clone());
            metadata.insert("changes".to_string(), self.changes.join(","));
            if let Some(updated_by) = self.updated_by {
                metadata.insert("updated_by".to_string(), updated_by.to_string());
            }
            metadata
        }
    }

    /// Event published when an entity is deleted
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EntityDeletedEvent {
        pub entity_id: Uuid,
        pub entity_name: String,
        pub deleted_by: Option<Uuid>,
        pub deleted_at: DateTime<Utc>,
    }

    impl Event for EntityDeletedEvent {
        fn event_type(&self) -> &'static str {
            "{{SERVICE_NAME}}.entity.deleted"
        }

        fn id(&self) -> Uuid {
            self.entity_id
        }

        fn occurred_at(&self) -> DateTime<Utc> {
            self.deleted_at
        }

        fn metadata(&self) -> std::collections::HashMap<String, String> {
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("entity_name".to_string(), self.entity_name.clone());
            if let Some(deleted_by) = self.deleted_by {
                metadata.insert("deleted_by".to_string(), deleted_by.to_string());
            }
            metadata
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::events::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_dummy_event_publisher() {
        let publisher = DummyEventPublisher::new();
        let event = EntityCreatedEvent {
            entity_id: Uuid::new_v4(),
            entity_name: "Test Entity".to_string(),
            created_by: Some(Uuid::new_v4()),
            created_at: Utc::now(),
        };

        let result = publisher.publish(&event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dummy_event_publisher_batch() {
        let publisher = DummyEventPublisher::new();
        let events = vec![
            Box::new(EntityCreatedEvent {
                entity_id: Uuid::new_v4(),
                entity_name: "Test Entity 1".to_string(),
                created_by: Some(Uuid::new_v4()),
                created_at: Utc::now(),
            }) as Box<dyn Event>,
            Box::new(EntityCreatedEvent {
                entity_id: Uuid::new_v4(),
                entity_name: "Test Entity 2".to_string(),
                created_by: Some(Uuid::new_v4()),
                created_at: Utc::now(),
            }) as Box<dyn Event>,
        ];

        let result = publisher.publish_batch(&events).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_entity_created_event() {
        let event = EntityCreatedEvent {
            entity_id: Uuid::new_v4(),
            entity_name: "Test Entity".to_string(),
            created_by: Some(Uuid::new_v4()),
            created_at: Utc::now(),
        };

        assert_eq!(event.event_type(), "{{SERVICE_NAME}}.entity.created");
        assert!(!event.metadata().is_empty());
    }
} 