//! Use case layer for Telegraph application

pub mod communication;
pub mod event_processing;

// Re-export all use cases
pub use communication::*;
pub use event_processing::*; 