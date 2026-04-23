//! Permission checker client for RustyCog microservices
//!
//! This crate exposes the `PermissionChecker` trait plus the OpenFGA-backed
//! implementation used by every RustyCog service to ask the centralized
//! authorization engine whether a subject can perform an action on a resource.
//!
//! The trait is engine-neutral: `OpenFgaPermissionChecker` is the production
//! implementation, `InMemoryPermissionChecker` is provided for unit tests, and
//! `CachedPermissionChecker` decorates any underlying checker with a short-TTL
//! LRU cache.

use async_trait::async_trait;
use rustycog_core::error::DomainError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod checker;

pub use checker::{
    CachedPermissionChecker, InMemoryPermissionChecker, MetricsPermissionChecker,
    OpenFgaClientConfig, OpenFgaPermissionChecker,
};

/// Permission verbs recognized by the platform.
///
/// Each variant maps to a relation on every object type defined in the OpenFGA
/// authorization model (`openfga/model.fga`). The mapping is intentionally flat
/// so client services can describe authorization requirements uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Admin,
    Owner,
}

impl Permission {
    /// Every supported permission, in ascending order of privilege.
    pub fn all() -> Vec<Permission> {
        vec![
            Permission::Read,
            Permission::Write,
            Permission::Admin,
            Permission::Owner,
        ]
    }

    /// Human-readable permission name (matches the old Casbin action names).
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::Read => "read",
            Permission::Write => "write",
            Permission::Admin => "admin",
            Permission::Owner => "owner",
        }
    }

    /// OpenFGA relation name for this permission.
    ///
    /// Every object type in the platform model exposes the same four derived
    /// relations (`read`, `write`, `administer`, `own`) so the checker only
    /// needs the `Permission` plus an object type to issue a `Check` call.
    pub fn relation(&self) -> &'static str {
        match self {
            Permission::Read => "read",
            Permission::Write => "write",
            Permission::Admin => "administer",
            Permission::Owner => "own",
        }
    }

    /// Parse a permission from its string representation.
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

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for Permission {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap()
    }
}

/// Identifier for a resource that a permission applies to.
///
/// Always a UUID; the permission middleware only binds UUID-shaped path
/// segments into a `ResourceId`, so service routes must use UUID path
/// parameters when they want a resource-scoped guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(Uuid);

impl From<Uuid> for ResourceId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<ResourceId> for Uuid {
    fn from(id: ResourceId) -> Self {
        id.0
    }
}

impl ResourceId {
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn id(&self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Discriminant for [`Subject`] — `User` for an authenticated UUID-bearing
/// caller, `Wildcard` for the anonymous "any user" subject (`user:*` on the
/// OpenFGA wire). Anonymous routes can hand a `Subject::wildcard()` to the
/// checker instead of failing closed before the call, so a public-read
/// tuple like `project:{id}#viewer@user:*` (written by `sentinel-sync` for
/// public projects) is honored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubjectKind {
    User,
    Wildcard,
}

fn default_subject_kind() -> SubjectKind {
    SubjectKind::User
}

/// Caller identified on the request — either an authenticated user
/// ([`Subject::new`]) or the wildcard "any user" subject
/// ([`Subject::wildcard`]) used by anonymous-callable routes.
///
/// On the OpenFGA wire `Subject::Display` renders as `user:{uuid}` for the
/// `User` kind and `user:*` for the `Wildcard` kind. The wildcard form is
/// only meaningful when the OpenFGA model declares the relevant relation
/// with a `[user, user:*]` type restriction (e.g. `project.viewer` in
/// `openfga/model.fga`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Subject {
    pub user_id: Uuid,
    /// Defaults to `SubjectKind::User` so payloads serialized before this
    /// field existed deserialize unchanged.
    #[serde(default = "default_subject_kind")]
    pub kind: SubjectKind,
}

impl Subject {
    /// Build a subject for a concrete authenticated user.
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            kind: SubjectKind::User,
        }
    }

    /// Build the wildcard "any user" subject. Renders as `user:*` on the
    /// OpenFGA wire. Use from `optional_permission_middleware` when the
    /// request carries no JWT, so public-read tuples
    /// (`...#viewer@user:*`) can be honored by the centralized checker.
    pub fn wildcard() -> Self {
        Self {
            user_id: Uuid::nil(),
            kind: SubjectKind::Wildcard,
        }
    }

    /// Returns `true` when the subject is the wildcard variant.
    pub fn is_wildcard(&self) -> bool {
        matches!(self.kind, SubjectKind::Wildcard)
    }
}

impl std::fmt::Display for Subject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            SubjectKind::Wildcard => write!(f, "user:*"),
            SubjectKind::User => write!(f, "user:{}", self.user_id),
        }
    }
}

/// Reference to a specific resource instance: a typed object id.
///
/// `object_type` must match an OpenFGA type defined in `openfga/model.fga`
/// (for example `"organization"`, `"project"`, `"component"`, `"notification"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceRef {
    pub object_type: &'static str,
    pub object_id: Uuid,
}

impl ResourceRef {
    pub fn new(object_type: &'static str, object_id: Uuid) -> Self {
        Self {
            object_type,
            object_id,
        }
    }

    /// `type:id` rendering used by OpenFGA tuple encoding.
    pub fn as_object_string(&self) -> String {
        format!("{}:{}", self.object_type, self.object_id)
    }
}

impl std::fmt::Display for ResourceRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_object_string())
    }
}

/// Engine-neutral permission checker used by HTTP middleware and domain code.
///
/// The production implementation (`OpenFgaPermissionChecker`) issues a
/// `Check` call against the OpenFGA server; tests use the in-memory
/// implementation. Wrap any checker in `CachedPermissionChecker` to add a
/// short-TTL LRU cache in front of the network call.
#[async_trait]
pub trait PermissionChecker: Send + Sync {
    /// Return `Ok(true)` when `subject` has `action` on `resource`.
    async fn check(
        &self,
        subject: Subject,
        action: Permission,
        resource: ResourceRef,
    ) -> Result<bool, DomainError>;
}

#[cfg(test)]
mod subject_tests {
    use super::*;

    #[test]
    fn user_subject_renders_as_user_uuid() {
        let id = Uuid::parse_str("01010101-0101-0101-0101-010101010101").unwrap();
        let subject = Subject::new(id);
        assert_eq!(subject.to_string(), "user:01010101-0101-0101-0101-010101010101");
        assert!(!subject.is_wildcard());
    }

    #[test]
    fn wildcard_subject_renders_as_user_star() {
        let subject = Subject::wildcard();
        assert_eq!(subject.to_string(), "user:*");
        assert!(subject.is_wildcard());
    }

    #[test]
    fn legacy_subject_payload_without_kind_field_deserializes_as_user() {
        // Payloads serialized before `SubjectKind` existed only carry
        // `user_id`; `#[serde(default)]` on `kind` keeps them readable.
        let json = r#"{"user_id":"01010101-0101-0101-0101-010101010101"}"#;
        let subject: Subject = serde_json::from_str(json).expect("legacy payload should parse");
        assert!(matches!(subject.kind, SubjectKind::User));
        assert_eq!(subject.to_string(), "user:01010101-0101-0101-0101-010101010101");
    }
}
