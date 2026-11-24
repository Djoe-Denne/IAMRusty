//! # IAM Events
//!
//! Shared domain events for IAM (Identity and Access Management) microservices.
//! This crate provides common event types that can be published by IAM services
//! and consumed by other services like Telegraph (communication service).

pub mod events;
mod profile_email_verification;

// Re-export for convenience
pub use events::*;
pub use profile_email_verification::{
    ProfileEmailVerificationRequested, PROFILE_EMAIL_VERIFICATION_REQUESTED,
};

// Re-export rustycog-events for consumers
pub use rustycog::events::*;
