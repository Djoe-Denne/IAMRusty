//! Runtime identity revalidation against the writer. No allow cache and no cluster
//! access. Consumes the integrated §13.2 shared DTOs (HEAD 866e527).

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use apparatus_contracts::runtime::*;
use lazaret_domain::{
    runtime::{
        RuntimeAdmissionGuard, RuntimeEnrollment, RuntimeError, RuntimePodProof, RuntimeStore,
        RuntimeTicketSigner,
    },
    CallOrigin, GrantAuthorizationRequest, GrantDecision, SessionProof, VerifiedClientCertificate,
};
use uuid::Uuid;

/// Authoritative runtime identity lookups. Call on every enrollment, /session and
/// effect admission, even if the certificate/session is cryptographically valid.
pub struct RuntimeIdentityAuthority {
    control: Arc<dyn RuntimeControlPort>,
}

/// Keeps historical /session reasons for non-migrated identities while mapping
/// runtime proof/directory failures through the frozen runtime error taxonomy.
pub enum RuntimeSessionError {
    Legacy(lazaret_domain::IdentityError),
    Runtime(RuntimeError),
}

pub struct RuntimeSessionService {
    identity: Arc<crate::IdentityService>,
    store: Arc<dyn RuntimeStore>,
    authority: Arc<RuntimeIdentityAuthority>,
    admission: Arc<dyn RuntimeAdmissionGuard>,
}

impl RuntimeSessionService {
    pub fn new(
        identity: Arc<crate::IdentityService>,
        store: Arc<dyn RuntimeStore>,
        authority: Arc<RuntimeIdentityAuthority>,
        admission: Arc<dyn RuntimeAdmissionGuard>,
    ) -> Self {
        Self {
            identity,
            store,
            authority,
            admission,
        }
    }

    pub async fn issue_session(
        &self,
        cert: &VerifiedClientCertificate,
    ) -> Result<String, RuntimeSessionError> {
        let fingerprint = cert.fingerprint_sha256();
        let runtime = self
            .store
            .enrollment_by_fingerprint(&fingerprint)
            .await
            .map_err(RuntimeSessionError::Runtime)?;
        let legacy = self
            .identity
            .enrollment_store()
            .get_checked(&fingerprint)
            .await
            .map_err(|_| RuntimeSessionError::Runtime(RuntimeError::RuntimeAuthorityUnavailable))?;
        match (runtime, legacy) {
            (Some(_), Some(_)) => Err(RuntimeSessionError::Runtime(
                RuntimeError::RuntimeInvalidProof,
            )),
            (None, None) => Err(RuntimeSessionError::Legacy(
                lazaret_domain::IdentityError::NotEnrolled,
            )),
            (None, Some(identity)) => {
                self.admission
                    .require_legacy_identity(&identity)
                    .await
                    .map_err(RuntimeSessionError::Runtime)?;
                // No RuntimeControlPort call on this branch. The local writer
                // negative admission lock is shared with install_legacy_fence.
                self.identity
                    .issue_session(cert)
                    .await
                    .map_err(RuntimeSessionError::Legacy)
            }
            (Some(enrolled), None) => {
                self.admission
                    .require_runtime(&enrolled)
                    .await
                    .map_err(RuntimeSessionError::Runtime)?;
                self.authority
                    .revalidate(&enrolled)
                    .await
                    .map_err(RuntimeSessionError::Runtime)?;
                self.identity
                    .sign_runtime_session(&enrolled.identity, enrolled.expires_at_unix_ms)
                    .map_err(|_| RuntimeSessionError::Runtime(RuntimeError::RuntimeInvalidProof))
            }
        }
    }
}

/// Required collaborators are injected only by the composition root. No optional
/// authorization collaborator and no digest/local fallback exists on this path.
pub struct RuntimeInvocationDependencies {
    pub identity: Arc<crate::IdentityService>,
    pub authority: Arc<RuntimeIdentityAuthority>,
    pub admission: Arc<dyn RuntimeAdmissionGuard>,
    pub store: Arc<dyn RuntimeStore>,
    pub signer: Arc<dyn RuntimeTicketSigner>,
    pub grants: Arc<crate::GrantService>,
    pub control: Arc<dyn RuntimeControlPort>,
    pub locator: Arc<dyn crate::PluginEndpointLocator>,
}

