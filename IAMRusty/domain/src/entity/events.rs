//! Re-export IAM domain events from the shared iam-events crate
//! 
//! This module provides backward compatibility for the IAMRusty service
//! while using the shared iam-events crate that can be consumed by other services.

// Re-export all events from the shared crate
pub use iam_events::*;

// Backward compatibility aliases
pub use iam_events::IamDomainEvent as DomainEvent;
