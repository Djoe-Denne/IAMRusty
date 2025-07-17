//! Event processors for Telegraph communication service

pub mod email;
pub mod sms;
pub mod notification;

// Re-export all processor types and traits from the processor submodule
pub use processor::{
    CommunicationEventProcessor,
    EmailEventProcessor,
    SmsEventProcessor,
    NotificationEventProcessor,
    DatabaseNotificationProcessor,
    CompositeEventProcessor,
};

mod processor {
    //! Core processor definitions and composite processor

    use async_trait::async_trait;
    use telegraph_domain::{DomainError, EmailService, SmsService, NotificationService};
    use iam_events::IamDomainEvent;
    use rustycog_events::DomainEvent;
    use std::sync::Arc;
    use tracing::{info, error, warn};

    pub use super::email::EmailEventProcessor;
    pub use super::sms::SmsEventProcessor;
    pub use super::notification::{NotificationEventProcessor, DatabaseNotificationProcessor};

    /// Communication event processor trait
    #[async_trait]
    pub trait CommunicationEventProcessor: Send + Sync {
        /// Process an IAM domain event
        async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError>;
        
        /// Check if this processor supports the given event type
        fn supports_event_type(&self, event_type: &str) -> bool;
    }

    /// Composite event processor that routes events to multiple processors
    pub struct CompositeEventProcessor {
        processors: Vec<Arc<dyn CommunicationEventProcessor>>,
    }

    impl CompositeEventProcessor {
        /// Create a new composite event processor
        pub fn new() -> Self {
            Self {
                processors: Vec::new(),
            }
        }
        
        /// Add a processor to the composite
        pub fn add_processor(mut self, processor: Arc<dyn CommunicationEventProcessor>) -> Self {
            self.processors.push(processor);
            self
        }
        
        /// Create a composite processor with all communication types
        pub fn with_all_processors(
            email_service: Arc<dyn EmailService>,
            sms_service: Arc<dyn SmsService>,
            notification_service: Arc<dyn NotificationService>,
        ) -> Self {
            Self::new()
                .add_processor(Arc::new(EmailEventProcessor::new(email_service)))
                .add_processor(Arc::new(SmsEventProcessor::new(sms_service)))
                .add_processor(Arc::new(NotificationEventProcessor::new(notification_service)))
        }
    }

    #[async_trait]
    impl CommunicationEventProcessor for CompositeEventProcessor {
        async fn process_event(&self, event: &IamDomainEvent) -> Result<(), DomainError> {
            let mut errors = Vec::new();
            let mut processed_count = 0;
            
            for processor in &self.processors {
                if processor.supports_event_type(event.event_type()) {
                    match processor.process_event(event).await {
                        Ok(()) => {
                            processed_count += 1;
                        }
                        Err(e) => {
                            error!(
                                event_type = event.event_type(),
                                event_id = %event.event_id(),
                                error = %e,
                                "Processor failed to handle event"
                            );
                            errors.push(e);
                        }
                    }
                }
            }
            
            if !errors.is_empty() {
                warn!(
                    event_type = event.event_type(),
                    event_id = %event.event_id(),
                    errors_count = errors.len(),
                    processed_count = processed_count,
                    "Some processors failed to handle event"
                );
                
                // Return the first error, but log all of them
                return Err(errors.into_iter().next().unwrap());
            }
            
            if processed_count == 0 {
                warn!(
                    event_type = event.event_type(),
                    event_id = %event.event_id(),
                    "No processors handled this event type"
                );
            } else {
                info!(
                    event_type = event.event_type(),
                    event_id = %event.event_id(),
                    processed_count = processed_count,
                    "Event processed successfully by all applicable processors"
                );
            }
            
            Ok(())
        }
        
        fn supports_event_type(&self, event_type: &str) -> bool {
            self.processors.iter().any(|p| p.supports_event_type(event_type))
        }
    }
} 