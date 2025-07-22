//! Domain services for Telegraph communication service

pub mod communication_factory;
pub mod notification_service;   
pub mod email_service;

// Re-export all services
pub use communication_factory::*; 
pub use notification_service::*; 
pub use email_service::*; 