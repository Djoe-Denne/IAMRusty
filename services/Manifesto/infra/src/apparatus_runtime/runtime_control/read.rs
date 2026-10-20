//! Writer-transactional binding and bounded instance reads.

use super::*;
use apparatus_contracts::runtime::{
    RuntimeBindingPageV1, RuntimeBindingSnapshotV1, RuntimeBindingStateV1, RuntimeCursorV1,
    RuntimeDigestV1, RuntimeInstanceCursorPayloadV1, RuntimeInstanceKindV1, RuntimeInstancePageV1,
    RuntimeInstanceSelectorV1, RuntimeInstanceSnapshotV1, RuntimeInstanceStateV1,
    RuntimeListCursorPayloadV1, RuntimeObservationSnapshotV1, RuntimePodIdentityV1,
};

impl SqlRuntimeControl {
    /// Read a bounded current writer page, including tombstones but not instance
    /// inventories. Absence in a page is never evidence of removal.
    ///
    /// # Errors
    /// Rejects cursor tampering/expiry, invalid page sizes, missing cursor keys
    /// when continuation is needed and inconsistent stored state.
    pub async fn list_bindings(
        &self,
        caller: &RuntimeControlCallerV1,
        project_id: Uuid,
        cursor: Option<&RuntimeCursorV1>,
        page_size: Option<u16>,
    ) -> Result<RuntimeBindingPageV1, RuntimeControlErrorV1> {
        if page_size.is_some_and(|size| !(1..=200).contains(&size)) {
            return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest));
        }
        let txn = self.db.begin().await.map_err(db_error)?;
        let initial_now = db_clock(&txn).await?;
        let mut scan = if let Some(cursor) = cursor {
            self.cursor_key
                .as_ref()
                .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?
                .decode(cursor, caller.role, project_id, page_size, initial_now)
                .map_err(error)?
        } else {
            let upper = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT binding_id FROM apparatus_runtime_bindings WHERE project_id=$1 ORDER BY binding_id DESC LIMIT 1", [project_id.into()]))
                .await.map_err(db_error)?.map(|row| row.try_get::<Uuid>("", "binding_id")).transpose().map_err(db_error)?.unwrap_or(Uuid::nil());
            RuntimeListCursorPayloadV1 {
                schema_version: 1,
                scan_id: Uuid::new_v4(),
                project_id,
                principal: caller.role.principal().to_owned(),
                after_binding_id: Uuid::nil(),
                upper_binding_id: upper,
                page_size: page_size.unwrap_or(100),
                issued_at_unix_ms: milliseconds(initial_now)?,
                expires_at_unix_ms: milliseconds(
                    initial_now
                        .checked_add(60_000)
                        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCounterExhausted))?,
                )?,
            }
        };
        let rows = txn.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT * FROM apparatus_runtime_bindings WHERE project_id=$1 AND binding_id>$2 AND binding_id<=$3 ORDER BY binding_id LIMIT $4 FOR SHARE",
            [project_id.into(),scan.after_binding_id.into(),scan.upper_binding_id.into(),(i64::from(scan.page_size)+1).into()])).await.map_err(db_error)?;
        let now = db_clock(&txn).await?;
        if now >= scan.expires_at_unix_ms.get() {
            return Err(error(RuntimeErrorCodeV1::RuntimeCursorExpired));
        }
        let mut items = Vec::new();
        let mut bytes = 0;
        for binding in rows.iter().take(usize::from(scan.page_size)) {
            let binding_id: Uuid = binding.try_get("", "binding_id").map_err(db_error)?;
            let cas = row_cas(binding)?;
            let lease = current_lease(binding, project_id, binding_id, cas.clone())?;
            let snapshot = RuntimeBindingSnapshotV1 {
                schema_version: 1,
                project_id,
                binding_id,
                observation: read_observation(
                    &txn,
                    project_id,
                    binding_id,
                    &cas,
                    lease.as_ref(),
                    now,
                )
                .await?,
                cas,
                state: binding_state(binding)?,
                release: RuntimeDigestV1::new(binding.try_get("", "release").map_err(db_error)?)
                    .map_err(error)?,
                profile: binding.try_get("", "profile").map_err(db_error)?,
                instances: None,
            };
            let size = serde_json::to_vec(&snapshot)
                .map_err(|_| error(RuntimeErrorCodeV1::RuntimeInternal))?
                .len();
            if bytes + size > 240_000 {
                break;
            }
            bytes += size;
            items.push(snapshot);
        }
        let next_cursor = if items.len() < rows.len() {
            scan.after_binding_id = items
                .last()
                .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCapacityExhausted))?
                .binding_id;
            Some(
                self.cursor_key
                    .as_ref()
                    .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?
                    .encode(&scan)
                    .map_err(error)?,
            )
        } else {
            None
        };
        txn.commit().await.map_err(db_error)?;
        Ok(RuntimeBindingPageV1 {
            schema_version: 1,
            items,
            next_cursor,
        })
    }
    /// Read one binding and its explicit instance selection on the writer.
    ///
    /// # Errors
    /// Rejects malformed selectors/cursors and inconsistent stored state; exact
    /// selectors never choose an arbitrary replica or return partial evidence.
    pub async fn get_binding(
        &self,
        caller: &RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        selector: &RuntimeInstanceSelectorV1,
    ) -> Result<RuntimeBindingSnapshotV1, RuntimeControlErrorV1> {
        // The closed role enum permits these two roles and no caller-provided SAN.
        let txn = self.db.begin().await.map_err(db_error)?;
        let binding = lock_binding(&txn, project_id, binding_id).await?;
        let cas = row_cas(&binding)?;
        let binding_release: String = binding.try_get("", "release").map_err(db_error)?;
        let now = db_clock(&txn).await?;
        let lease = current_lease(&binding, project_id, binding_id, cas.clone())?;
        let observation =
            read_observation(&txn, project_id, binding_id, &cas, lease.as_ref(), now).await?;
        let mut continuation = None;
        let rows = match selector {
            RuntimeInstanceSelectorV1::ByPodUid { pod_uid } => {
                apparatus_contracts::runtime::validate_runtime_uid(pod_uid).map_err(error)?;
                txn.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT * FROM apparatus_runtime_instances WHERE project_id=$1 AND binding_id=$2 AND pod_uid=$3 LIMIT 2",
                    [project_id.into(),binding_id.into(),pod_uid.clone().into()])).await.map_err(db_error)?
            }
            RuntimeInstanceSelectorV1::ByInstanceId { instance_id } => {
                txn.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT * FROM apparatus_runtime_instances WHERE project_id=$1 AND binding_id=$2 AND instance_id=$3 LIMIT 2",
                    [project_id.into(),binding_id.into(),(*instance_id).into()])).await.map_err(db_error)?
            }
            RuntimeInstanceSelectorV1::Page { cursor, page_size } => {
                if page_size.is_some_and(|size| !(1..=200).contains(&size)) { return Err(error(RuntimeErrorCodeV1::RuntimeInvalidRequest)); }
                let scan = if let Some(cursor) = cursor {
                    self.cursor_key.as_ref().ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?
                        .decode_instance(cursor, caller.role, project_id, binding_id, *page_size, now).map_err(error)?
                } else {
                    let upper = txn.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
                        "SELECT instance_id FROM apparatus_runtime_instances WHERE project_id=$1 AND binding_id=$2 ORDER BY instance_id DESC LIMIT 1",
                        [project_id.into(),binding_id.into()])).await.map_err(db_error)?
                        .map(|row| row.try_get::<Uuid>("", "instance_id")).transpose().map_err(db_error)?.unwrap_or(Uuid::nil());
                    RuntimeInstanceCursorPayloadV1 { schema_version: 1, scan_id: Uuid::new_v4(), project_id, binding_id,
                        principal: caller.role.principal().to_owned(), after_instance_id: Uuid::nil(), upper_instance_id: upper,
                        page_size: page_size.unwrap_or(100), issued_at_unix_ms: milliseconds(now)?,
                        expires_at_unix_ms: milliseconds(now.checked_add(60_000).ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCounterExhausted))?)? }
                };
                let rows = txn.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT * FROM apparatus_runtime_instances WHERE project_id=$1 AND binding_id=$2 AND instance_id>$3 AND instance_id<=$4 ORDER BY instance_id LIMIT $5",
                    [project_id.into(),binding_id.into(),scan.after_instance_id.into(),scan.upper_instance_id.into(),(i64::from(scan.page_size)+1).into()]))
                    .await.map_err(db_error)?;
                continuation = Some(scan);
                rows
            }
        };
        if !matches!(selector, RuntimeInstanceSelectorV1::Page { .. }) && rows.len() > 1 {
            return Err(error(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError));
        }
        let limit = continuation
            .as_ref()
            .map_or(1, |scan| usize::from(scan.page_size));
        let mut items = Vec::new();
        let mut bytes = 0;
        let total = rows.len();
        for row in rows.iter().take(limit) {
            let item = instance_snapshot(row, &cas, &binding_release, lease.as_ref(), now)?;
            let size = serde_json::to_vec(&item)
                .map_err(|_| error(RuntimeErrorCodeV1::RuntimeInternal))?
                .len();
            if bytes + size > 240_000 {
                break;
            }
            bytes += size;
            items.push(item);
        }
        let next_cursor = if items.len() < total {
            let mut scan = continuation
                .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError))?;
            scan.after_instance_id = items
                .last()
                .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCapacityExhausted))?
                .instance_id;
            Some(
                self.cursor_key
                    .as_ref()
                    .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeKeyUnavailable))?
                    .encode_instance(&scan)
                    .map_err(error)?,
            )
        } else {
            None
        };
        let snapshot = RuntimeBindingSnapshotV1 {
            schema_version: 1,
            project_id,
            binding_id,
            cas,
            state: binding_state(&binding)?,
            release: RuntimeDigestV1::new(binding.try_get("", "release").map_err(db_error)?)
                .map_err(error)?,
            profile: binding.try_get("", "profile").map_err(db_error)?,
            observation,
            instances: Some(RuntimeInstancePageV1 {
                schema_version: 1,
                items,
                next_cursor,
                read_at_unix_ms: milliseconds(now)?,
            }),
        };
        if serde_json::to_vec(&snapshot)
            .map_err(|_| error(RuntimeErrorCodeV1::RuntimeInternal))?
            .len()
            > 262_144
        {
            return Err(error(RuntimeErrorCodeV1::RuntimeCapacityExhausted));
        }
        txn.commit().await.map_err(db_error)?;
        Ok(snapshot)
    }
}

