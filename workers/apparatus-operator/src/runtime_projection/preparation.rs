//! Bounded create -> complete observation attempt (§13.4). No receipt, adoption,
//! retry POST, child mutation or automatic Open. All transient state dies here.

use super::*;
use crate::runtime_observation::{PreparationChainUids, RuntimePodObserver};
use apparatus_contracts::runtime::{
    RuntimeBindingPreparationPermitV1, RuntimeObservationV1, RuntimeServiceRoleV1,
    RuntimeWriteAckV1, RuntimeWriteGuardV1,
};
use std::time::Duration;
use tokio::time::{sleep_until, timeout_at, Instant};

const WRITER_STEP: Duration = Duration::from_secs(2);
const RENEW_PERIOD: Duration = Duration::from_secs(3);

/// Monotone timing policy. `at` permits independent virtual-clock oracles; no
/// duration/deadline comes from a publisher, request or renewed lease.
pub struct PreparationWindow {
    start: Instant,
    rounds: u8,
    previous_round: Option<Instant>,
}
impl PreparationWindow {
    /// Freeze t0 before the single POST; never reset it on a response/retry.
    #[must_use]
    pub fn at(start: Instant) -> Self {
        Self {
            start,
            rounds: 0,
            previous_round: None,
        }
    }
    /// Absolute final deadline including create, discovery, report/confirmation.
    #[must_use]
    pub fn total_deadline(&self) -> Instant {
        self.start + Duration::from_secs(30)
    }
    /// Discovery stops early to reserve the final writer step.
    #[must_use]
    pub fn discovery_deadline(&self) -> Instant {
        self.start + Duration::from_secs(28)
    }
    /// Rounds start at least one second apart, not 28 unbounded watches.
    #[must_use]
    pub fn next_round_at(&self) -> Instant {
        self.previous_round
            .map_or(self.start, |previous| previous + Duration::from_secs(1))
    }
    /// Start a bounded round, rejecting exhausted/early rounds.
    ///
    /// # Errors
    /// Enforces 28 rounds, >=1s spacing, <=2s per round and the discovery cutoff.
    pub fn begin_round(&mut self, now: Instant) -> Result<Instant, RuntimeErrorCodeV1> {
        if now >= self.discovery_deadline() || self.rounds >= 28 {
            return Err(RuntimeErrorCodeV1::RuntimeDeadlineExceeded);
        }
        if now < self.next_round_at() {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        self.rounds += 1;
        self.previous_round = Some(now);
        Ok((now + Duration::from_secs(2)).min(self.discovery_deadline()))
    }
}

/// Platform-approved private route convention, with explicit port (no guessed
/// gateway or plaintext fallback). The URL itself must be observed from Serving.
pub struct ServingEndpointPolicy {
    dns_suffix: String,
    port: u16,
    transport: ServingEndpointTransport,
}

/// Explicit B-qualified transport, never automatic https -> http fallback.
/// Mesh HTTP denotes application HTTP carried by the REQUIRED strict mTLS mesh;
/// it is not an assertion that a plaintext network hop is acceptable.
pub enum ServingEndpointTransport {
    Https,
    MeshMtlsHttp,
}
impl ServingEndpointPolicy {
    /// Configure the initial plugin namespace's private cluster DNS and TLS port.
    ///
    /// # Errors
    /// Rejects public/wildcard suffixes, another namespace or a missing port.
    pub fn new(
        dns_suffix: String,
        port: u16,
        transport: ServingEndpointTransport,
    ) -> Result<Self, RuntimeErrorCodeV1> {
        if dns_suffix != "apparatus-plugins.svc.cluster.local" || port == 0 {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        Ok(Self {
            dns_suffix,
            port,
            transport,
        })
    }
    pub(super) fn validate(
        &self,
        snapshot: &RuntimeBindingSnapshotV1,
        endpoint: &str,
    ) -> Result<(), RuntimeErrorCodeV1> {
        if endpoint.len() > 2048 {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        let url =
            reqwest::Url::parse(endpoint).map_err(|_| RuntimeErrorCodeV1::RuntimeInvalidRequest)?;
        let host = format!("{}.{}", serving_name(snapshot), self.dns_suffix);
        let expected_scheme = match self.transport {
            ServingEndpointTransport::Https => "https",
            ServingEndpointTransport::MeshMtlsHttp => "http",
        };
        if url.scheme() != expected_scheme
            || url.host_str() != Some(host.as_str())
            || url.port_or_known_default() != Some(self.port)
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
        {
            return Err(RuntimeErrorCodeV1::RuntimeProjectionUnavailable);
        }
        Ok(())
    }
}

pub(super) fn validate_permit(
    snapshot: &RuntimeBindingSnapshotV1,
    profile: &ServingPlatformProfile,
    permit: &RuntimeBindingPreparationPermitV1,
) -> Result<(), RuntimeErrorCodeV1> {
    if snapshot.state != RuntimeBindingStateV1::Closed {
        return Err(RuntimeErrorCodeV1::RuntimeBindingClosed);
    }
    if snapshot.schema_version != 1
        || snapshot.project_id != permit.project_id
        || snapshot.binding_id != permit.binding_id
        || snapshot.cas != permit.cas
        || snapshot.release != permit.release
        || snapshot.profile != permit.profile
        || profile.profile_id != permit.profile
    {
        return Err(RuntimeErrorCodeV1::RuntimeStaleState);
    }
    if profile.approved_lock.as_ref() != Some(&permit.profile_lock_sha256)
        || profile.serving_endpoint.is_none()
    {
        return Err(RuntimeErrorCodeV1::RuntimeKeyUnavailable);
    }
    Ok(())
}

struct LeaseCoordinator {
    lease: RuntimeWriterLeaseV1,
    next_renew: Instant,
}
impl LeaseCoordinator {
    async fn renew(
        &mut self,
        bridge: &ServingRuntimeBridge,
        caller: &RuntimeControlCallerV1,
        snapshot: &RuntimeBindingSnapshotV1,
        limit: Instant,
    ) -> Result<(), RuntimeErrorCodeV1> {
        let started = Instant::now();
        let deadline = (started + WRITER_STEP).min(limit);
        if started >= deadline {
            return Err(RuntimeErrorCodeV1::RuntimeDeadlineExceeded);
        }
        self.lease = timeout_at(deadline, async {
            let selector = RuntimeInstanceSelectorV1::Page {
                cursor: None,
                page_size: Some(1),
            };
            let current = bridge
                .control
                .get_binding(caller, snapshot.project_id, snapshot.binding_id, &selector)
                .await
                .map_err(|error| error.code)?;
            if current.cas != snapshot.cas
                || current.state != RuntimeBindingStateV1::Closed
                || current.profile != snapshot.profile
                || current.release != snapshot.release
            {
                return Err(RuntimeErrorCodeV1::RuntimeStaleState);
            }
            bridge
                .control
                .writer_lease(
                    caller,
                    snapshot.project_id,
                    snapshot.binding_id,
                    &RuntimeWriterLeaseRequestV1 {
                        schema_version: 1,
                        holder_id: self.lease.proof.holder_id,
                        expected: snapshot.cas.clone(),
                        renew: Some(self.lease.proof.clone()),
                    },
                )
                .await
                .map_err(|error| error.code)
        })
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeAuthorityUnavailable)??;
        self.next_renew = started + RENEW_PERIOD;
        Ok(())
    }
}

impl ServingRuntimeBridge {
    /// Re-observe only a Service UID already committed by the writer. No create,
    /// update, adoption, deletion or Open exists in this path. An incomplete or
    /// failed read simply lets the previous <=5s observation expire.
    pub async fn refresh_known_parent(
        &self,
        project_id: uuid::Uuid,
        binding_id: uuid::Uuid,
    ) -> Result<RuntimeWriteAckV1, RuntimeErrorCodeV1> {
        let caller = RuntimeControlCallerV1 {
            role: RuntimeServiceRoleV1::Projector,
        };
        let selector = RuntimeInstanceSelectorV1::Page {
            cursor: None,
            page_size: Some(1),
        };
        let read_deadline = Instant::now() + WRITER_STEP;
        let (current, mut observation) = timeout_at(read_deadline, async {
            let current = self
                .control
                .get_binding(&caller, project_id, binding_id, &selector)
                .await
                .map_err(|error| error.code)?;
            if !matches!(
                current.state,
                RuntimeBindingStateV1::Active | RuntimeBindingStateV1::Closed
            ) {
                return Err(RuntimeErrorCodeV1::RuntimeBindingClosed);
            }
            let stored = current
                .observation
                .as_ref()
                .ok_or(RuntimeErrorCodeV1::RuntimeProjectionUnavailable)?;
            let parent_uid = stored.service_uid.clone();
            let revision_name = stored.revision_name.clone();
            let image = self.preparation_image(&current).await?;
            let lease = self
                .control
                .writer_lease(
                    &caller,
                    project_id,
                    binding_id,
                    &RuntimeWriterLeaseRequestV1 {
                        schema_version: 1,
                        holder_id: self.holder_id,
                        expected: current.cas.clone(),
                        renew: None,
                    },
                )
                .await
                .map_err(|error| error.code)?;
            let lease = self
                .control
                .writer_lease(
                    &caller,
                    project_id,
                    binding_id,
                    &RuntimeWriterLeaseRequestV1 {
                        schema_version: 1,
                        holder_id: self.holder_id,
                        expected: current.cas.clone(),
                        renew: Some(lease.proof),
                    },
                )
                .await
                .map_err(|error| error.code)?;
            let guard = RuntimeWriteGuardV1 {
                expected: current.cas.clone(),
                lease: lease.proof,
            };
            let observer = RuntimePodObserver::new(self.parents.parents.clone().into_client());
            let mut chain = PreparationChainUids::default();
            self.require_same_admission(&image).await?;
            let observed = observer
                .preparation_metadata(
                    &current,
                    &parent_uid,
                    &revision_name,
                    &image.image,
                    &guard,
                    &self.profile,
                    &mut chain,
                )
                .await?
                .ok_or(RuntimeErrorCodeV1::RuntimeProjectionUnavailable)?;
            // Both durable UIDs remain fixed; a same-name replacement never
            // becomes a fresh report or a reason to delete the foreign object.
            if observed.revision_uid != stored.revision_uid {
                return Err(RuntimeErrorCodeV1::RuntimeFenceConflict);
            }
            Ok((current, observed))
        })
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeProjectionUnavailable)??;
        let writer_deadline = Instant::now() + WRITER_STEP;
        timeout_at(writer_deadline, async {
            let fresh = self
                .control
                .get_binding(&caller, project_id, binding_id, &selector)
                .await
                .map_err(|error| error.code)?;
            if fresh.cas != current.cas
                || fresh.release != current.release
                || fresh.profile != current.profile
                || fresh.state != current.state
                || fresh
                    .observation
                    .as_ref()
                    .map(|o| (&o.service_uid, &o.revision_uid))
                    != current
                        .observation
                        .as_ref()
                        .map(|o| (&o.service_uid, &o.revision_uid))
            {
                return Err(RuntimeErrorCodeV1::RuntimeStaleState);
            }
            let lease = self
                .control
                .writer_lease(
                    &caller,
                    project_id,
                    binding_id,
                    &RuntimeWriterLeaseRequestV1 {
                        schema_version: 1,
                        holder_id: self.holder_id,
                        expected: fresh.cas.clone(),
                        renew: Some(observation.guard.lease.clone()),
                    },
                )
                .await
                .map_err(|error| error.code)?;
            observation.guard = RuntimeWriteGuardV1 {
                expected: fresh.cas.clone(),
                lease: lease.proof,
            };
            self.report_preparation(&caller, &fresh, &observation, writer_deadline)
                .await
        })
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeAuthorityUnavailable)?
    }
    /// Explicit task invoked ONLY with the trusted local preparation result.
    /// Closed bindings encountered by the normal reconciliation loop never take
    /// this path. No maintenance proof is reconstructed from an HTTP body.
    ///
    /// # Errors
    /// Any timeout/missing metadata/identity conflict suspends the preparation.
    /// A possible residual is kept unknown, not retried, adopted or deleted.
    pub async fn prepare_parent(
        &self,
        permit: &RuntimeBindingPreparationPermitV1,
    ) -> Result<RuntimeWriteAckV1, RuntimeErrorCodeV1> {
        let caller = RuntimeControlCallerV1 {
            role: RuntimeServiceRoleV1::Projector,
        };
        let selector = RuntimeInstanceSelectorV1::Page {
            cursor: None,
            page_size: Some(1),
        };
        let current = timeout_at(
            Instant::now() + WRITER_STEP,
            self.control
                .get_binding(&caller, permit.project_id, permit.binding_id, &selector),
        )
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeAuthorityUnavailable)?
        .map_err(|error| error.code)?;
        validate_permit(&current, &self.profile, permit)?;
        let image = self.preparation_image(&current).await?;
        let name = serving_name(&current);
        let existing = timeout_at(
            Instant::now() + WRITER_STEP,
            self.parents.parents.get_opt(&name),
        )
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeRouteUnavailable)?
        .map_err(kube_error)?;
        require_absent_for_create(&current, existing.is_some())?;
        let acquired_at = Instant::now();
        let lease = timeout_at(
            acquired_at + WRITER_STEP,
            self.control.writer_lease(
                &caller,
                current.project_id,
                current.binding_id,
                &RuntimeWriterLeaseRequestV1 {
                    schema_version: 1,
                    holder_id: self.holder_id,
                    expected: current.cas.clone(),
                    renew: None,
                },
            ),
        )
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeAuthorityUnavailable)?
        .map_err(|error| error.code)?;
        self.prepare_attempt(permit, &current, &image, lease, acquired_at)
            .await
    }

    // Private sequencing engine. The only production entry above obtains the
    // current writer state/permit and real Adm-A/Cosign image first. Inline tests
    // can isolate timing/IO without pretending to qualify admission or TLS.
    async fn prepare_attempt(
        &self,
        permit: &RuntimeBindingPreparationPermitV1,
        current: &RuntimeBindingSnapshotV1,
        image: &AdmittedServingImage,
        lease: RuntimeWriterLeaseV1,
        acquired_at: Instant,
    ) -> Result<RuntimeWriteAckV1, RuntimeErrorCodeV1> {
        let caller = RuntimeControlCallerV1 {
            role: RuntimeServiceRoleV1::Projector,
        };
        let desired = render_parent(
            &current,
            &image,
            &self.profile,
            lease.proof.writer_fence,
            Some(permit),
        )?;
        let revision_name = desired["spec"]["template"]["metadata"]["name"]
            .as_str()
            .ok_or(RuntimeErrorCodeV1::RuntimeInternal)?
            .to_owned();
        let desired: DynamicObject =
            serde_json::from_value(desired).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
        let window_start = Instant::now(); // t0 is frozen BEFORE the single POST.
        let mut window = PreparationWindow::at(window_start);
        let mut coordinator = LeaseCoordinator {
            lease,
            next_renew: acquired_at + RENEW_PERIOD,
        };
        let parent = {
            let params = PostParams::default();
            let create = self.parents.parents.create(&params, &desired);
            tokio::pin!(create);
            loop {
                tokio::select! {
                    biased;
                    _ = sleep_until(window.total_deadline()) => return Err(RuntimeErrorCodeV1::RuntimeDeadlineExceeded),
                    _ = sleep_until(coordinator.next_renew) => coordinator.renew(self, &caller, &current, window.total_deadline()).await?,
                    result = &mut create => break result.map_err(kube_error)?,
                }
            }
        };
        let parent_uid = parent
            .metadata
            .uid
            .as_deref()
            .filter(|uid| !uid.is_empty())
            .ok_or(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError)?
            .to_owned();
        if parent
            .metadata
            .resource_version
            .as_deref()
            .is_none_or(str::is_empty)
        {
            return Err(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError);
        }
        // Nothing serializes/logs this create provenance. A crash here requires
        // explicit maintenance inventory unless a full report already committed.
        let observer = RuntimePodObserver::new(self.parents.parents.clone().into_client());
        let mut chain = PreparationChainUids::default();
        loop {
            if Instant::now() >= window.discovery_deadline() {
                return Err(RuntimeErrorCodeV1::RuntimeDeadlineExceeded);
            }
            tokio::select! {
                biased;
                _ = sleep_until(window.discovery_deadline()) => return Err(RuntimeErrorCodeV1::RuntimeDeadlineExceeded),
                _ = sleep_until(coordinator.next_renew) => { coordinator.renew(self, &caller, &current, window.discovery_deadline()).await?; continue; },
                _ = sleep_until(window.next_round_at()) => {},
            }
            let round_deadline = window.begin_round(Instant::now())?;
            let guard = RuntimeWriteGuardV1 {
                expected: current.cas.clone(),
                lease: coordinator.lease.proof.clone(),
            };
            enum Round {
                Read(
                    Result<
                        Result<Option<RuntimeObservationV1>, RuntimeErrorCodeV1>,
                        tokio::time::error::Elapsed,
                    >,
                ),
                Renew,
            }
            let result = {
                let read = async {
                    self.require_same_admission(&image).await?;
                    observer
                        .preparation_metadata(
                            &current,
                            &parent_uid,
                            &revision_name,
                            &image.image,
                            &guard,
                            &self.profile,
                            &mut chain,
                        )
                        .await
                };
                tokio::pin!(read);
                tokio::select! {
                    biased;
                    _ = sleep_until(coordinator.next_renew) => Round::Renew,
                    result = timeout_at(round_deadline, &mut read) => Round::Read(result),
                }
            }; // incomplete read is DROPPED before a renewal, never silently extended.
            match result {
                Round::Renew => {
                    coordinator
                        .renew(self, &caller, &current, window.discovery_deadline())
                        .await?
                }
                Round::Read(Err(_)) | Round::Read(Ok(Ok(None))) => continue,
                Round::Read(Ok(Err(error))) => return Err(error),
                Round::Read(Ok(Ok(Some(observation)))) => {
                    return self
                        .report_preparation(
                            &caller,
                            &current,
                            &observation,
                            window.total_deadline(),
                        )
                        .await;
                }
            }
        }
    }

    pub(super) async fn preparation_image(
        &self,
        current: &RuntimeBindingSnapshotV1,
    ) -> Result<AdmittedServingImage, RuntimeErrorCodeV1> {
        let source = self.admission.clone();
        let target = self.admit_target.clone();
        let digest = crate::digest::ReleaseDigest::new(current.release.as_str())
            .map_err(|_| RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?;
        timeout_at(
            Instant::now() + WRITER_STEP,
            tokio::task::spawn_blocking(move || {
                let record = source.load_current(&digest)?;
                let image = AdmittedServingImage::verify_record(
                    record.clone(),
                    &digest,
                    Some(target.as_ref()),
                )?;
                if source.load_current(&digest)? != record {
                    return Err(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted);
                }
                Ok(image)
            }),
        )
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeAuthorityUnavailable)?
        .map_err(|_| RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?
    }

    pub(super) async fn require_same_admission(
        &self,
        image: &AdmittedServingImage,
    ) -> Result<(), RuntimeErrorCodeV1> {
        let source = self.admission.clone();
        let digest = crate::digest::ReleaseDigest::new(image.descriptor.as_str())
            .map_err(|_| RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?;
        let expected = image
            .record
            .clone()
            .ok_or(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)?;
        tokio::task::spawn_blocking(move || {
            if source.load_current(&digest)? != expected {
                return Err(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted);
            }
            Ok(())
        })
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeAuthorityUnavailable)?
    }

    async fn report_preparation(
        &self,
        caller: &RuntimeControlCallerV1,
        snapshot: &RuntimeBindingSnapshotV1,
        observation: &RuntimeObservationV1,
        global_deadline: Instant,
    ) -> Result<RuntimeWriteAckV1, RuntimeErrorCodeV1> {
        let deadline = (Instant::now() + WRITER_STEP).min(global_deadline);
        if Instant::now() >= deadline {
            return Err(RuntimeErrorCodeV1::RuntimeDeadlineExceeded);
        }
        match timeout_at(
            deadline,
            self.control.report_observation(
                caller,
                snapshot.project_id,
                snapshot.binding_id,
                observation,
            ),
        )
        .await
        {
            Ok(Ok(ack)) => return Ok(ack),
            Ok(Err(error))
                if !matches!(
                    error.code,
                    RuntimeErrorCodeV1::RuntimeAuthorityUnavailable
                        | RuntimeErrorCodeV1::RuntimeDeadlineExceeded
                ) =>
            {
                return Err(error.code)
            }
            _ => {}
        }
        // At most ONE confirmation GET, within this SAME 2s step and global30s.
        if Instant::now() >= deadline {
            return Err(RuntimeErrorCodeV1::RuntimeDeadlineExceeded);
        }
        let selector = RuntimeInstanceSelectorV1::Page {
            cursor: None,
            page_size: Some(1),
        };
        let confirmed = timeout_at(
            deadline,
            self.control
                .get_binding(caller, snapshot.project_id, snapshot.binding_id, &selector),
        )
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeAuthorityUnavailable)?
        .map_err(|error| error.code)?;
        let stored = confirmed
            .observation
            .as_ref()
            .ok_or(RuntimeErrorCodeV1::RuntimeProjectionUnavailable)?;
        if confirmed.state != snapshot.state
            || confirmed.release != snapshot.release
            || confirmed.profile != snapshot.profile
            || confirmed.cas.generation != snapshot.cas.generation
            || confirmed.cas.grant_revision != snapshot.cas.grant_revision
            || confirmed.cas.revocation_epoch != snapshot.cas.revocation_epoch
            || stored.service_uid != observation.service_uid
            || stored.revision_uid != observation.revision_uid
            || stored.revision_name != observation.revision_name
            || stored.release != observation.release
            || stored.endpoint != observation.endpoint
            || stored.route_ready != observation.route_ready
            || stored.resource_generation != observation.resource_generation
            || stored.observed_resource_generation != observation.observed_resource_generation
            || stored.observed_at_unix_ms != observation.observed_at_unix_ms
            || stored.writer_fence != observation.guard.lease.writer_fence
        {
            return Err(RuntimeErrorCodeV1::RuntimeProjectionUnavailable);
        }
        Ok(RuntimeWriteAckV1 {
            schema_version: 1,
            project_id: snapshot.project_id,
            binding_id: snapshot.binding_id,
            cas: confirmed.cas,
            writer_fence: stored.writer_fence,
        })
    }
}

