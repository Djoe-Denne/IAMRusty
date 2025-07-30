//! # Telegraph Application
//!
//! Application layer for the Telegraph communication service.
//! This crate contains the application use cases, command handlers,
//! and application services that coordinate domain operations.

pub mod command;
pub mod usecase;

// Re-export commonly used types
pub use command::*;
pub use usecase::*;

// Re-export domain types for convenience
pub use telegraph_domain::*;
