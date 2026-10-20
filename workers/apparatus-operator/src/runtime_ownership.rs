//! Ownership guards for parent Serving Services, never their children.
//!
//! Labels alone are not proof of ownership. A mutation requires the UID already
//! recorded by the authoritative writer and a fresh resourceVersion from GET.

use apparatus_contracts::runtime::{RuntimeBindingSnapshotV1, RuntimeErrorCodeV1};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Server preconditions used by replace/delete, never force-apply/adoption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingMutationPreconditions {
    /// Previously recorded, exact Kubernetes UID.
    pub uid: String,
    /// Version from the same freshly verified GET.
    pub resource_version: String,
}

/// Stable parent name is per project/binding, not per shared release digest.
#[must_use]
pub fn serving_name(snapshot: &RuntimeBindingSnapshotV1) -> String {
    let mut hash = Sha256::new();
    hash.update(b"urn:aiforall:apparatus:serving-name:v1\0");
    hash.update(snapshot.project_id.as_bytes());
    hash.update(snapshot.binding_id.as_bytes());
    // Leave room for a deterministic Revision suffix under the DNS-label limit.
    // Ownership checks below still recoup both complete UUIDs, never this name alone.
    format!("ar-{}", &hex::encode(hash.finalize())[..32])
}

/// Validate a live object against authoritative ownership before mutation.
///
/// # Errors
///
/// Refuses missing/mismatched UID, identity, type, version or metadata. Unknown
/// objects are not adopted even when their names/labels happen to match.
pub fn mutation_preconditions(
    object: &Value,
    snapshot: &RuntimeBindingSnapshotV1,
    recorded_service_uid: &str,
) -> Result<ServingMutationPreconditions, RuntimeErrorCodeV1> {
    let conflict = RuntimeErrorCodeV1::RuntimeFenceConflict;
    if recorded_service_uid.is_empty()
        || object["apiVersion"] != "serving.knative.dev/v1"
        || object["kind"] != "Service"
        || object["metadata"]["uid"] != recorded_service_uid
        || object["metadata"]["name"] != serving_name(snapshot)
        || object["metadata"]["namespace"] != "apparatus-plugins"
        || object["metadata"]["labels"]["apparatus.aiforall.dev/project"]
            != snapshot.project_id.to_string()
        || object["metadata"]["labels"]["apparatus.aiforall.dev/binding"]
            != snapshot.binding_id.to_string()
    {
        return Err(conflict);
    }
    let resource_version = object["metadata"]["resourceVersion"]
        .as_str()
        .filter(|version| !version.is_empty())
        .ok_or(conflict)?;
    Ok(ServingMutationPreconditions {
        uid: recorded_service_uid.to_owned(),
        resource_version: resource_version.to_owned(),
    })
}
