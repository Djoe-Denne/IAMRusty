//! Revocation and completion never acquire a fictitious projector lease.

use super::*;
use apparatus_contracts::runtime::{
    RuntimeCompleteEffectRequestV1, RuntimeEffectCompletionStateV1, RuntimeInstanceRevokeRequestV1,
};

impl SqlRuntimeControl {
    /// Permanently revoke one identity, preserving the Pod mapping when known.
    /// Unknown historical identities create only a legacy-fence tombstone.
    ///
    /// # Errors
    /// Rejects role/path/context mismatch and stale non-idempotent writes.
    pub async fn revoke_instance(
        &self,
        caller: &RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instance_id: Uuid,
        request: &RuntimeInstanceRevokeRequestV1,
    ) -> Result<RuntimeWriteAckV1, RuntimeControlErrorV1> {
        require_lazaret(caller)?;
        if request.schema_version != 1 {
            return Err(error(RuntimeErrorCodeV1::RuntimeUnsupportedVersion));
        }
        if request.instance_id != instance_id {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
        }
        let txn = self.db.begin().await.map_err(db_error)?;
        let binding = lock_binding(&txn, project_id, binding_id).await?;
        let mut cas = row_cas(&binding)?;
        if cas.generation != request.expected.generation
            || cas.grant_revision != request.expected.grant_revision
            || cas.revocation_epoch != request.expected.revocation_epoch
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        let fence = counter(binding.try_get("", "writer_fence").map_err(db_error)?)?;
        let previous = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT state FROM apparatus_runtime_instances WHERE project_id=$1 AND binding_id=$2 AND instance_id=$3 FOR UPDATE",
            [project_id.into(),binding_id.into(),instance_id.into()])).await.map_err(db_error)?;
        let already_closed = previous
            .as_ref()
            .map(|row| row.try_get::<String>("", "state"))
            .transpose()
            .map_err(db_error)?
            .is_some_and(|state| state != "current");
        if already_closed {
            txn.commit().await.map_err(db_error)?;
            return Ok(RuntimeWriteAckV1 {
                schema_version: 1,
                project_id,
                binding_id,
                cas,
                writer_fence: fence,
            });
        }
        if cas != request.expected {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        let now = db_clock(&txn).await?;
        cas.projection_version = cas.projection_version.checked_next().map_err(error)?;
        if previous.is_some() {
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE apparatus_runtime_instances SET state='tombstone',closed_at_ms=$4,projection_version=$5 \
                 WHERE project_id=$1 AND binding_id=$2 AND instance_id=$3",
                [project_id.into(),binding_id.into(),instance_id.into(),now.into(),sql_counter(cas.projection_version)?.into()]))
                .await.map_err(db_error)?;
        } else {
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "INSERT INTO apparatus_runtime_instances \
                 (project_id,binding_id,instance_id,kind,state,generation,grant_revision,revocation_epoch,projection_version,release,closed_at_ms) \
                 VALUES ($1,$2,$3,'legacy_fence','tombstone',$4,$5,$6,$7,$8,$9)",
                [project_id.into(),binding_id.into(),instance_id.into(),sql_counter(cas.generation)?.into(),
                 sql_counter(cas.grant_revision)?.into(),sql_counter(cas.revocation_epoch)?.into(),sql_counter(cas.projection_version)?.into(),
                 binding.try_get::<String>("", "release").map_err(db_error)?.into(),now.into()])).await.map_err(db_error)?;
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
            writer_fence: fence,
        })
    }

    /// Journal completion of an already committed effect, even after revocation
    /// or deadline. This method never reserves/replays an I/O.
    ///
    /// # Errors
    /// Rejects substituted reservations and contradictory terminal completion.
    pub async fn complete_effect(
        &self,
        caller: &RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        effect_id: Uuid,
        request: &RuntimeCompleteEffectRequestV1,
    ) -> Result<RuntimeWriteAckV1, RuntimeControlErrorV1> {
        require_lazaret(caller)?;
        if request.schema_version != 1 || request.reservation.schema_version != 1 {
            return Err(error(RuntimeErrorCodeV1::RuntimeUnsupportedVersion));
        }
        let reservation = &request.reservation;
        if reservation.project_id != project_id
            || reservation.binding_id != binding_id
            || reservation.effect_id != effect_id
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
        }
        let txn = self.db.begin().await.map_err(db_error)?;
        let binding = lock_binding(&txn, project_id, binding_id).await?;
        let cas = row_cas(&binding)?;
        let fence = counter(binding.try_get("", "writer_fence").map_err(db_error)?)?;
        let effect = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT * FROM apparatus_runtime_effects WHERE project_id=$1 AND binding_id=$2 AND effect_id=$3 FOR UPDATE",
            [project_id.into(),binding_id.into(),effect_id.into()])).await.map_err(db_error)?
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInvalidRequest))?;
        if effect
            .try_get::<Uuid>("", "execution_id")
            .map_err(db_error)?
            != reservation.execution_id
            || effect
                .try_get::<String>("", "effect_sha256")
                .map_err(db_error)?
                != reservation.effect_sha256.as_str()
            || effect.try_get::<i64>("", "deadline_ms").map_err(db_error)?
                != reservation.deadline_unix_ms.get()
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeIdReused));
        }
        let completion = match request.completion_state {
            RuntimeEffectCompletionStateV1::Completed => "completed",
            RuntimeEffectCompletionStateV1::Indeterminate => "indeterminate",
        };
        if let Some(previous) = effect
            .try_get::<Option<String>>("", "completion_state")
            .map_err(db_error)?
        {
            if previous != completion {
                return Err(error(RuntimeErrorCodeV1::RuntimeIdReused));
            }
        } else {
            let now = db_clock(&txn).await?;
            if now < effect.try_get::<i64>("", "begun_at_ms").map_err(db_error)? {
                return Err(error(RuntimeErrorCodeV1::RuntimeInternal));
            }
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE apparatus_runtime_effects SET completion_state=$4,completed_at_ms=$5 \
                 WHERE project_id=$1 AND binding_id=$2 AND effect_id=$3 AND completed_at_ms IS NULL",
                [project_id.into(),binding_id.into(),effect_id.into(),completion.into(),now.into()])).await.map_err(db_error)?;
        }
        txn.commit().await.map_err(db_error)?;
        Ok(RuntimeWriteAckV1 {
            schema_version: 1,
            project_id,
            binding_id,
            cas,
            writer_fence: fence,
        })
    }
}

fn require_lazaret(caller: &RuntimeControlCallerV1) -> Result<(), RuntimeControlErrorV1> {
    if caller.role != RuntimeServiceRoleV1::Lazaret {
        return Err(error(RuntimeErrorCodeV1::RuntimeServiceForbidden));
    }
    Ok(())
}

fn error(code: RuntimeErrorCodeV1) -> RuntimeControlErrorV1 {
    code.error(
        RuntimeServiceRoleV1::Lazaret,
        RuntimeEffectOutcomeV1::NotStarted,
    )
}
