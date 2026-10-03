//! Binding grant snapshot DTOs (privileged caller). Domain language only.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Consent row as persisted for a binding (`consented` | `revoked`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityConsentSnapshot {
    /// Capability name stored on the consent row.
    pub capability: String,
    /// Consent status (`consented` or `revoked`).
    pub status: String,
    /// Grant revision recorded on the consent row.
    pub grant_revision: i64,
}

/// End-user membership used at call time when `principal` is queried.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrincipalMembershipSnapshot {
    /// Queried user id (not the privileged caller's JWT subject).
    pub user_id: Uuid,
    /// `true` when a `project_members` row exists with `removed_at IS NULL`.
    pub active: bool,
}

/// Privileged read of binding, consents, and optional principal membership.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BindingGrantSnapshotResponse {
    /// Binding identity (`project_components.id`).
    pub component_id: Uuid,
    /// Project that owns the binding.
    pub project_id: Uuid,
    /// Project status (`draft`, `active`, `suspended`, `archived`).
    pub project_status: String,
    /// Component status.
    pub component_status: String,
    /// Binding source (`legacy` | `managed`).
    pub source: String,
    /// Admitted artifact digest, if any.
    pub digest: Option<String>,
    /// Desired generation for this binding.
    pub desired_generation: i64,
    /// Observed generation reported for this binding.
    pub observed_generation: i64,
    /// Current grant revision on the binding.
    pub grant_revision: i64,
    /// Capabilities declared for this binding (empty when unknown).
    #[serde(default)]
    pub declared: Vec<String>,
    /// Consents for this binding (may be empty).
    pub consents: Vec<CapabilityConsentSnapshot>,
    /// Present only when the `principal` query is set.
    pub principal: Option<PrincipalMembershipSnapshot>,
}

/// Write or revoke a consented capability on a binding.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq, Eq)]
pub struct UpsertBindingConsentRequest {
    /// Capability name (`project.read`, `storage.kv.read`, …).
    #[validate(length(min = 1, max = 100))]
    pub capability: String,
    /// `consented` or `revoked`.
    #[validate(length(min = 1, max = 32))]
    pub status: String,
}
