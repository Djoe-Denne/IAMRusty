//! Writer-DB runtime control primitives. No cluster access or cached authority.
//!
//! Every clock check below occurs after the binding row lock. Transaction-start
//! `now()` and caller clocks cannot extend a lease while waiting for a lock.

use std::sync::Arc;

use apparatus_contracts::runtime::{
    RuntimeCasV1, RuntimeControlCallerV1, RuntimeControlErrorV1, RuntimeCounterV1,
    RuntimeEffectOutcomeV1, RuntimeErrorCodeV1, RuntimeLeaseProofV1, RuntimeObservationV1,
    RuntimeServiceRoleV1, RuntimeUnixMsV1, RuntimeWriteAckV1, RuntimeWriterLeaseRequestV1,
    RuntimeWriterLeaseV1, OBSERVATION_TTL_MS, WRITER_LEASE_TTL_MS,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, QueryResult, Statement,
    TransactionTrait,
};
use uuid::Uuid;

mod begin;
mod completion;
pub mod maintenance;
mod port;
mod read;
mod registry;

/// SQL authority connected exclusively to the writer database.
pub struct SqlRuntimeControl {
    db: Arc<DatabaseConnection>,
    cursor_key: Option<Arc<super::runtime_cursor::RuntimeCursorKey>>,
}

impl SqlRuntimeControl {
    /// Construct once at the composition root, never from a read replica.
    #[must_use]
    pub fn new(db: impl Into<Arc<DatabaseConnection>>) -> Self {
        Self {
            db: db.into(),
            cursor_key: None,
        }
    }

    /// Inject the dedicated secret-backed cursor codec from trusted setup.
    #[must_use]
    pub fn with_cursor_key(mut self, key: Arc<super::runtime_cursor::RuntimeCursorKey>) -> Self {
        self.cursor_key = Some(key);
        self
    }

