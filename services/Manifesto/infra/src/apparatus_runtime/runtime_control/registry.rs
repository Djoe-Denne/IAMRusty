//! Pod registry writes; immutable Pod-to-instance mapping survives restarts.

use super::*;
use apparatus_contracts::runtime::{RuntimeInstanceRegistrationV1, RuntimeInstanceStateV1};

impl SqlRuntimeControl {
    /// Register/refresh an observed Pod mapping or close its existing identity.
    ///
    /// # Errors
    /// Rejects stale guards, reused identifiers, remapping, closed-to-current
    /// transitions, expired observations and all unsupported/malformed inputs.
    pub async fn register_instance(
        &self,
        caller: &RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instance_id: Uuid,
        request: &RuntimeInstanceRegistrationV1,
    ) -> Result<RuntimeWriteAckV1, RuntimeControlErrorV1> {
        require_projector(caller)?;
        if request.schema_version != 1 {
            return Err(error(RuntimeErrorCodeV1::RuntimeUnsupportedVersion));
        }
        if request.project_id != project_id
            || request.binding_id != binding_id
            || request.instance_id != instance_id
            || instance_id.get_version_num() != 4
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
        }
        for uid in [
            &request.pod_uid,
            &request.revision_uid,
            &request.service_account_uid,
        ] {
            apparatus_contracts::runtime::validate_runtime_uid(uid).map_err(error)?;
        }
        apparatus_contracts::runtime::validate_runtime_dns_name(&request.namespace, false)
            .map_err(error)?;
        apparatus_contracts::runtime::validate_runtime_dns_name(&request.service_account, true)
            .map_err(error)?;
        let txn = self.db.begin().await.map_err(db_error)?;
        let binding = lock_binding(&txn, project_id, binding_id).await?;
        let mut cas = row_cas(&binding)?;
        let now = db_clock(&txn).await?;
        let lease = current_lease(&binding, project_id, binding_id, cas.clone())?
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeLeaseLost))?;
        if lease.proof.writer_fence != request.guard.lease.writer_fence {
            return Err(error(RuntimeErrorCodeV1::RuntimeFenceConflict));
        }
        if lease.proof != request.guard.lease || now >= lease.expires_at_unix_ms.get() {
            return Err(error(RuntimeErrorCodeV1::RuntimeLeaseLost));
        }
        if cas.generation != request.guard.expected.generation
            || cas.grant_revision != request.guard.expected.grant_revision
            || cas.revocation_epoch != request.guard.expected.revocation_epoch
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        let state: String = binding.try_get("", "state").map_err(db_error)?;
        if request.state == RuntimeInstanceStateV1::Current && state != "active" {
            return Err(error(RuntimeErrorCodeV1::RuntimeBindingClosed));
        }
        let release: String = binding.try_get("", "release").map_err(db_error)?;
        if request.state == RuntimeInstanceStateV1::Current && release != request.release.as_str() {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        let observed_at = request.observed_at_unix_ms.get();
        if observed_at > now || now - observed_at >= OBSERVATION_TTL_MS {
            return Err(error(RuntimeErrorCodeV1::RuntimeProjectionUnavailable));
        }
        let candidates = txn
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT * FROM apparatus_runtime_instances WHERE project_id=$1 AND binding_id=$2 \
             AND (instance_id=$3 OR pod_uid=$4) FOR UPDATE",
                [
                    project_id.into(),
                    binding_id.into(),
                    instance_id.into(),
                    request.pod_uid.clone().into(),
                ],
            ))
            .await
            .map_err(db_error)?;
        if candidates.len() > 1 {
            return Err(error(RuntimeErrorCodeV1::RuntimeIdReused));
        }
        let previous = candidates.first();
        let wire_state = match request.state {
            RuntimeInstanceStateV1::Current => "current",
            RuntimeInstanceStateV1::Closed => "closed",
            RuntimeInstanceStateV1::Tombstone => "tombstone",
        };
        let mut identical = false;
        if let Some(previous) = previous {
            let old_state: String = previous.try_get("", "state").map_err(db_error)?;
            if previous
                .try_get::<Uuid>("", "instance_id")
                .map_err(db_error)?
                != instance_id
                || previous.try_get::<String>("", "kind").map_err(db_error)? != "pod"
                || previous
                    .try_get::<String>("", "pod_uid")
                    .map_err(db_error)?
                    != request.pod_uid
                || previous
                    .try_get::<String>("", "revision_uid")
                    .map_err(db_error)?
                    != request.revision_uid
                || previous
                    .try_get::<String>("", "namespace")
                    .map_err(db_error)?
                    != request.namespace
                || previous
                    .try_get::<String>("", "service_account")
                    .map_err(db_error)?
                    != request.service_account
                || previous
                    .try_get::<String>("", "service_account_uid")
                    .map_err(db_error)?
                    != request.service_account_uid
                || previous
                    .try_get::<String>("", "release")
                    .map_err(db_error)?
                    != request.release.as_str()
            {
                return Err(error(RuntimeErrorCodeV1::RuntimeIdReused));
            }
            if (old_state != "current" && request.state == RuntimeInstanceStateV1::Current)
                || (old_state == "tombstone" && request.state != RuntimeInstanceStateV1::Tombstone)
            {
                return Err(error(RuntimeErrorCodeV1::RuntimeIdentityRevoked));
            }
            if request.state == RuntimeInstanceStateV1::Current
                && (counter(previous.try_get("", "generation").map_err(db_error)?)?
                    != cas.generation
                    || counter(previous.try_get("", "grant_revision").map_err(db_error)?)?
                        != cas.grant_revision
                    || counter(previous.try_get("", "revocation_epoch").map_err(db_error)?)?
                        != cas.revocation_epoch)
            {
                return Err(error(RuntimeErrorCodeV1::RuntimeIdentityRevoked));
            }
            identical = old_state == wire_state
                && (old_state != "current"
                    || (previous
                        .try_get::<i64>("", "writer_fence")
                        .map_err(db_error)?
                        == sql_counter(lease.proof.writer_fence)?
                        && previous
                            .try_get::<i64>("", "observed_at_ms")
                            .map_err(db_error)?
                            == observed_at));
        }
        if identical {
            txn.commit().await.map_err(db_error)?;
            return Ok(RuntimeWriteAckV1 {
                schema_version: 1,
                project_id,
                binding_id,
                cas,
                writer_fence: lease.proof.writer_fence,
            });
        }
        if cas != request.guard.expected {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        cas.projection_version = cas.projection_version.checked_next().map_err(error)?;
        let valid_until = observed_at
            .checked_add(OBSERVATION_TTL_MS)
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCounterExhausted))?
            .min(lease.expires_at_unix_ms.get());
        if previous.is_none() {
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "INSERT INTO apparatus_runtime_instances \
                 (project_id,binding_id,instance_id,kind,state,generation,grant_revision,revocation_epoch,projection_version,writer_fence, \
                  pod_uid,revision_uid,namespace,service_account,service_account_uid,release,observed_at_ms,valid_until_ms,closed_at_ms) \
                 VALUES ($1,$2,$3,'pod',$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
                [project_id.into(),binding_id.into(),instance_id.into(),wire_state.into(),sql_counter(cas.generation)?.into(),
                 sql_counter(cas.grant_revision)?.into(),sql_counter(cas.revocation_epoch)?.into(),sql_counter(cas.projection_version)?.into(),
                 sql_counter(lease.proof.writer_fence)?.into(),request.pod_uid.clone().into(),request.revision_uid.clone().into(),
                 request.namespace.clone().into(),request.service_account.clone().into(),request.service_account_uid.clone().into(),
                 request.release.as_str().into(),observed_at.into(),valid_until.into(),
                 (if request.state == RuntimeInstanceStateV1::Current { None } else { Some(now) }).into()]))
                .await.map_err(db_error)?;
        } else if request.state == RuntimeInstanceStateV1::Current {
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE apparatus_runtime_instances SET writer_fence=$4,observed_at_ms=$5,valid_until_ms=$6,projection_version=$7 \
                 WHERE project_id=$1 AND binding_id=$2 AND instance_id=$3 AND state='current'",
                [project_id.into(),binding_id.into(),instance_id.into(),sql_counter(lease.proof.writer_fence)?.into(),
                 observed_at.into(),valid_until.into(),sql_counter(cas.projection_version)?.into()])).await.map_err(db_error)?;
        } else {
            // Closure retains the historical generation/release/fence/dates.
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE apparatus_runtime_instances SET state=$4,closed_at_ms=COALESCE(closed_at_ms,$5),projection_version=$6 \
                 WHERE project_id=$1 AND binding_id=$2 AND instance_id=$3",
                [project_id.into(),binding_id.into(),instance_id.into(),wire_state.into(),now.into(),sql_counter(cas.projection_version)?.into()]))
                .await.map_err(db_error)?;
        }
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE apparatus_runtime_bindings SET projection_version=$3,updated_at_ms=$4 WHERE project_id=$1 AND binding_id=$2",
            [project_id.into(),binding_id.into(),sql_counter(cas.projection_version)?.into(),now.into()])).await.map_err(db_error)?;
        txn.commit().await.map_err(db_error)?;
        Ok(RuntimeWriteAckV1 {
            schema_version: 1,
            project_id,
            binding_id,
            cas,
            writer_fence: lease.proof.writer_fence,
        })
    }
}
