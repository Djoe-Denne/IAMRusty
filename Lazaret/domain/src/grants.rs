//! Call-time grant intersection (pure). Crypto-valid session is not enough.

use std::fmt;
use std::str::FromStr;

use apparatus_contracts::Capability;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::identity::WorkloadIdentity;

/// Origin of a capability call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallOrigin {
    /// Interactive call: end-user membership is part of the intersection.
    Interactive {
        /// End-user id (not an IAM JWT, not a workload claim).
        principal: Uuid,
    },
    /// Background call: skip end-user membership; still require the rest.
    Background,
}

/// Request evaluated against a live Manifesto snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantAuthorizationRequest {
    /// Workload identity from the dedicated session (not grants).
    pub identity: WorkloadIdentity,
    /// Capability name requested at call time.
    pub capability: String,
    /// Capabilities declared by the admitted release.
    ///
    /// Call-time evaluation uses [`BindingGrantSnapshot::declared`], not this field.
    pub declared: Vec<String>,
    /// Interactive vs background.
    pub origin: CallOrigin,
    /// Project the caller claims to operate in.
    pub project_id: Uuid,
}

/// Consent row from Manifesto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityConsent {
    /// Capability name.
    pub capability: String,
    /// `consented` or `revoked`.
    pub status: String,
    /// Revision recorded on the consent row.
    pub grant_revision: i64,
}

/// Principal membership from Manifesto (null when the query was omitted).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalMembership {
    /// Queried user id.
    pub user_id: Uuid,
    /// Active project member (`removed_at IS NULL`).
    pub active: bool,
}

/// Live binding snapshot matching Manifesto JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingGrantSnapshot {
    /// Binding identity (`project_components.id`).
    pub component_id: Uuid,
    /// Project that owns the binding.
    pub project_id: Uuid,
    /// Project status string from Manifesto.
    pub project_status: String,
    /// Component status string from Manifesto.
    pub component_status: String,
    /// Binding source (`legacy` | `managed`).
    pub source: String,
    /// Admitted release digest. Missing → [`GrantDenyReason::ReleaseNotAdmitted`].
    pub digest: Option<String>,
    /// Desired generation (`desired_state` for this slice).
    pub desired_generation: i64,
    /// Observed generation (informational).
    pub observed_generation: i64,
    /// Current grant revision on the binding.
    pub grant_revision: i64,
    /// Capabilities declared for the admitted release (`[]` when omitted).
    #[serde(default)]
    pub declared: Vec<String>,
    /// Consents for this binding.
    pub consents: Vec<CapabilityConsent>,
    /// Present only when Manifesto was queried with `principal`.
    pub principal: Option<PrincipalMembership>,
}

/// Outcome of [`evaluate_grant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantDecision {
    /// Every intersection clause held.
    Allow,
    /// At least one clause failed.
    Deny {
        /// Why the call was refused.
        reason: GrantDenyReason,
    },
}

/// Why a grant was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantDenyReason {
    /// Name is not in the operator policy (`Capability` parse).
    UnknownCapability,
    /// Known but not declared by the release.
    CapabilityNotDeclared,
    /// No consent row for the requested capability.
    ConsentMissing,
    /// Consent status is not `consented`.
    ConsentRevoked,
    /// Consent `grant_revision` differs from the binding.
    ConsentRevisionMismatch,
    /// Workload `grant_revision` differs from the binding.
    IdentityRevisionMismatch,
    /// Workload generation differs from `desired_generation`.
    GenerationMismatch,
    /// Missing or mismatched admitted digest vs workload release.
    ReleaseNotAdmitted,
    /// Project is suspended, archived, or otherwise inoperable.
    ProjectInactive,
    /// Snapshot project/binding does not match the request.
    ProjectMismatch,
    /// Interactive principal missing on the snapshot.
    PrincipalMissing,
    /// Interactive principal inactive or not the requested user.
    PrincipalInactive,
    /// Component status is not `active`.
    ComponentInactive,
}

