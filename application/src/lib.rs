//! Application layer: use cases and business logic
//!
//! This layer coordinates the interactions between domain entities, applying business rules,
//! and interacting with external systems through ports.

pub mod usecase;
pub mod auth;
pub mod service;
pub mod dto;
pub mod error; 