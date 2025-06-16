pub mod factory;
pub mod oauth_login;
pub mod password_login;
pub mod provider;
pub mod registration;
pub mod resend_verification_email;
pub mod signup;
pub mod token;
pub mod user;
pub mod verify_email;

// Re-export everything from rustycog-command
pub use rustycog_command::*;

// Re-export our factory for building the command registry
pub use factory::CommandRegistryFactory;

// Convenience alias for the new service
pub type ExtensibleCommandService = GenericCommandService;
