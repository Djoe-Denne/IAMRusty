//! # Telegraph Domain
//! 
//! Domain layer for the Telegraph communication service.
//! This crate contains the core business logic, entities, and domain services
//! for handling communication events and messaging.

pub mod entity;
pub mod error;
pub mod port;

// Re-export commonly used types
pub use error::DomainError;

// Re-export entities
pub use entity::{
    CommunicationMessage, MessageRecipient, MessageContent, MessageDelivery,
    CommunicationMode, MessagePriority, EmailAttachment,
    MessageTemplate, TemplateContent, RenderedTemplate
};

// Re-export ports (specific items to avoid conflicts)
pub use port::{
    CommunicationService, EmailService, NotificationService, SmsService,
    TemplateService, EventProcessor, EventHandler, EventContext, EventRecipient
};

// Re-export IAM events for convenience
pub use iam_events::*; 