//! Local, explicitly authorized maintenance. No HTTP/queue registration and no
//! deserialized receipt can obtain a maintenance session. All positive writes,
//! challenge consumption and journal entries commit together in the writer DB.

use super::*;
use crate::apparatus_runtime::admission_proof::{
    RejectRuntimeAdmissionProofVerifier, RuntimeAdmissionProofVerifier, VerifiedRuntimeAdmissionV1,
};
use apparatus_contracts::runtime::*;
use serde_json::{json, Value};
use std::{future::Future, pin::Pin};

#[derive(Clone, Copy)]
pub enum RuntimeMaintenanceAct {
    Inspect,
    Fence,
    Challenge(RuntimeAdmissionActionV1),
    Prepare,
    ResumePrepare,
    Open,
    BlockRelease,
}

/// Returned only by the setup-injected trusted maintenance channel. Fields are
/// data, not proof when copied out of this channel into JSON.
pub struct RuntimeMaintenanceFacts {
    pub operator_id: Uuid,
    pub installation_ack: RuntimeLegacyFenceInstallationAckV1,
    pub drain_evidence_sha256: Option<RuntimeDigestV1>,
}

/// Contract: authenticate the human operator and durable D/database/deployment
/// provenance, verify full inventory and (where required) drain/recovery scope.
/// No implementation is inferred from service mTLS or a caller-provided digest.
pub trait RuntimeMaintenanceChannel: Send + Sync {
    fn authorize<'a>(
        &'a self,
        act: RuntimeMaintenanceAct,
        plan: &'a RuntimeBindingCutoverPlanV1,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<RuntimeMaintenanceFacts, RuntimeControlErrorV1>> + Send + 'a,
        >,
    >;
}

/// Immutable B configuration, not loaded from the proof/operator request.
pub struct RuntimeMaintenanceProfile {
    pub authority_id: Uuid,
    pub installation_id: Uuid,
    pub profile: String,
    pub profile_lock_sha256: RuntimeDigestV1,
    /// Explicit B-qualified HTTPS or HTTP carried by required strict mesh mTLS.
    pub endpoint_scheme: String,
    pub endpoint_port: u16,
}

pub struct SqlRuntimeMaintenance {
    db: Arc<DatabaseConnection>,
    profile: Option<RuntimeMaintenanceProfile>,
    channel: Option<Arc<dyn RuntimeMaintenanceChannel>>,
    verifier: Arc<dyn RuntimeAdmissionProofVerifier>,
}

pub use manifesto_application::RuntimeAdmissionClosureAck;

impl manifesto_application::RuntimeMaintenanceExecutor for SqlRuntimeMaintenance {
    fn execute_maintenance<'a>(
        &'a self,
        action: manifesto_application::RuntimeMaintenanceAction,
    ) -> RuntimeControlFuture<'a, manifesto_application::RuntimeMaintenanceReply> {
        use manifesto_application::{RuntimeMaintenanceAction as A, RuntimeMaintenanceReply as R};
        Box::pin(async move {
            match action {
                A::Inspect(plan) => self.inspect(&plan).await.map(R::Inspection),
                A::Fence(command) => self.fence(&command).await.map(R::Cas),
                A::Challenge {
                    plan,
                    action,
                    expected,
                    service_uid,
                    revision_uid,
                } => self
                    .challenge(
                        &plan,
                        action,
                        &expected,
                        service_uid.as_deref(),
                        revision_uid.as_deref(),
                    )
                    .await
                    .map(R::Challenge),
                A::Prepare {
                    plan,
                    command,
                    challenge_id,
                    proof,
                } => self
                    .prepare(&plan, &command, challenge_id, &proof)
                    .await
                    .map(R::Permit),
                A::ResumePrepare {
                    plan,
                    command,
                    challenge_id,
                    proof,
                } => self
                    .resume_and_prepare(&plan, &command, challenge_id, &proof)
                    .await
                    .map(R::Permit),
                A::Open {
                    plan,
                    command,
                    challenge_id,
                    proof,
                } => self
                    .open(&plan, &command, challenge_id, &proof)
                    .await
                    .map(R::Cas),
                A::BlockRelease {
                    plan,
                    record_id,
                    revision,
                } => self
                    .block_release(&plan, record_id, revision)
                    .await
                    .map(R::Closure),
            }
        })
    }
}

