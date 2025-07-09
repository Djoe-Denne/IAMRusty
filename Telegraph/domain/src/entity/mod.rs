//! Domain entities for Telegraph communication service

pub mod message;
pub mod recipient;
pub mod template;
pub mod delivery;

// Re-export all entities
pub use message::*;
pub use template::*;
pub use delivery::*; 