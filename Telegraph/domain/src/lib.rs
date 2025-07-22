//! # Telegraph Domain
//! 
//! Domain layer for the Telegraph communication service.
//! This crate contains the core business logic, entities, and domain services
//! for handling communication events and messaging.

pub mod entity;
pub mod error;
pub mod port;
pub mod service;

// Re-export commonly used types
pub use error::DomainError;

// Re-export entities
pub use entity::{
    CommunicationMode, MessageTemplate, TemplateContent, RenderedTemplate, Communication, EmailCommunication, NotificationCommunication,
    CommunicationRecipient, CommunicationDescriptor, EmailDescriptor, NotificationDescriptor,
    MessageDelivery, DeliveryStatus,
};

// Re-export ports (specific items to avoid conflicts)
pub use port::{
    EmailProvider, NotificationRepository,
    TemplateService, EventExtractor, EventProcessor, EventHandler, EventContext, EventRecipient
};

// Re-export services
pub use service::{
    CommunicationFactory, EmailService, NotificationService
};

// Re-export IAM events for convenience
pub use iam_events::*; 