    /// Report a parent route observation with a current writer lease and CAS.
    ///
    /// # Errors
    /// Rejects unknown/closed bindings, stale business state, stale lease/fence,
    /// expired/future observations and non-identical stale projection writes.
    pub async fn report_observation(
        &self,
        caller: &RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &RuntimeObservationV1,
    ) -> Result<RuntimeWriteAckV1, RuntimeControlErrorV1> {
        require_projector(caller)?;
        apparatus_contracts::runtime::validate_runtime_uid(&request.service_uid).map_err(error)?;
        apparatus_contracts::runtime::validate_runtime_uid(&request.revision_uid).map_err(error)?;
        apparatus_contracts::runtime::validate_runtime_dns_name(&request.revision_name, true)
            .map_err(error)?;
        if request.schema_version != 1 {
            return Err(error(RuntimeErrorCodeV1::RuntimeUnsupportedVersion));
        }
        if request.project_id != project_id
            || request.binding_id != binding_id
            || request.service_uid.is_empty()
            || request.revision_uid.is_empty()
            || request.revision_name.is_empty()
            || request.endpoint.is_empty()
            || request.endpoint.len() > 2048
            || (request.route_ready
                && request.resource_generation != request.observed_resource_generation)
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
        }
        let txn = self.db.begin().await.map_err(db_error)?;
        let row = lock_binding(&txn, project_id, binding_id).await?;
        let mut cas = row_cas(&row)?;
        if row.try_get::<String>("", "release").map_err(db_error)? != request.release.as_str() {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        let state: String = row.try_get("", "state").map_err(db_error)?;
        let phase: Option<String> = row.try_get("", "cutover_phase").map_err(db_error)?;
        if state != "active"
            && state != "draining"
            && !(state == "closed" && phase.as_deref() == Some("prepared"))
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeBindingClosed));
        }
        if row
            .try_get::<Option<String>>("", "owned_service_uid")
            .map_err(db_error)?
            .is_some_and(|uid| uid != request.service_uid)
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeFenceConflict));
        }
        if cas.generation != request.guard.expected.generation
            || cas.grant_revision != request.guard.expected.grant_revision
            || cas.revocation_epoch != request.guard.expected.revocation_epoch
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        let now = db_clock(&txn).await?;
        let lease = current_lease(&row, project_id, binding_id, cas.clone())?
            .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeLeaseLost))?;
        if lease.proof.writer_fence != request.guard.lease.writer_fence {
            return Err(error(RuntimeErrorCodeV1::RuntimeFenceConflict));
        }
        if lease.proof != request.guard.lease || now >= lease.expires_at_unix_ms.get() {
            return Err(error(RuntimeErrorCodeV1::RuntimeLeaseLost));
        }
        let observed_at = request.observed_at_unix_ms.get();
        if observed_at > now || now - observed_at >= OBSERVATION_TTL_MS {
            return Err(error(RuntimeErrorCodeV1::RuntimeProjectionUnavailable));
        }
        let previous = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT * FROM apparatus_runtime_observations WHERE project_id=$1 AND binding_id=$2",
            [project_id.into(), binding_id.into()])).await.map_err(db_error)?;
        let identical = previous
            .as_ref()
            .map(|row| same_observation(row, request))
            .transpose()?
            .unwrap_or(false);
        if previous
            .as_ref()
            .map(|row| row.try_get::<String>("", "service_uid"))
            .transpose()
            .map_err(db_error)?
            .is_some_and(|uid| uid != request.service_uid)
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeFenceConflict));
        }
        if !identical {
            if cas != request.guard.expected {
                return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
            }
            cas.projection_version = cas.projection_version.checked_next().map_err(error)?;
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "INSERT INTO apparatus_runtime_observations \
                (project_id,binding_id,generation,grant_revision,revocation_epoch,projection_version,writer_fence, \
                 service_uid,revision_uid,revision_name,endpoint,observed_at_ms,resource_generation,observed_resource_generation,release,route_ready) \
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) \
                ON CONFLICT (project_id,binding_id) DO UPDATE SET generation=EXCLUDED.generation, \
                grant_revision=EXCLUDED.grant_revision,revocation_epoch=EXCLUDED.revocation_epoch, \
                projection_version=EXCLUDED.projection_version,writer_fence=EXCLUDED.writer_fence, \
                service_uid=EXCLUDED.service_uid,revision_uid=EXCLUDED.revision_uid, \
                revision_name=EXCLUDED.revision_name,endpoint=EXCLUDED.endpoint,observed_at_ms=EXCLUDED.observed_at_ms, \
                resource_generation=EXCLUDED.resource_generation,observed_resource_generation=EXCLUDED.observed_resource_generation, \
                release=EXCLUDED.release,route_ready=EXCLUDED.route_ready",
                [project_id.into(),binding_id.into(),sql_counter(cas.generation)?.into(),sql_counter(cas.grant_revision)?.into(),
                 sql_counter(cas.revocation_epoch)?.into(),sql_counter(cas.projection_version)?.into(),
                 sql_counter(lease.proof.writer_fence)?.into(),request.service_uid.clone().into(),request.revision_uid.clone().into(),
                 request.revision_name.clone().into(),request.endpoint.clone().into(),observed_at.into(),
                 sql_counter(request.resource_generation)?.into(),sql_counter(request.observed_resource_generation)?.into(),
                 request.release.as_str().into(),request.route_ready.into()]))
                .await.map_err(db_error)?;
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE apparatus_runtime_bindings SET projection_version=$3,updated_at_ms=$4 WHERE project_id=$1 AND binding_id=$2",
                [project_id.into(),binding_id.into(),sql_counter(cas.projection_version)?.into(),now.into()]))
                .await.map_err(db_error)?;
        }
        // First valid observation binds the actual create-response UID. Never
        // replace it on a replay, a takeover, a missing object or a new digest.
        txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE apparatus_runtime_bindings SET owned_service_uid=$3 WHERE project_id=$1 AND binding_id=$2 AND owned_service_uid IS NULL",
            [project_id.into(),binding_id.into(),request.service_uid.clone().into()])).await.map_err(db_error)?;
        txn.commit().await.map_err(db_error)?;
        Ok(RuntimeWriteAckV1 {
            schema_version: 1,
            project_id,
            binding_id,
            cas,
            writer_fence: lease.proof.writer_fence,
        })
    }

    /// Acquire/renew a server-bounded lease, preserving fences across tombstones.
    ///
    /// # Errors
    /// Returns a closed protocol error on role, CAS, holder/fence, clock or DB
    /// failure. An expired lease is never renewed or implicitly resurrected.
    pub async fn writer_lease(
        &self,
        caller: &RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &RuntimeWriterLeaseRequestV1,
    ) -> Result<RuntimeWriterLeaseV1, RuntimeControlErrorV1> {
        require_projector(caller)?;
        if request.schema_version != 1 {
            return Err(error(RuntimeErrorCodeV1::RuntimeUnsupportedVersion));
        }
        if request.holder_id.get_version_num() != 4 {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
        }
        let txn = self.db.begin().await.map_err(db_error)?;
        let row = lock_binding(&txn, project_id, binding_id).await?;
        let cas = row_cas(&row)?;
        if cas != request.expected {
            return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
        }
        let now = db_clock(&txn).await?;
        let current = current_lease(&row, project_id, binding_id, cas.clone())?;
        let live = current
            .as_ref()
            .is_some_and(|lease| now < lease.expires_at_unix_ms.get());
        let lease = if let Some(proof) = &request.renew {
            let existing = current
                .filter(|lease| live && lease.proof.holder_id == request.holder_id)
                .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeLeaseLost))?;
            if existing.proof.writer_fence != proof.writer_fence {
                return Err(error(RuntimeErrorCodeV1::RuntimeFenceConflict));
            }
            if existing.proof != *proof {
                return Err(error(RuntimeErrorCodeV1::RuntimeLeaseLost));
            }
            RuntimeWriterLeaseV1 {
                issued_at_unix_ms: milliseconds(now)?,
                expires_at_unix_ms: milliseconds(
                    now.checked_add(WRITER_LEASE_TTL_MS)
                        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCounterExhausted))?,
                )?,
                ..existing
            }
        } else if live {
            let existing = current.ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInternal))?;
            if existing.proof.holder_id != request.holder_id {
                return Err(error(RuntimeErrorCodeV1::RuntimeLeaseHeld));
            }
            // Same holder acquisition is idempotent, NOT a renewal.
            txn.commit().await.map_err(db_error)?;
            return Ok(existing);
        } else {
            let fence = counter(row.try_get("", "writer_fence").map_err(db_error)?)?
                .checked_next()
                .map_err(error)?;
            RuntimeWriterLeaseV1 {
                schema_version: 1,
                project_id,
                binding_id,
                proof: RuntimeLeaseProofV1 {
                    lease_id: Uuid::new_v4(),
                    holder_id: request.holder_id,
                    writer_fence: fence,
                },
                cas,
                issued_at_unix_ms: milliseconds(now)?,
                expires_at_unix_ms: milliseconds(
                    now.checked_add(WRITER_LEASE_TTL_MS)
                        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCounterExhausted))?,
                )?,
            }
        };
        txn.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE apparatus_runtime_bindings SET writer_fence=$3, lease_id=$4, lease_holder=$5, \
             lease_principal=$6, lease_issued_at_ms=$7, lease_expires_at_ms=$8 \
             WHERE project_id=$1 AND binding_id=$2",
            [
                project_id.into(),
                binding_id.into(),
                sql_counter(lease.proof.writer_fence)?.into(),
                lease.proof.lease_id.into(),
                request.holder_id.into(),
                caller.role.principal().into(),
                lease.issued_at_unix_ms.get().into(),
                lease.expires_at_unix_ms.get().into(),
            ],
        ))
        .await
        .map_err(db_error)?;
        txn.commit().await.map_err(db_error)?;
        Ok(lease)
    }
}

