use async_trait::async_trait;
use domain::entity::events::DomainEvent;
use domain::error::DomainError;
use domain::port::event_publisher::EventPublisher;

/// Mock event publisher for testing
pub struct MockEventPublisher {
    pub published_events: std::sync::Arc<std::sync::Mutex<Vec<DomainEvent>>>,
    pub should_fail: bool,
}

impl MockEventPublisher {
    pub fn new() -> Self {
        Self {
            published_events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            should_fail: false,
        }
    }

    pub fn with_failure() -> Self {
        Self {
            published_events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            should_fail: true,
        }
    }

    pub fn get_published_events(&self) -> Vec<DomainEvent> {
        self.published_events.lock().unwrap().clone()
    }

    pub fn clear_events(&self) {
        self.published_events.lock().unwrap().clear();
    }
}

#[async_trait]
impl EventPublisher for MockEventPublisher {
    async fn publish(&self, event: DomainEvent) -> Result<(), DomainError> {
        if self.should_fail {
            return Err(DomainError::RepositoryError("Mock publisher configured to fail".to_string()));
        }

        self.published_events.lock().unwrap().push(event);
        Ok(())
    }

    async fn health_check(&self) -> Result<(), DomainError> {
        if self.should_fail {
            return Err(DomainError::RepositoryError("Mock publisher health check failed".to_string()));
        }
        Ok(())
    }
} 