pub struct RuntimeInvocationService {
    dependencies: RuntimeInvocationDependencies,
    http: reqwest::Client,
}

impl RuntimeInvocationService {
    /// mTLS assets are platform configuration; neither URL nor credentials are
    /// accepted from an invoking workload.
    pub fn new(
        dependencies: RuntimeInvocationDependencies,
        identity: reqwest::Identity,
        server_ca: reqwest::Certificate,
    ) -> Result<Self, RuntimeError> {
        let http = reqwest::Client::builder()
            .use_rustls_tls()
            .identity(identity)
            .add_root_certificate(server_ca)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(28))
            .build()
            .map_err(|_| RuntimeError::RuntimeInternal)?;
        Ok(Self { dependencies, http })
    }

    pub async fn invoke(
        &self,
        certificate: &VerifiedClientCertificate,
        session: &str,
        project_id: Uuid,
        mut request: RuntimeInvokeRequestV1,
    ) -> Result<apparatus_contracts::InvokeResponse, RuntimeError> {
        if request.schema_version != 1 {
            return Err(RuntimeError::RuntimeUnsupportedVersion);
        }
        if request.project_id != project_id {
            return Err(RuntimeError::RuntimeInvalidRequest);
        }
        let proof = self
            .dependencies
            .identity
            .verify_session(session)
            .map_err(|_| RuntimeError::RuntimeUnauthenticated)?;
        let now = chrono::Utc::now().timestamp_millis();
        let deadline = request
            .deadline_unix_ms
            .get()
            .min(now.saturating_add(30_000))
            .min(proof.expires_at_unix.saturating_mul(1000));
        if deadline <= now {
            return Err(RuntimeError::RuntimeDeadlineExceeded);
        }
        request.deadline_unix_ms = RuntimeUnixMsV1::new(deadline)?;
        let sent = AtomicBool::new(false);
        match tokio::time::timeout(
            Duration::from_millis((deadline - now) as u64),
            self.invoke_inner(certificate, &proof, request, &sent),
        )
        .await
        {
            Ok(result) => result,
            Err(_) if sent.load(Ordering::Acquire) => {
                Err(RuntimeError::RuntimeEffectOutcomeUnknown)
            }
            Err(_) => Err(RuntimeError::RuntimeDeadlineExceeded),
        }
    }

    async fn invoke_inner(
        &self,
        certificate: &VerifiedClientCertificate,
        proof: &SessionProof,
        request: RuntimeInvokeRequestV1,
        sent: &AtomicBool,
    ) -> Result<apparatus_contracts::InvokeResponse, RuntimeError> {
        let d = &self.dependencies;
        let fingerprint = certificate.fingerprint_sha256();
        let runtime = d.store.enrollment_by_fingerprint(&fingerprint).await?;
        let legacy = d
            .identity
            .enrollment_store()
            .get_checked(&fingerprint)
            .await
            .map_err(|_| RuntimeError::RuntimeAuthorityUnavailable)?;
        let enrolled = match (runtime, legacy) {
            (Some(_), Some(_)) => return Err(RuntimeError::RuntimeInvalidProof),
            (Some(record), None) => record,
            (None, Some(_)) => return Err(RuntimeError::RuntimeLegacyDisabled),
            (None, None) => return Err(RuntimeError::RuntimeUnauthenticated),
        };
        // Known runtime credentials never fall back to the legacy registry.
        if enrolled.identity != proof.identity
            || enrolled.project_id != request.project_id
            || proof.expires_at_unix.saturating_mul(1000) > enrolled.expires_at_unix_ms
        {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        if enrolled.identity.binding == request.target_binding_id {
            return Err(RuntimeError::RuntimeCallerForbidden);
        }
        d.admission.require_runtime(&enrolled).await?;
        let source_instance = d.authority.revalidate(&enrolled).await?;
        let source = RuntimeSourceV1 {
            instance: enrolled.identity.instance,
            binding: enrolled.identity.binding,
            release: source_instance.release.clone(),
            generation: source_instance.cas.generation,
            grant_revision: source_instance.cas.grant_revision,
        };
        let snapshot = d
            .control
            .get_binding(
                &RuntimeControlCallerV1 {
                    role: RuntimeServiceRoleV1::Lazaret,
                },
                request.project_id,
                request.target_binding_id,
                &RuntimeInstanceSelectorV1::Page {
                    cursor: None,
                    page_size: Some(1),
                },
            )
            .await
            .map_err(|error| error.code)?;
        if snapshot.project_id != request.project_id
            || snapshot.binding_id != request.target_binding_id
        {
            return Err(RuntimeError::RuntimeAuthorityProtocolError);
        }
        if snapshot.cas.generation != request.expected_generation {
            return Err(RuntimeError::RuntimeStaleState);
        }
        let target = RuntimeTargetV1 {
            binding_id: snapshot.binding_id,
            generation: snapshot.cas.generation,
            release: snapshot.release.clone(),
            grant_revision: snapshot.cas.grant_revision,
            revocation_epoch: snapshot.cas.revocation_epoch,
        };
        let decision = d
            .grants
            .authorize(&GrantAuthorizationRequest {
                identity: enrolled.identity.clone(),
                capability: "runtime.invoke".to_owned(),
                declared: Vec::new(),
                origin: CallOrigin::Background,
                project_id: enrolled.project_id,
            })
            .await
            .map_err(map_grant_failure)?;
        if decision != GrantDecision::Allow {
            return Err(RuntimeError::RuntimeCallerForbidden);
        }
        if d.grants
            .authorize_runtime_target(request.project_id, &target)
            .await
            .map_err(map_grant_failure)?
            != GrantDecision::Allow
        {
            return Err(RuntimeError::RuntimeCallerForbidden);
        }
        // Validate existing P0 operation/payload syntax, without fabricating a
        // session or identity for the not-yet-running target.
        let operation: apparatus_contracts::InvokeRequest =
            serde_json::from_value(serde_json::json!({
                "binding_id": request.target_binding_id, "operation_id": request.operation_id,
                "operation": request.operation, "params": request.payload,
            }))
            .map_err(|_| RuntimeError::RuntimeInvalidRequest)?;
        operation
            .validate()
            .map_err(|_| RuntimeError::RuntimeInvalidRequest)?;
        let endpoint = d.locator.locate_runtime(&snapshot, &target).await?;
        let authorization_id = Uuid::new_v4();
        let hash = RuntimeOperationHashInputV1 {
            schema_version: 1,
            domain: OPERATION_HASH_DOMAIN.to_owned(),
            authorization_id,
            project_id: request.project_id,
            source: source.clone(),
            source_revocation_epoch: source_instance.cas.revocation_epoch,
            target: target.clone(),
            operation_id: request.operation_id,
            operation: request.operation.clone(),
            payload: request.payload.clone(),
            deadline_unix_ms: request.deadline_unix_ms,
        }
        .sha256()?;
        let issued_at = chrono::Utc::now().timestamp();
        let expires_at = request.deadline_unix_ms.get() / 1000;
        if expires_at <= issued_at {
            return Err(RuntimeError::RuntimeDeadlineExceeded);
        }
        let ticket = RuntimeInvocationTicketV1 {
            schema_version: 1,
            iss: INVOKE_ISSUER.to_owned(),
            aud: INVOKE_AUDIENCE.to_owned(),
            purpose: INVOKE_PURPOSE.to_owned(),
            jti: authorization_id,
            iat: issued_at,
            exp: expires_at,
            operation_sha256: hash,
            source,
            source_revocation_epoch: source_instance.cas.revocation_epoch,
            project_id: request.project_id,
            target,
            operation_id: request.operation_id,
            deadline_unix_ms: request.deadline_unix_ms,
        };
        let signed = d.signer.sign(&ticket).await?;
        d.store.issue(&ticket).await?; // durable BEFORE a byte is forwarded
        let remaining = request
            .deadline_unix_ms
            .get()
            .saturating_sub(chrono::Utc::now().timestamp_millis());
        if remaining <= 0 {
            return Err(RuntimeError::RuntimeDeadlineExceeded);
        }
        sent.store(true, Ordering::Release);
        let mut response = self
            .http
            .post(format!("{endpoint}/invoke"))
            .header("Apparatus-Invocation", signed)
            .json(&request)
            .timeout(Duration::from_millis(remaining.min(28_000) as u64))
            .send()
            .await
            .map_err(|_| RuntimeError::RuntimeEffectOutcomeUnknown)?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(RuntimeError::RuntimeEffectOutcomeUnknown);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| RuntimeError::RuntimeEffectOutcomeUnknown)?
        {
            if chunk.len() > MAX_RUNTIME_BODY_BYTES.saturating_sub(bytes.len()) {
                return Err(RuntimeError::RuntimeEffectOutcomeUnknown);
            }
            bytes.extend_from_slice(&chunk);
        }
        let result: apparatus_contracts::InvokeResponse =
            parse_runtime_json(&bytes).map_err(|_| RuntimeError::RuntimeEffectOutcomeUnknown)?;
        if result.binding_id.as_str() != request.target_binding_id.to_string()
            || result.operation_id.to_string() != request.operation_id.to_string()
            || result.validate().is_err()
        {
            return Err(RuntimeError::RuntimeEffectOutcomeUnknown);
        }
        Ok(result)
    }
}