fn same_observation(
    row: &QueryResult,
    request: &RuntimeObservationV1,
) -> Result<bool, RuntimeControlErrorV1> {
    Ok(
        row.try_get::<String>("", "service_uid").map_err(db_error)? == request.service_uid
            && row
                .try_get::<String>("", "revision_uid")
                .map_err(db_error)?
                == request.revision_uid
            && row
                .try_get::<String>("", "revision_name")
                .map_err(db_error)?
                == request.revision_name
            && row.try_get::<String>("", "endpoint").map_err(db_error)? == request.endpoint
            && row.try_get::<String>("", "release").map_err(db_error)? == request.release.as_str()
            && row.try_get::<bool>("", "route_ready").map_err(db_error)? == request.route_ready
            && counter(row.try_get("", "resource_generation").map_err(db_error)?)?
                == request.resource_generation
            && counter(
                row.try_get("", "observed_resource_generation")
                    .map_err(db_error)?,
            )? == request.observed_resource_generation
            && row.try_get::<i64>("", "observed_at_ms").map_err(db_error)?
                == request.observed_at_unix_ms.get()
            && counter(row.try_get("", "generation").map_err(db_error)?)?
                == request.guard.expected.generation
            && counter(row.try_get("", "grant_revision").map_err(db_error)?)?
                == request.guard.expected.grant_revision
            && counter(row.try_get("", "revocation_epoch").map_err(db_error)?)?
                == request.guard.expected.revocation_epoch
            && counter(row.try_get("", "writer_fence").map_err(db_error)?)?
                == request.guard.lease.writer_fence,
    )
}

