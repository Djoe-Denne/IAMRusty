// Export all repository modules
pub mod postgres;
pub mod example_entity_repository;

// Re-export commonly used repositories
pub use example_entity_repository::*;
pub use postgres::*; 