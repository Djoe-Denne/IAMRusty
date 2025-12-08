//! Manifesto Domain Layer
//!
//! Contains business entities, value objects, domain services, and business rules.

pub mod entity;
pub mod error;
pub mod port;
pub mod service;
pub mod value_objects;

// Re-export commonly used types
pub use entity::*;
pub use port::*;
pub use service::*;
pub use value_objects::*;

// Re-export DomainError from rustycog-core
pub use rustycog_core::error::DomainError;

