//! Read-only ownership attestation. Pod labels/name/digest alone never establish
//! ownership, and Ready is deliberately not required for enrollment bootstrap.

use apparatus_contracts::runtime::{
    RuntimeBindingSnapshotV1, RuntimeErrorCodeV1, RuntimeInstanceStateV1, RuntimePodIdentityV1,
    RuntimeUnixMsV1,
};

/// Attempt-local identity history. Not serializable or recoverable from labels.
#[derive(Default)]
pub(crate) struct PreparationChainUids {
    configuration: Option<String>,
    revision: Option<String>,
}

fn bind_chain_uid(previous: &mut Option<String>, current: &str) -> Result<(), RuntimeErrorCodeV1> {
    if previous.as_deref().is_some_and(|old| old != current) {
        return Err(RuntimeErrorCodeV1::RuntimeFenceConflict);
    }
    *previous = Some(current.to_owned());
    Ok(())
}
use k8s_openapi::{
    api::{
        apps::v1::{Deployment, ReplicaSet},
        core::v1::{Pod, ServiceAccount},
    },
    apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference},
};
use kube::{
    api::ListParams,
    core::{ApiResource, DynamicObject, GroupVersionKind},
    Api, Client,
};

/// Locally observed proof, not deserializable from a publisher or HTTP request.
pub struct ObservedRuntimePod {
    identity: RuntimePodIdentityV1,
    state: RuntimeInstanceStateV1,
    observed_at: RuntimeUnixMsV1,
}
impl ObservedRuntimePod {
    /// Exact Pod/Revision/ServiceAccount UID mapping from verified API reads.
    #[must_use]
    pub fn identity(&self) -> &RuntimePodIdentityV1 {
        &self.identity
    }
    /// Pending/running Pods may be current without readiness/enrollment.
    #[must_use]
    pub const fn state(&self) -> RuntimeInstanceStateV1 {
        self.state
    }
    /// Capture time, still bounded/validated by the writer DB on registration.
    #[must_use]
    pub const fn observed_at(&self) -> RuntimeUnixMsV1 {
        self.observed_at
    }

    /// Resolve a durable mapping by Pod UID before allocating a fresh UUID. An
    /// operator restart therefore reuses the authoritative identity, never a name
    /// or digest-derived identity. No retry with a new ID after an ambiguous PUT.
    ///
    /// # Errors
    /// Rejects missing protocol pages, mismatched mappings, revoked identities,
    /// stale business context and lost lease/fence.
    pub async fn register(
        &self,
        control: &dyn apparatus_contracts::runtime::RuntimeControlPort,
        caller: &apparatus_contracts::runtime::RuntimeControlCallerV1,
        observed_binding: &RuntimeBindingSnapshotV1,
        lease: &apparatus_contracts::runtime::RuntimeWriterLeaseV1,
    ) -> Result<apparatus_contracts::runtime::RuntimeWriteAckV1, RuntimeErrorCodeV1> {
        let mut pending = None;
        self.register_with_pending(control, caller, observed_binding, lease, &mut pending)
            .await
    }

