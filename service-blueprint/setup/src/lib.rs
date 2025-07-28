//! Setup and dependency injection for the {{SERVICE_NAME}} service.
//! This crate contains application configuration and service initialization.

pub mod app;
pub mod config;

// Re-export commonly used types
pub use app::*;
pub use config::*; 