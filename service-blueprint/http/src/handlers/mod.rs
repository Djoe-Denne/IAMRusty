// Export all handler modules
pub mod entity;
pub mod health;

// Re-export commonly used handlers
pub use entity::*;
pub use health::*; 