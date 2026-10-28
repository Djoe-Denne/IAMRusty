//! Effect linearization: project -> ordered domain bindings -> ordered runtime
//! bindings, then grant/instance checks and durable reservation before any I/O.

use super::*;
use apparatus_contracts::runtime::{
    RuntimeBeginEffectRequestV1, RuntimeEffectHashInputV1, RuntimeEffectReservationV1,
    EFFECT_HASH_DOMAIN,
};
use std::collections::BTreeMap;

impl SqlRuntimeControl {
    /// Authorize and durably reserve one effect; retries never replay its I/O.
    ///
    /// # Errors
    /// Fails closed on stale/revoked context, missing current instances/grants,
    /// hash mismatch, replay, saturation, deadline or writer failure.
    pub async fn begin_effect(
        &self,
        caller: &RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &RuntimeBeginEffectRequestV1,
    ) -> Result<RuntimeEffectReservationV1, RuntimeControlErrorV1> {
        if caller.role != RuntimeServiceRoleV1::Lazaret {
            return Err(error(RuntimeErrorCodeV1::RuntimeServiceForbidden));
        }
        if request.schema_version != 1 || request.context.schema_version != 1 {
            return Err(error(RuntimeErrorCodeV1::RuntimeUnsupportedVersion));
        }
        let context = &request.context;
        if context.project_id != project_id
            || context.target.binding_id != binding_id
            || request.invoke.binding_id != binding_id
            || context.instance_id == context.source.instance
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
        }
        let capability = match request.invoke.operation.as_str() {
            "kv.get" => "storage.kv.read",
            "kv.put" | "kv.delete" => "storage.kv.write",
            "connector.fetch" => "connector.fetch",
            _ => return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest)),
        };
        let expected_hash = RuntimeEffectHashInputV1 {
            schema_version: 1,
            domain: EFFECT_HASH_DOMAIN.to_owned(),
            operation_sha256: context.operation_sha256.clone(),
            execution_id: context.execution_id,
            effect_id: request.effect_id,
            instance_id: context.instance_id,
            invoke: request.invoke.clone(),
        }
        .sha256()
        .map_err(error)?;
        if expected_hash != request.effect_sha256 {
            return Err(error(RuntimeErrorCodeV1::RuntimeHashMismatch));
        }
        let txn = self.db.begin().await.map_err(db_error)?;
        crate::transaction::lock_project_row(&txn, project_id)
            .await
            .map_err(|_| error(RuntimeErrorCodeV1::RuntimeAuthorityUnavailable))?;
        let project = txn
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT 1 FROM projects WHERE id=$1 AND status='active'",
                [project_id.into()],
            ))
            .await
            .map_err(db_error)?;
        if project.is_none() {
            return Err(error(RuntimeErrorCodeV1::RuntimeCallerForbidden));
        }
        let ids = std::collections::BTreeSet::from([context.source.binding, binding_id]);
        let mut domains = BTreeMap::new();
        for id in &ids {
            let domain = txn
                .query_one(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT ab.*,pc.status AS component_status FROM apparatus_bindings ab \
                 JOIN project_components pc ON pc.id=ab.component_id \
                 WHERE pc.project_id=$1 AND pc.id=$2 FOR UPDATE OF ab,pc",
                    [project_id.into(), (*id).into()],
                ))
                .await
                .map_err(db_error)?
                .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCallerForbidden))?;
            if domain
                .try_get::<String>("", "component_status")
                .map_err(db_error)?
                != "active"
            {
                return Err(error(RuntimeErrorCodeV1::RuntimeBindingClosed));
            }
            domains.insert(*id, domain);
        }
        let mut bindings = BTreeMap::new();
        for id in &ids {
            bindings.insert(*id, lock_binding(&txn, project_id, *id).await?);
        }
        let source = bindings
            .get(&context.source.binding)
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInternal))?;
        let target = bindings
            .get(&binding_id)
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInternal))?;
        let source_cas = row_cas(source)?;
        let target_cas = row_cas(target)?;
        if source.try_get::<String>("", "state").map_err(db_error)? != "active"
            || target.try_get::<String>("", "state").map_err(db_error)? != "active"
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeBindingClosed));
        }
        if source_cas.generation != context.source.generation
            || source_cas.grant_revision != context.source.grant_revision
            || source_cas.revocation_epoch != context.source_revocation_epoch
            || target_cas.generation != context.target.generation
            || target_cas.grant_revision != context.target.grant_revision
            || target_cas.revocation_epoch != context.target.revocation_epoch
            || source.try_get::<String>("", "release").map_err(db_error)?
                != context.source.release.as_str()
            || target.try_get::<String>("", "release").map_err(db_error)?
                != context.target.release.as_str()
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        for (id, cas, release) in [
            (
                context.source.binding,
                &source_cas,
                context.source.release.as_str(),
            ),
            (binding_id, &target_cas, context.target.release.as_str()),
        ] {
            let domain = domains
                .get(&id)
                .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInternal))?;
            if counter(domain.try_get("", "desired_generation").map_err(db_error)?)?
                != cas.generation
                || counter(domain.try_get("", "grant_revision").map_err(db_error)?)?
                    != cas.grant_revision
                || domain
                    .try_get::<Option<String>>("", "digest")
                    .map_err(db_error)?
                    .as_deref()
                    != Some(release)
            {
                return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
            }
            require_consent(&txn, domain, id, cas.grant_revision, "runtime.invoke").await?;
        }
        require_consent(
            &txn,
            domains
                .get(&binding_id)
                .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInternal))?,
            binding_id,
            target_cas.grant_revision,
            capability,
        )
        .await?;
        let now = db_clock(&txn).await?;
        if now >= context.deadline_unix_ms.get() {
            return Err(error(RuntimeErrorCodeV1::RuntimeDeadlineExceeded));
        }
        require_current_instance(
            &txn,
            source,
            project_id,
            context.source.binding,
            context.source.instance,
            &source_cas,
            now,
        )
        .await?;
        require_current_instance(
            &txn,
            target,
            project_id,
            binding_id,
            context.instance_id,
            &target_cas,
            now,
        )
        .await?;
        let existing = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT effect_sha256 FROM apparatus_runtime_effects WHERE project_id=$1 AND binding_id=$2 AND effect_id=$3",
            [project_id.into(),binding_id.into(),request.effect_id.into()])).await.map_err(db_error)?;
        if let Some(existing) = existing {
            let code = if existing
                .try_get::<String>("", "effect_sha256")
                .map_err(db_error)?
                == request.effect_sha256.as_str()
            {
                RuntimeErrorCodeV1::RuntimeReplay
            } else {
                RuntimeErrorCodeV1::RuntimeIdReused
            };
            return Err(error(code));
        }
        let context_json = serde_json::to_value(context)
            .map_err(|_| error(RuntimeErrorCodeV1::RuntimeInternal))?;
        let previous_context = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT execution_context FROM apparatus_runtime_effects WHERE project_id=$1 AND binding_id=$2 AND execution_id=$3 LIMIT 1",
            [project_id.into(),binding_id.into(),context.execution_id.into()])).await.map_err(db_error)?;
        if let Some(previous) = previous_context {
            if previous
                .try_get::<serde_json::Value>("", "execution_context")
                .map_err(db_error)?
                != context_json
            {
                return Err(error(RuntimeErrorCodeV1::RuntimeIdReused));
            }
        }
        let count = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT count(*)::bigint AS total FROM apparatus_runtime_effects WHERE project_id=$1 AND binding_id=$2 AND deadline_ms>$3",
            [project_id.into(),binding_id.into(),now.into()])).await.map_err(db_error)?
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInternal))?;
        if count.try_get::<i64>("", "total").map_err(db_error)? >= 1000 {
            return Err(error(RuntimeErrorCodeV1::RuntimeCapacityExhausted));
        }
        let deadline = context.deadline_unix_ms.get().min(
            now.checked_add(5_000)
                .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCounterExhausted))?,
        );
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO apparatus_runtime_effects \
             (project_id,binding_id,execution_id,effect_id,instance_id,authorization_id,execution_context,source_binding_id,source_instance_id, \
              source_revocation_epoch,target_revocation_epoch,grant_revision,operation_sha256,effect_sha256,deadline_ms,begun_at_ms) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
            [project_id.into(),binding_id.into(),context.execution_id.into(),request.effect_id.into(),context.instance_id.into(),
             context.authorization_id.into(),context_json.into(),context.source.binding.into(),context.source.instance.into(),
             sql_counter(context.source_revocation_epoch)?.into(),sql_counter(context.target.revocation_epoch)?.into(),
             sql_counter(context.target.grant_revision)?.into(),context.operation_sha256.as_str().into(),request.effect_sha256.as_str().into(),deadline.into(),now.into()]))
            .await.map_err(db_error)?;
        txn.commit().await.map_err(db_error)?;
        Ok(RuntimeEffectReservationV1 {
            schema_version: 1,
            project_id,
            binding_id,
            execution_id: context.execution_id,
            effect_id: request.effect_id,
            effect_sha256: request.effect_sha256.clone(),
            deadline_unix_ms: milliseconds(deadline)?,
        })
    }
}

