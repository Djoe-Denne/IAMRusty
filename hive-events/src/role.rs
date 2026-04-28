//! Hive Domain Events
//!
//! This crate contains all domain events for the Hive organization management service.
//! Events are used for inter-service communication, particularly with the Telegraph
//! notification service.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub permission: String,
    pub resource: String,
}

impl Role {
    #[must_use]
    pub const fn new(permission: String, resource: String) -> Self {
        Self {
            permission,
            resource,
        }
    }
}