fn require_absent_for_create(
    current: &RuntimeBindingSnapshotV1,
    exists: bool,
) -> Result<(), RuntimeErrorCodeV1> {
    if exists {
        // A durable UID goes to bounded re-observation, never this create task.
        return Err(if current.observation.is_some() {
            RuntimeErrorCodeV1::RuntimeStaleState
        } else {
            RuntimeErrorCodeV1::RuntimeFenceConflict
        });
    }
    if current.observation.is_some() {
        return Err(RuntimeErrorCodeV1::RuntimeFenceConflict);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn preparation_clock_is_absolute_and_rounds_reserve_final_writer_budget() {
        let start = Instant::now();
        let mut window = PreparationWindow::at(start);
        assert_eq!(
            window.begin_round(start).expect("first"),
            start + Duration::from_secs(2)
        );
        assert!(window
            .begin_round(start + Duration::from_millis(999))
            .is_err());
        for second in 1..28 {
            let now = start + Duration::from_secs(second);
            assert!(window.begin_round(now).expect("round") <= window.discovery_deadline());
        }
        assert!(window.begin_round(start + Duration::from_secs(28)).is_err());
        tokio::time::advance(Duration::from_secs(30)).await;
        assert_eq!(Instant::now(), window.total_deadline());
        assert_eq!(window.discovery_deadline(), start + Duration::from_secs(28));
    }
}

#[cfg(test)]
#[path = "preparation_sequence_tests.rs"]
mod sequence_tests;
