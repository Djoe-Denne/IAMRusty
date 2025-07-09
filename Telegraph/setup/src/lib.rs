//! # Telegraph Setup
//! 
//! Setup and dependency injection for the Telegraph communication service.
//! This crate contains application configuration and service initialization.

pub mod app;
pub mod config;

// Re-export commonly used types
pub use app::*;
pub use config::*; 