fn map_grant_failure(error: lazaret_domain::GrantFetchError) -> RuntimeError {
    match error {
        lazaret_domain::GrantFetchError::NotFound => RuntimeError::RuntimeCallerForbidden,
        lazaret_domain::GrantFetchError::Transport(_) => RuntimeError::RuntimeAuthorityUnavailable,
    }
}

/// Resolve only a writer-observed, binding/generation-specific HTTPS route. A
/// missing/stale route is an error, never a request for local/digest fallback.
pub fn runtime_endpoint(
    snapshot: &RuntimeBindingSnapshotV1,
    target: &RuntimeTargetV1,
) -> Result<String, RuntimeError> {
    if snapshot.state != RuntimeBindingStateV1::Active {
        return Err(RuntimeError::RuntimeBindingClosed);
    }
    if snapshot.binding_id != target.binding_id
        || snapshot.cas.generation != target.generation
        || snapshot.cas.grant_revision != target.grant_revision
        || snapshot.cas.revocation_epoch != target.revocation_epoch
        || snapshot.release != target.release
    {
        return Err(RuntimeError::RuntimeStaleState);
    }
    let route = snapshot
        .observation
        .as_ref()
        .ok_or(RuntimeError::RuntimeProjectionUnavailable)?;
    let now = chrono::Utc::now().timestamp_millis();
    if route.schema_version != 1
        || route.project_id != snapshot.project_id
        || route.binding_id != snapshot.binding_id
        || route.cas.generation != snapshot.cas.generation
        || route.cas.grant_revision != snapshot.cas.grant_revision
        || route.cas.revocation_epoch != snapshot.cas.revocation_epoch
        || route.cas.projection_version > snapshot.cas.projection_version
        || route.release != snapshot.release
    {
        return Err(RuntimeError::RuntimeStaleState);
    }
    if !route.route_ready
        || route.service_uid.is_empty()
        || route.revision_uid.is_empty()
        || route.revision_name.is_empty()
        || route.writer_fence.get() == 0
        || route.resource_generation.get() == 0
        || route.resource_generation != route.observed_resource_generation
        || route.observed_at_unix_ms.get() > now
        || now >= route.valid_until_unix_ms.get()
        || route.valid_until_unix_ms.get()
            > route
                .observed_at_unix_ms
                .get()
                .saturating_add(OBSERVATION_TTL_MS)
    {
        return Err(RuntimeError::RuntimeProjectionUnavailable);
    }
    let endpoint =
        reqwest::Url::parse(&route.endpoint).map_err(|_| RuntimeError::RuntimeRouteUnavailable)?;
    if endpoint.scheme() != "https"
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || endpoint.path() != "/"
    {
        return Err(RuntimeError::RuntimeRouteUnavailable);
    }
    Ok(endpoint.as_str().trim_end_matches('/').to_owned())
}

