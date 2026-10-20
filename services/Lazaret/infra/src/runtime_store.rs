//! Lazaret-owned durable runtime registry and authorization consumption journal.
//! No raw JWS/projected token/private key is stored or logged.

use std::sync::Arc;

use apparatus_contracts::runtime::*;
use async_trait::async_trait;
use lazaret_domain::{
    runtime::{RuntimeEnrollment, RuntimeStore},
    WorkloadIdentity,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, QueryResult, Statement,
    TransactionTrait, Value,
};
use uuid::Uuid;

pub struct PostgresRuntimeStore {
    db: Arc<DatabaseConnection>,
}

impl PostgresRuntimeStore {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

fn statement(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

fn unavailable(_: sea_orm::DbErr) -> RuntimeErrorCodeV1 {
    RuntimeErrorCodeV1::RuntimeAuthorityUnavailable
}

async fn lock(
    tx: &DatabaseTransaction,
    kind: &str,
    project: Uuid,
    binding: Uuid,
) -> Result<(), RuntimeErrorCodeV1> {
    tx.query_one(statement(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        vec![format!("runtime-{kind}:{project}:{binding}").into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

const ENROLLMENT_SELECT: &str = "SELECT *,
    floor(extract(epoch FROM issued_at)*1000)::bigint AS issued_ms,
    floor(extract(epoch FROM expires_at)*1000)::bigint AS expires_ms,
    (closed_at IS NOT NULL) AS closed FROM apparatus_runtime_enrollments";

fn enrollment(row: &QueryResult) -> Result<RuntimeEnrollment, RuntimeErrorCodeV1> {
    macro_rules! get {
        ($name:literal) => {
            row.try_get("", $name)
                .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?
        };
    }
    let identity = WorkloadIdentity::try_new(
        get!("instance_id"),
        get!("binding_id"),
        get!("release"),
        get!("generation"),
        get!("grant_revision"),
    )
    .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
    let epoch: i64 = get!("revocation_epoch");
    Ok(RuntimeEnrollment {
        project_id: get!("project_id"),
        identity,
        pod_uid: get!("pod_uid"),
        revision_uid: get!("revision_uid"),
        namespace: get!("namespace"),
        service_account: get!("service_account"),
        service_account_uid: get!("service_account_uid"),
        revocation_epoch: u64::try_from(epoch).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?,
        csr_sha256: get!("csr_sha256"),
        fingerprint: get!("certificate_fingerprint"),
        certificate_pem: get!("certificate_pem"),
        issued_at_unix_ms: get!("issued_ms"),
        expires_at_unix_ms: get!("expires_ms"),
        closed: get!("closed"),
    })
}

fn same_enrollment(a: &RuntimeEnrollment, b: &RuntimeEnrollment) -> bool {
    // A repeated issuance may have produced another serial/certificate. Return
    // the original one only for the identical binding, instance, Pod and CSR.
    a.project_id == b.project_id
        && a.identity == b.identity
        && a.pod_uid == b.pod_uid
        && a.revision_uid == b.revision_uid
        && a.namespace == b.namespace
        && a.service_account == b.service_account
        && a.service_account_uid == b.service_account_uid
        && a.revocation_epoch == b.revocation_epoch
        && a.csr_sha256 == b.csr_sha256
}

const FENCE_SELECT: &str = "SELECT *,
    floor(extract(epoch FROM installed_at)*1000)::bigint AS installed_ms
    FROM apparatus_runtime_cutover_fences";

fn fence(row: &QueryResult) -> Result<RuntimeCutoverFenceV1, RuntimeErrorCodeV1> {
    macro_rules! get {
        ($name:literal) => {
            row.try_get("", $name)
                .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?
        };
    }
    let inventory: String = get!("inventory_sha256");
    let observed: Option<serde_json::Value> = get!("observed_writer_cas");
    Ok(RuntimeCutoverFenceV1 {
        migration_id: get!("migration_id"),
        project_id: get!("project_id"),
        binding_id: get!("binding_id"),
        inventory_sha256: RuntimeDigestV1::new(inventory)
            .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?,
        installed_at_unix_ms: RuntimeUnixMsV1::new(get!("installed_ms"))
            .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?,
        observed_writer_cas: observed
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?,
    })
}

async fn cutover_lock(
    tx: &DatabaseTransaction,
    binding_id: Uuid,
) -> Result<(), RuntimeErrorCodeV1> {
    tx.query_one(statement(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        vec![format!("runtime-cutover:{binding_id}").into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

async fn quota(
    tx: &DatabaseTransaction,
    table: &str,
    expiry: &str,
    project: Uuid,
    binding: Uuid,
) -> Result<(), RuntimeErrorCodeV1> {
    // Only private call sites supply these two SQL identifiers.
    let row = tx
        .query_one(statement(
            &format!(
                "SELECT count(*)::bigint AS n FROM {table}
        WHERE project_id=$1 AND binding_id=$2 AND closed_at IS NULL
        AND {expiry}>clock_timestamp()"
            ),
            vec![project.into(), binding.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(RuntimeErrorCodeV1::RuntimeInternal)?;
    let n: i64 = row
        .try_get("", "n")
        .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
    if n >= 1000 {
        return Err(RuntimeErrorCodeV1::RuntimeCapacityExhausted);
    }
    Ok(())
}

#[async_trait]
impl RuntimeStore for PostgresRuntimeStore {
    async fn list_cutover_fences(
        &self,
        project_id: Uuid,
        after: Option<Uuid>,
        limit: u16,
    ) -> Result<Vec<RuntimeCutoverFenceV1>, RuntimeErrorCodeV1> {
        if !(1..=200).contains(&limit) {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        let rows = self
            .db
            .query_all(statement(
                &format!(
                    "{FENCE_SELECT}
            WHERE project_id=$1 AND ($2::uuid IS NULL OR binding_id>$2)
            ORDER BY binding_id LIMIT $3"
                ),
                vec![project_id.into(), after.into(), i64::from(limit).into()],
            ))
            .await
            .map_err(unavailable)?;
        rows.iter().map(fence).collect()
    }

    async fn cutover_fence(
        &self,
        binding_id: Uuid,
    ) -> Result<Option<RuntimeCutoverFenceV1>, RuntimeErrorCodeV1> {
        self.db
            .query_one(statement(
                &format!("{FENCE_SELECT} WHERE binding_id=$1"),
                vec![binding_id.into()],
            ))
            .await
            .map_err(unavailable)?
            .map(|row| fence(&row))
            .transpose()
    }

    async fn admit_legacy(&self, binding_id: Uuid) -> Result<(), RuntimeErrorCodeV1> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        cutover_lock(&tx, binding_id).await?;
        let denied = tx
            .query_one(statement(
                "SELECT binding_id FROM apparatus_runtime_cutover_fences
            WHERE binding_id=$1",
                vec![binding_id.into()],
            ))
            .await
            .map_err(unavailable)?
            .is_some();
        if denied {
            return Err(RuntimeErrorCodeV1::RuntimeLegacyDisabled);
        }
        tx.commit().await.map_err(unavailable)
    }

    async fn install_legacy_fence(
        &self,
        command: &InstallRuntimeLegacyFenceV1,
    ) -> Result<RuntimeCutoverFenceV1, RuntimeErrorCodeV1> {
        if command.schema_version != 1 {
            return Err(RuntimeErrorCodeV1::RuntimeUnsupportedVersion);
        }
        let tx = self.db.begin().await.map_err(unavailable)?;
        cutover_lock(&tx, command.binding_id).await?;
        if let Some(row) = tx
            .query_one(statement(
                &format!("{FENCE_SELECT} WHERE binding_id=$1"),
                vec![command.binding_id.into()],
            ))
            .await
            .map_err(unavailable)?
        {
            let existing = fence(&row)?;
            if existing.migration_id != command.migration_id
                || existing.project_id != command.project_id
                || existing.inventory_sha256 != command.inventory_sha256
            {
                return Err(RuntimeErrorCodeV1::RuntimeIdReused);
            }
            // A replay returns the original ack; it must not close credentials
            // that were legitimately issued after the original cutover.
            tx.commit().await.map_err(unavailable)?;
            return Ok(existing);
        }
        lock(&tx, "enrollment", command.project_id, command.binding_id).await?;
        lock(&tx, "execution", command.project_id, command.binding_id).await?;
        let mismatch = tx
            .query_one(statement(
                "SELECT 1 FROM apparatus_runtime_enrollments
            WHERE binding_id=$1 AND project_id<>$2 LIMIT 1",
                vec![command.binding_id.into(), command.project_id.into()],
            ))
            .await
            .map_err(unavailable)?;
        if mismatch.is_some() {
            return Err(RuntimeErrorCodeV1::RuntimeIdReused);
        }
        tx.execute(statement(
            "INSERT INTO apparatus_runtime_cutover_fences
            (binding_id,migration_id,project_id,inventory_sha256) VALUES($1,$2,$3,$4)",
            vec![
                command.binding_id.into(),
                command.migration_id.into(),
                command.project_id.into(),
                command.inventory_sha256.as_str().into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
        tx.execute(statement(
            "UPDATE apparatus_runtime_enrollments SET closed_at=clock_timestamp()
            WHERE project_id=$1 AND binding_id=$2 AND closed_at IS NULL",
            vec![command.project_id.into(), command.binding_id.into()],
        ))
        .await
        .map_err(unavailable)?;
        tx.execute(statement(
            "UPDATE apparatus_runtime_executions SET closed_at=clock_timestamp()
            WHERE project_id=$1 AND binding_id=$2 AND closed_at IS NULL",
            vec![command.project_id.into(), command.binding_id.into()],
        ))
        .await
        .map_err(unavailable)?;
        let row = tx
            .query_one(statement(
                &format!("{FENCE_SELECT} WHERE binding_id=$1"),
                vec![command.binding_id.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(RuntimeErrorCodeV1::RuntimeInternal)?;
        let installed = fence(&row)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(installed)
    }

    async fn enrollment_by_instance(
        &self,
        instance: Uuid,
    ) -> Result<Option<RuntimeEnrollment>, RuntimeErrorCodeV1> {
        self.db
            .query_one(statement(
                &format!("{ENROLLMENT_SELECT} WHERE instance_id=$1"),
                vec![instance.into()],
            ))
            .await
            .map_err(unavailable)?
            .map(|row| enrollment(&row))
            .transpose()
    }

    async fn enrollment_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<Option<RuntimeEnrollment>, RuntimeErrorCodeV1> {
        self.db
            .query_one(statement(
                &format!("{ENROLLMENT_SELECT} WHERE certificate_fingerprint=$1"),
                vec![fingerprint.into()],
            ))
            .await
            .map_err(unavailable)?
            .map(|row| enrollment(&row))
            .transpose()
    }

    async fn enroll(
        &self,
        record: &RuntimeEnrollment,
    ) -> Result<RuntimeEnrollment, RuntimeErrorCodeV1> {
        if record.closed
            || record.expires_at_unix_ms <= record.issued_at_unix_ms
            || record
                .expires_at_unix_ms
                .saturating_sub(record.issued_at_unix_ms)
                > 300_000
        {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        let tx = self.db.begin().await.map_err(unavailable)?;
        lock(
            &tx,
            "enrollment",
            record.project_id,
            record.identity.binding,
        )
        .await?;
        if let Some(row) = tx
            .query_one(statement(
                &format!("{ENROLLMENT_SELECT} WHERE instance_id=$1 FOR UPDATE"),
                vec![record.identity.instance.into()],
            ))
            .await
            .map_err(unavailable)?
        {
            let existing = enrollment(&row)?;
            if existing.closed {
                return Err(RuntimeErrorCodeV1::RuntimeIdentityRevoked);
            }
            if !same_enrollment(&existing, record) {
                return Err(RuntimeErrorCodeV1::RuntimeIdReused);
            }
            tx.commit().await.map_err(unavailable)?;
            return Ok(existing);
        }
        quota(
            &tx,
            "apparatus_runtime_enrollments",
            "expires_at",
            record.project_id,
            record.identity.binding,
        )
        .await?;
        let epoch = i64::try_from(record.revocation_epoch)
            .map_err(|_| RuntimeErrorCodeV1::RuntimeCounterExhausted)?;
        let inserted = tx.query_one(statement("INSERT INTO apparatus_runtime_enrollments
            (instance_id,project_id,binding_id,pod_uid,revision_uid,namespace,service_account,
             service_account_uid,release,generation,grant_revision,revocation_epoch,
             csr_sha256,certificate_fingerprint,certificate_pem,issued_at,expires_at)
            SELECT $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,
             to_timestamp($16::bigint::double precision/1000),to_timestamp($17::bigint::double precision/1000)
            WHERE to_timestamp($17::bigint::double precision/1000)>clock_timestamp()
            ON CONFLICT DO NOTHING RETURNING instance_id",
            vec![record.identity.instance.into(), record.project_id.into(), record.identity.binding.into(),
                record.pod_uid.clone().into(), record.revision_uid.clone().into(), record.namespace.clone().into(),
                record.service_account.clone().into(), record.service_account_uid.clone().into(),
                record.identity.release.clone().into(), record.identity.generation.into(), record.identity.grant_revision.into(),
                epoch.into(), record.csr_sha256.clone().into(), record.fingerprint.clone().into(),
                record.certificate_pem.clone().into(), record.issued_at_unix_ms.into(), record.expires_at_unix_ms.into()]))
            .await.map_err(unavailable)?;
        if inserted.is_none() {
            return Err(RuntimeErrorCodeV1::RuntimeIdReused);
        }
        tx.commit().await.map_err(unavailable)?;
        Ok(record.clone())
    }

    async fn issue(&self, ticket: &RuntimeInvocationTicketV1) -> Result<(), RuntimeErrorCodeV1> {
        if ticket.schema_version != 1
            || ticket.source.binding == ticket.target.binding_id
            || ticket.exp <= ticket.iat
            || ticket.exp.saturating_sub(ticket.iat) > 30
            || ticket.deadline_unix_ms.get() / 1000 != ticket.exp
        {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        let claims =
            serde_json::to_value(ticket).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
        let tx = self.db.begin().await.map_err(unavailable)?;
        lock(
            &tx,
            "execution",
            ticket.project_id,
            ticket.target.binding_id,
        )
        .await?;
        // Only the bounded journal is pruned after the retention window. Identity
        // tombstones and Manifesto epochs/fences are never compacted here.
        tx.execute(statement(
            "DELETE FROM apparatus_runtime_executions
            WHERE project_id=$1 AND binding_id=$2 AND retain_until<=clock_timestamp()",
            vec![ticket.project_id.into(), ticket.target.binding_id.into()],
        ))
        .await
        .map_err(unavailable)?;
        quota(
            &tx,
            "apparatus_runtime_executions",
            "deadline",
            ticket.project_id,
            ticket.target.binding_id,
        )
        .await?;
        let inserted = tx
            .query_one(statement(
                "INSERT INTO apparatus_runtime_executions
            (authorization_id,project_id,binding_id,source_instance_id,source_binding_id,
             generation,grant_revision,revocation_epoch,operation_id,operation_sha256,
             issued_at,deadline,retain_until,ticket_claims)
            SELECT $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,clock_timestamp(),
             to_timestamp($12::bigint::double precision/1000),
             to_timestamp($12::bigint::double precision/1000)+INTERVAL '24 hours',$13
            WHERE clock_timestamp()<to_timestamp($12::bigint::double precision/1000)
              AND clock_timestamp()<to_timestamp($14::bigint::double precision)
              AND clock_timestamp()>=to_timestamp($11::bigint::double precision)
            ON CONFLICT DO NOTHING RETURNING authorization_id",
                vec![
                    ticket.jti.into(),
                    ticket.project_id.into(),
                    ticket.target.binding_id.into(),
                    ticket.source.instance.into(),
                    ticket.source.binding.into(),
                    (ticket.target.generation.get() as i64).into(),
                    (ticket.target.grant_revision.get() as i64).into(),
                    (ticket.target.revocation_epoch.get() as i64).into(),
                    ticket.operation_id.into(),
                    ticket
                        .operation_sha256
                        .as_str()
                        .trim_start_matches("sha256:")
                        .to_owned()
                        .into(),
                    ticket.iat.into(),
                    ticket.deadline_unix_ms.get().into(),
                    claims.into(),
                    ticket.exp.into(),
                ],
            ))
            .await
            .map_err(unavailable)?;
        if inserted.is_none() {
            return Err(RuntimeErrorCodeV1::RuntimeReplay);
        }
        tx.commit().await.map_err(unavailable)
    }

    async fn issued(
        &self,
        authorization_id: Uuid,
    ) -> Result<Option<RuntimeInvocationTicketV1>, RuntimeErrorCodeV1> {
        let row = self
            .db
            .query_one(statement(
                "SELECT ticket_claims FROM apparatus_runtime_executions
            WHERE authorization_id=$1 AND closed_at IS NULL AND deadline>clock_timestamp()",
                vec![authorization_id.into()],
            ))
            .await
            .map_err(unavailable)?;
        row.map(|row| {
            let value: serde_json::Value = row
                .try_get("", "ticket_claims")
                .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
            serde_json::from_value(value).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)
        })
        .transpose()
    }

    async fn accept(
        &self,
        authorization_id: Uuid,
        instance_id: Uuid,
    ) -> Result<RuntimeExecutionContextV1, RuntimeErrorCodeV1> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        // Lock order agrees with close_binding: identity first, execution second.
        let identity_row = tx
            .query_one(statement(
                &format!(
                    "{ENROLLMENT_SELECT}
            WHERE instance_id=$1 FOR UPDATE"
                ),
                vec![instance_id.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(RuntimeErrorCodeV1::RuntimeInvalidProof)?;
        let enrolled = enrollment(&identity_row)?;
        if enrolled.closed {
            return Err(RuntimeErrorCodeV1::RuntimeIdentityRevoked);
        }
        let row = tx
            .query_one(statement(
                "SELECT ticket_claims, execution_id FROM apparatus_runtime_executions
            WHERE authorization_id=$1 AND closed_at IS NULL AND deadline>clock_timestamp()
            FOR UPDATE",
                vec![authorization_id.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(RuntimeErrorCodeV1::RuntimeReplay)?;
        let consumed: Option<Uuid> = row
            .try_get("", "execution_id")
            .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
        if consumed.is_some() {
            return Err(RuntimeErrorCodeV1::RuntimeReplay);
        }
        let claims: serde_json::Value = row
            .try_get("", "ticket_claims")
            .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
        let ticket: RuntimeInvocationTicketV1 =
            serde_json::from_value(claims).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
        if enrolled.project_id != ticket.project_id
            || enrolled.identity.binding != ticket.target.binding_id
            || enrolled.identity.release != ticket.target.release.as_str()
            || enrolled.identity.generation != ticket.target.generation.get() as i64
            || enrolled.identity.grant_revision != ticket.target.grant_revision.get() as i64
            || enrolled.revocation_epoch != ticket.target.revocation_epoch.get()
        {
            return Err(RuntimeErrorCodeV1::RuntimeStaleState);
        }
        let context = RuntimeExecutionContextV1 {
            schema_version: 1,
            execution_id: Uuid::new_v4(),
            instance_id,
            authorization_id,
            project_id: ticket.project_id,
            source: ticket.source,
            source_revocation_epoch: ticket.source_revocation_epoch,
            target: ticket.target,
            operation_sha256: ticket.operation_sha256,
            deadline_unix_ms: ticket.deadline_unix_ms,
        };
        let context_json =
            serde_json::to_value(&context).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
        let consumed = tx
            .execute(statement(
                "UPDATE apparatus_runtime_executions
            SET execution_id=$2,instance_id=$3,accepted_at=clock_timestamp(),execution_context=$4
            WHERE authorization_id=$1 AND execution_id IS NULL AND closed_at IS NULL
              AND deadline>clock_timestamp()
              AND to_timestamp($5::bigint::double precision/1000)>clock_timestamp()",
                vec![
                    authorization_id.into(),
                    context.execution_id.into(),
                    instance_id.into(),
                    context_json.into(),
                    enrolled.expires_at_unix_ms.into(),
                ],
            ))
            .await
            .map_err(unavailable)?;
        if consumed.rows_affected() != 1 {
            return Err(RuntimeErrorCodeV1::RuntimeReplay);
        }
        tx.commit().await.map_err(unavailable)?;
        Ok(context)
    }

    async fn execution(
        &self,
        execution_id: Uuid,
    ) -> Result<RuntimeExecutionContextV1, RuntimeErrorCodeV1> {
        let row = self
            .db
            .query_one(statement(
                "SELECT e.execution_context
            FROM apparatus_runtime_executions e JOIN apparatus_runtime_enrollments i
              ON i.instance_id=e.instance_id
            WHERE e.execution_id=$1 AND e.closed_at IS NULL AND i.closed_at IS NULL
              AND e.deadline>clock_timestamp() AND i.expires_at>clock_timestamp()",
                vec![execution_id.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(RuntimeErrorCodeV1::RuntimeIdentityRevoked)?;
        let value: serde_json::Value = row
            .try_get("", "execution_context")
            .map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
        serde_json::from_value(value).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)
    }

    async fn close_binding(
        &self,
        project_id: Uuid,
        binding_id: Uuid,
    ) -> Result<(), RuntimeErrorCodeV1> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        lock(&tx, "enrollment", project_id, binding_id).await?;
        lock(&tx, "execution", project_id, binding_id).await?;
        tx.execute(statement(
            "UPDATE apparatus_runtime_enrollments SET closed_at=clock_timestamp()
            WHERE project_id=$1 AND binding_id=$2 AND closed_at IS NULL",
            vec![project_id.into(), binding_id.into()],
        ))
        .await
        .map_err(unavailable)?;
        tx.execute(statement(
            "UPDATE apparatus_runtime_executions SET closed_at=clock_timestamp()
            WHERE project_id=$1 AND binding_id=$2 AND closed_at IS NULL",
            vec![project_id.into(), binding_id.into()],
        ))
        .await
        .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)
    }
}