    /// Controller-owned uncertain attempt ID survives later scans. Re-reading
    /// ByPodUid never allocates a second ID after a timed-out/ambiguous write.
    pub(crate) async fn register_with_pending(
        &self,
        control: &dyn apparatus_contracts::runtime::RuntimeControlPort,
        caller: &apparatus_contracts::runtime::RuntimeControlCallerV1,
        observed_binding: &RuntimeBindingSnapshotV1,
        lease: &apparatus_contracts::runtime::RuntimeWriterLeaseV1,
        pending: &mut Option<uuid::Uuid>,
    ) -> Result<apparatus_contracts::runtime::RuntimeWriteAckV1, RuntimeErrorCodeV1> {
        use apparatus_contracts::runtime::{
            RuntimeInstanceKindV1, RuntimeInstanceRegistrationV1, RuntimeInstanceSelectorV1,
            RuntimeWriteGuardV1, RuntimeWriterLeaseRequestV1,
        };
        let selector = RuntimeInstanceSelectorV1::ByPodUid {
            pod_uid: self.identity.pod_uid.clone(),
        };
        let current = control
            .get_binding(
                caller,
                observed_binding.project_id,
                observed_binding.binding_id,
                &selector,
            )
            .await
            .map_err(|error| error.code)?;
        if current.cas.generation != observed_binding.cas.generation
            || current.cas.grant_revision != observed_binding.cas.grant_revision
            || current.cas.revocation_epoch != observed_binding.cas.revocation_epoch
            || current.release != observed_binding.release
        {
            return Err(RuntimeErrorCodeV1::RuntimeStaleState);
        }
        let instances = current
            .instances
            .as_ref()
            .ok_or(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError)?;
        if instances.items.len() > 1 || instances.next_cursor.is_some() {
            return Err(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError);
        }
        let instance_id = match instances.items.as_slice() {
            [] => *pending.get_or_insert_with(uuid::Uuid::new_v4),
            [known] => {
                if pending.is_some_and(|id| id != known.instance_id) {
                    return Err(RuntimeErrorCodeV1::RuntimeIdReused);
                }
                if known.project_id != current.project_id
                    || known.binding_id != current.binding_id
                    || known.kind != RuntimeInstanceKindV1::Pod
                    || known.pod.as_ref() != Some(&self.identity)
                    || known.release != current.release
                {
                    return Err(RuntimeErrorCodeV1::RuntimeInvalidProof);
                }
                if known.state != RuntimeInstanceStateV1::Current
                    && self.state == RuntimeInstanceStateV1::Current
                {
                    return Err(RuntimeErrorCodeV1::RuntimeIdentityRevoked);
                }
                known.instance_id
            }
            _ => return Err(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError),
        };
        let renewed = control
            .writer_lease(
                caller,
                current.project_id,
                current.binding_id,
                &RuntimeWriterLeaseRequestV1 {
                    schema_version: 1,
                    holder_id: lease.proof.holder_id,
                    expected: current.cas.clone(),
                    renew: Some(lease.proof.clone()),
                },
            )
            .await
            .map_err(|error| error.code)?;
        control
            .register_instance(
                caller,
                current.project_id,
                current.binding_id,
                instance_id,
                &RuntimeInstanceRegistrationV1 {
                    schema_version: 1,
                    project_id: current.project_id,
                    binding_id: current.binding_id,
                    instance_id,
                    guard: RuntimeWriteGuardV1 {
                        expected: renewed.cas,
                        lease: renewed.proof,
                    },
                    state: self.state,
                    pod_uid: self.identity.pod_uid.clone(),
                    revision_uid: self.identity.revision_uid.clone(),
                    namespace: self.identity.namespace.clone(),
                    service_account: self.identity.service_account.clone(),
                    service_account_uid: self.identity.service_account_uid.clone(),
                    release: current.release,
                    observed_at_unix_ms: self.observed_at,
                },
            )
            .await
            .map_err(|error| error.code)
    }
}

/// Bounded API page. A continuation is not evidence that unlisted Pods vanished.
pub struct ObservedRuntimePodPage {
    /// Successfully checked mappings, at most 200 per request.
    pub items: Vec<ObservedRuntimePod>,
    /// Pass back unchanged to continue this API inventory.
    pub next_continue: Option<String>,
}

/// Operator-only read APIs. This observer never creates/patches/deletes children.
pub struct RuntimePodObserver {
    parents: Api<DynamicObject>,
    configurations: Api<DynamicObject>,
    revisions: Api<DynamicObject>,
    deployments: Api<Deployment>,
    replica_sets: Api<ReplicaSet>,
    pods: Api<Pod>,
    accounts: Api<ServiceAccount>,
}