fn binding_state(row: &QueryResult) -> Result<RuntimeBindingStateV1, RuntimeControlErrorV1> {
    match row
        .try_get::<String>("", "state")
        .map_err(db_error)?
        .as_str()
    {
        "active" => Ok(RuntimeBindingStateV1::Active),
        "closed" => Ok(RuntimeBindingStateV1::Closed),
        "draining" => Ok(RuntimeBindingStateV1::Draining),
        "tombstone" => Ok(RuntimeBindingStateV1::Tombstone),
        _ => Err(error(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError)),
    }
}

fn instance_snapshot(
    row: &QueryResult,
    binding_cas: &RuntimeCasV1,
    binding_release: &str,
    lease: Option<&RuntimeWriterLeaseV1>,
    now: i64,
) -> Result<RuntimeInstanceSnapshotV1, RuntimeControlErrorV1> {
    let cas = row_cas(row)?;
    let kind = match row
        .try_get::<String>("", "kind")
        .map_err(db_error)?
        .as_str()
    {
        "pod" => RuntimeInstanceKindV1::Pod,
        "legacy_fence" => RuntimeInstanceKindV1::LegacyFence,
        _ => return Err(error(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError)),
    };
    let state = match row
        .try_get::<String>("", "state")
        .map_err(db_error)?
        .as_str()
    {
        "current" => RuntimeInstanceStateV1::Current,
        "closed" => RuntimeInstanceStateV1::Closed,
        "tombstone" => RuntimeInstanceStateV1::Tombstone,
        _ => return Err(error(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError)),
    };
    let fence = row
        .try_get::<Option<i64>>("", "writer_fence")
        .map_err(db_error)?
        .map(counter)
        .transpose()?;
    let observed = row
        .try_get::<Option<i64>>("", "observed_at_ms")
        .map_err(db_error)?;
    let mut valid_until = row
        .try_get::<Option<i64>>("", "valid_until_ms")
        .map_err(db_error)?;
    if state == RuntimeInstanceStateV1::Current {
        let current = lease.filter(|lease| {
            Some(lease.proof.writer_fence) == fence && now < lease.expires_at_unix_ms.get()
        });
        let max = current.map_or(now, |lease| lease.expires_at_unix_ms.get());
        valid_until = valid_until.map(|until| until.min(max));
        if cas.generation != binding_cas.generation
            || cas.revocation_epoch != binding_cas.revocation_epoch
            || cas.grant_revision != binding_cas.grant_revision
            || row.try_get::<String>("", "release").map_err(db_error)? != binding_release
            || observed.is_some_and(|time| time > now)
        {
            valid_until = valid_until.map(|until| until.min(now));
        }
    }
    Ok(RuntimeInstanceSnapshotV1 {
        schema_version: 1,
        project_id: row.try_get("", "project_id").map_err(db_error)?,
        binding_id: row.try_get("", "binding_id").map_err(db_error)?,
        instance_id: row.try_get("", "instance_id").map_err(db_error)?,
        kind,
        state,
        cas,
        release: RuntimeDigestV1::new(row.try_get("", "release").map_err(db_error)?)
            .map_err(error)?,
        pod: if kind == RuntimeInstanceKindV1::Pod {
            Some(RuntimePodIdentityV1 {
                pod_uid: row.try_get("", "pod_uid").map_err(db_error)?,
                revision_uid: row.try_get("", "revision_uid").map_err(db_error)?,
                namespace: row.try_get("", "namespace").map_err(db_error)?,
                service_account: row.try_get("", "service_account").map_err(db_error)?,
                service_account_uid: row.try_get("", "service_account_uid").map_err(db_error)?,
            })
        } else {
            None
        },
        writer_fence: fence,
        observed_at_unix_ms: observed.map(milliseconds).transpose()?,
        valid_until_unix_ms: valid_until.map(milliseconds).transpose()?,
        closed_at_unix_ms: row
            .try_get::<Option<i64>>("", "closed_at_ms")
            .map_err(db_error)?
            .map(milliseconds)
            .transpose()?,
    })
}

