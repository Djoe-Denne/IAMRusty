//! SQL reader/writer for binding consents and grants (privileged caller).

use async_trait::async_trait;
use manifesto_application::{
    ApplicationError, BindingConsentWriter, BindingGrantSnapshotReader,
    BindingGrantSnapshotResponse, CapabilityConsentSnapshot, PrincipalMembershipSnapshot,
    UpsertBindingConsentRequest,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait};
use uuid::Uuid;

const SELECT_BINDING: &str = "SELECT pc.id AS component_id, pc.project_id, \
     p.status AS project_status, pc.status AS component_status, \
     ab.source, ab.digest, ab.desired_generation, ab.observed_generation, \
     ab.grant_revision, ab.declared_capabilities AS declared \
     FROM project_components pc \
     INNER JOIN projects p ON p.id = pc.project_id \
     INNER JOIN apparatus_bindings ab ON ab.component_id = pc.id \
     WHERE pc.project_id = $1 AND pc.id = $2";

const SELECT_CONSENTS: &str = "SELECT capability, status, grant_revision \
     FROM apparatus_capability_consents WHERE component_id = $1 \
     ORDER BY capability";

const SELECT_ACTIVE_MEMBER: &str = "SELECT 1 FROM project_members \
     WHERE project_id = $1 AND user_id = $2 AND removed_at IS NULL \
     LIMIT 1";

const BUMP_REVISION: &str = "UPDATE apparatus_bindings AS ab \
     SET grant_revision = ab.grant_revision + 1 \
     FROM project_components AS pc \
     WHERE ab.component_id = pc.id AND pc.id = $1 AND pc.project_id = $2 \
     RETURNING ab.grant_revision";

const STAMP_CONSENTS: &str = "UPDATE apparatus_capability_consents \
     SET grant_revision = $2 WHERE component_id = $1";

const UPSERT_CONSENT: &str = "INSERT INTO apparatus_capability_consents \
     (component_id, capability, status, grant_revision) \
     VALUES ($1, $2, $3, $4) \
     ON CONFLICT (component_id, capability) DO UPDATE \
     SET status = EXCLUDED.status, grant_revision = EXCLUDED.grant_revision";

/// SeaORM reader for binding grant snapshots.
pub struct SqlBindingGrantSnapshotReader {
    db: DatabaseConnection,
}

impl SqlBindingGrantSnapshotReader {
    /// Construct a reader on a database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BindingGrantSnapshotReader for SqlBindingGrantSnapshotReader {
    async fn load(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshotResponse, ApplicationError> {
        load_snapshot(&self.db, project_id, component_id, principal).await
    }
}

/// SeaORM writer: upsert consent + bump `grant_revision` in one transaction.
pub struct SqlBindingConsentWriter {
    db: DatabaseConnection,
}

impl SqlBindingConsentWriter {
    /// Construct a writer on a write connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BindingConsentWriter for SqlBindingConsentWriter {
    async fn upsert(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        request: &UpsertBindingConsentRequest,
    ) -> Result<BindingGrantSnapshotResponse, ApplicationError> {
        validate_consent(request)?;
        let txn = self
            .db
            .begin()
            .await
            .map_err(|error| map_db_write(&error))?;
        let bumped = txn
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                BUMP_REVISION,
                [component_id.into(), project_id.into()],
            ))
            .await
            .map_err(|error| map_db_write(&error))?;
        let Some(bumped) = bumped else {
            return Err(ApplicationError::NotFound(format!(
                "Binding not found for project {project_id} component {component_id}"
            )));
        };
        let grant_revision: i64 = bumped
            .try_get("", "grant_revision")
            .map_err(|error| map_db_write(&error))?;
        txn.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            STAMP_CONSENTS,
            [component_id.into(), grant_revision.into()],
        ))
        .await
        .map_err(|error| map_db_write(&error))?;
        txn.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            UPSERT_CONSENT,
            [
                component_id.into(),
                request.capability.clone().into(),
                request.status.clone().into(),
                grant_revision.into(),
            ],
        ))
        .await
        .map_err(|error| map_db_write(&error))?;
        let snapshot = load_snapshot(&txn, project_id, component_id, None).await?;
        txn.commit().await.map_err(|error| map_db_write(&error))?;
        Ok(snapshot)
    }
}

