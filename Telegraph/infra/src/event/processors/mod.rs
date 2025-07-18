//! Event processors for Telegraph communication service

pub mod email;
pub mod notification;

// Re-export all processor types and traits from the processor submodule
pub use processor::{
    EmailEventProcessor,
    NotificationEventProcessor,
    DatabaseNotificationProcessor,
    CompositeEventProcessor,
};

mod processor {
    //! Core processor definitions and composite processor

    use async_trait::async_trait;
    use telegraph_domain::{DomainError, EmailService, NotificationService, TemplateService, EventHandler, EventProcessor, EventContext};
    use std::sync::Arc;
    use tracing::{info, error, warn};

    pub use super::email::EmailEventProcessor;
    pub use super::notification::{NotificationEventProcessor, DatabaseNotificationProcessor};

    /// Composite event processor that routes events to multiple processors
    pub struct CompositeEventProcessor {
        event_handlers: Vec<Arc<dyn EventHandler>>,
    }

    impl CompositeEventProcessor {
        /// Create a new composite event processor
        pub fn new() -> Self {
            Self {
                event_handlers: Vec::new(),
            }
        }
        
        /// Add an event handler to the composite
        pub fn add_handler(mut self, handler: Arc<dyn EventHandler>) -> Self {
            self.event_handlers.push(handler);
            self
        }
        
        /// Create a composite processor with all communication types
        pub fn with_all_processors(
            email_service: Arc<dyn EmailService>,
            template_service: Arc<dyn TemplateService>,
            notification_service: Arc<dyn NotificationService>,
        ) -> Self {
            Self::new()
                .add_handler(Arc::new(EmailEventProcessor::new(email_service, template_service)))
                .add_handler(Arc::new(NotificationEventProcessor::new(notification_service)))
        }
    }

    // Implement EventHandler trait for CompositeEventProcessor
    #[async_trait]
    impl EventHandler for CompositeEventProcessor {
        async fn handle_event(&self, event: &EventContext) -> Result<(), DomainError> {
            let mut errors = Vec::new();
            let mut processed_count = 0;
            
            for handler in &self.event_handlers {
                if handler.supports_event_type(event.event_type.as_str()) {
                    match handler.handle_event(event).await {
                        Ok(()) => {
                            processed_count += 1;
                        }
                        Err(e) => {
                            error!(
                                event_type = event.event_type,
                                event_id = %event.event_id,
                                error = %e,
                                "Event handler failed to handle event"
                            );
                            errors.push(e);
                        }
                    }
                }
            }
            
            if !errors.is_empty() {
                warn!(
                    event_type = event.event_type,
                    event_id = %event.event_id,
                    errors_count = errors.len(),
                    processed_count = processed_count,
                    "Some handlers failed to handle event"
                );
                
                // Return the first error, but log all of them
                return Err(errors.into_iter().next().unwrap());
            }
            
            if processed_count == 0 {
                warn!(
                    event_type = event.event_type,
                    event_id = %event.event_id,
                    "No handlers handled this event type"
                );
            } else {
                info!(
                    event_type = event.event_type,
                    event_id = %event.event_id,
                    processed_count = processed_count,
                    "Event processed successfully by all applicable handlers"
                );
            }
            
            Ok(())
        }
        
        fn supports_event_type(&self, event_type: &str) -> bool {
            self.event_handlers.iter().any(|h| h.supports_event_type(event_type))
        }
        
        fn priority(&self) -> u32 {
            // Use default priority for composite processor
            100
        }
    }

    // Implement EventProcessor trait for CompositeEventProcessor
    #[async_trait]
    impl EventProcessor for CompositeEventProcessor {
        async fn process_event(&self, event: &EventContext) -> Result<(), DomainError> {
            // Delegate to EventHandler implementation
            self.handle_event(event).await
        }
        
        fn get_handlers_for_event(&self, event_type: &str) -> Vec<&dyn EventHandler> {
            self.event_handlers
                .iter()
                .filter(|handler| handler.supports_event_type(event_type))
                .map(|handler| handler.as_ref())
                .collect()
        }
    }
} 