async fn read_observation(
    txn: &DatabaseTransaction,
    project_id: Uuid,
    binding_id: Uuid,
    cas: &RuntimeCasV1,
    lease: Option<&RuntimeWriterLeaseV1>,
    now: i64,
) -> Result<Option<RuntimeObservationSnapshotV1>, RuntimeControlErrorV1> {
    let row = txn
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM apparatus_runtime_observations WHERE project_id=$1 AND binding_id=$2",
            [project_id.into(), binding_id.into()],
        ))
        .await
        .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let observed_cas = row_cas(&row)?;
    let fence = counter(row.try_get("", "writer_fence").map_err(db_error)?)?;
    let observed_at: i64 = row.try_get("", "observed_at_ms").map_err(db_error)?;
    let mut until = observed_at
        .checked_add(OBSERVATION_TTL_MS)
        .ok_or_else(|| error(RuntimeErrorCodeV1::RuntimeCounterExhausted))?;
    until = until.min(
        lease
            .filter(|lease| lease.proof.writer_fence == fence)
            .map_or(now, |lease| lease.expires_at_unix_ms.get()),
    );
    if observed_cas.generation != cas.generation
        || observed_cas.grant_revision != cas.grant_revision
        || observed_cas.revocation_epoch != cas.revocation_epoch
    {
        until = until.min(now);
    }
    Ok(Some(RuntimeObservationSnapshotV1 {
        schema_version: 1,
        project_id,
        binding_id,
        cas: observed_cas,
        writer_fence: fence,
        service_uid: row.try_get("", "service_uid").map_err(db_error)?,
        resource_generation: counter(row.try_get("", "resource_generation").map_err(db_error)?)?,
        observed_resource_generation: counter(
            row.try_get("", "observed_resource_generation")
                .map_err(db_error)?,
        )?,
        revision_uid: row.try_get("", "revision_uid").map_err(db_error)?,
        revision_name: row.try_get("", "revision_name").map_err(db_error)?,
        release: RuntimeDigestV1::new(row.try_get("", "release").map_err(db_error)?)
            .map_err(error)?,
        endpoint: row.try_get("", "endpoint").map_err(db_error)?,
        route_ready: row.try_get("", "route_ready").map_err(db_error)?,
        observed_at_unix_ms: milliseconds(observed_at)?,
        valid_until_unix_ms: milliseconds(until)?,
    }))
}