pub(super) async fn require_consent(
    txn: &DatabaseTransaction,
    domain: &QueryResult,
    binding_id: Uuid,
    revision: RuntimeCounterV1,
    capability: &str,
) -> Result<(), RuntimeControlErrorV1> {
    let declared: serde_json::Value = domain
        .try_get("", "declared_capabilities")
        .map_err(db_error)?;
    if !declared
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(capability)))
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeCallerForbidden));
    }
    let consent = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT 1 FROM apparatus_capability_consents WHERE component_id=$1 AND capability=$2 AND status='consented' AND grant_revision=$3",
        [binding_id.into(),capability.into(),sql_counter(revision)?.into()])).await.map_err(db_error)?;
    if consent.is_none() {
        return Err(error(RuntimeErrorCodeV1::RuntimeCallerForbidden));
    }
    Ok(())
}

async fn require_current_instance(
    txn: &DatabaseTransaction,
    binding: &QueryResult,
    project_id: Uuid,
    binding_id: Uuid,
    instance_id: Uuid,
    cas: &RuntimeCasV1,
    now: i64,
) -> Result<(), RuntimeControlErrorV1> {
    let instance = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT * FROM apparatus_runtime_instances WHERE project_id=$1 AND binding_id=$2 AND instance_id=$3",
        [project_id.into(),binding_id.into(),instance_id.into()])).await.map_err(db_error)?
        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeProjectionUnavailable))?;
    if instance.try_get::<String>("", "state").map_err(db_error)? != "current"
        || instance.try_get::<String>("", "kind").map_err(db_error)? != "pod"
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeIdentityRevoked));
    }
    let current = row_cas(&instance)?;
    if current.generation != cas.generation
        || current.grant_revision != cas.grant_revision
        || current.revocation_epoch != cas.revocation_epoch
        || instance
            .try_get::<String>("", "release")
            .map_err(db_error)?
            != binding.try_get::<String>("", "release").map_err(db_error)?
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
    }
    let lease = current_lease(binding, project_id, binding_id, cas.clone())?
        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeProjectionUnavailable))?;
    let observed: i64 = instance.try_get("", "observed_at_ms").map_err(db_error)?;
    let until: i64 = instance.try_get("", "valid_until_ms").map_err(db_error)?;
    if counter(instance.try_get("", "writer_fence").map_err(db_error)?)? != lease.proof.writer_fence
        || now >= lease.expires_at_unix_ms.get()
        || now >= until
        || observed > now
        || now - observed >= OBSERVATION_TTL_MS
    {
        return Err(error(RuntimeErrorCodeV1::RuntimeProjectionUnavailable));
    }
    Ok(())
}

fn error(code: RuntimeErrorCodeV1) -> RuntimeControlErrorV1 {
    code.error(
        RuntimeServiceRoleV1::Lazaret,
        RuntimeEffectOutcomeV1::NotStarted,
    )
}
