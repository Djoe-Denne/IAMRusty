//! Event processors for Telegraph communication service

pub mod email;
pub mod notification;

// Re-export all processor types and traits from the processor submodule
pub use processor::{
    EmailEventProcessor,
    DatabaseNotificationProcessor,
    CompositeEventProcessor,
    EventHandlerConfig,
};

mod processor {
    //! Core processor definitions and composite processor

    use async_trait::async_trait;
    use telegraph_domain::{DomainError, EmailService, NotificationService, CommunicationFactory, EventHandler, EventProcessor, EventContext};
    use std::sync::Arc;
    use tracing::{info, error, warn};
    use std::collections::HashMap;
    pub use super::email::EmailEventProcessor;
    pub use super::notification::DatabaseNotificationProcessor;

    pub struct EventHandlerConfig {
        pub event_mapping: HashMap<String, Vec<String>>,
    }

    /// Composite event processor that routes events to multiple processors
    pub struct CompositeEventProcessor {
        event_handlers: HashMap<String, Arc<dyn EventHandler>>,
        config: EventHandlerConfig,
    }

    impl CompositeEventProcessor {
        /// Create a new composite event processor
        pub fn new(config: EventHandlerConfig) -> Self {
            Self {
                event_handlers: HashMap::new(),
                config,
            }
        }
        
        /// Add an event handler to the composite
        pub fn add_handler(mut self, name: String, handler: Arc<dyn EventHandler>) -> Self {
            self.event_handlers.insert(name, handler);
            self
        }
        
        /// Create a composite processor with all communication types
        pub fn with_all_processors(
            config: EventHandlerConfig,
            email_service: Arc<EmailService>,
            communication_factory: Arc<CommunicationFactory>,
            notification_service: Arc<NotificationService>,
        ) -> Self {
            Self::new(config)
                .add_handler("email".to_string(), Arc::new(EmailEventProcessor::new(email_service, communication_factory.clone())))
                .add_handler("notification".to_string(), Arc::new(DatabaseNotificationProcessor::new(notification_service, communication_factory)))
        }
    }

    // Implement EventHandler trait for CompositeEventProcessor
    #[async_trait]
    impl EventHandler for CompositeEventProcessor {
        async fn handle_event(&self, event: &EventContext) -> Result<(), DomainError> {
            let mut errors = Vec::new();
            let mut processed_count = 0;
            
            if let Some(handlers) = self.config.event_mapping.get(&event.event_type) {
                for handler in handlers {
                    match self.event_handlers.get(handler).unwrap().handle_event(event).await {
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
            if let Some(handlers) = self.config.event_mapping.get(event_type) {
                handlers.iter().map(|handler| self.event_handlers.get(handler).unwrap().as_ref()).collect()
            } else {
                Vec::new()
            }
        }
    }
} 