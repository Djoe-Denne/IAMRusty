//! # IAM Events
//! 
//! Shared domain events for IAM (Identity and Access Management) microservices.
//! This crate provides common event types that can be published by IAM services
//! and consumed by other services like Telegraph (communication service).

pub mod events;

// Re-export for convenience
pub use events::*;

// Re-export rustycog-events for consumers
pub use rustycog_events::*; 