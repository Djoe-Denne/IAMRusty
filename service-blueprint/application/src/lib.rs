//! Application layer: use cases and business logic
//!
//! This layer coordinates the interactions between domain entities, applying business rules,
//! and interacting with external systems through ports.

pub mod command;
pub mod dto;
pub mod usecase;

// Re-export commonly used types
pub use command::*;
pub use dto::*;
pub use usecase::*;

// Re-export domain types for convenience
pub use {{SERVICE_NAME}}_domain::*; 