//! Infrastructure layer: implementations of domain ports
//!
//! This crate provides implementations for the interfaces defined in the domain layer,
//! connecting the business logic to the outside world (databases, external services, etc.).

pub mod repository;
pub mod auth;
pub mod token;
pub mod event_adapter;

// Re-export database functionality from rustycog-db
pub use rustycog_db as db;

// Re-export event functionality from rustycog-events for internal use
pub use rustycog_events as rustycog_event; 
