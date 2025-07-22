//! Port interfaces for Telegraph communication service

pub mod communication;
pub mod event_extractor;
pub mod event_handler;
pub mod repository;
pub mod template;

// Re-export all ports
pub use communication::*;
pub use event_extractor::*;
pub use event_handler::*;
pub use repository::*;
pub use template::*; 