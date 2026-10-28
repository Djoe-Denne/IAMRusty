//! Parent Serving projection from writer state and independent Adm-A evidence.
//! No Pod/Revision/Deployment ownership or publisher-provided charts/settings.

use apparatus_contracts::runtime::{
    RuntimeBindingSnapshotV1, RuntimeBindingStateV1, RuntimeCounterV1, RuntimeErrorCodeV1,
};
use apparatus_contracts::runtime::{
    RuntimeControlCallerV1, RuntimeControlPort, RuntimeInstanceSelectorV1,
    RuntimeWriterLeaseRequestV1, RuntimeWriterLeaseV1,
};
use kube::{
    api::{DeleteParams, PostParams, Preconditions},
    core::{ApiResource, DynamicObject, GroupVersionKind},
    Api, Client,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::runtime_ownership::mutation_preconditions;
use crate::runtime_ownership::serving_name;

mod observation;
mod preparation;
pub use observation::RuntimePodObservationCursor;
pub use preparation::{PreparationWindow, ServingEndpointPolicy, ServingEndpointTransport};

/// Qualified local admission reader. Implementations must re-read current
/// authoritative decision/key policy, not return a store snapshot loaded at
/// controller startup. There is deliberately no adapter for a cached store.
pub trait CurrentServingAdmissionSource: Send + Sync {
    /// Return current independent Adm-A evidence, refusing withdrawn/uncertain
    /// decisions. This does not transfer admission authority to the controller.
    ///
    /// # Errors
    /// Missing/stale/withdrawn or unavailable authority fails closed.
    fn load_current(
        &self,
        digest: &crate::digest::ReleaseDigest,
    ) -> Result<crate::admission::AdmissionRecord, RuntimeErrorCodeV1>;
}

/// Kubernetes adapter restricted to parent Serving Services in the plugin pool.
/// Does not hold APIs for Pods, Deployments, Revisions or network Services.
pub struct ServingParentApi {
    parents: Api<DynamicObject>,
}

/// One process-scoped reconciler. Construct once after startup, never persist its
/// fresh holder UUID or reuse it across controller restarts.
pub struct ServingRuntimeBridge {
    holder_id: uuid::Uuid,
    control: std::sync::Arc<dyn RuntimeControlPort>,
    parents: ServingParentApi,
    admission: std::sync::Arc<dyn CurrentServingAdmissionSource>,
    admit_target: std::sync::Arc<crate::admit::AdmitTarget>,
    profile: ServingPlatformProfile,
    pending_instances: tokio::sync::Mutex<
        std::collections::BTreeMap<(uuid::Uuid, uuid::Uuid, String), uuid::Uuid>,
    >,
}

impl ServingRuntimeBridge {
    /// Inject transport, independent admission and platform-only configuration.
    #[must_use]
    pub fn new(
        control: std::sync::Arc<dyn RuntimeControlPort>,
        parents: ServingParentApi,
        admission: std::sync::Arc<dyn CurrentServingAdmissionSource>,
        admit_target: std::sync::Arc<crate::admit::AdmitTarget>,
        profile: ServingPlatformProfile,
    ) -> Self {
        Self {
            holder_id: uuid::Uuid::new_v4(),
            control,
            parents,
            admission,
            admit_target,
            profile,
            pending_instances: tokio::sync::Mutex::new(std::collections::BTreeMap::new()),
        }
    }

    /// Reconcile a discovered binding after a fresh writer lookup. No removal is
    /// inferred from missing list entries; only explicit closed/tombstone state.
    ///
    /// # Errors
    /// Stops on authority/admission/lease failure, unknown ownership or API race.
    pub async fn reconcile_parent(
        &self,
        snapshot: &RuntimeBindingSnapshotV1,
    ) -> Result<Option<DynamicObject>, RuntimeErrorCodeV1> {
        let caller = RuntimeControlCallerV1 {
            role: apparatus_contracts::runtime::RuntimeServiceRoleV1::Projector,
        };
        let selector = RuntimeInstanceSelectorV1::Page {
            cursor: None,
            page_size: Some(1),
        };
        let current = self
            .control
            .get_binding(&caller, snapshot.project_id, snapshot.binding_id, &selector)
            .await
            .map_err(|error| error.code)?;
        if matches!(
            current.state,
            RuntimeBindingStateV1::Closed | RuntimeBindingStateV1::Draining
        ) {
            // The wire snapshot intentionally has no preparation phase. A
            // prepared closed binding must not be mistaken for cleanup work.
            return Ok(None);
        }
        let image = if current.state == RuntimeBindingStateV1::Active {
            let admission = self.admission.clone();
            let target = self.admit_target.clone();
            let verify_snapshot = current.clone();
            Some(
                tokio::task::spawn_blocking(move || {
                    let digest =
                        crate::digest::ReleaseDigest::new(verify_snapshot.release.as_str())
                            .map_err(|_| RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?;
                    let record = admission.load_current(&digest)?;
                    AdmittedServingImage::verify_record(record, &digest, Some(target.as_ref()))
                })
                .await
                .map_err(|_| RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)??,
            )
        } else {
            None
        };
        // Re-read after blocking signature verification; never apply its old CAS.
        let current = self
            .control
            .get_binding(&caller, current.project_id, current.binding_id, &selector)
            .await
            .map_err(|error| error.code)?;
        let lease = self
            .control
            .writer_lease(
                &caller,
                current.project_id,
                current.binding_id,
                &RuntimeWriterLeaseRequestV1 {
                    schema_version: 1,
                    holder_id: self.holder_id,
                    expected: current.cas.clone(),
                    renew: None,
                },
            )
            .await
            .map_err(|error| error.code)?;
        if current.state == RuntimeBindingStateV1::Active {
            let image = image
                .as_ref()
                .ok_or(RuntimeErrorCodeV1::RuntimeStaleState)?;
            self.parents
                .project(
                    self.control.as_ref(),
                    &caller,
                    &current,
                    &lease,
                    image,
                    &self.profile,
                )
                .await
                .map(Some)
        } else {
            self.parents
                .remove_closed(self.control.as_ref(), &caller, &current, &lease)
                .await?;
            Ok(None)
        }
    }
}

impl ServingParentApi {
    /// Use an already-configured operator client; never construct one in a domain.
    #[must_use]
    pub fn new(client: Client) -> Self {
        let mut resource = ApiResource::from_gvk(&GroupVersionKind::gvk(
            "serving.knative.dev",
            "v1",
            "Service",
        ));
        resource.plural = "services".to_owned();
        Self {
            parents: Api::namespaced_with(client, "apparatus-plugins", &resource),
        }
    }

    /// Create/replace a parent only after writer re-read and successful renewal.
    /// No force/SSA/adoption. A create-response UID must be durably observed;
    /// crashing before that leaves an unknown object requiring explicit inventory.
    ///
    /// # Errors
    /// Refuses stale/lost leases, unknown ownership, resourceVersion conflicts,
    /// closed authority, admission mismatch and any Kubernetes API failure.
    pub async fn project(
        &self,
        control: &dyn RuntimeControlPort,
        caller: &RuntimeControlCallerV1,
        snapshot: &RuntimeBindingSnapshotV1,
        lease: &RuntimeWriterLeaseV1,
        image: &AdmittedServingImage,
        profile: &ServingPlatformProfile,
    ) -> Result<DynamicObject, RuntimeErrorCodeV1> {
        if profile.approved_lock.is_none() || profile.serving_endpoint.is_none() {
            return Err(RuntimeErrorCodeV1::RuntimeKeyUnavailable);
        }
        let name = serving_name(snapshot);
        let existing = self.parents.get_opt(&name).await.map_err(kube_error)?;
        let selector = RuntimeInstanceSelectorV1::Page {
            cursor: None,
            page_size: Some(1),
        };
        let current = control
            .get_binding(caller, snapshot.project_id, snapshot.binding_id, &selector)
            .await
            .map_err(|error| error.code)?;
        if current.cas != snapshot.cas
            || current.release != snapshot.release
            || current.profile != snapshot.profile
        {
            return Err(RuntimeErrorCodeV1::RuntimeStaleState);
        }
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
        let desired = serving_parent(&current, image, profile, renewed.proof.writer_fence)?;
        let result = if let Some(existing) = existing {
            let recorded = current
                .observation
                .as_ref()
                .ok_or(RuntimeErrorCodeV1::RuntimeFenceConflict)?;
            let mut object = serde_json::to_value(existing)
                .map_err(|_| RuntimeErrorCodeV1::RuntimeAuthorityProtocolError)?;
            let preconditions = mutation_preconditions(&object, &current, &recorded.service_uid)?;
            object["spec"] = desired["spec"].clone();
            // Keep controller-owned finalizers/other metadata, replace only our keys.
            for field in ["labels", "annotations"] {
                let updates = desired["metadata"][field]
                    .as_object()
                    .ok_or(RuntimeErrorCodeV1::RuntimeInternal)?;
                if object["metadata"][field].is_null() {
                    object["metadata"][field] = json!({});
                }
                let map = object["metadata"][field]
                    .as_object_mut()
                    .ok_or(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError)?;
                for (key, value) in updates {
                    map.insert(key.clone(), value.clone());
                }
            }
            object["metadata"]["uid"] = json!(preconditions.uid);
            object["metadata"]["resourceVersion"] = json!(preconditions.resource_version);
            let parent: DynamicObject =
                serde_json::from_value(object).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
            self.parents
                .replace(&name, &PostParams::default(), &parent)
                .await
                .map_err(kube_error)?
        } else {
            let parent: DynamicObject =
                serde_json::from_value(desired).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
            if current.observation.is_some() {
                // Missing Serving parent is not a zero-scale Pod: recreation
                // requires a new approved preparation retaining old UID history.
                return Err(RuntimeErrorCodeV1::RuntimeFenceConflict);
            }
            self.parents
                .create(&PostParams::default(), &parent)
                .await
                .map_err(kube_error)?
        };
        if result.metadata.uid.as_deref().is_none_or(str::is_empty) {
            return Err(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError);
        }
        // Renewal cannot atomically fence an API call already in flight. Any
        // residual must remain unable to enroll/accept/effect under stale writer state.
        Ok(result)
    }

    /// Remove only an owned parent after authoritative closure/tombstone. Serving
    /// and Kubernetes controllers, not this adapter, terminate their children.
    ///
    /// # Errors
    /// Refuses active bindings, unknown UID ownership, lost lease or version races.
    pub async fn remove_closed(
        &self,
        control: &dyn RuntimeControlPort,
        caller: &RuntimeControlCallerV1,
        snapshot: &RuntimeBindingSnapshotV1,
        lease: &RuntimeWriterLeaseV1,
    ) -> Result<(), RuntimeErrorCodeV1> {
        let name = serving_name(snapshot);
        let existing = self.parents.get_opt(&name).await.map_err(kube_error)?;
        let selector = RuntimeInstanceSelectorV1::Page {
            cursor: None,
            page_size: Some(1),
        };
        let current = control
            .get_binding(caller, snapshot.project_id, snapshot.binding_id, &selector)
            .await
            .map_err(|error| error.code)?;
        if current.state == RuntimeBindingStateV1::Active {
            return Err(RuntimeErrorCodeV1::RuntimeStaleState);
        }
        if current.cas != snapshot.cas {
            return Err(RuntimeErrorCodeV1::RuntimeStaleState);
        }
        control
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
        let Some(existing) = existing else {
            return Ok(());
        };
        let recorded = current
            .observation
            .as_ref()
            .ok_or(RuntimeErrorCodeV1::RuntimeFenceConflict)?;
        let object = serde_json::to_value(existing)
            .map_err(|_| RuntimeErrorCodeV1::RuntimeAuthorityProtocolError)?;
        let preconditions = mutation_preconditions(&object, &current, &recorded.service_uid)?;
        self.parents
            .delete(
                &name,
                &DeleteParams {
                    preconditions: Some(Preconditions {
                        uid: Some(preconditions.uid),
                        resource_version: Some(preconditions.resource_version),
                    }),
                    ..DeleteParams::default()
                },
            )
            .await
            .map_err(kube_error)?;
        Ok(())
    }
}

fn kube_error(error: kube::Error) -> RuntimeErrorCodeV1 {
    match error {
        kube::Error::Api(response) if response.code == 409 => RuntimeErrorCodeV1::RuntimeStaleState,
        _ => RuntimeErrorCodeV1::RuntimeRouteUnavailable,
    }
}
use crate::{
    admission::{validate_cri_pin, AdmissionStore},
    conformance::POLICY_ID,
    controller::gate_schedule_signature,
};

/// Configuration supplied only by the approved platform profile, never the
/// publisher/request. No defaults for the TLS origin or public CA reference.
pub struct ServingPlatformProfile {
    profile_id: String,
    lazaret_base_url: String,
    lazaret_public_ca_configmap: String,
    approved_lock: Option<apparatus_contracts::runtime::RuntimeDigestV1>,
    serving_endpoint: Option<ServingEndpointPolicy>,
}

impl ServingPlatformProfile {
    /// Validate the exact canonical private TLS endpoint from B's profile.
    ///
    /// # Errors
    /// Rejects missing/noncanonical TLS URLs, userinfo/query/fragment, a wrong
    /// prefix or invalid CA ConfigMap name. Network/privacy proof remains B/E.
    pub fn new(
        profile_id: String,
        lazaret_base_url: String,
        lazaret_public_ca_configmap: String,
    ) -> Result<Self, RuntimeErrorCodeV1> {
        if profile_id.is_empty()
            || profile_id.len() > 128
            || !profile_id.is_ascii()
            || profile_id.chars().any(char::is_control)
        {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        let url = reqwest::Url::parse(&lazaret_base_url)
            .map_err(|_| RuntimeErrorCodeV1::RuntimeInvalidRequest)?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/lazaret"
            || url.as_str() != lazaret_base_url
        {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        apparatus_contracts::runtime::validate_runtime_dns_name(
            &lazaret_public_ca_configmap,
            true,
        )?;
        Ok(Self {
            profile_id,
            lazaret_base_url,
            lazaret_public_ca_configmap,
            approved_lock: None,
            serving_endpoint: None,
        })
    }

    /// Bind B's separately approved immutable lock and private endpoint policy.
    /// This setup-only value is never deserialized from a control request.
    #[must_use]
    pub fn with_approved_preparation(
        mut self,
        lock: apparatus_contracts::runtime::RuntimeDigestV1,
        endpoint: ServingEndpointPolicy,
    ) -> Self {
        self.approved_lock = Some(lock);
        self.serving_endpoint = Some(endpoint);
        self
    }

    pub(crate) fn check_endpoint(
        &self,
        snapshot: &RuntimeBindingSnapshotV1,
        endpoint: &str,
    ) -> Result<(), RuntimeErrorCodeV1> {
        self.serving_endpoint
            .as_ref()
            .ok_or(RuntimeErrorCodeV1::RuntimeKeyUnavailable)?
            .validate(snapshot, endpoint)
    }
}

/// Image pin retrieved from the TCB admission store and reverified envelope.
/// Private fields prevent an arbitrary publisher image being passed to render.
pub struct AdmittedServingImage {
    image: String,
    descriptor: String,
    record: Option<crate::admission::AdmissionRecord>,
}

impl AdmittedServingImage {
    /// Check independent admission and signature before constructing a template.
    /// Caller must execute this synchronous signature verifier off async workers.
    ///
    /// # Errors
    /// Refuses missing admission, wrong policy/descriptor, missing envelope,
    /// invalid signature or non-digest CRI image. Does not write admission state.
    pub fn verify(
        store: &(impl AdmissionStore + ?Sized),
        snapshot: &RuntimeBindingSnapshotV1,
        target: Option<&crate::admit::AdmitTarget>,
    ) -> Result<Self, RuntimeErrorCodeV1> {
        let digest = crate::digest::ReleaseDigest::new(snapshot.release.as_str())
            .map_err(|_| RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?;
        let record = store
            .get(&digest)
            .ok_or(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?;
        Self::verify_record(record, &digest, target)
    }

    fn verify_record(
        record: crate::admission::AdmissionRecord,
        digest: &crate::digest::ReleaseDigest,
        target: Option<&crate::admit::AdmitTarget>,
    ) -> Result<Self, RuntimeErrorCodeV1> {
        if &record.descriptor_digest != digest || record.policy_version != POLICY_ID {
            return Err(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted);
        }
        gate_schedule_signature(record.envelope_reference.as_deref(), target)
            .map_err(|_| RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?;
        let image = record
            .cri_image
            .clone()
            .ok_or(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?;
        validate_cri_pin(&image).map_err(|_| RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?;
        Ok(Self {
            image,
            descriptor: digest.as_str().to_owned(),
            record: Some(record),
        })
    }
}

/// Render only the parent Service. Admission/ownership/fence checks are required
/// again at the external mutation boundary; this function performs no I/O.
///
/// # Errors
/// Refuses closed bindings, malformed generations, unknown profiles or descriptor
/// substitution. This is not proof that the pinned Serving schema accepts it.
pub fn serving_parent(
    snapshot: &RuntimeBindingSnapshotV1,
    image: &AdmittedServingImage,
    profile: &ServingPlatformProfile,
    fence: RuntimeCounterV1,
) -> Result<Value, RuntimeErrorCodeV1> {
    render_parent(snapshot, image, profile, fence, None)
}

fn render_parent(
    snapshot: &RuntimeBindingSnapshotV1,
    image: &AdmittedServingImage,
    profile: &ServingPlatformProfile,
    fence: RuntimeCounterV1,
    preparation: Option<&apparatus_contracts::runtime::RuntimeBindingPreparationPermitV1>,
) -> Result<Value, RuntimeErrorCodeV1> {
    if snapshot.schema_version != 1 {
        return Err(RuntimeErrorCodeV1::RuntimeUnsupportedVersion);
    }
    if let Some(permit) = preparation {
        preparation::validate_permit(snapshot, profile, permit)?;
    } else if snapshot.state != RuntimeBindingStateV1::Active {
        return Err(RuntimeErrorCodeV1::RuntimeBindingClosed);
    }
    if snapshot.cas.generation.get() == 0
        || fence.get() == 0
        || image.descriptor != snapshot.release.as_str()
    {
        return Err(RuntimeErrorCodeV1::RuntimeStaleState);
    }
    if snapshot.profile != profile.profile_id {
        return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
    }
    let name = serving_name(snapshot);
    let mut revision_hash = Sha256::new();
    revision_hash.update(b"urn:aiforall:apparatus:serving-revision:v1\0");
    revision_hash.update(snapshot.project_id.as_bytes());
    revision_hash.update(snapshot.binding_id.as_bytes());
    revision_hash.update(snapshot.cas.generation.get().to_be_bytes());
    revision_hash.update(snapshot.release.as_str().as_bytes());
    revision_hash.update(snapshot.profile.as_bytes());
    let revision_name = format!("{name}-{}", &hex::encode(revision_hash.finalize())[..27]);
    let labels = json!({"apparatus.aiforall.dev/project":snapshot.project_id.to_string(),
        "apparatus.aiforall.dev/binding":snapshot.binding_id.to_string(), "apparatus.aiforall.dev/runtime":"knative"});
    Ok(json!({
        "apiVersion":"serving.knative.dev/v1", "kind":"Service",
        "metadata":{"name":name,"namespace":"apparatus-plugins","labels":labels,
            "annotations":{"networking.knative.dev/visibility":"cluster-local",
                "apparatus.aiforall.dev/generation":snapshot.cas.generation.get().to_string(),
                "apparatus.aiforall.dev/profile":snapshot.profile,
                "apparatus.aiforall.dev/profile-lock":profile.approved_lock.as_ref().map_or("", |lock| lock.as_str()),
                "apparatus.aiforall.dev/writer-fence":fence.get().to_string()}},
        "spec":{"template":{"metadata":{"name":revision_name,"labels":labels,"annotations":{
            "autoscaling.knative.dev/class":"kpa.autoscaling.knative.dev",
            "autoscaling.knative.dev/min-scale":"0","autoscaling.knative.dev/initial-scale":"0",
            "autoscaling.knative.dev/max-scale":"2","autoscaling.knative.dev/metric":"concurrency",
            "autoscaling.knative.dev/target":"1","autoscaling.knative.dev/window":"60s",
            "autoscaling.knative.dev/scale-to-zero-pod-retention-period":"30s",
            "queue.sidecar.serving.knative.dev/cpu-resource-request":"50m",
            "queue.sidecar.serving.knative.dev/memory-resource-request":"64Mi",
            "queue.sidecar.serving.knative.dev/cpu-resource-limit":"100m",
            "queue.sidecar.serving.knative.dev/memory-resource-limit":"128Mi",
            "sidecar.istio.io/inject":"true", "sidecar.istio.io/proxyCPU":"100m",
            "sidecar.istio.io/proxyMemory":"128Mi", "sidecar.istio.io/proxyCPULimit":"200m",
            "sidecar.istio.io/proxyMemoryLimit":"256Mi"
        }},"spec":{
            "serviceAccountName":"apparatus-runtime-plugin","automountServiceAccountToken":false,
            "runtimeClassName":"apparatus-gvisor","containerConcurrency":1,"timeoutSeconds":28,
            "nodeSelector":{"apparatus.aiforall.dev/pool":"plugins"},
            "tolerations":[{"key":"apparatus.aiforall.dev/plugins","operator":"Equal","value":"true","effect":"NoSchedule"}],
            "securityContext":{"runAsNonRoot":true,"runAsUser":65532,"runAsGroup":65532,"fsGroup":65532,
                "seccompProfile":{"type":"RuntimeDefault"}},
            "containers":[{"name":"user-container","image":image.image,
                "ports":[{"name":"http1","containerPort":8080}],
                "resources":{"requests":{"cpu":"250m","memory":"256Mi"},"limits":{"cpu":"500m","memory":"512Mi"}},
                "securityContext":{"allowPrivilegeEscalation":false,"readOnlyRootFilesystem":true,"capabilities":{"drop":["ALL"]}},
                "readinessProbe":{"httpGet":{"path":"/ready","port":8080}},
                "env":[
                    {"name":"APPARATUS_RUNTIME_MODE","value":"knative-v1"},
                    {"name":"APPARATUS_RUNTIME_PROJECT_ID","value":snapshot.project_id.to_string()},
                    {"name":"APPARATUS_RUNTIME_BINDING_ID","value":snapshot.binding_id.to_string()},
                    {"name":"APPARATUS_RUNTIME_GENERATION","value":snapshot.cas.generation.get().to_string()},
                    {"name":"APPARATUS_RUNTIME_RELEASE","value":snapshot.release.as_str()},
                    {"name":"APPARATUS_RUNTIME_POD_UID","valueFrom":{"fieldRef":{"apiVersion":"v1","fieldPath":"metadata.uid"}}},
                    {"name":"APPARATUS_RUNTIME_NAMESPACE","valueFrom":{"fieldRef":{"apiVersion":"v1","fieldPath":"metadata.namespace"}}},
                    {"name":"APPARATUS_RUNTIME_SERVICE_ACCOUNT","valueFrom":{"fieldRef":{"apiVersion":"v1","fieldPath":"spec.serviceAccountName"}}},
                    {"name":"APPARATUS_RUNTIME_LAZARET_BASE_URL","value":profile.lazaret_base_url},
                    {"name":"APPARATUS_RUNTIME_ENROLLMENT_TOKEN_PATH","value":"/var/run/apparatus-runtime/enrollment/token"},
                    {"name":"APPARATUS_RUNTIME_LAZARET_CA_PATH","value":"/var/run/apparatus-runtime/trust/lazaret-ca.pem"}
                ],"volumeMounts":[
                    {"name":"apparatus-runtime-enrollment","mountPath":"/var/run/apparatus-runtime/enrollment","readOnly":true},
                    {"name":"apparatus-runtime-trust","mountPath":"/var/run/apparatus-runtime/trust","readOnly":true},
                    {"name":"apparatus-runtime-tmp","mountPath":"/tmp"}
                ]}],
            "volumes":[
                {"name":"apparatus-runtime-enrollment","projected":{"defaultMode":288,"sources":[{"serviceAccountToken":{"audience":"urn:aiforall:apparatus:runtime:enrollment:v1","expirationSeconds":600,"path":"token"}}]}},
                {"name":"apparatus-runtime-trust","configMap":{"name":profile.lazaret_public_ca_configmap,"items":[{"key":"ca.pem","path":"lazaret-ca.pem"}]}},
                {"name":"apparatus-runtime-tmp","emptyDir":{"medium":"Memory","sizeLimit":"64Mi"}}
            ]
        }},"traffic":[{"revisionName":revision_name,"percent":100}]}
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use apparatus_contracts::runtime::{RuntimeCasV1, RuntimeDigestV1};

    #[test]
    fn template_is_zero_and_pod_bound_without_static_instance_or_legacy_aliases() {
        let count = RuntimeCounterV1::new(1).expect("count");
        let release = format!("sha256:{}", "1".repeat(64));
        let snapshot = RuntimeBindingSnapshotV1 {
            schema_version: 1,
            project_id: uuid::Uuid::new_v4(),
            binding_id: uuid::Uuid::new_v4(),
            cas: RuntimeCasV1 {
                generation: count,
                grant_revision: count,
                revocation_epoch: count,
                projection_version: count,
            },
            state: RuntimeBindingStateV1::Active,
            release: RuntimeDigestV1::new(release.clone()).expect("digest"),
            profile: "test-profile".to_owned(),
            observation: None,
            instances: None,
        };
        // Pure render fixture, NOT evidence that Adm-A accepted an image.
        let image = AdmittedServingImage {
            image: format!("registry.test/ref@{release}"),
            descriptor: release,
            record: None,
        };
        let profile = ServingPlatformProfile::new(
            "test-profile".to_owned(),
            "https://gateway.test/lazaret".to_owned(),
            "runtime-ca".to_owned(),
        )
        .expect("profile");
        let object = serving_parent(&snapshot, &image, &profile, count).expect("template");
        let template = &object["spec"]["template"];
        assert_eq!(
            template["metadata"]["annotations"]["autoscaling.knative.dev/min-scale"],
            "0"
        );
        assert_eq!(
            template["metadata"]["annotations"]["autoscaling.knative.dev/initial-scale"],
            "0"
        );
        assert_eq!(
            template["metadata"]["annotations"]["autoscaling.knative.dev/max-scale"],
            "2"
        );
        assert_eq!(template["spec"]["automountServiceAccountToken"], false);
        let env = template["spec"]["containers"][0]["env"]
            .as_array()
            .expect("env");
        assert!(env
            .iter()
            .all(|item| item["name"] != "APPARATUS_RUNTIME_INSTANCE_ID"
                && item["name"] != "INSTANCE"));
        let uid = env
            .iter()
            .find(|item| item["name"] == "APPARATUS_RUNTIME_POD_UID")
            .expect("uid env");
        assert_eq!(uid["valueFrom"]["fieldRef"]["fieldPath"], "metadata.uid");
        assert_eq!(
            template["spec"]["volumes"][0]["projected"]["sources"][0]["serviceAccountToken"]
                ["expirationSeconds"],
            600
        );
        let name = object["metadata"]["name"].as_str().expect("name");
        let revision = template["metadata"]["name"].as_str().expect("revision");
        assert!(revision.starts_with(&format!("{name}-")) && revision.len() <= 63);
        let takeover = serving_parent(
            &snapshot,
            &image,
            &profile,
            count.checked_next().expect("next"),
        )
        .expect("takeover");
        assert_eq!(
            object["spec"], takeover["spec"],
            "fence-only update must not create a new Revision/warm-up"
        );
    }
}
