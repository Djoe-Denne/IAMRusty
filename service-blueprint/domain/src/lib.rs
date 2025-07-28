//! Domain layer: core business logic free of implementation details
//! This crate contains pure business logic with no external dependencies

pub mod entity;
pub mod error;
pub mod port;
pub mod service;

// Re-export commonly used types
pub use error::DomainError;

// Re-export entities
pub use entity::*;

// Re-export ports
pub use port::*;

// Re-export services
pub use service::*; 