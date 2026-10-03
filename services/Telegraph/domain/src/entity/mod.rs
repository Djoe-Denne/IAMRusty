//! Domain entities for Telegraph communication service

pub mod communication;
pub mod delivery;
pub mod template;

// Re-export all entities
pub use communication::*;
pub use delivery::*;
pub use template::*;