impl SqlRuntimeMaintenance {
    /// Authenticated local read after an uncertain act. No reapplication, nonce
    /// refresh or reconstruction of trusted provenance from the returned data.
    pub async fn inspect(
        &self,
        plan: &RuntimeBindingCutoverPlanV1,
    ) -> Result<manifesto_application::RuntimeMaintenanceInspection, RuntimeControlErrorV1> {
        let facts = self.authorize(RuntimeMaintenanceAct::Inspect, plan).await?;
        let txn = self.db.begin().await.map_err(db_error)?;
        // Pure inspection takes no admission guard and no business mutation
        // locks; a shared runtime lock makes phase+journal+consumption coherent.
        let row=txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT * FROM apparatus_runtime_bindings WHERE project_id=$1 AND binding_id=$2 FOR SHARE",
            [plan.project_id.into(),plan.binding_id.into()])).await.map_err(db_error)?.ok_or_else(||error(RuntimeErrorCodeV1::RuntimeBindingNotFound))?;
        require_plan(&row, plan)?;
        check_ack_time(&facts, db_clock(&txn).await?)?;
        let consumed=txn.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT challenge_id FROM apparatus_runtime_admission_challenges WHERE project_id=$1 AND binding_id=$2 AND consumed_at_unix_ms IS NOT NULL ORDER BY challenge_id",
            [plan.project_id.into(),plan.binding_id.into()])).await.map_err(db_error)?;
        let mut ids = Vec::with_capacity(consumed.len());
        for record in consumed {
            ids.push(record.try_get("", "challenge_id").map_err(db_error)?);
        }
        let journal: Value = row.try_get("", "cutover_journal").map_err(db_error)?;
        let inspection = manifesto_application::RuntimeMaintenanceInspection {
            cas: row_cas(&row)?,
            phase: row.try_get("", "cutover_phase").map_err(db_error)?,
            journal_json: journal.to_string(),
            consumed_challenges: ids,
        };
        txn.commit().await.map_err(db_error)?;
        Ok(inspection)
    }
    /// Disabled by default, including Fence (D provenance is still mandatory).
    pub fn disabled(db: impl Into<Arc<DatabaseConnection>>) -> Self {
        Self {
            db: db.into(),
            profile: None,
            channel: None,
            verifier: Arc::new(RejectRuntimeAdmissionProofVerifier),
        }
    }

    /// Composition-root injection only. This does not register generic commands
    /// or fabricate a trusted channel when one is not available.
    pub fn with_verified_channel(
        mut self,
        profile: RuntimeMaintenanceProfile,
        channel: Arc<dyn RuntimeMaintenanceChannel>,
        verifier: Arc<dyn RuntimeAdmissionProofVerifier>,
    ) -> Result<Self, RuntimeControlErrorV1> {
        if profile.authority_id.is_nil()
            || profile.installation_id.is_nil()
            || profile.profile.is_empty()
            || profile.profile.len() > 128
            || !matches!(profile.endpoint_scheme.as_str(), "https" | "http")
            || profile.endpoint_port == 0
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeKeyUnavailable));
        }
        self.profile = Some(profile);
        self.channel = Some(channel);
        self.verifier = verifier;
        Ok(self)
    }

    async fn authorize(
        &self,
        act: RuntimeMaintenanceAct,
        plan: &RuntimeBindingCutoverPlanV1,
    ) -> Result<RuntimeMaintenanceFacts, RuntimeControlErrorV1> {
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?;
        if plan.schema_version != 1
            || plan.migration_id.is_nil()
            || plan.profile != profile.profile
            || plan.profile_lock_sha256 != profile.profile_lock_sha256
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
        }
        let facts = self
            .channel
            .as_ref()
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeAuthorityUnavailable))?
            .authorize(act, plan)
            .await?;
        let ack = &facts.installation_ack;
        if facts.operator_id.is_nil()
            || ack.migration_id != plan.migration_id
            || ack.project_id != plan.project_id
            || ack.binding_id != plan.binding_id
            || ack.inventory_sha256 != plan.inventory_sha256
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidProof));
        }
        Ok(facts)
    }

    /// Materialize exactly one already disabled business binding. Existing
    /// migration/tombstone history is never replaced or reset by a retry.
    pub async fn fence(
        &self,
        command: &FenceRuntimeBindingV1,
    ) -> Result<RuntimeCasV1, RuntimeControlErrorV1> {
        let plan = &command.plan;
        let facts = self.authorize(RuntimeMaintenanceAct::Fence, plan).await?;
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?;
        let txn = self.db.begin().await.map_err(db_error)?;
        // The release guard is always first, even though Fence needs no positive
        // admission proof. Initial guard creation is explicit trusted setup work.
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO apparatus_runtime_admission_guards(authority_id,release,blocked,reconciliation_required) VALUES($1,$2,false,false) ON CONFLICT DO NOTHING",
            [profile.authority_id.into(),plan.release.as_str().into()])).await.map_err(db_error)?;
        lock_guard(&txn, profile.authority_id, &plan.release).await?;
        let domain = lock_business(&txn, plan).await?;
        require_disabled(&domain)?;
        let existing = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT * FROM apparatus_runtime_bindings WHERE project_id=$1 AND binding_id=$2 FOR UPDATE",
            [plan.project_id.into(),plan.binding_id.into()])).await.map_err(db_error)?;
        let now = db_clock(&txn).await?;
        check_ack_time(&facts, now)?;
        if let Some(existing) = existing {
            require_plan(&existing, plan)?;
            if existing.try_get::<String>("", "state").map_err(db_error)? != "closed" {
                return Err(error(RuntimeErrorCodeV1::RuntimeBindingClosed));
            }
            let cas = row_cas(&existing)?;
            if facts
                .installation_ack
                .observed_writer_cas
                .as_ref()
                .is_some_and(|expected| expected != &cas)
            {
                return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
            }
            txn.commit().await.map_err(db_error)?;
            return Ok(cas);
        }
        if facts.installation_ack.observed_writer_cas.is_some() {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        let entry = json!({"act":"fence","migration_id":plan.migration_id,"operator_id":facts.operator_id,"at_unix_ms":now});
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO apparatus_runtime_bindings(project_id,binding_id,generation,grant_revision,revocation_epoch,projection_version,state,release,profile,cutover_migration_id,cutover_phase,cutover_plan,cutover_journal,profile_lock_sha256,inventory_sha256,updated_at_ms) \
             VALUES($1,$2,$3,$4,1,1,'closed',$5,$6,$7,'fenced',$8,$9,$10,$11,$12)",
            [plan.project_id.into(),plan.binding_id.into(),sql_counter(plan.expected_generation)?.into(),sql_counter(plan.expected_grant_revision)?.into(),
                plan.release.as_str().into(),plan.profile.clone().into(),plan.migration_id.into(),plan_json(plan).into(),json!([entry]).into(),
                plan.profile_lock_sha256.as_str().into(),plan.inventory_sha256.as_str().into(),now.into()])).await.map_err(db_error)?;
        let row = lock_binding(&txn, plan.project_id, plan.binding_id).await?;
        let cas = row_cas(&row)?;
        txn.commit().await.map_err(db_error)?;
        Ok(cas)
    }

    /// Writer clock/nonce, full CAS, exact binding/action and UID intent. Issuing
    /// an intention enables nothing; every prior pending same-action is fenced.
    pub async fn challenge(
        &self,
        plan: &RuntimeBindingCutoverPlanV1,
        action: RuntimeAdmissionActionV1,
        expected: &RuntimeCasV1,
        service_uid: Option<&str>,
        revision_uid: Option<&str>,
    ) -> Result<RuntimeAdmissionChallengeV1, RuntimeControlErrorV1> {
        let facts = self
            .authorize(RuntimeMaintenanceAct::Challenge(action), plan)
            .await?;
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?;
        let txn = self.db.begin().await.map_err(db_error)?;
        lock_guard(&txn, profile.authority_id, &plan.release).await?;
        let domain = lock_business(&txn, plan).await?;
        require_disabled(&domain)?;
        let row = lock_binding(&txn, plan.project_id, plan.binding_id).await?;
        require_intent(
            &row,
            plan,
            expected,
            match action {
                RuntimeAdmissionActionV1::Prepare => "fenced",
                RuntimeAdmissionActionV1::Open => "prepared",
            },
        )?;
        match action {
            RuntimeAdmissionActionV1::Prepare
                if service_uid.is_none() && revision_uid.is_none() => {}
            RuntimeAdmissionActionV1::Open => {
                let service =
                    service_uid.ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInvalidRequest))?;
                let revision =
                    revision_uid.ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInvalidRequest))?;
                require_observation(&txn, &row, plan, service, revision, profile).await?;
            }
            _ => return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest)),
        }
        let now = db_clock(&txn).await?;
        check_ack_time(&facts, now)?;
        let challenge = RuntimeAdmissionChallengeV1 {
            schema_version: 1,
            challenge_id: Uuid::new_v4(),
            authority_id: profile.authority_id,
            installation_id: profile.installation_id,
            action,
            migration_id: plan.migration_id,
            project_id: plan.project_id,
            binding_id: plan.binding_id,
            expected: expected.clone(),
            release: plan.release.clone(),
            profile: plan.profile.clone(),
            profile_lock_sha256: plan.profile_lock_sha256.clone(),
            inventory_sha256: plan.inventory_sha256.clone(),
            service_uid: service_uid.map(str::to_owned),
            revision_uid: revision_uid.map(str::to_owned),
            issued_at_unix_ms: milliseconds(now)?,
            expires_at_unix_ms: milliseconds(
                now.checked_add(ADMISSION_CHALLENGE_TTL_MS)
                    .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInternal))?,
            )?,
        };
        let action = action_string(action);
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE apparatus_runtime_admission_challenges SET invalidated=true WHERE migration_id=$1 AND action=$2 AND consumed_at_unix_ms IS NULL",
            [plan.migration_id.into(),action.into()])).await.map_err(db_error)?;
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO apparatus_runtime_admission_challenges(challenge_id,authority_id,installation_id,migration_id,project_id,binding_id,action,release,intention,issued_at_unix_ms,expires_at_unix_ms) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            [challenge.challenge_id.into(),profile.authority_id.into(),profile.installation_id.into(),plan.migration_id.into(),plan.project_id.into(),plan.binding_id.into(),action.into(),plan.release.as_str().into(),
                serde_json::to_value(&challenge).map_err(|_| error(RuntimeErrorCodeV1::RuntimeInternal))?.into(),now.into(),challenge.expires_at_unix_ms.get().into()])).await.map_err(db_error)?;
        txn.commit().await.map_err(db_error)?;
        Ok(challenge)
    }

    pub async fn prepare(
        &self,
        plan: &RuntimeBindingCutoverPlanV1,
        command: &PrepareRuntimeBindingV1,
        challenge_id: Uuid,
        proof: &str,
    ) -> Result<RuntimeBindingPreparationPermitV1, RuntimeControlErrorV1> {
        self.prepare_inner(plan, command, challenge_id, proof, false)
            .await
    }

    /// Explicit privileged resume, not an automatic response to a newer proof.
    /// Channel must verify recovery/provenance and the human-approved unblock.
    pub async fn resume_and_prepare(
        &self,
        plan: &RuntimeBindingCutoverPlanV1,
        command: &PrepareRuntimeBindingV1,
        challenge_id: Uuid,
        proof: &str,
    ) -> Result<RuntimeBindingPreparationPermitV1, RuntimeControlErrorV1> {
        self.prepare_inner(plan, command, challenge_id, proof, true)
            .await
    }

    async fn prepare_inner(
        &self,
        plan: &RuntimeBindingCutoverPlanV1,
        command: &PrepareRuntimeBindingV1,
        challenge_id: Uuid,
        proof: &str,
        explicit_resume: bool,
    ) -> Result<RuntimeBindingPreparationPermitV1, RuntimeControlErrorV1> {
        let facts = self
            .authorize(
                if explicit_resume {
                    RuntimeMaintenanceAct::ResumePrepare
                } else {
                    RuntimeMaintenanceAct::Prepare
                },
                plan,
            )
            .await?;
        require_command(
            plan,
            command.schema_version,
            command.migration_id,
            command.project_id,
            command.binding_id,
        )?;
        if facts.drain_evidence_sha256.as_ref() != Some(&command.drain_evidence_sha256) {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidProof));
        }
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?;
        let txn = self.db.begin().await.map_err(db_error)?;
        let guard = lock_guard(&txn, profile.authority_id, &plan.release).await?;
        let domain = lock_business(&txn, plan).await?;
        require_disabled(&domain)?;
        require_consents(&txn, &domain, plan).await?;
        let row = lock_binding(&txn, plan.project_id, plan.binding_id).await?;
        require_intent(&row, plan, &command.expected, "fenced")?;
        let now = db_clock(&txn).await?;
        check_ack_time(&facts, now)?;
        let verified = self
            .verify_challenge(
                &txn,
                plan,
                &command.expected,
                RuntimeAdmissionActionV1::Prepare,
                challenge_id,
                proof,
                now,
            )
            .await?;
        check_guard(&txn, &guard, &verified, explicit_resume).await?;
        let now = db_clock(&txn).await?;
        self.verifier
            .verify(&verified.claims().challenge, proof, milliseconds(now)?)
            .map_err(error)?;
        consume(&txn, &verified, profile.authority_id, now, explicit_resume).await?;
        let mut cas = row_cas(&row)?;
        cas.projection_version = cas.projection_version.checked_next().map_err(error)?;
        let entry = json!({"act":"prepare","operator_id":facts.operator_id,"challenge_id":challenge_id,"at_unix_ms":now,"cas":cas});
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE apparatus_runtime_bindings SET cutover_phase='prepared',drain_evidence_sha256=$3,projection_version=$4,cutover_journal=cutover_journal || $5::jsonb,updated_at_ms=$6 WHERE project_id=$1 AND binding_id=$2",
            [plan.project_id.into(),plan.binding_id.into(),command.drain_evidence_sha256.as_str().into(),sql_counter(cas.projection_version)?.into(),json!([entry]).into(),now.into()])).await.map_err(db_error)?;
        txn.commit().await.map_err(db_error)?;
        Ok(RuntimeBindingPreparationPermitV1 {
            migration_id: plan.migration_id,
            project_id: plan.project_id,
            binding_id: plan.binding_id,
            cas,
            release: plan.release.clone(),
            profile: plan.profile.clone(),
            profile_lock_sha256: plan.profile_lock_sha256.clone(),
            inventory_sha256: plan.inventory_sha256.clone(),
        })
    }

    pub async fn open(
        &self,
        plan: &RuntimeBindingCutoverPlanV1,
        command: &OpenRuntimeBindingV1,
        challenge_id: Uuid,
        proof: &str,
    ) -> Result<RuntimeCasV1, RuntimeControlErrorV1> {
        let facts = self.authorize(RuntimeMaintenanceAct::Open, plan).await?;
        require_command(
            plan,
            command.schema_version,
            command.migration_id,
            command.project_id,
            command.binding_id,
        )?;
        if command.profile_lock_sha256 != plan.profile_lock_sha256 {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidProof));
        }
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?;
        let txn = self.db.begin().await.map_err(db_error)?;
        let guard = lock_guard(&txn, profile.authority_id, &plan.release).await?;
        let domain = lock_business(&txn, plan).await?;
        require_disabled(&domain)?;
        require_consents(&txn, &domain, plan).await?;
        let row = lock_binding(&txn, plan.project_id, plan.binding_id).await?;
        require_intent(&row, plan, &command.expected, "prepared")?;
        if row
            .try_get::<Option<String>>("", "drain_evidence_sha256")
            .map_err(db_error)?
            .as_deref()
            != facts
                .drain_evidence_sha256
                .as_ref()
                .map(|digest| digest.as_str())
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidProof));
        }
        require_observation(
            &txn,
            &row,
            plan,
            &command.service_uid,
            &command.revision_uid,
            profile,
        )
        .await?;
        let now = db_clock(&txn).await?;
        check_ack_time(&facts, now)?;
        let verified = self
            .verify_challenge(
                &txn,
                plan,
                &command.expected,
                RuntimeAdmissionActionV1::Open,
                challenge_id,
                proof,
                now,
            )
            .await?;
        let claims = verified.claims();
        if claims.challenge.service_uid.as_deref() != Some(command.service_uid.as_str())
            || claims.challenge.revision_uid.as_deref() != Some(command.revision_uid.as_str())
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidProof));
        }
        let prepared=txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT j.* FROM apparatus_runtime_admission_journal j JOIN apparatus_runtime_admission_challenges c USING(challenge_id) WHERE c.migration_id=$1 AND c.action='prepare' ORDER BY j.committed_at_unix_ms DESC,c.challenge_id DESC LIMIT 1",
            [plan.migration_id.into()])).await.map_err(db_error)?.ok_or_else(||error(RuntimeErrorCodeV1::RuntimeInvalidProof))?;
        if prepared
            .try_get::<Uuid>("", "record_id")
            .map_err(db_error)?
            != claims.record_id
            || prepared
                .try_get::<i64>("", "admission_revision")
                .map_err(db_error)?
                != sql_counter(claims.admission_revision)?
            || prepared
                .try_get::<String>("", "policy_id")
                .map_err(db_error)?
                != claims.policy_id
            || prepared
                .try_get::<String>("", "report_sha256")
                .map_err(db_error)?
                != claims.report_sha256.as_str()
            || prepared
                .try_get::<String>("", "envelope_reference")
                .map_err(db_error)?
                != claims.envelope_reference
            || prepared
                .try_get::<String>("", "cri_image")
                .map_err(db_error)?
                != claims.cri_image
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted));
        }
        check_guard(&txn, &guard, &verified, false).await?;
        let now = db_clock(&txn).await?;
        self.verifier
            .verify(&verified.claims().challenge, proof, milliseconds(now)?)
            .map_err(error)?;
        // Recheck freshness after any metadata reads and verification work.
        require_observation(
            &txn,
            &row,
            plan,
            &command.service_uid,
            &command.revision_uid,
            profile,
        )
        .await?;
        consume(&txn, &verified, profile.authority_id, now, false).await?;
        let mut cas = row_cas(&row)?;
        cas.projection_version = cas.projection_version.checked_next().map_err(error)?;
        // Explicit runtime-exclusive business activation: no legacy activation
        // command/event is emitted. The irreversible runtime marker stays put.
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE project_components SET status='active',updated_at=clock_timestamp() WHERE id=$1 AND project_id=$2 AND status='disabled'",
            [plan.binding_id.into(),plan.project_id.into()])).await.map_err(db_error)?;
        let entry = json!({"act":"open","operator_id":facts.operator_id,"challenge_id":challenge_id,"service_uid":command.service_uid,"revision_uid":command.revision_uid,"at_unix_ms":now,"cas":cas});
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE apparatus_runtime_bindings SET state='active',cutover_phase='opened',projection_version=$3,cutover_journal=cutover_journal || $4::jsonb,updated_at_ms=$5 WHERE project_id=$1 AND binding_id=$2",
            [plan.project_id.into(),plan.binding_id.into(),sql_counter(cas.projection_version)?.into(),json!([entry]).into(),now.into()])).await.map_err(db_error)?;
        txn.commit().await.map_err(db_error)?;
        Ok(cas)
    }

    /// Negative barrier FIRST: stop new Prepare/Open and close all writer
    /// consumers before Adm-A may commit withdrawal. The common release guard
    /// prevents a concurrent new consumer escaping this enumeration.
    pub async fn block_release(
        &self,
        plan: &RuntimeBindingCutoverPlanV1,
        record_id: Uuid,
        revision: RuntimeCounterV1,
    ) -> Result<RuntimeAdmissionClosureAck, RuntimeControlErrorV1> {
        let facts = self
            .authorize(RuntimeMaintenanceAct::BlockRelease, plan)
            .await?;
        if record_id.is_nil() || revision.get() == 0 {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
        }
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?;
        let txn = self.db.begin().await.map_err(db_error)?;
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO apparatus_runtime_admission_guards(authority_id,release) VALUES($1,$2) ON CONFLICT DO NOTHING",
            [profile.authority_id.into(),plan.release.as_str().into()])).await.map_err(db_error)?;
        let guard = lock_guard(&txn, profile.authority_id, &plan.release).await?;
        if guard
            .try_get::<Option<Uuid>>("", "record_id")
            .map_err(db_error)?
            .is_some_and(|old| old != record_id)
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeIdReused));
        }
        let maximum =
            counter(guard.try_get("", "maximum_revision").map_err(db_error)?)?.max(revision);
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE apparatus_runtime_admission_guards SET record_id=$3,maximum_revision=$4,blocked=true WHERE authority_id=$1 AND release=$2",
            [profile.authority_id.into(),plan.release.as_str().into(),record_id.into(),sql_counter(maximum)?.into()])).await.map_err(db_error)?;
        let consumers=txn.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT project_id,binding_id FROM apparatus_runtime_bindings WHERE release=$1 ORDER BY project_id,binding_id",[plan.release.as_str().into()])).await.map_err(db_error)?;
        let mut closed = Vec::new();
        for consumer in consumers {
            let project: Uuid = consumer.try_get("", "project_id").map_err(db_error)?;
            let binding: Uuid = consumer.try_get("", "binding_id").map_err(db_error)?;
            closed.push((project, binding));
        }
        for project in closed
            .iter()
            .map(|(project, _)| *project)
            .collect::<std::collections::BTreeSet<_>>()
        {
            crate::transaction::lock_project_row(&txn, project)
                .await
                .map_err(|_| error(RuntimeErrorCodeV1::RuntimeAuthorityUnavailable))?;
        }
        // All business locks precede ALL runtime locks, including two bindings
        // of one project. Interleaving domain/runtime per binding deadlocks with
        // begin-effect's sorted multi-binding lock order.
        for &(project, binding) in &closed {
            txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT ab.component_id FROM apparatus_bindings ab JOIN project_components pc ON pc.id=ab.component_id WHERE pc.project_id=$1 AND pc.id=$2 FOR UPDATE OF ab,pc",
                [project.into(),binding.into()])).await.map_err(db_error)?;
        }
        for &(project, binding) in &closed {
            let row = lock_binding(&txn, project, binding).await?;
            let cas = row_cas(&row)?;
            close_binding_in_transaction(
                &txn,
                project,
                binding,
                cas.generation,
                cas.grant_revision,
                false,
            )
            .await?;
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE project_components SET status='disabled',updated_at=clock_timestamp() WHERE id=$1 AND project_id=$2 AND status='active'",
                [binding.into(),project.into()])).await.map_err(db_error)?;
        }
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE apparatus_runtime_admission_challenges SET invalidated=true WHERE authority_id=$1 AND release=$2 AND consumed_at_unix_ms IS NULL",
            [profile.authority_id.into(),plan.release.as_str().into()])).await.map_err(db_error)?;
        let now = db_clock(&txn).await?;
        check_ack_time(&facts, now)?;
        txn.commit().await.map_err(db_error)?;
        Ok(RuntimeAdmissionClosureAck {
            authority_id: profile.authority_id,
            release: plan.release.clone(),
            record_id,
            maximum_revision: maximum,
            closed_bindings: closed,
        })
    }

    async fn verify_challenge(
        &self,
        txn: &DatabaseTransaction,
        plan: &RuntimeBindingCutoverPlanV1,
        expected: &RuntimeCasV1,
        action: RuntimeAdmissionActionV1,
        challenge_id: Uuid,
        proof: &str,
        _now: i64,
    ) -> Result<VerifiedRuntimeAdmissionV1, RuntimeControlErrorV1> {
        let stored=txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT * FROM apparatus_runtime_admission_challenges WHERE challenge_id=$1 FOR UPDATE",[challenge_id.into()])).await.map_err(db_error)?
            .ok_or_else(||error(RuntimeErrorCodeV1::RuntimeInvalidProof))?;
        if stored
            .try_get::<bool>("", "invalidated")
            .map_err(db_error)?
            || stored
                .try_get::<Option<i64>>("", "consumed_at_unix_ms")
                .map_err(db_error)?
                .is_some()
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeReplay));
        }
        let intention: Value = stored.try_get("", "intention").map_err(db_error)?;
        let challenge: RuntimeAdmissionChallengeV1 = serde_json::from_value(intention)
            .map_err(|_| error(RuntimeErrorCodeV1::RuntimeInvalidProof))?;
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?;
        if challenge.challenge_id != challenge_id
            || challenge.authority_id != profile.authority_id
            || challenge.installation_id != profile.installation_id
            || challenge.migration_id != plan.migration_id
            || challenge.project_id != plan.project_id
            || challenge.binding_id != plan.binding_id
            || challenge.action != action
            || challenge.expected != *expected
            || challenge.release != plan.release
            || challenge.profile != plan.profile
            || challenge.profile_lock_sha256 != plan.profile_lock_sha256
            || challenge.inventory_sha256 != plan.inventory_sha256
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        let now = db_clock(txn).await?;
        self.verifier
            .verify(&challenge, proof, milliseconds(now)?)
            .map_err(error)
    }
}

