//! Wire types for the OpenFGA `Check` HTTP endpoint.
//!
//! These mirror the request and response shapes that
//! `rustycog_permission::OpenFgaPermissionChecker` posts to and decodes from
//! `POST {api_url}/stores/{store_id}/check`. The production checker keeps its
//! versions of these types `pub(crate)`, so the fixture re-declares them here
//! for `set_body_json` and for body-aware matchers.

use rustycog_permission::{Permission, ResourceRef, Subject};
use serde::{Deserialize, Serialize};

/// `POST /stores/{store_id}/check` request body, as serialized by
/// `OpenFgaPermissionChecker`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRequestBody {
    pub tuple_key: CheckTupleKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_model_id: Option<String>,
}

/// `tuple_key` field carried inside [`CheckRequestBody`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckTupleKey {
    /// Subject in OpenFGA tuple form, e.g. `user:8d3...`.
    pub user: String,
    /// Relation name (`read`, `write`, `administer`, `own`).
    pub relation: String,
    /// Object in OpenFGA tuple form, e.g. `organization:9c1...`.
    pub object: String,
}

impl CheckTupleKey {
    /// Build a tuple key from the high-level types `OpenFgaPermissionChecker`
    /// uses, applying the same `Permission::relation()` mapping the production
    /// checker applies (`Admin -> "administer"`, `Owner -> "own"`).
    pub fn from_subject_action_resource(
        subject: Subject,
        action: Permission,
        resource: ResourceRef,
    ) -> Self {
        Self {
            user: subject.to_string(),
            relation: action.relation().to_string(),
            object: resource.as_object_string(),
        }
    }
}

/// Response body returned by OpenFGA's `Check` endpoint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CheckResponseBody {
    #[serde(default)]
    pub allowed: bool,
}

impl CheckResponseBody {
    pub fn allow() -> Self {
        Self { allowed: true }
    }

    pub fn deny() -> Self {
        Self { allowed: false }
    }
}
