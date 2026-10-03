//! Application layer: use cases and business logic
//!
//! This layer coordinates the interactions between domain entities, applying business rules,
//! and interacting with external systems through ports.

pub mod auth;
pub mod command;
pub mod dto;
pub mod usecase;

// Re-export configuration for backward compatibility
pub use iam_configuration::*;