fn action_string(action: RuntimeAdmissionActionV1) -> &'static str {
    match action {
        RuntimeAdmissionActionV1::Prepare => "prepare",
        RuntimeAdmissionActionV1::Open => "open",
    }
}
fn plan_json(p: &RuntimeBindingCutoverPlanV1) -> Value {
    json!({"schema_version":p.schema_version,"migration_id":p.migration_id,"project_id":p.project_id,"binding_id":p.binding_id,"expected_generation":p.expected_generation,"expected_grant_revision":p.expected_grant_revision,"release":p.release,"profile":p.profile,"profile_lock_sha256":p.profile_lock_sha256,"inventory_sha256":p.inventory_sha256})
}
fn require_command(
    p: &RuntimeBindingCutoverPlanV1,
    version: u16,
    migration: Uuid,
    project: Uuid,
    binding: Uuid,
) -> Result<(), RuntimeControlErrorV1> {
    if version != 1
        || migration != p.migration_id
        || project != p.project_id
        || binding != p.binding_id
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
    }
    Ok(())
}
fn check_ack_time(facts: &RuntimeMaintenanceFacts, now: i64) -> Result<(), RuntimeControlErrorV1> {
    if facts.installation_ack.installed_at_unix_ms.get() > now {
        return Err(error(RuntimeErrorCodeV1::RuntimeInvalidProof));
    }
    Ok(())
}
fn require_plan(
    row: &QueryResult,
    p: &RuntimeBindingCutoverPlanV1,
) -> Result<(), RuntimeControlErrorV1> {
    if row
        .try_get::<Option<Uuid>>("", "cutover_migration_id")
        .map_err(db_error)?
        != Some(p.migration_id)
        || row
            .try_get::<Option<Value>>("", "cutover_plan")
            .map_err(db_error)?
            != Some(plan_json(p))
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeIdReused));
    }
    Ok(())
}
fn require_intent(
    row: &QueryResult,
    p: &RuntimeBindingCutoverPlanV1,
    expected: &RuntimeCasV1,
    phase: &str,
) -> Result<(), RuntimeControlErrorV1> {
    require_plan(row, p)?;
    if row_cas(row)? != *expected
        || row.try_get::<String>("", "state").map_err(db_error)? != "closed"
        || row
            .try_get::<Option<String>>("", "cutover_phase")
            .map_err(db_error)?
            .as_deref()
            != Some(phase)
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
    }
    Ok(())
}
async fn lock_guard(
    txn: &DatabaseTransaction,
    authority: Uuid,
    release: &RuntimeDigestV1,
) -> Result<QueryResult, RuntimeControlErrorV1> {
    txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT * FROM apparatus_runtime_admission_guards WHERE authority_id=$1 AND release=$2 FOR UPDATE",[authority.into(),release.as_str().into()])).await.map_err(db_error)?.ok_or_else(||error(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted))
}
async fn lock_business(
    txn: &DatabaseTransaction,
    p: &RuntimeBindingCutoverPlanV1,
) -> Result<QueryResult, RuntimeControlErrorV1> {
    crate::transaction::lock_project_row(txn, p.project_id)
        .await
        .map_err(|_| error(RuntimeErrorCodeV1::RuntimeAuthorityUnavailable))?;
    let row=txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT ab.*,pc.status AS component_status,p.status AS project_status FROM apparatus_bindings ab JOIN project_components pc ON pc.id=ab.component_id JOIN projects p ON p.id=pc.project_id WHERE pc.project_id=$1 AND pc.id=$2 FOR UPDATE OF ab,pc",
        [p.project_id.into(),p.binding_id.into()])).await.map_err(db_error)?.ok_or_else(||error(RuntimeErrorCodeV1::RuntimeBindingNotFound))?;
    if row
        .try_get::<String>("", "project_status")
        .map_err(db_error)?
        != "active"
        || counter(row.try_get("", "desired_generation").map_err(db_error)?)?
            != p.expected_generation
        || counter(row.try_get("", "grant_revision").map_err(db_error)?)?
            != p.expected_grant_revision
        || row
            .try_get::<Option<String>>("", "digest")
            .map_err(db_error)?
            .as_deref()
            != Some(p.release.as_str())
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
    }
    Ok(row)
}
fn require_disabled(row: &QueryResult) -> Result<(), RuntimeControlErrorV1> {
    if row
        .try_get::<String>("", "component_status")
        .map_err(db_error)?
        != "disabled"
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeBindingClosed));
    }
    Ok(())
}
async fn require_consents(
    txn: &DatabaseTransaction,
    row: &QueryResult,
    p: &RuntimeBindingCutoverPlanV1,
) -> Result<(), RuntimeControlErrorV1> {
    let declared: Value = row.try_get("", "declared_capabilities").map_err(db_error)?;
    let capabilities = declared
        .as_array()
        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCallerForbidden))?;
    if !capabilities
        .iter()
        .any(|v| v.as_str() == Some("runtime.invoke"))
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeCallerForbidden));
    }
    for capability in capabilities {
        let name = capability
            .as_str()
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCallerForbidden))?;
        super::begin::require_consent(txn, row, p.binding_id, p.expected_grant_revision, name)
            .await?;
    }
    Ok(())
}
async fn check_guard(
    txn: &DatabaseTransaction,
    row: &QueryResult,
    proof: &VerifiedRuntimeAdmissionV1,
    resume: bool,
) -> Result<(), RuntimeControlErrorV1> {
    let claims = proof.claims();
    let max = counter(row.try_get("", "maximum_revision").map_err(db_error)?)?;
    let record: Option<Uuid> = row.try_get("", "record_id").map_err(db_error)?;
    let blocked = row.try_get::<bool>("", "blocked").map_err(db_error)?
        || row
            .try_get::<bool>("", "reconciliation_required")
            .map_err(db_error)?;
    if record.is_some_and(|id| id != claims.record_id)
        || claims.admission_revision < max
        || (blocked && (!resume || claims.admission_revision <= max))
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted));
    }
    let previous=txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT j.* FROM apparatus_runtime_admission_journal j JOIN apparatus_runtime_admission_challenges c USING(challenge_id) WHERE c.authority_id=$1 AND c.release=$2 AND j.admission_revision=$3 ORDER BY j.challenge_id LIMIT 1",
        [claims.challenge.authority_id.into(),claims.challenge.release.as_str().into(),sql_counter(claims.admission_revision)?.into()])).await.map_err(db_error)?;
    if let Some(previous) = previous {
        if previous
            .try_get::<Uuid>("", "record_id")
            .map_err(db_error)?
            != claims.record_id
            || previous
                .try_get::<String>("", "policy_id")
                .map_err(db_error)?
                != claims.policy_id
            || previous
                .try_get::<String>("", "report_sha256")
                .map_err(db_error)?
                != claims.report_sha256.as_str()
            || previous
                .try_get::<String>("", "envelope_reference")
                .map_err(db_error)?
                != claims.envelope_reference
            || previous
                .try_get::<String>("", "cri_image")
                .map_err(db_error)?
                != claims.cri_image
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeIdReused));
        }
    }
    Ok(())
}
async fn consume(
    txn: &DatabaseTransaction,
    proof: &VerifiedRuntimeAdmissionV1,
    authority: Uuid,
    now: i64,
    resume: bool,
) -> Result<(), RuntimeControlErrorV1> {
    let c = proof.claims();
    let changed=txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE apparatus_runtime_admission_challenges SET consumed_at_unix_ms=$2 WHERE challenge_id=$1 AND consumed_at_unix_ms IS NULL AND NOT invalidated AND expires_at_unix_ms>$2",
        [c.jti.into(),now.into()])).await.map_err(db_error)?;
    if changed.rows_affected() != 1 {
        return Err(error(RuntimeErrorCodeV1::RuntimeReplay));
    }
    txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE apparatus_runtime_admission_guards SET record_id=$3,maximum_revision=GREATEST(maximum_revision,$4),blocked=CASE WHEN $5 THEN false ELSE blocked END,reconciliation_required=CASE WHEN $5 THEN false ELSE reconciliation_required END WHERE authority_id=$1 AND release=$2",
        [authority.into(),c.challenge.release.as_str().into(),c.record_id.into(),sql_counter(c.admission_revision)?.into(),resume.into()])).await.map_err(db_error)?;
    txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO apparatus_runtime_admission_journal(challenge_id,jws_sha256,record_id,admission_revision,policy_id,report_sha256,envelope_reference,cri_image,kid,observed_at_unix_ms,valid_until_unix_ms,committed_at_unix_ms) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        [c.jti.into(),proof.fingerprint().to_vec().into(),c.record_id.into(),sql_counter(c.admission_revision)?.into(),c.policy_id.clone().into(),c.report_sha256.as_str().into(),c.envelope_reference.clone().into(),c.cri_image.clone().into(),proof.kid().into(),c.observed_at_unix_ms.get().into(),c.valid_until_unix_ms.get().into(),now.into()])).await.map_err(db_error)?;
    Ok(())
}
async fn require_observation(
    txn: &DatabaseTransaction,
    row: &QueryResult,
    p: &RuntimeBindingCutoverPlanV1,
    service: &str,
    revision: &str,
    profile: &RuntimeMaintenanceProfile,
) -> Result<(), RuntimeControlErrorV1> {
    validate_runtime_uid(service).map_err(error)?;
    validate_runtime_uid(revision).map_err(error)?;
    let observation = txn
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM apparatus_runtime_observations WHERE project_id=$1 AND binding_id=$2",
            [p.project_id.into(), p.binding_id.into()],
        ))
        .await
        .map_err(db_error)?
        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeProjectionUnavailable))?;
    let now = db_clock(txn).await?;
    let cas = row_cas(row)?;
    let lease = current_lease(row, p.project_id, p.binding_id, cas.clone())?
        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeLeaseLost))?;
    let observed: i64 = observation
        .try_get("", "observed_at_ms")
        .map_err(db_error)?;
    let endpoint: String = observation.try_get("", "endpoint").map_err(db_error)?;
    let endpoint = reqwest::Url::parse(&endpoint)
        .map_err(|_| error(RuntimeErrorCodeV1::RuntimeProjectionUnavailable))?;
    let host = format!(
        "{}.apparatus-plugins.svc.cluster.local",
        runtime_serving_name(p.project_id, p.binding_id)
    );
    if endpoint.scheme() != profile.endpoint_scheme
        || endpoint.port_or_known_default() != Some(profile.endpoint_port)
        || endpoint.host_str() != Some(host.as_str())
        || endpoint.path() != "/"
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeProjectionUnavailable));
    }
    if row
        .try_get::<Option<String>>("", "owned_service_uid")
        .map_err(db_error)?
        .as_deref()
        != Some(service)
        || observation
            .try_get::<String>("", "service_uid")
            .map_err(db_error)?
            != service
        || observation
            .try_get::<String>("", "revision_uid")
            .map_err(db_error)?
            != revision
        || !observation
            .try_get::<bool>("", "route_ready")
            .map_err(db_error)?
        || observation
            .try_get::<String>("", "release")
            .map_err(db_error)?
            != p.release.as_str()
        || row_cas(&observation)? != cas
        || counter(observation.try_get("", "writer_fence").map_err(db_error)?)?
            != lease.proof.writer_fence
        || observation
            .try_get::<i64>("", "resource_generation")
            .map_err(db_error)?
            != observation
                .try_get::<i64>("", "observed_resource_generation")
                .map_err(db_error)?
        || observed > now
        || now - observed >= OBSERVATION_TTL_MS
        || now >= lease.expires_at_unix_ms.get()
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeProjectionUnavailable));
    }
    Ok(())
}
