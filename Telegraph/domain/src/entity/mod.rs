//! Domain entities for Telegraph communication service

pub mod communication;
pub mod template;
pub mod delivery;

// Re-export all entities
pub use communication::*;
pub use template::*;
pub use delivery::*; 