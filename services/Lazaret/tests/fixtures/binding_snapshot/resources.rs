//! Wire types for Manifesto binding grant snapshots.

use lazaret_domain::{BindingGrantSnapshot, CapabilityConsent, PrincipalMembership};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Body returned by the Manifesto binding grant snapshot GET.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingGrantSnapshotBody {
    pub component_id: Uuid,
    pub project_id: Uuid,
    pub project_status: String,
    pub component_status: String,
    pub source: String,
    pub digest: Option<String>,
    pub desired_generation: i64,
    pub observed_generation: i64,
    pub grant_revision: i64,
    #[serde(default)]
    pub declared: Vec<String>,
    pub consents: Vec<CapabilityConsent>,
    pub principal: Option<PrincipalMembership>,
}

impl From<BindingGrantSnapshot> for BindingGrantSnapshotBody {
    fn from(value: BindingGrantSnapshot) -> Self {
        Self {
            component_id: value.component_id,
            project_id: value.project_id,
            project_status: value.project_status,
            component_status: value.component_status,
            source: value.source,
            digest: value.digest,
            desired_generation: value.desired_generation,
            observed_generation: value.observed_generation,
            grant_revision: value.grant_revision,
            declared: value.declared,
            consents: value.consents,
            principal: value.principal,
        }
    }
}