fn validate_consent(request: &UpsertBindingConsentRequest) -> Result<(), ApplicationError> {
    if request.status != "consented" && request.status != "revoked" {
        return Err(ApplicationError::Validation(
            "status must be consented or revoked".to_owned(),
        ));
    }
    request
        .capability
        .parse::<apparatus_contracts::Capability>()
        .map_err(|_| {
            ApplicationError::Validation(format!("unknown capability {}", request.capability))
        })?;
    Ok(())
}

async fn load_snapshot<C: ConnectionTrait>(
    conn: &C,
    project_id: Uuid,
    component_id: Uuid,
    principal: Option<Uuid>,
) -> Result<BindingGrantSnapshotResponse, ApplicationError> {
    let row = conn
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            SELECT_BINDING,
            [project_id.into(), component_id.into()],
        ))
        .await
        .map_err(|error| map_db(&error))?;
    let Some(row) = row else {
        return Err(ApplicationError::NotFound(format!(
            "Binding not found for project {project_id} component {component_id}"
        )));
    };

    let consents = load_consents(conn, component_id).await?;
    let principal = match principal {
        Some(user_id) => Some(PrincipalMembershipSnapshot {
            user_id,
            active: load_principal_active(conn, project_id, user_id).await?,
        }),
        None => None,
    };

    Ok(BindingGrantSnapshotResponse {
        component_id: row
            .try_get("", "component_id")
            .map_err(|error| map_db(&error))?,
        project_id: row
            .try_get("", "project_id")
            .map_err(|error| map_db(&error))?,
        project_status: row
            .try_get("", "project_status")
            .map_err(|error| map_db(&error))?,
        component_status: row
            .try_get("", "component_status")
            .map_err(|error| map_db(&error))?,
        source: row.try_get("", "source").map_err(|error| map_db(&error))?,
        digest: row.try_get("", "digest").map_err(|error| map_db(&error))?,
        desired_generation: row
            .try_get("", "desired_generation")
            .map_err(|error| map_db(&error))?,
        observed_generation: row
            .try_get("", "observed_generation")
            .map_err(|error| map_db(&error))?,
        grant_revision: row
            .try_get("", "grant_revision")
            .map_err(|error| map_db(&error))?,
        declared: decode_declared(&row),
        consents,
        principal,
    })
}

fn decode_declared(row: &sea_orm::QueryResult) -> Vec<String> {
    if let Ok(value) = row.try_get::<serde_json::Value>("", "declared") {
        return serde_json::from_value(value).unwrap_or_default();
    }
    if let Ok(bytes) = row.try_get::<Vec<u8>>("", "declared") {
        return serde_json::from_slice(&bytes).unwrap_or_default();
    }
    Vec::new()
}

async fn load_consents<C: ConnectionTrait>(
    conn: &C,
    component_id: Uuid,
) -> Result<Vec<CapabilityConsentSnapshot>, ApplicationError> {
    let rows = conn
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            SELECT_CONSENTS,
            [component_id.into()],
        ))
        .await
        .map_err(|error| map_db(&error))?;
    let mut consents = Vec::with_capacity(rows.len());
    for row in rows {
        consents.push(CapabilityConsentSnapshot {
            capability: row
                .try_get("", "capability")
                .map_err(|error| map_db(&error))?,
            status: row.try_get("", "status").map_err(|error| map_db(&error))?,
            grant_revision: row
                .try_get("", "grant_revision")
                .map_err(|error| map_db(&error))?,
        });
    }
    Ok(consents)
}

async fn load_principal_active<C: ConnectionTrait>(
    conn: &C,
    project_id: Uuid,
    user_id: Uuid,
) -> Result<bool, ApplicationError> {
    let row = conn
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            SELECT_ACTIVE_MEMBER,
            [project_id.into(), user_id.into()],
        ))
        .await
        .map_err(|error| map_db(&error))?;
    Ok(row.is_some())
}

fn map_db(error: &DbErr) -> ApplicationError {
    ApplicationError::Internal(format!("failed to read binding grant snapshot: {error}"))
}

fn map_db_write(error: &DbErr) -> ApplicationError {
    ApplicationError::Internal(format!("failed to write binding consent: {error}"))
}
