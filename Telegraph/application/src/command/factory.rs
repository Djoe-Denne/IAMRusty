//! Command factory for Telegraph application
//! 
//! This module provides factory methods for creating command registries
//! with Telegraph-specific commands registered.

use super::{
    ProcessEventCommand, ProcessEventCommandHandler, ProcessEventErrorMapper,
};
use crate::usecase::EventProcessingUseCaseTrait;
use rustycog_command::{CommandRegistry, CommandRegistryBuilder};
use std::sync::Arc;

/// Factory for creating a command registry with Telegraph commands registered
pub struct TelegraphCommandRegistryFactory;

impl TelegraphCommandRegistryFactory {
    /// Create a command registry with all Telegraph commands registered
    pub fn create_telegraph_registry(
        event_processing_usecase: Arc<dyn EventProcessingUseCaseTrait>,
    ) -> CommandRegistry {
        let mut builder = CommandRegistryBuilder::new();

        // Register process event command
        let process_event_handler = Arc::new(ProcessEventCommandHandler::new(event_processing_usecase));
        let process_event_error_mapper = Arc::new(ProcessEventErrorMapper);

        builder = builder.register::<ProcessEventCommand, _>(
            "process_event".to_string(),
            process_event_handler,
            process_event_error_mapper,
        );

        builder.build()
    }

    /// Create an empty registry builder for custom command registration
    pub fn create_empty_builder() -> CommandRegistryBuilder {
        CommandRegistryBuilder::new()
    }

    /// Create a registry builder with only event processing commands
    pub fn create_builder_with_event_processing(
        event_processing_usecase: Arc<dyn EventProcessingUseCaseTrait>,
    ) -> CommandRegistryBuilder {
        let process_event_handler = Arc::new(ProcessEventCommandHandler::new(event_processing_usecase));
        let process_event_error_mapper = Arc::new(ProcessEventErrorMapper);

        CommandRegistryBuilder::new().register::<ProcessEventCommand, _>(
            "process_event".to_string(),
            process_event_handler,
            process_event_error_mapper,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_empty_builder() {
        let builder = TelegraphCommandRegistryFactory::create_empty_builder();
        let registry = builder.build();
        let command_types = registry.list_command_types();

        assert!(command_types.is_empty());
    }
} 