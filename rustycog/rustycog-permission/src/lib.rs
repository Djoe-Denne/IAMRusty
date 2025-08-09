//! Permission management and authorization engine for RustyCog microservices
//!
//! This crate provides the core permission abstractions and implementations
//! used across RustyCog services for authorization and access control.

use async_trait::async_trait;
use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod casbin;
pub mod adapter;


// PermissionContext was previously used to pass state to a custom adapter.
// It's no longer needed since the engine directly injects policies.

// Main types are exported directly

/// Permission levels available in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Admin,
    Owner,
}

impl Permission {
    /// Get all available permission levels
    pub fn all() -> Vec<Permission> {
        vec![
            Permission::Read,
            Permission::Write,
            Permission::Admin,
            Permission::Owner,
        ]
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::Read => "read",
            Permission::Write => "write",
            Permission::Admin => "admin",
            Permission::Owner => "owner",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "read" => Ok(Permission::Read),
            "write" => Ok(Permission::Write),
            "admin" => Ok(Permission::Admin),
            "owner" => Ok(Permission::Owner),
            _ => Err(DomainError::invalid_input(&format!(
                "Invalid permission level: {}",
                s
            ))),
        }
    }
}

/// Permission engine trait for checking authorization
#[async_trait]
pub trait PermissionEngine: Send + Sync {
    /// Check if user has the target permission based on their current roles
    async fn has_permission(
        &self,
        user_id: Uuid,
        resource_ids: Vec<Uuid>,
        target_permission: Permission,
        settings: serde_json::Value,
    ) -> Result<bool, DomainError>;
}

// Re-export the permissions fetcher trait for consumers to implement
pub use adapter::permission_fetcher::PermissionsFetch;

// Types are available directly from the crate root