impl RuntimeIdentityAuthority {
    pub fn new(control: Arc<dyn RuntimeControlPort>) -> Self {
        Self { control }
    }

    async fn snapshot(
        &self,
        project: Uuid,
        binding: Uuid,
        selector: &RuntimeInstanceSelectorV1,
    ) -> Result<RuntimeBindingSnapshotV1, RuntimeError> {
        self.control
            .get_binding(
                &RuntimeControlCallerV1 {
                    role: RuntimeServiceRoleV1::Lazaret,
                },
                project,
                binding,
                selector,
            )
            .await
            .map_err(|error| error.code)
    }

    /// Discovery is Pod-bound; the operator's instance ID is never supplied by
    /// the enrolling caller, and a missing registration never creates one.
    pub async fn by_pod(
        &self,
        project: Uuid,
        binding: Uuid,
        proof: &RuntimePodProof,
    ) -> Result<RuntimeInstanceSnapshotV1, RuntimeError> {
        let selector = RuntimeInstanceSelectorV1::ByPodUid {
            pod_uid: proof.pod_uid.clone(),
        };
        let snapshot = self.snapshot(project, binding, &selector).await?;
        let instance = exact_instance(&snapshot, project, binding)?;
        check_current(&snapshot, instance)?;
        let pod = instance
            .pod
            .as_ref()
            .ok_or(RuntimeError::RuntimeAuthorityProtocolError)?;
        if pod.pod_uid != proof.pod_uid
            || pod.namespace != proof.namespace
            || pod.service_account != proof.service_account
            || pod.service_account_uid != proof.service_account_uid
        {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        if chrono::Utc::now().timestamp() >= proof.expires_at_unix {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        Ok(instance.clone())
    }

    /// Re-read by platform-issued instance ID, then compare every binding and
    /// Pod identity component with the local certificate registry.
    pub async fn revalidate(
        &self,
        enrollment: &RuntimeEnrollment,
    ) -> Result<RuntimeInstanceSnapshotV1, RuntimeError> {
        if enrollment.closed {
            return Err(RuntimeError::RuntimeIdentityRevoked);
        }
        if chrono::Utc::now().timestamp_millis() >= enrollment.expires_at_unix_ms {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        let selector = RuntimeInstanceSelectorV1::ByInstanceId {
            instance_id: enrollment.identity.instance,
        };
        let snapshot = self
            .snapshot(
                enrollment.project_id,
                enrollment.identity.binding,
                &selector,
            )
            .await?;
        let instance = exact_instance(
            &snapshot,
            enrollment.project_id,
            enrollment.identity.binding,
        )?;
        check_current(&snapshot, instance)?;
        let pod = instance
            .pod
            .as_ref()
            .ok_or(RuntimeError::RuntimeAuthorityProtocolError)?;
        if instance.instance_id != enrollment.identity.instance
            || pod.pod_uid != enrollment.pod_uid
            || pod.revision_uid != enrollment.revision_uid
            || pod.namespace != enrollment.namespace
            || pod.service_account != enrollment.service_account
            || pod.service_account_uid != enrollment.service_account_uid
            || instance.release.as_str() != enrollment.identity.release
            || instance.cas.generation.get()
                != u64::try_from(enrollment.identity.generation)
                    .map_err(|_| RuntimeError::RuntimeInvalidProof)?
            || instance.cas.grant_revision.get()
                != u64::try_from(enrollment.identity.grant_revision)
                    .map_err(|_| RuntimeError::RuntimeInvalidProof)?
            || instance.cas.revocation_epoch.get() != enrollment.revocation_epoch
        {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        Ok(instance.clone())
    }
}

fn exact_instance(
    snapshot: &RuntimeBindingSnapshotV1,
    project: Uuid,
    binding: Uuid,
) -> Result<&RuntimeInstanceSnapshotV1, RuntimeError> {
    if snapshot.schema_version != 1
        || snapshot.project_id != project
        || snapshot.binding_id != binding
    {
        return Err(RuntimeError::RuntimeAuthorityProtocolError);
    }
    let page = snapshot
        .instances
        .as_ref()
        .ok_or(RuntimeError::RuntimeAuthorityProtocolError)?;
    if page.schema_version != 1 || page.next_cursor.is_some() || page.items.len() > 1 {
        return Err(RuntimeError::RuntimeAuthorityProtocolError);
    }
    page.items
        .first()
        .ok_or(RuntimeError::RuntimeProjectionUnavailable)
}

fn check_current(
    snapshot: &RuntimeBindingSnapshotV1,
    instance: &RuntimeInstanceSnapshotV1,
) -> Result<(), RuntimeError> {
    if instance.schema_version != 1
        || instance.project_id != snapshot.project_id
        || instance.binding_id != snapshot.binding_id
    {
        return Err(RuntimeError::RuntimeAuthorityProtocolError);
    }
    if instance.state != RuntimeInstanceStateV1::Current {
        return Err(RuntimeError::RuntimeIdentityRevoked);
    }
    if snapshot.state != RuntimeBindingStateV1::Active {
        return Err(RuntimeError::RuntimeBindingClosed);
    }
    if instance.kind != RuntimeInstanceKindV1::Pod
        || instance.closed_at_unix_ms.is_some()
        || instance.pod.is_none()
        || instance.writer_fence.is_none()
    {
        return Err(RuntimeError::RuntimeAuthorityProtocolError);
    }
    let observed = instance
        .observed_at_unix_ms
        .ok_or(RuntimeError::RuntimeAuthorityProtocolError)?
        .get();
    let valid = instance
        .valid_until_unix_ms
        .ok_or(RuntimeError::RuntimeAuthorityProtocolError)?
        .get();
    let now = chrono::Utc::now().timestamp_millis();
    if observed > now || now >= valid || valid > observed.saturating_add(OBSERVATION_TTL_MS) {
        return Err(RuntimeError::RuntimeProjectionUnavailable);
    }
    // projection_version can lag after another replica changes, but cannot lead.
    if instance.cas.generation != snapshot.cas.generation
        || instance.cas.grant_revision != snapshot.cas.grant_revision
        || instance.cas.revocation_epoch != snapshot.cas.revocation_epoch
        || instance.cas.projection_version > snapshot.cas.projection_version
        || instance.release != snapshot.release
    {
        return Err(RuntimeError::RuntimeStaleState);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current() -> (RuntimeBindingSnapshotV1, RuntimeInstanceSnapshotV1) {
        let project_id = Uuid::new_v4();
        let binding_id = Uuid::new_v4();
        let counter = RuntimeCounterV1::new(1).expect("counter");
        let cas = RuntimeCasV1 {
            generation: counter,
            revocation_epoch: counter,
            projection_version: counter,
            grant_revision: counter,
        };
        let release = RuntimeDigestV1::new(format!("sha256:{}", "a".repeat(64))).expect("digest");
        let now = chrono::Utc::now().timestamp_millis();
        let instance = RuntimeInstanceSnapshotV1 {
            schema_version: 1,
            project_id,
            binding_id,
            instance_id: Uuid::new_v4(),
            kind: RuntimeInstanceKindV1::Pod,
            state: RuntimeInstanceStateV1::Current,
            cas: cas.clone(),
            release: release.clone(),
            pod: Some(RuntimePodIdentityV1 {
                pod_uid: "pod".to_owned(),
                revision_uid: "revision".to_owned(),
                namespace: "plugins".to_owned(),
                service_account: "plugin".to_owned(),
                service_account_uid: "sa-uid".to_owned(),
            }),
            writer_fence: Some(counter),
            observed_at_unix_ms: Some(RuntimeUnixMsV1::new(now).expect("time")),
            valid_until_unix_ms: Some(RuntimeUnixMsV1::new(now + 4000).expect("time")),
            closed_at_unix_ms: None,
        };
        let snapshot = RuntimeBindingSnapshotV1 {
            schema_version: 1,
            project_id,
            binding_id,
            cas,
            state: RuntimeBindingStateV1::Active,
            release,
            profile: "knative-v1".to_owned(),
            observation: None,
            instances: Some(RuntimeInstancePageV1 {
                schema_version: 1,
                items: vec![instance.clone()],
                next_cursor: None,
                read_at_unix_ms: RuntimeUnixMsV1::new(now).expect("time"),
            }),
        };
        (snapshot, instance)
    }

    #[test]
    fn bound_locator_requires_ready_fresh_revision_and_never_uses_digest_fallback() {
        let (mut snapshot, _) = current();
        let target = RuntimeTargetV1 {
            binding_id: snapshot.binding_id,
            generation: snapshot.cas.generation,
            release: snapshot.release.clone(),
            grant_revision: snapshot.cas.grant_revision,
            revocation_epoch: snapshot.cas.revocation_epoch,
        };
        assert!(matches!(
            runtime_endpoint(&snapshot, &target),
            Err(RuntimeError::RuntimeProjectionUnavailable)
        ));
        let now = chrono::Utc::now().timestamp_millis();
        snapshot.observation = Some(RuntimeObservationSnapshotV1 {
            schema_version: 1,
            project_id: snapshot.project_id,
            binding_id: snapshot.binding_id,
            cas: snapshot.cas.clone(),
            writer_fence: RuntimeCounterV1::new(1).expect("counter"),
            service_uid: "service-uid".to_owned(),
            resource_generation: RuntimeCounterV1::new(1).expect("counter"),
            observed_resource_generation: RuntimeCounterV1::new(1).expect("counter"),
            revision_uid: "revision-uid".to_owned(),
            revision_name: "revision".to_owned(),
            release: snapshot.release.clone(),
            endpoint: "https://binding.private.example".to_owned(),
            route_ready: false,
            observed_at_unix_ms: RuntimeUnixMsV1::new(now).expect("time"),
            valid_until_unix_ms: RuntimeUnixMsV1::new(now + 4000).expect("time"),
        });
        assert!(matches!(
            runtime_endpoint(&snapshot, &target),
            Err(RuntimeError::RuntimeProjectionUnavailable)
        ));
        snapshot.observation.as_mut().expect("route").route_ready = true;
        assert_eq!(
            runtime_endpoint(&snapshot, &target).expect("bound route"),
            "https://binding.private.example"
        );
        let mut wrong_target = target.clone();
        wrong_target.binding_id = Uuid::new_v4();
        assert!(matches!(
            runtime_endpoint(&snapshot, &wrong_target),
            Err(RuntimeError::RuntimeStaleState)
        ));
        snapshot.observation.as_mut().expect("route").endpoint =
            "http://digest-only.invalid".to_owned();
        assert!(matches!(
            runtime_endpoint(&snapshot, &target),
            Err(RuntimeError::RuntimeRouteUnavailable)
        ));
    }

    #[test]
    fn exact_lookup_never_treats_unread_or_ambiguous_registry_as_authority() {
        let (mut snapshot, instance) = current();
        assert!(exact_instance(&snapshot, snapshot.project_id, snapshot.binding_id).is_ok());
        snapshot.instances = None;
        assert!(matches!(
            exact_instance(&snapshot, snapshot.project_id, snapshot.binding_id),
            Err(RuntimeError::RuntimeAuthorityProtocolError)
        ));
        let (mut snapshot, _) = current();
        snapshot.instances.as_mut().expect("page").items.clear();
        assert!(matches!(
            exact_instance(&snapshot, snapshot.project_id, snapshot.binding_id),
            Err(RuntimeError::RuntimeProjectionUnavailable)
        ));
        snapshot.instances.as_mut().expect("page").items = vec![instance.clone(), instance];
        assert!(matches!(
            exact_instance(&snapshot, snapshot.project_id, snapshot.binding_id),
            Err(RuntimeError::RuntimeAuthorityProtocolError)
        ));
    }

    #[test]
    fn current_mapping_requires_live_fresh_context_not_equal_replica_projection_version() {
        let (mut snapshot, mut instance) = current();
        assert!(check_current(&snapshot, &instance).is_ok());
        snapshot.cas.projection_version = RuntimeCounterV1::new(2).expect("counter");
        assert!(check_current(&snapshot, &instance).is_ok());
        instance.cas.grant_revision = RuntimeCounterV1::new(2).expect("counter");
        assert!(matches!(
            check_current(&snapshot, &instance),
            Err(RuntimeError::RuntimeStaleState)
        ));
        instance.cas.grant_revision = snapshot.cas.grant_revision;
        instance.valid_until_unix_ms =
            Some(RuntimeUnixMsV1::new(chrono::Utc::now().timestamp_millis()).expect("time"));
        assert!(matches!(
            check_current(&snapshot, &instance),
            Err(RuntimeError::RuntimeProjectionUnavailable)
        ));
        instance.state = RuntimeInstanceStateV1::Tombstone;
        assert!(matches!(
            check_current(&snapshot, &instance),
            Err(RuntimeError::RuntimeIdentityRevoked)
        ));
    }
}