async fn lock_binding(
    txn: &DatabaseTransaction,
    project_id: Uuid,
    binding_id: Uuid,
) -> Result<QueryResult, RuntimeControlErrorV1> {
    txn.query_one(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT * FROM apparatus_runtime_bindings WHERE project_id=$1 AND binding_id=$2 FOR UPDATE",
        [project_id.into(), binding_id.into()],
    ))
    .await
    .map_err(db_error)?
    .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeBindingNotFound))
}

async fn db_clock(txn: &DatabaseTransaction) -> Result<i64, RuntimeControlErrorV1> {
    let row = txn
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS now_ms",
        ))
        .await
        .map_err(db_error)?
        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeInternal))?;
    let now: i64 = row.try_get("", "now_ms").map_err(db_error)?;
    milliseconds(now)?;
    Ok(now)
}

fn row_cas(row: &QueryResult) -> Result<RuntimeCasV1, RuntimeControlErrorV1> {
    Ok(RuntimeCasV1 {
        generation: counter(row.try_get("", "generation").map_err(db_error)?)?,
        revocation_epoch: counter(row.try_get("", "revocation_epoch").map_err(db_error)?)?,
        projection_version: counter(row.try_get("", "projection_version").map_err(db_error)?)?,
        grant_revision: counter(row.try_get("", "grant_revision").map_err(db_error)?)?,
    })
}

fn current_lease(
    row: &QueryResult,
    project_id: Uuid,
    binding_id: Uuid,
    cas: RuntimeCasV1,
) -> Result<Option<RuntimeWriterLeaseV1>, RuntimeControlErrorV1> {
    let lease_id: Option<Uuid> = row.try_get("", "lease_id").map_err(db_error)?;
    let Some(lease_id) = lease_id else {
        return Ok(None);
    };
    let principal: String = row.try_get("", "lease_principal").map_err(db_error)?;
    if principal != RuntimeServiceRoleV1::Projector.principal() {
        return Err(error(RuntimeErrorCodeV1::RuntimeLeaseLost));
    }
    Ok(Some(RuntimeWriterLeaseV1 {
        schema_version: 1,
        project_id,
        binding_id,
        cas,
        proof: RuntimeLeaseProofV1 {
            lease_id,
            holder_id: row.try_get("", "lease_holder").map_err(db_error)?,
            writer_fence: counter(row.try_get("", "writer_fence").map_err(db_error)?)?,
        },
        issued_at_unix_ms: milliseconds(row.try_get("", "lease_issued_at_ms").map_err(db_error)?)?,
        expires_at_unix_ms: milliseconds(
            row.try_get("", "lease_expires_at_ms").map_err(db_error)?,
        )?,
    }))
}