impl fmt::Display for GrantDenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownCapability => "unknown_capability",
            Self::CapabilityNotDeclared => "capability_not_declared",
            Self::ConsentMissing => "consent_missing",
            Self::ConsentRevoked => "consent_revoked",
            Self::ConsentRevisionMismatch => "consent_revision_mismatch",
            Self::IdentityRevisionMismatch => "identity_revision_mismatch",
            Self::GenerationMismatch => "generation_mismatch",
            Self::ReleaseNotAdmitted => "release_not_admitted",
            Self::ProjectInactive => "project_inactive",
            Self::ProjectMismatch => "project_mismatch",
            Self::PrincipalMissing => "principal_missing",
            Self::PrincipalInactive => "principal_inactive",
            Self::ComponentInactive => "component_inactive",
        })
    }
}

/// Failure to consult Manifesto for a live snapshot.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GrantFetchError {
    /// Binding snapshot was not found.
    #[error("binding grant snapshot not found")]
    NotFound,
    /// Transport or decode failure (fail closed).
    #[error("binding grant snapshot consult failed: {0}")]
    Transport(String),
}

/// Port: fetch a live binding grant snapshot from Manifesto.
#[async_trait::async_trait]
pub trait BindingGrantSnapshotPort: Send + Sync {
    /// Fetch the snapshot. `principal` is set for interactive calls only.
    ///
    /// # Errors
    ///
    /// Returns [`GrantFetchError`] when the snapshot is missing or the consult fails.
    async fn fetch(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshot, GrantFetchError>;
}

/// Evaluate the call-time intersection against a live snapshot.
#[must_use]
pub fn evaluate_grant(
    req: &GrantAuthorizationRequest,
    snapshot: &BindingGrantSnapshot,
) -> GrantDecision {
    if Capability::from_str(&req.capability).is_err() {
        return deny(GrantDenyReason::UnknownCapability);
    }
    if !snapshot.declared.iter().any(|name| name == &req.capability) {
        return deny(GrantDenyReason::CapabilityNotDeclared);
    }
    if snapshot.project_id != req.project_id || snapshot.component_id != req.identity.binding {
        return deny(GrantDenyReason::ProjectMismatch);
    }
    if !project_is_operable(&snapshot.project_status) {
        return deny(GrantDenyReason::ProjectInactive);
    }
    if !component_is_active(&snapshot.component_status) {
        return deny(GrantDenyReason::ComponentInactive);
    }
    match snapshot.digest.as_deref() {
        Some(digest) if digest == req.identity.release => {}
        _ => return deny(GrantDenyReason::ReleaseNotAdmitted),
    }
    if req.identity.generation != snapshot.desired_generation {
        return deny(GrantDenyReason::GenerationMismatch);
    }
    if req.identity.grant_revision != snapshot.grant_revision {
        return deny(GrantDenyReason::IdentityRevisionMismatch);
    }
    if let Some(reason) = consent_denial(req, snapshot) {
        return deny(reason);
    }
    if let Some(reason) = principal_denial(req, snapshot) {
        return deny(reason);
    }
    GrantDecision::Allow
}

const fn deny(reason: GrantDenyReason) -> GrantDecision {
    GrantDecision::Deny { reason }
}

fn project_is_operable(status: &str) -> bool {
    status.eq_ignore_ascii_case("active") || status.eq_ignore_ascii_case("draft")
}

fn component_is_active(status: &str) -> bool {
    status.eq_ignore_ascii_case("active")
}

fn consent_denial(
    req: &GrantAuthorizationRequest,
    snapshot: &BindingGrantSnapshot,
) -> Option<GrantDenyReason> {
    let Some(consent) = snapshot
        .consents
        .iter()
        .find(|row| row.capability == req.capability)
    else {
        return Some(GrantDenyReason::ConsentMissing);
    };
    if !consent.status.eq_ignore_ascii_case("consented") {
        return Some(GrantDenyReason::ConsentRevoked);
    }
    if consent.grant_revision != snapshot.grant_revision {
        return Some(GrantDenyReason::ConsentRevisionMismatch);
    }
    None
}

fn principal_denial(
    req: &GrantAuthorizationRequest,
    snapshot: &BindingGrantSnapshot,
) -> Option<GrantDenyReason> {
    match req.origin {
        CallOrigin::Background => None,
        CallOrigin::Interactive { principal } => match &snapshot.principal {
            None => Some(GrantDenyReason::PrincipalMissing),
            Some(membership) if membership.user_id == principal && membership.active => None,
            Some(_) => Some(GrantDenyReason::PrincipalInactive),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(release: &str, generation: i64, grant_revision: i64) -> WorkloadIdentity {
        WorkloadIdentity::try_new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            release.to_owned(),
            generation,
            grant_revision,
        )
        .expect("identity")
    }

    fn snapshot_for(req: &GrantAuthorizationRequest, status: &str) -> BindingGrantSnapshot {
        let principal = match req.origin {
            CallOrigin::Interactive { principal } => Some(PrincipalMembership {
                user_id: principal,
                active: true,
            }),
            CallOrigin::Background => None,
        };
        BindingGrantSnapshot {
            component_id: req.identity.binding,
            project_id: req.project_id,
            project_status: status.to_owned(),
            component_status: "active".to_owned(),
            source: "managed".to_owned(),
            digest: Some(req.identity.release.clone()),
            desired_generation: req.identity.generation,
            observed_generation: 0,
            grant_revision: req.identity.grant_revision,
            declared: req.declared.clone(),
            consents: vec![CapabilityConsent {
                capability: req.capability.clone(),
                status: "consented".to_owned(),
                grant_revision: snapshot_revision(req),
            }],
            principal,
        }
    }

    fn snapshot_revision(req: &GrantAuthorizationRequest) -> i64 {
        req.identity.grant_revision
    }

    fn allow_request(origin: CallOrigin) -> GrantAuthorizationRequest {
        let identity = identity("release-1", 1, 0);
        GrantAuthorizationRequest {
            identity,
            capability: "project.read".to_owned(),
            declared: vec!["project.read".to_owned()],
            origin,
            project_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn allow_interactive_when_all_clauses_hold() {
        let req = allow_request(CallOrigin::Interactive {
            principal: Uuid::new_v4(),
        });
        let snapshot = snapshot_for(&req, "active");
        assert_eq!(evaluate_grant(&req, &snapshot), GrantDecision::Allow);
    }

    #[test]
    fn allow_draft_project_as_operable() {
        let req = allow_request(CallOrigin::Background);
        let snapshot = snapshot_for(&req, "draft");
        assert_eq!(evaluate_grant(&req, &snapshot), GrantDecision::Allow);
    }

    #[test]
    fn deny_unknown_capability() {
        let mut req = allow_request(CallOrigin::Background);
        req.capability = "network.egress".to_owned();
        let mut snapshot = snapshot_for(&req, "active");
        snapshot.declared.push("network.egress".to_owned());
        assert_eq!(
            evaluate_grant(&req, &snapshot),
            deny(GrantDenyReason::UnknownCapability)
        );
    }

    #[test]
    fn deny_not_declared() {
        let req = allow_request(CallOrigin::Background);
        let mut snapshot = snapshot_for(&req, "active");
        snapshot.declared.clear();
        assert_eq!(
            evaluate_grant(&req, &snapshot),
            deny(GrantDenyReason::CapabilityNotDeclared)
        );
    }

    #[test]
    fn declared_comes_from_snapshot_not_request() {
        let mut req = allow_request(CallOrigin::Background);
        let mut snapshot = snapshot_for(&req, "active");
        req.declared.clear();
        snapshot.declared = vec!["project.read".to_owned()];
        assert_eq!(evaluate_grant(&req, &snapshot), GrantDecision::Allow);
    }

    #[test]
    fn deny_disabled_component() {
        let req = allow_request(CallOrigin::Background);
        let mut snapshot = snapshot_for(&req, "active");
        snapshot.component_status = "disabled".to_owned();
        assert_eq!(
            evaluate_grant(&req, &snapshot),
            deny(GrantDenyReason::ComponentInactive)
        );
    }

    #[test]
    fn deny_pending_component() {
        let req = allow_request(CallOrigin::Background);
        let mut snapshot = snapshot_for(&req, "active");
        snapshot.component_status = "pending".to_owned();
        assert_eq!(
            evaluate_grant(&req, &snapshot),
            deny(GrantDenyReason::ComponentInactive)
        );
    }

    #[test]
    fn allow_active_component_case_insensitive() {
        let req = allow_request(CallOrigin::Background);
        let mut snapshot = snapshot_for(&req, "ACTIVE");
        snapshot.component_status = "Active".to_owned();
        assert_eq!(evaluate_grant(&req, &snapshot), GrantDecision::Allow);
    }

    #[test]
    fn allow_consent_status_case_insensitive() {
        let req = allow_request(CallOrigin::Background);
        let mut snapshot = snapshot_for(&req, "active");
        snapshot.consents[0].status = "Consented".to_owned();
        assert_eq!(evaluate_grant(&req, &snapshot), GrantDecision::Allow);
    }

    #[test]
    fn deny_reason_display_is_stable_snake_case() {
        let cases = [
            (GrantDenyReason::UnknownCapability, "unknown_capability"),
            (
                GrantDenyReason::CapabilityNotDeclared,
                "capability_not_declared",
            ),
            (GrantDenyReason::ConsentMissing, "consent_missing"),
            (GrantDenyReason::ConsentRevoked, "consent_revoked"),
            (
                GrantDenyReason::ConsentRevisionMismatch,
                "consent_revision_mismatch",
            ),
            (
                GrantDenyReason::IdentityRevisionMismatch,
                "identity_revision_mismatch",
            ),
            (GrantDenyReason::GenerationMismatch, "generation_mismatch"),
            (GrantDenyReason::ReleaseNotAdmitted, "release_not_admitted"),
            (GrantDenyReason::ProjectInactive, "project_inactive"),
            (GrantDenyReason::ProjectMismatch, "project_mismatch"),
            (GrantDenyReason::PrincipalMissing, "principal_missing"),
            (GrantDenyReason::PrincipalInactive, "principal_inactive"),
            (GrantDenyReason::ComponentInactive, "component_inactive"),
        ];
        for (reason, expected) in cases {
            assert_eq!(reason.to_string(), expected);
            assert_ne!(reason.to_string(), format!("{reason:?}"));
        }
    }

    #[test]
    fn deny_revoked_consent() {
        let req = allow_request(CallOrigin::Background);
        let mut snapshot = snapshot_for(&req, "active");
        snapshot.consents[0].status = "revoked".to_owned();
        assert_eq!(
            evaluate_grant(&req, &snapshot),
            deny(GrantDenyReason::ConsentRevoked)
        );
    }

    #[test]
    fn deny_identity_revision_mismatch() {
        let mut req = allow_request(CallOrigin::Background);
        req.identity.grant_revision = 3;
        let mut snapshot = snapshot_for(&req, "active");
        snapshot.grant_revision = 0;
        snapshot.consents[0].grant_revision = 0;
        assert_eq!(
            evaluate_grant(&req, &snapshot),
            deny(GrantDenyReason::IdentityRevisionMismatch)
        );
    }

    #[test]
    fn background_skips_principal() {
        let req = allow_request(CallOrigin::Background);
        let snapshot = snapshot_for(&req, "active");
        assert!(snapshot.principal.is_none());
        assert_eq!(evaluate_grant(&req, &snapshot), GrantDecision::Allow);
    }

    #[test]
    fn deny_when_digest_missing() {
        let req = allow_request(CallOrigin::Background);
        let mut snapshot = snapshot_for(&req, "active");
        snapshot.digest = None;
        assert_eq!(
            evaluate_grant(&req, &snapshot),
            deny(GrantDenyReason::ReleaseNotAdmitted)
        );
    }
}