impl RuntimePodObserver {
    /// Capture only complete real metadata, independent of Pod readiness. Missing
    /// fields/desired Revision are incomplete; identity substitution is an error.
    pub(crate) async fn preparation_metadata(
        &self,
        snapshot: &RuntimeBindingSnapshotV1,
        parent_uid: &str,
        revision_name: &str,
        image: &str,
        guard: &apparatus_contracts::runtime::RuntimeWriteGuardV1,
        profile: &crate::runtime_projection::ServingPlatformProfile,
        chain: &mut PreparationChainUids,
    ) -> Result<Option<apparatus_contracts::runtime::RuntimeObservationV1>, RuntimeErrorCodeV1>
    {
        let unavailable = RuntimeErrorCodeV1::RuntimeProjectionUnavailable;
        let name = crate::runtime_ownership::serving_name(snapshot);
        let parent = self
            .parents
            .get_opt(&name)
            .await
            .map_err(|_| unavailable)?
            .ok_or(RuntimeErrorCodeV1::RuntimeFenceConflict)?;
        let json = serde_json::to_value(&parent).map_err(|_| unavailable)?;
        crate::runtime_ownership::mutation_preconditions(&json, snapshot, parent_uid)?;
        if parent.data["spec"]["template"]["metadata"]["name"] != revision_name
            || parent
                .metadata
                .annotations
                .as_ref()
                .and_then(|values| values.get("apparatus.aiforall.dev/generation"))
                != Some(&snapshot.cas.generation.get().to_string())
        {
            return Err(RuntimeErrorCodeV1::RuntimeStaleState);
        }
        let Some(revision) = self
            .revisions
            .get_opt(revision_name)
            .await
            .map_err(|_| unavailable)?
        else {
            return Ok(None);
        };
        let revision_uid = uid(&revision.metadata)?;
        bind_chain_uid(&mut chain.revision, revision_uid)?;
        let configuration_ref = controller_owner(
            &revision.metadata,
            "serving.knative.dev/v1",
            "Configuration",
        )?;
        let configuration = self
            .configurations
            .get_opt(&configuration_ref.name)
            .await
            .map_err(|_| unavailable)?
            .ok_or(RuntimeErrorCodeV1::RuntimeFenceConflict)?;
        let configuration_uid = uid(&configuration.metadata)?;
        bind_chain_uid(&mut chain.configuration, configuration_uid)?;
        if configuration_uid != configuration_ref.uid {
            return Err(RuntimeErrorCodeV1::RuntimeFenceConflict);
        }
        let owner = controller_owner(&configuration.metadata, "serving.knative.dev/v1", "Service")?;
        if owner.uid != parent_uid || owner.name != name {
            return Err(RuntimeErrorCodeV1::RuntimeFenceConflict);
        }
        let containers = revision.data["spec"]["containers"]
            .as_array()
            .ok_or(unavailable)?;
        let users: Vec<_> = containers
            .iter()
            .filter(|container| container["name"] == "user-container")
            .collect();
        if users.len() != 1 || users[0]["image"] != image {
            return Err(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted);
        }
        let Some(resource_generation) = parent
            .metadata
            .generation
            .and_then(|n| u64::try_from(n).ok())
        else {
            return Ok(None);
        };
        let Some(observed_generation) = parent.data["status"]["observedGeneration"].as_u64() else {
            return Ok(None);
        };
        let Some(endpoint) = parent.data["status"]["address"]["url"].as_str() else {
            return Ok(None);
        };
        profile.check_endpoint(snapshot, endpoint)?;
        let ready_conditions: Vec<_> = parent.data["status"]["conditions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|condition| condition["type"] == "Ready")
            .collect();
        let route_ready = resource_generation == observed_generation
            && parent.data["status"]["latestCreatedRevisionName"] == revision_name
            && parent.data["status"]["latestReadyRevisionName"] == revision_name
            && ready_conditions.len() == 1
            && ready_conditions[0]["status"] == "True";
        let utc_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| unavailable)?
            .as_millis();
        let observed_at = RuntimeUnixMsV1::new(i64::try_from(utc_ms).map_err(|_| unavailable)?)?;
        Ok(Some(apparatus_contracts::runtime::RuntimeObservationV1 {
            schema_version: 1,
            project_id: snapshot.project_id,
            binding_id: snapshot.binding_id,
            guard: guard.clone(),
            service_uid: parent_uid.to_owned(),
            resource_generation: apparatus_contracts::runtime::RuntimeCounterV1::new(
                resource_generation,
            )?,
            observed_resource_generation: apparatus_contracts::runtime::RuntimeCounterV1::new(
                observed_generation,
            )?,
            revision_uid: revision_uid.to_owned(),
            revision_name: revision_name.to_owned(),
            release: snapshot.release.clone(),
            endpoint: endpoint.to_owned(),
            route_ready,
            observed_at_unix_ms: observed_at,
        }))
    }
    /// Reuse the operator's configured client, never pass it into Manifesto.
    #[must_use]
    pub fn new(client: Client) -> Self {
        let dynamic = |kind, plural: &str| {
            let mut resource =
                ApiResource::from_gvk(&GroupVersionKind::gvk("serving.knative.dev", "v1", kind));
            resource.plural = plural.to_owned();
            Api::namespaced_with(client.clone(), "apparatus-plugins", &resource)
        };
        Self {
            parents: dynamic("Service", "services"),
            configurations: dynamic("Configuration", "configurations"),
            revisions: dynamic("Revision", "revisions"),
            deployments: Api::namespaced(client.clone(), "apparatus-plugins"),
            replica_sets: Api::namespaced(client.clone(), "apparatus-plugins"),
            pods: Api::namespaced(client.clone(), "apparatus-plugins"),
            accounts: Api::namespaced(client, "apparatus-plugins"),
        }
    }