fn require_projector(caller: &RuntimeControlCallerV1) -> Result<(), RuntimeControlErrorV1> {
    if caller.role != RuntimeServiceRoleV1::Projector {
        return Err(error(RuntimeErrorCodeV1::RuntimeServiceForbidden));
    }
    Ok(())
}
fn counter(value: i64) -> Result<RuntimeCounterV1, RuntimeControlErrorV1> {
    let value = u64::try_from(value).map_err(|_| error(RuntimeErrorCodeV1::RuntimeInternal))?;
    RuntimeCounterV1::new(value).map_err(error)
}
fn sql_counter(value: RuntimeCounterV1) -> Result<i64, RuntimeControlErrorV1> {
    i64::try_from(value.get()).map_err(|_| error(RuntimeErrorCodeV1::RuntimeCounterExhausted))
}
fn milliseconds(value: i64) -> Result<RuntimeUnixMsV1, RuntimeControlErrorV1> {
    RuntimeUnixMsV1::new(value).map_err(error)
}
fn error(code: RuntimeErrorCodeV1) -> RuntimeControlErrorV1 {
    code.error(
        RuntimeServiceRoleV1::Projector,
        RuntimeEffectOutcomeV1::NotStarted,
    )
}
fn db_error(_error: sea_orm::DbErr) -> RuntimeControlErrorV1 {
    error(RuntimeErrorCodeV1::RuntimeAuthorityUnavailable)
}

/// Close/fence an already-materialized runtime binding in the caller's business
/// transaction. No row is inserted and no legacy binding is migrated/opened.
///
/// The caller must acquire its project/domain locks before this runtime row,
/// using the same order as begin-effect. A committed tombstone survives removal
/// of the project component and cannot be reopened by delayed observations.
///
/// # Errors
/// Returns a closed error on SQL failure, stale generation/grant or overflow.
pub async fn close_binding_in_transaction(
    txn: &DatabaseTransaction,
    project_id: Uuid,
    binding_id: Uuid,
    generation: RuntimeCounterV1,
    grant_revision: RuntimeCounterV1,
    tombstone: bool,
) -> Result<(), RuntimeControlErrorV1> {
    let row = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT * FROM apparatus_runtime_bindings WHERE project_id=$1 AND binding_id=$2 FOR UPDATE",
        [project_id.into(),binding_id.into()])).await.map_err(db_error)?;
    let Some(row) = row else {
        return Ok(());
    };
    let cas = row_cas(&row)?;
    txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE apparatus_runtime_admission_challenges SET invalidated=true WHERE project_id=$1 AND binding_id=$2 AND consumed_at_unix_ms IS NULL",
        [project_id.into(), binding_id.into()])).await.map_err(db_error)?;
    if generation < cas.generation || grant_revision < cas.grant_revision {
        return Err(error(RuntimeErrorCodeV1::RuntimeStaleState));
    }
    let state: String = row.try_get("", "state").map_err(db_error)?;
    // Tombstones are durable, not a state a stale status update can overwrite.
    let next_state = if tombstone || state == "tombstone" {
        "tombstone"
    } else {
        "closed"
    };
    let phase: Option<String> = row.try_get("", "cutover_phase").map_err(db_error)?;
    if phase.as_deref() != Some("prepared")
        && state == next_state
        && generation == cas.generation
        && grant_revision == cas.grant_revision
    {
        return Ok(());
    }
    let now = db_clock(txn).await?;
    let epoch = cas.revocation_epoch.checked_next().map_err(error)?;
    let version = cas.projection_version.checked_next().map_err(error)?;
    txn.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE apparatus_runtime_bindings SET state=$3,generation=$4,grant_revision=$5, \
         cutover_phase=CASE WHEN cutover_phase IS NULL THEN NULL ELSE 'fenced' END, \
         revocation_epoch=$6,projection_version=$7,lease_id=NULL,lease_holder=NULL, \
         lease_principal=NULL,lease_issued_at_ms=NULL,lease_expires_at_ms=NULL,updated_at_ms=$8 \
         WHERE project_id=$1 AND binding_id=$2",
        [
            project_id.into(),
            binding_id.into(),
            next_state.into(),
            sql_counter(generation)?.into(),
            sql_counter(grant_revision)?.into(),
            sql_counter(epoch)?.into(),
            sql_counter(version)?.into(),
            now.into(),
        ],
    ))
    .await
    .map_err(db_error)?;
    txn.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE apparatus_runtime_instances SET closed_at_ms=COALESCE(closed_at_ms,$3), \
         state=CASE WHEN $4 THEN 'tombstone' ELSE 'closed' END \
         WHERE project_id=$1 AND binding_id=$2 AND state<>'tombstone'",
        [
            project_id.into(),
            binding_id.into(),
            now.into(),
            tombstone.into(),
        ],
    ))
    .await
    .map_err(db_error)?;
    Ok(())
}

