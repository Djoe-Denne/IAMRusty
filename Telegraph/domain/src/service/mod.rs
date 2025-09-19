//! Domain services for Telegraph communication service

pub mod communication_factory;
pub mod email_service;
pub mod notification_service;
pub mod permission_service;

// Re-export all services
pub use communication_factory::*;
pub use email_service::*;
pub use notification_service::*;
pub use permission_service::ResourcePermissionFetcher;