    /// Read a page after verifying Service -> Configuration -> Revision and each
    /// Revision -> Deployment -> ReplicaSet -> Pod chain by exact controller UID.
    /// `parent_uid` must come from a verified create response or durable writer
    /// ownership, never from the name/labels of an unknown existing object.
    ///
    /// # Errors
    /// Fails closed on missing/ambiguous owners, mismatched UID/image/SA/sandbox,
    /// API failure or an invalid capture clock. Never chooses a first replica.
    pub async fn observe_page(
        &self,
        snapshot: &RuntimeBindingSnapshotV1,
        parent_uid: &str,
        revision_name: &str,
        image: &str,
        continue_token: Option<&str>,
    ) -> Result<ObservedRuntimePodPage, RuntimeErrorCodeV1> {
        let unavailable = RuntimeErrorCodeV1::RuntimeProjectionUnavailable;
        let name = crate::runtime_ownership::serving_name(snapshot);
        let parent = self.parents.get(&name).await.map_err(|_| unavailable)?;
        let parent_json = serde_json::to_value(&parent).map_err(|_| unavailable)?;
        crate::runtime_ownership::mutation_preconditions(&parent_json, snapshot, parent_uid)?;
        let generation = snapshot.cas.generation.get().to_string();
        if parent
            .metadata
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.get("apparatus.aiforall.dev/generation"))
            != Some(&generation)
        {
            return Err(RuntimeErrorCodeV1::RuntimeStaleState);
        }
        if parent.data["spec"]["template"]["metadata"]["name"] != revision_name {
            return Err(RuntimeErrorCodeV1::RuntimeStaleState);
        }
        let revision = self
            .revisions
            .get(revision_name)
            .await
            .map_err(|_| unavailable)?;
        let revision_uid = uid(&revision.metadata)?;
        let configuration_ref = controller_owner(
            &revision.metadata,
            "serving.knative.dev/v1",
            "Configuration",
        )?;
        let configuration = self
            .configurations
            .get(&configuration_ref.name)
            .await
            .map_err(|_| unavailable)?;
        if uid(&configuration.metadata)? != configuration_ref.uid {
            return Err(unavailable);
        }
        let service_ref =
            controller_owner(&configuration.metadata, "serving.knative.dev/v1", "Service")?;
        if service_ref.uid != parent_uid || service_ref.name != name {
            return Err(unavailable);
        }
        let mut params = ListParams::default()
            .labels(&format!("serving.knative.dev/revision={revision_name}"))
            .limit(200);
        if let Some(token) = continue_token {
            params = params.continue_token(token);
        }
        let page = self.pods.list(&params).await.map_err(|_| unavailable)?;
        let mut items = Vec::with_capacity(page.items.len());
        for pod in page.items {
            let replica_ref = controller_owner(&pod.metadata, "apps/v1", "ReplicaSet")?;
            let replica = self
                .replica_sets
                .get(&replica_ref.name)
                .await
                .map_err(|_| unavailable)?;
            if uid(&replica.metadata)? != replica_ref.uid {
                return Err(unavailable);
            }
            let deployment_ref = controller_owner(&replica.metadata, "apps/v1", "Deployment")?;
            let deployment = self
                .deployments
                .get(&deployment_ref.name)
                .await
                .map_err(|_| unavailable)?;
            if uid(&deployment.metadata)? != deployment_ref.uid {
                return Err(unavailable);
            }
            let revision_ref =
                controller_owner(&deployment.metadata, "serving.knative.dev/v1", "Revision")?;
            if revision_ref.uid != revision_uid || revision_ref.name != revision_name {
                return Err(unavailable);
            }
            let spec = pod.spec.as_ref().ok_or(unavailable)?;
            if pod.metadata.namespace.as_deref() != Some("apparatus-plugins")
                || spec.service_account_name.as_deref() != Some("apparatus-runtime-plugin")
                || spec.automount_service_account_token != Some(false)
                || spec.runtime_class_name.as_deref() != Some("apparatus-gvisor")
            {
                return Err(unavailable);
            }
            let user_containers: Vec<_> = spec
                .containers
                .iter()
                .filter(|container| container.name == "user-container")
                .collect();
            if user_containers.len() != 1 || user_containers[0].image.as_deref() != Some(image) {
                return Err(unavailable);
            }
            let env = user_containers[0].env.as_deref().unwrap_or_default();
            for (key, expected) in [
                ("APPARATUS_RUNTIME_MODE", "knative-v1".to_owned()),
                (
                    "APPARATUS_RUNTIME_PROJECT_ID",
                    snapshot.project_id.to_string(),
                ),
                (
                    "APPARATUS_RUNTIME_BINDING_ID",
                    snapshot.binding_id.to_string(),
                ),
                ("APPARATUS_RUNTIME_GENERATION", generation.clone()),
                (
                    "APPARATUS_RUNTIME_RELEASE",
                    snapshot.release.as_str().to_owned(),
                ),
            ] {
                let values: Vec<_> = env.iter().filter(|variable| variable.name == key).collect();
                if values.len() != 1 || values[0].value.as_deref() != Some(expected.as_str()) {
                    return Err(unavailable);
                }
            }
            let account = self
                .accounts
                .get("apparatus-runtime-plugin")
                .await
                .map_err(|_| unavailable)?;
            let state = if pod.metadata.deletion_timestamp.is_some()
                || pod
                    .status
                    .as_ref()
                    .and_then(|status| status.phase.as_deref())
                    .is_some_and(|phase| phase == "Failed" || phase == "Succeeded")
            {
                RuntimeInstanceStateV1::Closed
            } else {
                RuntimeInstanceStateV1::Current
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| unavailable)?;
            let milliseconds = i64::try_from(now.as_millis()).map_err(|_| unavailable)?;
            items.push(ObservedRuntimePod {
                identity: RuntimePodIdentityV1 {
                    pod_uid: uid(&pod.metadata)?.to_owned(),
                    revision_uid: revision_uid.to_owned(),
                    namespace: "apparatus-plugins".to_owned(),
                    service_account: "apparatus-runtime-plugin".to_owned(),
                    service_account_uid: uid(&account.metadata)?.to_owned(),
                },
                state,
                observed_at: RuntimeUnixMsV1::new(milliseconds).map_err(|_| unavailable)?,
            });
        }
        Ok(ObservedRuntimePodPage {
            items,
            next_continue: page.metadata.continue_.filter(|token| !token.is_empty()),
        })
    }
}