/// Fence a materialized runtime after a legacy-domain mutation, in the same
/// transaction. Returns whether the binding has migrated (including tombstones).
/// Project and legacy binding locks must already be held in the common order.
///
/// # Errors
/// Refuses missing domain state, SQL failure, stale counters or overflow.
pub(crate) async fn fence_business_change(
    txn: &DatabaseTransaction,
    project_id: Uuid,
    binding_id: Uuid,
    tombstone: bool,
) -> Result<bool, RuntimeControlErrorV1> {
    let row = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT 1 FROM apparatus_runtime_bindings WHERE project_id=$1 AND binding_id=$2 FOR UPDATE",
        [project_id.into(),binding_id.into()])).await.map_err(db_error)?;
    if row.is_none() {
        return Ok(false);
    }
    let domain = txn
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT ab.desired_generation,ab.grant_revision FROM apparatus_bindings ab \
         JOIN project_components pc ON pc.id=ab.component_id \
         WHERE pc.project_id=$1 AND pc.id=$2",
            [project_id.into(), binding_id.into()],
        ))
        .await
        .map_err(db_error)?
        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeBindingNotFound))?;
    let generation = counter(domain.try_get("", "desired_generation").map_err(db_error)?)?;
    let grant_revision = counter(domain.try_get("", "grant_revision").map_err(db_error)?)?;
    close_binding_in_transaction(
        txn,
        project_id,
        binding_id,
        generation,
        grant_revision,
        tombstone,
    )
    .await?;
    Ok(true)
}

/// Close a project's materialized runtime entries in UUID order while its
/// project row is locked by the caller. Retains all ownership/history records.
///
/// # Errors
/// Propagates DB, counter exhaustion or fencing failure to roll back the caller.
pub(crate) async fn close_project_in_transaction(
    txn: &DatabaseTransaction,
    project_id: Uuid,
    tombstone: bool,
) -> Result<(), RuntimeControlErrorV1> {
    let rows = txn
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT binding_id,generation,grant_revision FROM apparatus_runtime_bindings \
         WHERE project_id=$1 ORDER BY binding_id FOR UPDATE",
            [project_id.into()],
        ))
        .await
        .map_err(db_error)?;
    for row in rows {
        close_binding_in_transaction(
            txn,
            project_id,
            row.try_get("", "binding_id").map_err(db_error)?,
            counter(row.try_get("", "generation").map_err(db_error)?)?,
            counter(row.try_get("", "grant_revision").map_err(db_error)?)?,
            tombstone,
        )
        .await?;
    }
    Ok(())
}
