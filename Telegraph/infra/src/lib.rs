//! # Telegraph Infrastructure
//! 
//! Infrastructure layer for the Telegraph communication service.
//! This crate contains adapters for external services like email,
//! SMS, push notifications, and event queues.

pub mod communication;
pub mod event;
pub mod repository;

// Re-export commonly used types
pub use communication::*;
pub use event::*;
pub use repository::*; 