fn uid(metadata: &ObjectMeta) -> Result<&str, RuntimeErrorCodeV1> {
    let uid = metadata
        .uid
        .as_deref()
        .ok_or(RuntimeErrorCodeV1::RuntimeProjectionUnavailable)?;
    apparatus_contracts::runtime::validate_runtime_uid(uid)?;
    Ok(uid)
}

fn controller_owner<'a>(
    metadata: &'a ObjectMeta,
    api_version: &str,
    kind: &str,
) -> Result<&'a OwnerReference, RuntimeErrorCodeV1> {
    let unavailable = RuntimeErrorCodeV1::RuntimeProjectionUnavailable;
    let mut controllers = metadata
        .owner_references
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|owner| owner.controller == Some(true));
    let owner = controllers.next().ok_or(unavailable)?;
    if controllers.next().is_some() || owner.api_version != api_version || owner.kind != kind {
        return Err(unavailable);
    }
    apparatus_contracts::runtime::validate_runtime_uid(&owner.uid)?;
    Ok(owner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_chain_requires_one_exact_controller_and_opaque_uid() {
        let reference = OwnerReference {
            api_version: "apps/v1".to_owned(),
            kind: "ReplicaSet".to_owned(),
            name: "replica".to_owned(),
            uid: "opaque-server-issued-uid".to_owned(),
            controller: Some(true),
            ..OwnerReference::default()
        };
        let mut metadata = ObjectMeta {
            owner_references: Some(vec![reference.clone()]),
            ..ObjectMeta::default()
        };
        assert!(controller_owner(&metadata, "apps/v1", "ReplicaSet").is_ok());
        assert!(controller_owner(&metadata, "apps/v1", "Deployment").is_err());
        metadata
            .owner_references
            .as_mut()
            .expect("refs")
            .push(reference);
        assert!(controller_owner(&metadata, "apps/v1", "ReplicaSet").is_err());
        metadata
            .owner_references
            .as_mut()
            .expect("refs")
            .truncate(1);
        metadata.owner_references.as_mut().expect("refs")[0].controller = Some(false);
        assert!(controller_owner(&metadata, "apps/v1", "ReplicaSet").is_err());
        metadata.owner_references.as_mut().expect("refs")[0].controller = Some(true);
        metadata.owner_references.as_mut().expect("refs")[0].uid = "bad\nuid".to_owned();
        assert!(controller_owner(&metadata, "apps/v1", "ReplicaSet").is_err());
    }
}
