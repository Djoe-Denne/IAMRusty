pub mod login;
pub mod link_provider;
pub mod token;
pub mod user;
pub mod signup;
pub mod password_login;
pub mod verify_email;
pub mod factory;

// Re-export everything from rustycog-command
pub use rustycog_command::*;

// Re-export our factory for building the command registry
pub use factory::CommandRegistryFactory;

// Convenience alias for the new service
pub type ExtensibleCommandService = GenericCommandService; 