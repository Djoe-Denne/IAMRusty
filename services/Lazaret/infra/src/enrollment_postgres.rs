//! Postgres `apparatus_enrollments` adapter (Lazaret DB).

use std::sync::Arc;

use async_trait::async_trait;
use lazaret_domain::{EnrollmentStore, IdentityError, WorkloadIdentity};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, QueryResult, Statement};
use uuid::Uuid;

/// Postgres enrollment registry keyed by certificate fingerprint.
#[derive(Clone)]
pub struct PostgresEnrollmentRegistry {
    db: DatabaseConnection,
}

impl PostgresEnrollmentRegistry {
    /// Bind to the Lazaret write connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Bind from a shared pool handle.
    #[must_use]
    pub fn from_arc(db: &Arc<DatabaseConnection>) -> Self {
        Self {
            db: db.as_ref().clone(),
        }
    }
}

fn store_failed() -> IdentityError {
    IdentityError::EnrollmentStore("query failed".to_owned())
}

fn identity_from_row(row: &QueryResult) -> Result<WorkloadIdentity, IdentityError> {
    let instance: Uuid = row.try_get("", "instance").map_err(|_| store_failed())?;
    let binding: Uuid = row.try_get("", "binding_id").map_err(|_| store_failed())?;
    let release: String = row.try_get("", "release").map_err(|_| store_failed())?;
    let generation: i64 = row.try_get("", "generation").map_err(|_| store_failed())?;
    let grant_revision: i64 = row
        .try_get("", "grant_revision")
        .map_err(|_| store_failed())?;
    WorkloadIdentity::try_new(instance, binding, release, generation, grant_revision)
        .map_err(|_| IdentityError::EnrollmentStore("invalid stored identity".to_owned()))
}

async fn put_async(
    db: &DatabaseConnection,
    fingerprint: &str,
    identity: &WorkloadIdentity,
) -> Result<(), IdentityError> {
    let inserted = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r"
INSERT INTO apparatus_enrollments
    (fingerprint, binding_id, instance, release, generation, grant_revision)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT DO NOTHING
RETURNING fingerprint
            ",
            [
                fingerprint.into(),
                identity.binding.into(),
                identity.instance.into(),
                identity.release.clone().into(),
                identity.generation.into(),
                identity.grant_revision.into(),
            ],
        ))
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "enrollment insert failed");
            store_failed()
        })?;
    if inserted.is_some() {
        return Ok(());
    }
    let existing = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT binding_id FROM apparatus_enrollments WHERE fingerprint = $1",
            [fingerprint.into()],
        ))
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "enrollment conflict lookup failed");
            store_failed()
        })?;
    match existing {
        Some(row) => {
            let binding: Uuid = row.try_get("", "binding_id").map_err(|_| store_failed())?;
            if binding == identity.binding {
                Ok(())
            } else {
                Err(IdentityError::BindingAlreadyEnrolled)
            }
        }
        None => Err(IdentityError::BindingAlreadyEnrolled),
    }
}

async fn get_async(
    db: &DatabaseConnection,
    fingerprint: &str,
) -> Result<Option<WorkloadIdentity>, IdentityError> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r"
SELECT instance, binding_id, release, generation, grant_revision
FROM apparatus_enrollments
WHERE fingerprint = $1
            ",
            [fingerprint.into()],
        ))
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "enrollment lookup failed");
            store_failed()
        })?;
    row.map(|row| identity_from_row(&row)).transpose()
}

async fn binding_enrolled_async(
    db: &DatabaseConnection,
    binding: Uuid,
) -> Result<bool, IdentityError> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT 1 FROM apparatus_enrollments WHERE binding_id = $1",
            [binding.into()],
        ))
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "enrollment binding lookup failed");
            store_failed()
        })?;
    Ok(row.is_some())
}

async fn revoke_binding_async(db: &DatabaseConnection, binding: Uuid) -> Result<(), IdentityError> {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM apparatus_enrollments WHERE binding_id = $1",
        [binding.into()],
    ))
    .await
    .map_err(|err| {
        tracing::error!(error = %err, "enrollment revoke failed");
        store_failed()
    })?;
    Ok(())
}

#[async_trait]
impl EnrollmentStore for PostgresEnrollmentRegistry {
    async fn put(
        &self,
        fingerprint: String,
        identity: WorkloadIdentity,
    ) -> Result<(), IdentityError> {
        put_async(&self.db, &fingerprint, &identity).await
    }

    async fn get(&self, fingerprint: &str) -> Option<WorkloadIdentity> {
        match get_async(&self.db, fingerprint).await {
            Ok(identity) => identity,
            Err(err) => {
                tracing::error!(error = %err, "enrollment get failed closed");
                None
            }
        }
    }

    async fn binding_enrolled(&self, binding: Uuid) -> Result<bool, IdentityError> {
        binding_enrolled_async(&self.db, binding).await
    }

    async fn revoke_binding(&self, binding: Uuid) -> Result<(), IdentityError> {
        revoke_binding_async(&self.db, binding).await
    }
}
