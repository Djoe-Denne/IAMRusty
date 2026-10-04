//! SeaORM SigningKeyRegistry implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use iam_domain::entity::signing_key::{
    admission_denied, same_effective_signing_binding, SigningKey, SigningKeyAdmissionHistory,
    SigningKeyAdmissionReason, SigningKeyLifecyclePolicy, SigningKeyLifecyclePreflight,
    SigningKeyPublicationSnapshot, SigningKeyStatus, SigningProviderType, TrustScope,
};
use iam_domain::error::DomainError;
use iam_domain::port::repository::SigningKeyRegistry;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection,
    DatabaseTransaction, DbBackend, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, Statement, TransactionTrait,
};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::repository::entity::signing_keys::{self, Entity as SigningKeys};

/// Postgres-backed signing key registry.
#[derive(Clone)]
pub struct SeaOrmSigningKeyRegistry {
    db: Arc<DatabaseConnection>,
    policy: Arc<SigningKeyLifecyclePolicy>,
    access_token_expiration_seconds: u64,
}

impl SeaOrmSigningKeyRegistry {
    #[must_use]
    /// `db` must be the primary writer's autocommit connection, never a replica.
    pub fn new(
        db: Arc<DatabaseConnection>,
        policy: Arc<SigningKeyLifecyclePolicy>,
        access_token_expiration_seconds: u64,
    ) -> Result<Self, DomainError> {
        policy.validate()?;
        policy.access_token_retention(access_token_expiration_seconds, Utc::now())?;
        Ok(Self {
            db,
            policy,
            access_token_expiration_seconds,
        })
    }

    /// Non-secret preflight; report recovery, never use this as a global auth gate.
    pub async fn preflight(&self) -> Result<SigningKeyLifecyclePreflight, DomainError> {
        let snapshot = self.jwks_publication_snapshot().await?;
        let report = self
            .policy
            .preflight_publication(&snapshot, self.access_token_expiration_seconds)?;
        if report.recovery_required {
            tracing::warn!(
                event = "signing_admission_recovery_required",
                "Existing signing publication requires explicit recovery; new admission blocked"
            );
        }
        Ok(report)
    }

    async fn begin_lifecycle(
        &self,
        scope: &TrustScope,
        organization_id: Option<Uuid>,
        issuer: Option<&str>,
        operation: LifecycleOperation<'_>,
    ) -> Result<(DatabaseTransaction, LifecycleState), DomainError> {
        let tx = self.db.begin().await.map_err(registry_error)?;
        lock_registry_scope(&tx, "iam-signing-jwks-admission-v1").await?;
        match (scope, organization_id) {
            (TrustScope::Platform, None) => lock_registry_scope(&tx, "signer-platform").await?,
            (TrustScope::Organization, Some(org)) => {
                lock_registry_scope(&tx, &format!("signer-org:{org}")).await?
            }
            _ => return Err(DomainError::InvalidSigningKeyMaterial),
        }
        let affected = affected_rows(scope, organization_id, operation);
        // Project only issuers of rows we may mutate/inspect, never historical PEMs.
        let mut issuers: std::collections::BTreeSet<String> = SigningKeys::find()
            .select_only()
            .column(signing_keys::Column::Issuer)
            .distinct()
            .filter(affected.clone())
            .into_tuple::<String>()
            .all(&tx)
            .await
            .map_err(registry_error)?
            .into_iter()
            .collect();
        if let Some(issuer) = issuer {
            issuers.insert(issuer.to_string());
        }
        for issuer in issuers {
            lock_registry_scope(&tx, &format!("signer-issuer:{issuer}")).await?;
        }
        let affected = SigningKeys::find()
            .filter(affected)
            .order_by_asc(signing_keys::Column::Id)
            .lock_exclusive()
            .all(&tx)
            .await
            .map_err(registry_error)?;
        // Capture AFTER every lock wait; subsequent reads share this clock and the
        // global writer exclusion, not transaction_timestamp() or another app clock.
        let clock = tx
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                "SELECT statement_timestamp() AS as_of",
            ))
            .await
            .map_err(registry_error)?
            .ok_or(DomainError::InvalidToken)?;
        let as_of: chrono::DateTime<chrono::FixedOffset> =
            clock.try_get("", "as_of").map_err(registry_error)?;
        let as_of = as_of.with_timezone(&Utc);
        let affected = affected
            .into_iter()
            .map(to_domain)
            .collect::<Result<Vec<_>, _>>()?;
        if matches!(operation, LifecycleOperation::Revoke) {
            // All non-Revoked target rows are affected, including expired Retiring.
            // No unrelated publication/history is needed for this reducing operation.
            return Ok((
                tx,
                LifecycleState {
                    keys: affected,
                    history: vec![],
                    as_of,
                    issuer_conflict: false,
                    invalid_retirement: false,
                    platform_epoch_exists: false,
                },
            ));
        }
        let retention = self
            .policy
            .access_token_retention(self.access_token_expiration_seconds, as_of)?;
        let cutoff = as_of
            .checked_sub_signed(retention)
            .ok_or(DomainError::InvalidSigningKeyMaterial)?;
        let publication = SigningKeys::find()
            .filter(publication_rows(cutoff, as_of))
            .order_by_asc(signing_keys::Column::Id)
            .all(&tx)
            .await
            .map_err(registry_error)?
            .into_iter()
            .map(to_domain)
            .collect::<Result<Vec<_>, _>>()?;
        // Global lock makes these targeted historical existence reads stable. They
        // retain issuer ownership and no-bootstrap-reactivation without loading rows.
        let facts = tx.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM signing_keys WHERE status='retiring' AND updated_at > $1::timestamp) AS invalid_retirement, EXISTS(SELECT 1 FROM signing_keys WHERE issuer=$2::text AND (trust_scope IS DISTINCT FROM $3::text OR organization_id IS DISTINCT FROM $4::uuid)) AS issuer_conflict, CASE WHEN $5::boolean THEN EXISTS(SELECT 1 FROM signing_keys WHERE trust_scope='platform' AND organization_id IS NULL) ELSE false END AS platform_epoch_exists",
        vec![as_of.naive_utc().into(), issuer.map(str::to_string).into(), scope_name(scope).into(), organization_id.into(), matches!(operation, LifecycleOperation::Bootstrap(_)).into()]))
        .await.map_err(registry_error)?.ok_or(DomainError::InvalidToken)?;
        let mut history = Vec::new();
        if operation.needs_churn() {
            if let Some(org) = organization_id {
                // The newest N admissions give the exact Nth-newest retry deadline
                // and threshold. This LIMIT applies to quota evidence, NEVER JWKS.
                let limit = i64::try_from(self.policy.organization_churn_evidence_limit())
                    .map_err(|_| DomainError::InvalidSigningKeyMaterial)?;
                let rows = tx.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT organization_id,lifecycle_admitted_at FROM signing_keys WHERE organization_id=$1::uuid AND lifecycle_admitted_at > $2::timestamptz - INTERVAL '3600 seconds' ORDER BY lifecycle_admitted_at DESC LIMIT $3",
                vec![org.into(), as_of.into(), limit.into()])).await.map_err(registry_error)?;
                for row in rows {
                    let admitted_at: chrono::DateTime<chrono::FixedOffset> = row
                        .try_get("", "lifecycle_admitted_at")
                        .map_err(registry_error)?;
                    history.push(SigningKeyAdmissionHistory {
                        organization_id: row
                            .try_get("", "organization_id")
                            .map_err(registry_error)?,
                        admitted_at: admitted_at.with_timezone(&Utc),
                    });
                }
            }
        }
        Ok((
            tx,
            LifecycleState {
                keys: merge_lifecycle_rows(publication, affected),
                history,
                as_of,
                issuer_conflict: facts
                    .try_get("", "issuer_conflict")
                    .map_err(registry_error)?,
                invalid_retirement: facts
                    .try_get("", "invalid_retirement")
                    .map_err(registry_error)?,
                platform_epoch_exists: facts
                    .try_get("", "platform_epoch_exists")
                    .map_err(registry_error)?,
            },
        ))
    }

    fn check_plan(
        &self,
        state: &LifecycleState,
        proposed: &[SigningKey],
        new_key: Option<&SigningKey>,
    ) -> Result<(), DomainError> {
        if state.invalid_retirement {
            tracing::warn!(
                event = "signing_admission_recovery_required",
                "Future legacy retirement requires explicit recovery"
            );
            return Err(admission_denied(SigningKeyAdmissionReason::Capacity));
        }
        let result = self.policy.check_admission(
            proposed,
            &state.history,
            new_key,
            self.access_token_expiration_seconds,
            state.as_of,
        );
        if result.is_err()
            && self
                .policy
                .check_admission(
                    &state.keys,
                    &[],
                    None,
                    self.access_token_expiration_seconds,
                    state.as_of,
                )
                .is_err()
        {
            tracing::warn!(
                event = "signing_admission_recovery_required",
                "Existing signing publication requires explicit recovery; no eviction performed"
            );
        }
        result
    }
}

struct LifecycleState {
    keys: Vec<SigningKey>,
    history: Vec<SigningKeyAdmissionHistory>,
    as_of: DateTime<Utc>,
    issuer_conflict: bool,
    invalid_retirement: bool,
    platform_epoch_exists: bool,
}

#[derive(Clone, Copy)]
enum LifecycleOperation<'a> {
    Insert(&'a SigningKey),
    Replace(&'a SigningKey),
    Update(&'a SigningKey),
    Bootstrap(&'a SigningKey),
    Revoke,
}
impl LifecycleOperation<'_> {
    fn needs_churn(self) -> bool {
        matches!(self, Self::Insert(_) | Self::Replace(_))
    }
}

fn scope_rows(scope: &TrustScope, organization_id: Option<Uuid>) -> Condition {
    let owner = match organization_id {
        Some(org) => signing_keys::Column::OrganizationId.eq(org),
        None => signing_keys::Column::OrganizationId.is_null(),
    };
    Condition::all()
        .add(signing_keys::Column::TrustScope.eq(scope_name(scope)))
        .add(owner)
}

fn affected_rows(
    scope: &TrustScope,
    organization_id: Option<Uuid>,
    operation: LifecycleOperation<'_>,
) -> Condition {
    let active = || {
        Condition::all()
            .add(scope_rows(scope, organization_id))
            .add(signing_keys::Column::Status.eq("active"))
    };
    let identity = |key: &SigningKey| {
        Condition::any()
            .add(signing_keys::Column::Id.eq(key.id))
            .add(signing_keys::Column::Kid.eq(key.kid.clone()))
    };
    match operation {
        LifecycleOperation::Insert(key) => identity(key),
        LifecycleOperation::Replace(key) | LifecycleOperation::Bootstrap(key) => {
            Condition::any().add(identity(key)).add(active())
        }
        LifecycleOperation::Update(key)
            if *scope == TrustScope::Platform && key.status == SigningKeyStatus::Active =>
        {
            Condition::any().add(identity(key)).add(active())
        }
        LifecycleOperation::Update(key) => identity(key),
        LifecycleOperation::Revoke => Condition::all()
            .add(scope_rows(scope, organization_id))
            .add(signing_keys::Column::Status.ne("revoked")),
    }
}

fn publication_rows(cutoff: DateTime<Utc>, as_of: DateTime<Utc>) -> Condition {
    // Inverse cutoff avoids adding an interval to arbitrary legacy timestamps.
    // Complete eligible set; no LIMIT, no locking unrelated published rows.
    Condition::any()
        .add(signing_keys::Column::Status.is_in(["pending", "active"]))
        .add(
            Condition::all()
                .add(signing_keys::Column::Status.eq("retiring"))
                .add(signing_keys::Column::UpdatedAt.gt(cutoff.naive_utc()))
                .add(signing_keys::Column::UpdatedAt.lte(as_of.naive_utc())),
        )
}

fn merge_lifecycle_rows(
    publication: Vec<SigningKey>,
    affected: Vec<SigningKey>,
) -> Vec<SigningKey> {
    let mut by_id = std::collections::BTreeMap::new();
    for key in publication.into_iter().chain(affected) {
        by_id.insert(key.id, key);
    }
    by_id.into_values().collect()
}
fn scope_name(scope: &TrustScope) -> &'static str {
    match scope {
        TrustScope::Platform => "platform",
        TrustScope::Organization => "organization",
    }
}
fn conflict() -> DomainError {
    admission_denied(SigningKeyAdmissionReason::EpochConflict)
}

fn check_issuer_owner(state: &LifecycleState) -> Result<(), DomainError> {
    if state.issuer_conflict {
        return Err(conflict());
    }
    Ok(())
}

async fn insert_admitted(
    tx: &DatabaseTransaction,
    key: &SigningKey,
    as_of: DateTime<Utc>,
) -> Result<SigningKey, DomainError> {
    // Explicit same DB as_of for admission, creation and lifecycle timestamps.
    let row = tx.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO signing_keys (id,kid,algorithm,trust_scope,issuer,provider_type,provider_key_ref,credential_ref,public_key,status,organization_id,created_at,updated_at,lifecycle_admitted_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,($12::timestamptz AT TIME ZONE 'UTC'),($12::timestamptz AT TIME ZONE 'UTC'),$12::timestamptz) RETURNING *",
        vec![key.id.into(), key.kid.clone().into(), key.algorithm.clone().into(), scope_name(&key.trust_scope).into(), key.issuer.clone().into(), String::from(&key.provider_type).into(), key.provider_key_ref.clone().into(), key.credential_ref.clone().into(), key.public_key.clone().into(), String::from(&key.status).into(), key.organization_id.into(), as_of.into()]))
        .await.map_err(registry_error)?.ok_or_else(conflict)?;
    to_domain(signing_keys::Model::from_query_result(&row, "").map_err(registry_error)?)
}

async fn persist_changes(
    tx: &DatabaseTransaction,
    changed: &mut [SigningKey],
) -> Result<(), DomainError> {
    // Retire old Active BEFORE promoting Pending; otherwise the partial unique
    // Active index can fail merely because the Pending UUID sorts first.
    order_changes(changed);
    for key in changed {
        to_active(key).update(tx).await.map_err(registry_error)?;
    }
    Ok(())
}

fn order_changes(changed: &mut [SigningKey]) {
    changed.sort_by_key(|key| (key.status == SigningKeyStatus::Active, key.id));
}

fn registry_error(_: sea_orm::DbErr) -> DomainError {
    DomainError::RepositoryError("signing registry transaction failed".into())
}

// The registry has no organization row (Hive owns it). A PostgreSQL transaction
// advisory lock supplies the shared writer critical section, even for zero rows.
async fn lock_registry_scope(tx: &DatabaseTransaction, scope: &str) -> Result<(), DomainError> {
    tx.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        vec![scope.into()],
    ))
    .await
    .map_err(registry_error)?;
    Ok(())
}

fn to_domain(model: signing_keys::Model) -> Result<SigningKey, DomainError> {
    Ok(SigningKey {
        id: model.id,
        kid: model.kid,
        algorithm: model.algorithm,
        trust_scope: match model.trust_scope.as_str() {
            "platform" => TrustScope::Platform,
            "organization" => TrustScope::Organization,
            other => {
                return Err(DomainError::RepositoryError(format!(
                    "unknown trust_scope: {other}"
                )))
            }
        },
        issuer: model.issuer,
        provider_type: SigningProviderType::from_str(&model.provider_type)
            .map_err(DomainError::RepositoryError)?,
        provider_key_ref: model.provider_key_ref,
        credential_ref: model.credential_ref,
        public_key: model.public_key,
        status: SigningKeyStatus::from_str(&model.status).map_err(DomainError::RepositoryError)?,
        organization_id: model.organization_id,
        created_at: DateTime::<Utc>::from_naive_utc_and_offset(model.created_at, Utc),
        updated_at: DateTime::<Utc>::from_naive_utc_and_offset(model.updated_at, Utc),
    })
}

fn to_active(key: &SigningKey) -> signing_keys::ActiveModel {
    signing_keys::ActiveModel {
        id: ActiveValue::Set(key.id),
        kid: ActiveValue::Set(key.kid.clone()),
        algorithm: ActiveValue::Set(key.algorithm.clone()),
        trust_scope: ActiveValue::Set(match key.trust_scope {
            TrustScope::Platform => "platform".to_string(),
            TrustScope::Organization => "organization".to_string(),
        }),
        issuer: ActiveValue::Set(key.issuer.clone()),
        provider_type: ActiveValue::Set(String::from(&key.provider_type)),
        provider_key_ref: ActiveValue::Set(key.provider_key_ref.clone()),
        credential_ref: ActiveValue::Set(key.credential_ref.clone()),
        public_key: ActiveValue::Set(key.public_key.clone()),
        status: ActiveValue::Set(String::from(&key.status)),
        organization_id: ActiveValue::Set(key.organization_id),
        created_at: ActiveValue::Set(key.created_at.naive_utc()),
        updated_at: ActiveValue::Set(key.updated_at.naive_utc()),
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use sea_orm::{Iden, Iterable, MockDatabase, MockExecResult, ModelTrait, Value};
    use std::collections::BTreeMap;

    fn row(org: Uuid, status: &str) -> signing_keys::Model {
        let now = Utc::now().naive_utc();
        signing_keys::Model {
            id: Uuid::new_v4(),
            kid: iam_domain::entity::signing_key::opaque_kid(),
            algorithm: "RS256".into(),
            trust_scope: "organization".into(),
            issuer: "https://iam.example.test/iam/orgs/own".into(),
            provider_type: "remote_http".into(),
            provider_key_ref: "key-ref".into(),
            credential_ref: Some("credential-ref".into()),
            public_key: include_str!("../../../config/keys/test-platform.pub").into(),
            status: status.into(),
            organization_id: Some(org),
            created_at: now,
            updated_at: now,
        }
    }
    // Match SELECT DISTINCT issuer, not a full model: tuple extraction uses the
    // first projected column, which a full-model mock would make `algorithm`.
    fn issuer_projection(rows: &[signing_keys::Model]) -> Vec<BTreeMap<String, Value>> {
        rows.iter()
            .map(|row| row.issuer.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|issuer| BTreeMap::from([("issuer".into(), issuer.into())]))
            .collect()
    }

    fn database(
        rows: Vec<signing_keys::Model>,
        history: Vec<BTreeMap<String, Value>>,
        as_of: DateTime<Utc>,
    ) -> MockDatabase {
        let clock = BTreeMap::from([("as_of".to_string(), as_of.fixed_offset().into())]);
        let platform_epoch_exists = rows.iter().any(|row| row.trust_scope == "platform");
        let needs_churn = rows.iter().any(|row| row.organization_id.is_some());
        let facts = BTreeMap::from([
            ("invalid_retirement".to_string(), false.into()),
            ("issuer_conflict".to_string(), false.into()),
            (
                "platform_epoch_exists".to_string(),
                platform_epoch_exists.into(),
            ),
        ]);
        let database = MockDatabase::new(DbBackend::Postgres)
            .append_exec_results((0..3).map(|_| MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }))
            .append_query_results([issuer_projection(&rows)])
            .append_query_results([rows.clone()])
            .append_query_results([vec![clock]])
            .append_query_results([rows])
            .append_query_results([vec![facts]]);
        if needs_churn {
            database.append_query_results([history])
        } else {
            database
        }
    }
    fn repository(db: Arc<DatabaseConnection>) -> SeaOrmSigningKeyRegistry {
        SeaOrmSigningKeyRegistry::new(db, Arc::new(SigningKeyLifecyclePolicy::new().unwrap()), 900)
            .unwrap()
    }
    fn log(db: Arc<DatabaseConnection>) -> String {
        format!("{:?}", Arc::try_unwrap(db).unwrap().into_transaction_log()).replace("\\\"", "\"")
    }
    fn assert_order(log: &str) {
        let global = log.find("iam-signing-jwks-admission-v1").unwrap();
        let org = log.find("signer-org:").unwrap();
        let issuer = log.find("signer-issuer:").unwrap();
        let rows = log.find("FOR UPDATE").unwrap();
        let clock = log.find("SELECT statement_timestamp() AS as_of").unwrap();
        assert!(global < org && org < issuer && issuer < rows && rows < clock);
    }

    mod rp1 {
        use super::*;
        use sea_orm::QueryTrait;

        fn targeted_database(
            affected: Vec<signing_keys::Model>,
            publication: Vec<signing_keys::Model>,
            history: Option<Vec<BTreeMap<String, Value>>>,
            issuer_conflict: bool,
            platform_epoch_exists: bool,
            as_of: DateTime<Utc>,
        ) -> MockDatabase {
            let clock = BTreeMap::from([("as_of".to_string(), as_of.fixed_offset().into())]);
            let facts = BTreeMap::from([
                ("invalid_retirement".to_string(), false.into()),
                ("issuer_conflict".to_string(), issuer_conflict.into()),
                (
                    "platform_epoch_exists".to_string(),
                    platform_epoch_exists.into(),
                ),
            ]);
            let database = MockDatabase::new(DbBackend::Postgres)
                .append_exec_results((0..3).map(|_| MockExecResult {
                    rows_affected: 1,
                    last_insert_id: 0,
                }))
                .append_query_results([issuer_projection(&affected)])
                .append_query_results([affected])
                .append_query_results([vec![clock]])
                .append_query_results([publication])
                .append_query_results([vec![facts]]);
            match history {
                Some(history) => database.append_query_results([history]),
                None => database,
            }
        }

        #[test]
        fn query_shapes_lock_only_affected_rows_and_never_limit_complete_publication() {
            let org = Uuid::new_v4();
            let candidate = to_domain(row(org, "active")).unwrap();
            for operation in [
                LifecycleOperation::Insert(&candidate),
                LifecycleOperation::Replace(&candidate),
                LifecycleOperation::Update(&candidate),
            ] {
                let query = SigningKeys::find()
                    .filter(affected_rows(
                        &TrustScope::Organization,
                        Some(org),
                        operation,
                    ))
                    .order_by_asc(signing_keys::Column::Id)
                    .lock_exclusive()
                    .build(DbBackend::Postgres)
                    .to_string();
                assert!(query.contains("WHERE"));
                assert!(query.contains(&candidate.id.to_string()));
                assert!(query.contains(&candidate.kid));
                assert!(query.contains("FOR UPDATE"));
                let issuers = SigningKeys::find()
                    .select_only()
                    .column(signing_keys::Column::Issuer)
                    .distinct()
                    .filter(affected_rows(
                        &TrustScope::Organization,
                        Some(org),
                        operation,
                    ))
                    .build(DbBackend::Postgres)
                    .to_string();
                assert!(issuers.starts_with("SELECT DISTINCT"));
                assert!(!issuers.contains("public_key"));
            }
            let query = SigningKeys::find()
                .filter(publication_rows(
                    Utc::now() - chrono::Duration::seconds(960),
                    Utc::now(),
                ))
                .build(DbBackend::Postgres)
                .to_string();
            assert!(!query.contains("LIMIT"));
            assert!(!query.contains("FOR UPDATE"));
            assert!(query.contains("'pending', 'active'"));
            assert!(query.contains("'retiring'"));
            assert!(query.contains(" > "));
            assert!(query.contains(" <= "));
            let revoke = SigningKeys::find()
                .filter(affected_rows(
                    &TrustScope::Organization,
                    Some(org),
                    LifecycleOperation::Revoke,
                ))
                .build(DbBackend::Postgres)
                .to_string();
            assert!(revoke.contains(&org.to_string()));
            assert!(revoke.contains("<> 'revoked'"));
        }

        #[tokio::test]
        async fn one_key_change_has_one_explicit_write_with_1k_or_10k_historical_rows() {
            let org = Uuid::new_v4();
            let as_of = Utc::now();
            let live = row(org, "active");
            for historical_count in [1000, 10000] {
                let mut oracle: Vec<_> = (0..historical_count)
                    .map(|index| {
                        let mut historical = row(
                            org,
                            if index % 2 == 0 {
                                "revoked"
                            } else {
                                "retiring"
                            },
                        );
                        historical.updated_at = (as_of - chrono::Duration::days(2)).naive_utc();
                        historical.public_key =
                            "historical payload must not be parsed by admission".into();
                        to_domain(historical).unwrap()
                    })
                    .collect();
                oracle.push(to_domain(live.clone()).unwrap());
                let eligible = iam_domain::entity::signing_key::filter_jwks_publication_keys_at(
                    oracle, 900, as_of,
                );
                assert_eq!(eligible.len(), 1);
                assert_eq!(eligible[0].id, live.id);
                // Mock only the rows admitted by the asserted SQL predicates.
                // Actual PostgreSQL plans/cardinality remain E-owned evidence.
                let mut retired = live.clone();
                retired.status = "retiring".into();
                retired.updated_at = as_of.naive_utc();
                let db = Arc::new(
                    targeted_database(
                        vec![live.clone()],
                        vec![live.clone()],
                        None,
                        false,
                        false,
                        as_of,
                    )
                    .append_query_results([vec![retired]])
                    .into_connection(),
                );
                let registry = repository(db.clone());
                let mut candidate = to_domain(live.clone()).unwrap();
                candidate.status = SigningKeyStatus::Retiring;
                registry.update(&candidate).await.unwrap();
                drop(registry);
                let log = log(db);
                assert_order(&log);
                assert_eq!(log.matches("UPDATE \"signing_keys\"").count(), 1);
                assert_eq!(log.matches("FOR UPDATE").count(), 1);
                assert!(!log.contains("lifecycle_admitted_at"));
                assert!(!log.contains("historical payload"));
                let locked = log.split("FOR UPDATE").next().unwrap();
                assert!(locked.contains(&live.id.to_string()));
            }
        }

        #[tokio::test]
        async fn targeted_revoked_or_expired_identity_conflicts_cannot_reactivate() {
            let org = Uuid::new_v4();
            let as_of = Utc::now();
            for status in ["revoked", "retiring"] {
                let mut historical = row(org, status);
                historical.updated_at = (as_of - chrono::Duration::days(2)).naive_utc();
                for duplicate_id in [true, false] {
                    let mut candidate = to_domain(historical.clone()).unwrap();
                    candidate.status = SigningKeyStatus::Pending;
                    if duplicate_id {
                        candidate.kid = iam_domain::entity::signing_key::opaque_kid();
                    } else {
                        candidate.id = Uuid::new_v4();
                    }
                    let db = Arc::new(
                        targeted_database(
                            vec![historical.clone()],
                            vec![],
                            Some(vec![]),
                            false,
                            false,
                            as_of,
                        )
                        .into_connection(),
                    );
                    let registry = repository(db.clone());
                    assert!(matches!(
                        registry.insert(&candidate).await,
                        Err(DomainError::SigningKeyAdmissionDenied {
                            reason: SigningKeyAdmissionReason::EpochConflict,
                            ..
                        })
                    ));
                    drop(registry);
                    let log = log(db);
                    assert!(log.contains("ROLLBACK"));
                    assert!(!log.contains("INSERT INTO"));
                }
                let db = Arc::new(
                    targeted_database(vec![historical.clone()], vec![], None, false, false, as_of)
                        .into_connection(),
                );
                let registry = repository(db.clone());
                let mut candidate = to_domain(historical).unwrap();
                candidate.status = SigningKeyStatus::Active;
                assert!(registry.update(&candidate).await.is_err());
                drop(registry);
                let log = log(db);
                assert!(log.contains("ROLLBACK"));
                assert!(!log.contains("UPDATE "));
            }
        }

        #[tokio::test]
        async fn historical_issuer_ownership_and_platform_epoch_exist_without_payload_reads() {
            let org = Uuid::new_v4();
            let as_of = Utc::now();
            let candidate = to_domain(row(org, "pending")).unwrap();
            let db = Arc::new(
                targeted_database(vec![], vec![], Some(vec![]), true, false, as_of)
                    .into_connection(),
            );
            let registry = repository(db.clone());
            assert!(registry.insert(&candidate).await.is_err());
            drop(registry);
            let issuer_log = log(db);
            assert!(issuer_log.contains("WHERE issuer=$2::text"));
            assert!(issuer_log.contains("IS DISTINCT FROM"));
            assert!(issuer_log.contains("ROLLBACK"));
            let db = Arc::new(
                targeted_database(vec![], vec![], None, false, true, as_of).into_connection(),
            );
            let registry = repository(db.clone());
            assert!(bootstrap_platform_signing_key(
                &registry,
                &iam_domain::entity::signing_key::opaque_kid(),
                "https://iam.example.test/iam",
                &candidate.public_key,
                "platform-ref",
                SigningProviderType::PemFile
            )
            .await
            .is_err());
            drop(registry);
            let platform_log = log(db);
            assert!(platform_log.contains("AS platform_epoch_exists"));
            assert!(!platform_log.contains("INSERT INTO"));
            assert!(platform_log.contains("ROLLBACK"));
        }

        #[tokio::test]
        async fn revoke_affects_expired_rows_preserves_history_and_repeat_is_bounded() {
            let org = Uuid::new_v4();
            let as_of = Utc::now();
            let active = row(org, "active");
            let mut expired = row(org, "retiring");
            expired.updated_at = (as_of - chrono::Duration::days(2)).naive_utc();
            let clock = BTreeMap::from([("as_of".to_string(), as_of.fixed_offset().into())]);
            let mut terminal = active.clone();
            terminal.status = "revoked".into();
            terminal.updated_at = as_of.naive_utc();
            for repeat in [false, true] {
                let affected = if repeat {
                    vec![]
                } else {
                    vec![active.clone(), expired.clone()]
                };
                let mut mock = MockDatabase::new(DbBackend::Postgres)
                    .append_exec_results((0..3).map(|_| MockExecResult {
                        rows_affected: 1,
                        last_insert_id: 0,
                    }))
                    .append_query_results([issuer_projection(&affected)])
                    .append_query_results([affected])
                    .append_query_results([vec![clock.clone()]]);
                mock = if repeat {
                    mock.append_query_results([vec![terminal.clone()]])
                } else {
                    mock.append_query_results([vec![terminal.clone()], vec![terminal.clone()]])
                };
                let db = Arc::new(mock.into_connection());
                let registry = repository(db.clone());
                let result = registry.revoke_organization_keys(org).await.unwrap();
                assert_eq!(result.len(), if repeat { 1 } else { 2 });
                assert!(result
                    .iter()
                    .all(|key| key.status == SigningKeyStatus::Revoked));
                if repeat {
                    assert_eq!(result[0], to_domain(terminal.clone()).unwrap());
                }
                drop(registry);
                let log = log(db);
                assert!(log.contains("COMMIT"));
                assert!(!log.contains("lifecycle_admitted_at"));
                assert_eq!(
                    log.matches("UPDATE \"signing_keys\"").count(),
                    if repeat { 0 } else { 2 }
                );
                assert!(log.contains("<>"));
                assert!(!log.contains("AS invalid_retirement"));
                if repeat {
                    assert!(log.contains("LIMIT"));
                }
            }
        }

        #[tokio::test]
        async fn recent_churn_is_scoped_newest_four_and_publishes_no_subset() {
            let org = Uuid::new_v4();
            let as_of = Utc::now();
            let current = row(org, "active");
            let history = vec![
                BTreeMap::from([
                    ("organization_id".into(), org.into()),
                    (
                        "lifecycle_admitted_at".into(),
                        (as_of - chrono::Duration::seconds(100))
                            .fixed_offset()
                            .into()
                    )
                ]);
                4
            ];
            let db = Arc::new(
                targeted_database(
                    vec![current.clone()],
                    vec![current.clone()],
                    Some(history),
                    false,
                    false,
                    as_of,
                )
                .into_connection(),
            );
            let registry = repository(db.clone());
            let mut candidate = to_domain(current).unwrap();
            candidate.id = Uuid::new_v4();
            candidate.kid = iam_domain::entity::signing_key::opaque_kid();
            candidate.provider_key_ref = "new-ref".into();
            assert!(matches!(
                registry
                    .replace_active_organization_key(&candidate, None)
                    .await,
                Err(DomainError::SigningKeyAdmissionDenied {
                    reason: SigningKeyAdmissionReason::ChurnRate,
                    retry_after_seconds: Some(3500)
                })
            ));
            drop(registry);
            let log = log(db);
            assert!(log.contains("WHERE organization_id=$1::uuid"));
            assert!(log.contains("ORDER BY lifecycle_admitted_at DESC LIMIT $3"));
            assert!(!log.contains("UPDATE "));
            assert!(!log.contains("INSERT INTO"));
            assert!(log.contains("ROLLBACK"));
        }
    }

    #[tokio::test]
    async fn same_complete_binding_returns_actual_row_kid_timestamps_without_mutations() {
        let org = Uuid::new_v4();
        let current = row(org, "active");
        let expected = to_domain(current.clone()).unwrap();
        let mut candidate = expected.clone();
        candidate.id = Uuid::new_v4();
        candidate.kid = iam_domain::entity::signing_key::opaque_kid();
        candidate.created_at += chrono::Duration::hours(1);
        candidate.updated_at += chrono::Duration::hours(1);
        // The Windows checkout may already use CRLF; do not create invalid CRCRLF.
        candidate.public_key = candidate
            .public_key
            .replace("\r\n", "\n")
            .replace('\n', "\r\n");
        // No-op survives charged churn history; nothing is re-admitted.
        let history = vec![
            BTreeMap::from([
                ("organization_id".into(), org.into()),
                (
                    "lifecycle_admitted_at".into(),
                    Utc::now().fixed_offset().into()
                )
            ]);
            4
        ];
        let db = Arc::new(database(vec![current], history, Utc::now()).into_connection());
        let registry = repository(db.clone());
        let actual = registry
            .replace_active_organization_key(&candidate, Some(&expected.kid))
            .await
            .unwrap();
        assert_eq!(actual, expected);
        drop(registry);
        let log = log(db);
        assert_order(&log);
        assert!(!log.contains("UPDATE "));
        assert!(!log.contains("INSERT INTO"));
        assert!(log.contains("COMMIT"));
    }

    #[tokio::test]
    async fn expected_epoch_checked_before_idempotent_noop_and_rejection_rolls_back() {
        let org = Uuid::new_v4();
        let current = row(org, "active");
        let candidate = to_domain(current.clone()).unwrap();
        let db = Arc::new(database(vec![current], vec![], Utc::now()).into_connection());
        let registry = repository(db.clone());
        assert!(matches!(
            registry
                .replace_active_organization_key(&candidate, Some("stale-epoch"))
                .await,
            Err(DomainError::SigningKeyAdmissionDenied {
                reason: SigningKeyAdmissionReason::EpochConflict,
                ..
            })
        ));
        drop(registry);
        let log = log(db);
        assert_order(&log);
        assert!(log.contains("ROLLBACK"));
        assert!(!log.contains("COMMIT"));
        assert!(!log.contains("UPDATE "));
        assert!(!log.contains("INSERT INTO"));
    }

    #[tokio::test]
    async fn changed_credential_is_new_admission_and_churn_rejects_before_retirement() {
        let org = Uuid::new_v4();
        let current = row(org, "active");
        let mut candidate = to_domain(current.clone()).unwrap();
        candidate.id = Uuid::new_v4();
        candidate.kid = iam_domain::entity::signing_key::opaque_kid();
        candidate.credential_ref = Some("changed-credential-ref".into());
        let as_of = Utc::now();
        let history = vec![
            BTreeMap::from([
                ("organization_id".into(), org.into()),
                (
                    "lifecycle_admitted_at".into(),
                    (as_of - chrono::Duration::seconds(100))
                        .fixed_offset()
                        .into()
                )
            ]);
            4
        ];
        let db = Arc::new(database(vec![current], history, as_of).into_connection());
        let registry = repository(db.clone());
        assert!(matches!(
            registry
                .replace_active_organization_key(&candidate, None)
                .await,
            Err(DomainError::SigningKeyAdmissionDenied {
                reason: SigningKeyAdmissionReason::ChurnRate,
                retry_after_seconds: Some(3500)
            })
        ));
        drop(registry);
        let log = log(db);
        assert_order(&log);
        assert!(log.contains("ROLLBACK"));
        assert!(!log.contains("COMMIT"));
        assert!(!log.contains("UPDATE "));
        assert!(!log.contains("INSERT INTO"));
    }

    #[tokio::test]
    async fn tenant_epoch_cap_blocks_insert_and_configure_without_any_partial_write() {
        let org = Uuid::new_v4();
        let mut rows: Vec<_> = (0..8).map(|_| row(org, "pending")).collect();
        rows[0].status = "active".into();
        let mut candidate = to_domain(row(org, "active")).unwrap();
        candidate.provider_key_ref = "new-ref".into();
        for insert in [false, true] {
            let db = Arc::new(database(rows.clone(), vec![], Utc::now()).into_connection());
            let registry = repository(db.clone());
            let error = if insert {
                candidate.status = SigningKeyStatus::Pending;
                registry.insert(&candidate).await.unwrap_err()
            } else {
                registry
                    .replace_active_organization_key(&candidate, None)
                    .await
                    .unwrap_err()
            };
            assert!(matches!(
                error,
                DomainError::SigningKeyAdmissionDenied {
                    reason: SigningKeyAdmissionReason::TenantEpochLimit,
                    ..
                }
            ));
            drop(registry);
            let log = log(db);
            assert_order(&log);
            assert!(log.contains("ROLLBACK"));
            assert!(!log.contains("COMMIT"));
            assert!(!log.contains("UPDATE "));
            assert!(!log.contains("INSERT INTO"));
        }
    }

    #[tokio::test]
    async fn exact_epoch_budget_and_repeated_denials_preserve_last_known_good() {
        let org = Uuid::new_v4();
        let as_of = Utc::now();
        // Historical epochs remain published but no longer consume recent churn.
        let mut rows: Vec<_> = (0..7).map(|_| row(org, "retiring")).collect();
        for row in &mut rows {
            row.created_at = as_of.naive_utc();
            row.updated_at = as_of.naive_utc();
        }
        rows[0].status = "active".into();
        let current = rows[0].clone();
        let mut next = row(org, "active");
        next.provider_key_ref = "boundary-rotation".into();
        next.created_at = as_of.naive_utc();
        next.updated_at = as_of.naive_utc();
        let mut retired = current.clone();
        retired.status = "retiring".into();
        retired.updated_at = as_of.naive_utc();
        let mut terminal = rows.clone();
        terminal[0] = retired.clone();
        terminal.push(next.clone());
        let publication: Vec<BTreeMap<String, Value>> = terminal
            .iter()
            .map(|row| {
                let mut values: BTreeMap<String, Value> = signing_keys::Column::iter()
                    .map(|column| (column.to_string(), row.get(column)))
                    .collect();
                values.insert("as_of".into(), as_of.fixed_offset().into());
                values
            })
            .collect();
        let mut mock = database(rows, vec![], as_of)
            .append_query_results([vec![retired], vec![next.clone()]])
            .append_query_results([publication.clone()]);
        for _ in 0..2 {
            // Same connection, same terminal state: refusal cannot mutate either
            // publication or the active row, even when retried verbatim.
            mock = mock
                .append_exec_results((0..3).map(|_| MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 1,
                }))
                .append_query_results([issuer_projection(&terminal)])
                .append_query_results([terminal.clone()])
                .append_query_results([vec![BTreeMap::from([(
                    "as_of".to_string(),
                    as_of.fixed_offset().into(),
                )])]])
                .append_query_results([terminal.clone()])
                .append_query_results([vec![BTreeMap::from([
                    ("invalid_retirement".to_string(), false.into()),
                    ("issuer_conflict".to_string(), false.into()),
                    ("platform_epoch_exists".to_string(), false.into()),
                ])]])
                .append_query_results([Vec::<BTreeMap<String, Value>>::new()])
                .append_query_results([publication.clone()]);
        }
        let db = Arc::new(mock.into_connection());
        let registry = repository(db.clone());
        // Configure and rotate both use this atomic replacement boundary.
        let admitted = registry
            .replace_active_organization_key(&to_domain(next.clone()).unwrap(), Some(&current.kid))
            .await
            .expect("eighth published epoch is admitted at the exact budget");
        assert_eq!(admitted, to_domain(next).unwrap());
        let last_known_good = registry.jwks_publication_snapshot().await.unwrap();
        assert_eq!(last_known_good.keys.len(), 8);
        let good_bytes =
            iam_domain::entity::token::JwkSet::from_registry_keys_checked(&last_known_good.keys)
                .unwrap()
                .compact_bytes()
                .unwrap();
        let mut rejected = to_domain(row(org, "active")).unwrap();
        rejected.provider_key_ref = "ninth-epoch".into();
        for _ in 0..2 {
            let error = registry
                .replace_active_organization_key(&rejected, Some(&admitted.kid))
                .await
                .expect_err("budget+1 must refuse, not retire the last known good epoch");
            assert!(
                matches!(
                    error,
                    DomainError::SigningKeyAdmissionDenied {
                        reason: SigningKeyAdmissionReason::TenantEpochLimit,
                        retry_after_seconds: None,
                    }
                ),
                "distinct admission code, never invalid key material or SQL failure"
            );
            let after = registry.jwks_publication_snapshot().await.unwrap();
            assert_eq!(after.keys, last_known_good.keys);
            assert_eq!(after.as_of, last_known_good.as_of);
            assert_eq!(
                iam_domain::entity::token::JwkSet::from_registry_keys_checked(&after.keys)
                    .unwrap()
                    .compact_bytes()
                    .unwrap(),
                good_bytes,
            );
        }
        drop(registry);
        let log = log(db);
        assert_order(&log);
        assert_eq!(log.matches("COMMIT").count(), 1);
        assert_eq!(log.matches("ROLLBACK").count(), 2);
        assert_eq!(log.matches("INSERT INTO").count(), 1);
        assert_eq!(log.matches("UPDATE \"signing_keys\"").count(), 1);
    }

    #[test]
    fn exact_organization_byte_budget_accepts_but_one_more_byte_has_capacity_code() {
        let policy = SigningKeyLifecyclePolicy::new().unwrap();
        let as_of = Utc::now();
        // Ratified total minus platform reserve, not a test-only policy override.
        let budget = 786_432 - 65_536;
        let template = to_domain(row(Uuid::new_v4(), "active")).unwrap();
        let cost = policy.reserved_entry_bytes(&template).unwrap() + 1;
        let mut keys: Vec<_> = (0..budget / cost)
            .map(|_| {
                let mut key = template.clone();
                key.id = Uuid::new_v4();
                key.kid = iam_domain::entity::signing_key::opaque_kid();
                key.organization_id = Some(Uuid::new_v4());
                key
            })
            .collect();
        let mut remaining = budget % cost;
        for key in &mut keys {
            // ASCII URL path padding has an exact one-byte serialization cost.
            let padding = remaining.min(1023 - key.issuer.len());
            key.issuer.push_str(&"x".repeat(padding));
            remaining -= padding;
        }
        assert_eq!(remaining, 0);
        assert_eq!(
            policy
                .publication_usage(&keys, 900, as_of)
                .unwrap()
                .organization_reserved_bytes,
            budget,
        );
        policy
            .check_admission(&keys, &[], keys.last(), 900, as_of)
            .expect("exact reserved-byte budget is admitted");
        let last_known_good = iam_domain::entity::token::JwkSet::from_registry_keys_checked(&keys)
            .unwrap()
            .compact_bytes()
            .unwrap();
        let mut over = keys.clone();
        over.last_mut().unwrap().issuer.push('x');
        assert_eq!(
            policy
                .publication_usage(&over, 900, as_of)
                .unwrap()
                .organization_reserved_bytes,
            budget + 1,
        );
        for _ in 0..2 {
            assert!(matches!(
                policy.check_admission(&over, &[], over.last(), 900, as_of),
                Err(DomainError::SigningKeyAdmissionDenied {
                    reason: SigningKeyAdmissionReason::Capacity,
                    retry_after_seconds: None,
                })
            ));
            assert_eq!(
                iam_domain::entity::token::JwkSet::from_registry_keys_checked(&keys)
                    .unwrap()
                    .compact_bytes()
                    .unwrap(),
                last_known_good,
            );
        }
    }

    #[tokio::test]
    async fn pending_promotion_preserves_admission_and_returns_writer_row_at_db_clock() {
        let org = Uuid::new_v4();
        let active = row(org, "active");
        let mut pending = row(org, "pending");
        pending.provider_key_ref = "next-ref".into();
        let as_of = Utc::now();
        let mut retired = active.clone();
        retired.status = "retiring".into();
        retired.updated_at = as_of.naive_utc();
        let mut promoted = pending.clone();
        promoted.status = "active".into();
        promoted.updated_at = as_of.naive_utc();
        let history = vec![
            BTreeMap::from([
                ("organization_id".into(), org.into()),
                ("lifecycle_admitted_at".into(), as_of.fixed_offset().into())
            ]);
            4
        ];
        let db = Arc::new(
            database(vec![pending.clone(), active.clone()], history, as_of)
                .append_query_results([
                    vec![retired],
                    vec![promoted.clone()],
                    vec![promoted.clone()],
                ])
                .into_connection(),
        );
        let registry = repository(db.clone());
        let mut candidate = to_domain(pending.clone()).unwrap();
        candidate.status = SigningKeyStatus::Active;
        candidate.updated_at += chrono::Duration::hours(10);
        let committed = registry
            .replace_active_organization_key(&candidate, Some(&active.kid))
            .await
            .unwrap();
        assert_eq!(committed, to_domain(promoted).unwrap());
        assert_eq!(committed.created_at, to_domain(pending).unwrap().created_at);
        drop(registry);
        let log = log(db);
        assert_order(&log);
        assert!(log.contains("COMMIT"));
        assert!(!log.contains("INSERT INTO"));
        assert_eq!(log.matches("UPDATE \"signing_keys\"").count(), 2);
        assert!(!log.contains("SET \"lifecycle_admitted_at\""));
    }

    #[tokio::test]
    async fn retired_timestamps_cannot_be_renewed_and_revoked_rows_cannot_reactivate() {
        let org = Uuid::new_v4();
        for status in ["retiring", "revoked"] {
            let current = row(org, status);
            let mut candidate = to_domain(current.clone()).unwrap();
            candidate.updated_at += chrono::Duration::days(1);
            let db = Arc::new(database(vec![current], vec![], Utc::now()).into_connection());
            let registry = repository(db.clone());
            if status == "revoked" {
                candidate.status = SigningKeyStatus::Active;
                assert!(registry.update(&candidate).await.is_err());
            } else {
                registry.update(&candidate).await.unwrap();
            }
            drop(registry);
            let log = log(db);
            assert!(!log.contains("UPDATE "));
            assert_eq!(log.contains("COMMIT"), status == "retiring");
            assert_eq!(log.contains("ROLLBACK"), status == "revoked");
        }
    }

    #[tokio::test]
    async fn new_admission_uses_one_db_clock_and_insert_failure_rolls_back_retirement() {
        let org = Uuid::new_v4();
        let current = row(org, "active");
        let as_of = Utc::now();
        let mut retired = current.clone();
        retired.status = "retiring".into();
        retired.updated_at = as_of.naive_utc();
        let mut inserted = row(org, "active");
        inserted.provider_key_ref = "next-ref".into();
        inserted.created_at = as_of.naive_utc();
        inserted.updated_at = as_of.naive_utc();
        for fail_insert in [false, true] {
            let database = database(vec![current.clone()], vec![], as_of)
                .append_query_results([vec![retired.clone()]]);
            let database = if fail_insert {
                database
                    .append_query_errors([sea_orm::DbErr::Custom("secret-storage-failure".into())])
            } else {
                database.append_query_results([vec![inserted.clone()]])
            };
            let db = Arc::new(database.into_connection());
            let registry = repository(db.clone());
            let mut candidate = to_domain(inserted.clone()).unwrap();
            candidate.created_at -= chrono::Duration::days(20);
            candidate.updated_at -= chrono::Duration::days(20);
            let result = registry
                .replace_active_organization_key(&candidate, None)
                .await;
            if fail_insert {
                let error = result.unwrap_err();
                assert!(matches!(error, DomainError::RepositoryError(_)));
                assert!(!format!("{error:?}").contains("secret"));
            } else {
                assert_eq!(result.unwrap(), to_domain(inserted.clone()).unwrap());
            }
            drop(registry);
            let log = log(db);
            assert_order(&log);
            assert!(log.contains("lifecycle_admitted_at"));
            assert!(log.contains("$12::timestamptz AT TIME ZONE 'UTC'"));
            assert!(log.find("UPDATE ").unwrap() < log.find("INSERT INTO").unwrap());
            assert_eq!(log.contains("COMMIT"), !fail_insert);
            assert_eq!(log.contains("ROLLBACK"), fail_insert);
        }
    }

    #[tokio::test]
    async fn platform_bootstrap_and_insert_cannot_bypass_global_admission() {
        let as_of = Utc::now();
        let mut platform = row(Uuid::new_v4(), "active");
        platform.trust_scope = "platform".into();
        platform.organization_id = None;
        platform.created_at = as_of.naive_utc();
        platform.updated_at = as_of.naive_utc();
        let db = Arc::new(
            database(vec![], vec![], as_of)
                .append_query_results([vec![platform.clone()]])
                .into_connection(),
        );
        let registry = repository(db.clone());
        let admitted = bootstrap_platform_signing_key(
            &registry,
            &platform.kid,
            &platform.issuer,
            &platform.public_key,
            &platform.provider_key_ref,
            SigningProviderType::RemoteHttp,
        )
        .await
        .unwrap();
        assert_eq!(admitted, to_domain(platform.clone()).unwrap());
        drop(registry);
        let bootstrap_log = log(db);
        assert!(
            bootstrap_log.find("iam-signing-jwks-admission-v1").unwrap()
                < bootstrap_log.find("signer-platform").unwrap()
        );
        assert!(bootstrap_log.contains("INSERT INTO"));
        assert!(bootstrap_log.contains("COMMIT"));

        let pending: Vec<_> = (0..16)
            .map(|_| {
                let mut row = platform.clone();
                row.id = Uuid::new_v4();
                row.kid = iam_domain::entity::signing_key::opaque_kid();
                row.status = "pending".into();
                row
            })
            .collect();
        let db = Arc::new(database(pending, vec![], as_of).into_connection());
        let registry = repository(db.clone());
        let candidate = to_domain(platform).unwrap();
        assert!(matches!(
            registry.insert(&candidate).await,
            Err(DomainError::SigningKeyAdmissionDenied {
                reason: SigningKeyAdmissionReason::Capacity,
                ..
            })
        ));
        drop(registry);
        let log = log(db);
        assert!(log.contains("ROLLBACK"));
        assert!(!log.contains("INSERT INTO"));
        assert!(!log.contains("UPDATE "));
    }

    #[tokio::test]
    async fn preflight_reports_legacy_policy_drift_without_rejecting_complete_publication() {
        let as_of = Utc::now();
        let legacy = row(Uuid::new_v4(), "active");
        let mut snapshot: BTreeMap<String, Value> = signing_keys::Column::iter()
            .map(|column| (column.to_string(), legacy.get(column)))
            .collect();
        snapshot.insert("as_of".into(), as_of.fixed_offset().into());
        // The test PEM is valid RSA; a non-opaque old kid is publication-readable
        // while no new admission may retain this preexisting policy deviation.
        snapshot.insert("kid".into(), "legacy-kid".into());
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![snapshot]])
                .into_connection(),
        );
        let registry = repository(db.clone());
        let report = registry.preflight().await.unwrap();
        assert_eq!(report.snapshot_key_count, 1);
        assert!(report.recovery_required);
        assert!(report.usage.is_some());
        drop(registry);
        let log = log(db);
        assert!(!log.contains("UPDATE "));
        assert!(!log.contains("INSERT INTO"));
    }

    #[tokio::test]
    async fn publication_empty_snapshot_still_has_one_primary_db_clock_select() {
        let as_of = Utc::now();
        let row: BTreeMap<String, Value> = BTreeMap::from([
            ("as_of".into(), as_of.fixed_offset().into()),
            ("id".into(), Option::<Uuid>::None.into()),
        ]);
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![row]])
                .into_connection(),
        );
        let registry = repository(db.clone());
        let snapshot = registry.jwks_publication_snapshot().await.unwrap();
        assert!(snapshot.keys.is_empty());
        assert_eq!(snapshot.as_of, as_of);
        drop(registry);
        let log = log(db);
        assert_eq!(log.matches("SELECT statement_timestamp()").count(), 1);
        assert!(log.contains("LEFT JOIN"));
        assert!(!log.contains("BEGIN"));
    }
}

#[cfg(test)]
mod emission_fence_tests {
    use super::*;
    use sea_orm::MockDatabase;

    fn row() -> signing_keys::Model {
        let now = Utc::now().naive_utc();
        signing_keys::Model {
            id: Uuid::new_v4(),
            kid: "opaque-epoch".into(),
            algorithm: "RS256".into(),
            trust_scope: "platform".into(),
            issuer: "iamrusty".into(),
            provider_type: "remote_http".into(),
            provider_key_ref: "signer-ref".into(),
            credential_ref: Some("credential-ref".into()),
            public_key: "public-material".into(),
            status: "active".into(),
            organization_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn final_fence_fresh_autocommit_select_compares_every_binding_not_timestamps() {
        let original = row();
        let expected = to_domain(original.clone()).unwrap();
        // Each binding mutant independently fails. SQL mocks prove query shape,
        // not isolation or primary topology: those remain real PostgreSQL IT.
        for case in 0..=15 {
            let mut current = original.clone();
            match case {
                0 => {}
                1 => current.id = Uuid::new_v4(),
                2 => current.kid.push_str("changed"),
                3 => current.algorithm = "HS256".into(),
                4 => current.issuer.push_str("changed"),
                5 => current.trust_scope = "organization".into(),
                6 => current.organization_id = Some(Uuid::new_v4()),
                7 => current.public_key.push_str("changed"),
                8 => current.provider_type = "pem_file".into(),
                9 => current.provider_key_ref.push_str("changed"),
                10 => current.credential_ref = None,
                11 => current.status = "pending".into(),
                12 => current.status = "retiring".into(),
                13 => current.status = "revoked".into(),
                14 => {
                    current.created_at += chrono::Duration::seconds(1);
                    current.updated_at += chrono::Duration::seconds(2);
                }
                _ => {}
            }
            let rows = if case == 15 { vec![] } else { vec![current] };
            let db = Arc::new(
                MockDatabase::new(DbBackend::Postgres)
                    .append_query_results([rows])
                    .into_connection(),
            );
            let registry = SeaOrmSigningKeyRegistry::new(
                db.clone(),
                Arc::new(SigningKeyLifecyclePolicy::new().unwrap()),
                900,
            )
            .unwrap();
            assert_eq!(
                registry
                    .confirm_active_for_emission(&expected)
                    .await
                    .unwrap(),
                case == 0 || case == 14,
                "binding case {case}"
            );
            drop(registry);
            let transactions = Arc::try_unwrap(db).unwrap().into_transaction_log();
            assert_eq!(transactions.len(), 1);
            let log = format!("{transactions:?}");
            assert!(log.contains("SELECT"));
            assert!(log.contains("signing_keys"));
            assert!(!log.contains("BEGIN"));
            assert!(!log.contains("FOR UPDATE"));
        }
    }

    #[tokio::test]
    async fn final_fence_writer_error_and_invalid_snapshot_are_sanitized() {
        let expected = to_domain(row()).unwrap();
        let mut invalid = row();
        invalid.status = "secret-invalid-status".into();
        for (case, db) in [
            MockDatabase::new(DbBackend::Postgres)
                .append_query_errors([sea_orm::DbErr::Custom("secret-db-payload".into())])
                .into_connection(),
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![invalid]])
                .into_connection(),
        ]
        .into_iter()
        .enumerate()
        {
            let registry = SeaOrmSigningKeyRegistry::new(
                Arc::new(db),
                Arc::new(SigningKeyLifecyclePolicy::new().unwrap()),
                900,
            )
            .unwrap();
            let result = registry.confirm_active_for_emission(&expected).await;
            if case == 0 {
                let error = result.unwrap_err();
                assert!(!format!("{error:?} {error}").contains("secret"));
            } else {
                assert!(!result.unwrap());
            }
        }
    }
}

#[async_trait]
impl SigningKeyRegistry for SeaOrmSigningKeyRegistry {
    type Error = DomainError;

    async fn jwks_publication_snapshot(
        &self,
    ) -> Result<SigningKeyPublicationSnapshot, Self::Error> {
        // One autocommit SELECT supplies rows AND clock, including empty sets.
        let rows = self.db.query_all(Statement::from_string(DbBackend::Postgres,
            "SELECT statement_timestamp() AS as_of,k.* FROM (SELECT 1) anchor LEFT JOIN signing_keys k ON k.status IN ('pending','active','retiring') ORDER BY k.id"))
            .await.map_err(registry_error)?;
        let clock = rows
            .first()
            .ok_or_else(|| registry_error(sea_orm::DbErr::Custom(String::new())))?;
        let as_of: chrono::DateTime<chrono::FixedOffset> =
            clock.try_get("", "as_of").map_err(registry_error)?;
        let mut keys = Vec::new();
        for row in &rows {
            if row
                .try_get::<Option<Uuid>>("", "id")
                .map_err(registry_error)?
                .is_some()
            {
                keys.push(to_domain(
                    signing_keys::Model::from_query_result(row, "").map_err(registry_error)?,
                )?);
            }
        }
        Ok(SigningKeyPublicationSnapshot {
            keys,
            as_of: as_of.with_timezone(&Utc),
        })
    }

    async fn confirm_active_for_emission(
        &self,
        expected: &SigningKey,
    ) -> Result<bool, Self::Error> {
        // No transaction predating a remote signer call: this SELECT's snapshot
        // is the emission linearization point on the primary writer.
        let current = SigningKeys::find_by_id(expected.id)
            .one(self.db.as_ref())
            .await
            .map_err(registry_error)?;
        let Some(current) = current else {
            return Ok(false);
        };
        let Ok(current) = to_domain(current) else {
            return Ok(false);
        };
        Ok(expected.status == SigningKeyStatus::Active
            && current.status == SigningKeyStatus::Active
            && current.id == expected.id
            && current.kid == expected.kid
            && current.algorithm == expected.algorithm
            && current.issuer == expected.issuer
            && current.trust_scope == expected.trust_scope
            && current.organization_id == expected.organization_id
            && current.public_key == expected.public_key
            && current.provider_type == expected.provider_type
            && current.provider_key_ref == expected.provider_key_ref
            && current.credential_ref == expected.credential_ref)
    }

    async fn insert(&self, candidate: &SigningKey) -> Result<(), Self::Error> {
        if !matches!(
            candidate.status,
            SigningKeyStatus::Pending | SigningKeyStatus::Active
        ) {
            return Err(conflict());
        }
        self.policy.validate_new_key(candidate)?;
        let (tx, state) = self
            .begin_lifecycle(
                &candidate.trust_scope,
                candidate.organization_id,
                Some(&candidate.issuer),
                LifecycleOperation::Insert(candidate),
            )
            .await?;
        let result = async {
            check_issuer_owner(&state)?;
            if state
                .keys
                .iter()
                .any(|row| row.id == candidate.id || row.kid == candidate.kid)
            {
                return Err(conflict());
            }
            if candidate.status == SigningKeyStatus::Active
                && state.keys.iter().any(|row| {
                    row.organization_id == candidate.organization_id
                        && row.trust_scope == candidate.trust_scope
                        && row.status == SigningKeyStatus::Active
                })
            {
                return Err(conflict());
            }
            let mut admitted = candidate.clone();
            admitted.created_at = state.as_of;
            admitted.updated_at = state.as_of;
            let mut proposed = state.keys.clone();
            proposed.push(admitted.clone());
            self.check_plan(&state, &proposed, Some(&admitted))?;
            insert_admitted(&tx, &admitted, state.as_of).await?;
            Ok(())
        }
        .await;
        match result {
            Ok(()) => tx.commit().await.map_err(registry_error),
            Err(error) => {
                tx.rollback().await.map_err(registry_error)?;
                Err(error)
            }
        }
    }

    async fn replace_active_organization_key(
        &self,
        candidate: &SigningKey,
        expected_active_kid: Option<&str>,
    ) -> Result<SigningKey, Self::Error> {
        if candidate.trust_scope != TrustScope::Organization
            || candidate.organization_id.is_none()
            || candidate.status != SigningKeyStatus::Active
        {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        let candidate_validation = self.policy.validate_new_key(candidate);
        let (tx, state) = self
            .begin_lifecycle(
                &candidate.trust_scope,
                candidate.organization_id,
                Some(&candidate.issuer),
                LifecycleOperation::Replace(candidate),
            )
            .await?;
        let result = async {
            check_issuer_owner(&state)?;
            let active: Vec<_> = state
                .keys
                .iter()
                .filter(|row| {
                    row.organization_id == candidate.organization_id
                        && row.trust_scope == candidate.trust_scope
                        && row.status == SigningKeyStatus::Active
                })
                .collect();
            if active.len() > 1 {
                return Err(conflict());
            }
            if expected_active_kid
                .is_some_and(|expected| !active.iter().any(|row| row.kid == expected))
            {
                return Err(conflict());
            }
            let existing = state
                .keys
                .iter()
                .find(|row| row.id == candidate.id || row.kid == candidate.kid);
            // An already-admitted Pending rotation is a promotion, never configure-no-op.
            let promotion = existing.filter(|row| row.status == SigningKeyStatus::Pending);
            if promotion.is_none() {
                if let Some(current) = active
                    .first()
                    .filter(|current| same_effective_signing_binding(current, candidate))
                {
                    return Ok((**current).clone());
                }
            }
            candidate_validation?;
            let mut committed = if let Some(pending) = promotion {
                if pending.id != candidate.id
                    || pending.kid != candidate.kid
                    || !same_effective_signing_binding(pending, candidate)
                {
                    return Err(conflict());
                }
                pending.clone()
            } else {
                if existing.is_some() {
                    return Err(conflict());
                }
                let mut new = candidate.clone();
                new.created_at = state.as_of;
                new
            };
            committed.status = SigningKeyStatus::Active;
            committed.updated_at = state.as_of;
            let mut proposed = state.keys.clone();
            let mut changes = Vec::new();
            for key in proposed.iter_mut().filter(|key| {
                key.organization_id == candidate.organization_id
                    && key.trust_scope == candidate.trust_scope
                    && key.status == SigningKeyStatus::Active
            }) {
                key.status = SigningKeyStatus::Retiring;
                key.updated_at = state.as_of;
                changes.push(key.clone());
            }
            if promotion.is_some() {
                *proposed
                    .iter_mut()
                    .find(|key| key.id == committed.id)
                    .ok_or_else(conflict)? = committed.clone();
                changes.push(committed.clone());
            } else {
                proposed.push(committed.clone());
            }
            self.check_plan(
                &state,
                &proposed,
                if promotion.is_some() {
                    None
                } else {
                    Some(&committed)
                },
            )?;
            persist_changes(&tx, &mut changes).await?;
            if promotion.is_none() {
                committed = insert_admitted(&tx, &committed, state.as_of).await?;
            } else {
                committed = to_domain(
                    SigningKeys::find_by_id(committed.id)
                        .one(&tx)
                        .await
                        .map_err(registry_error)?
                        .ok_or_else(conflict)?,
                )?;
            }
            Ok(committed)
        }
        .await;
        match result {
            Ok(key) => {
                tx.commit().await.map_err(registry_error)?;
                Ok(key)
            }
            Err(error) => {
                tx.rollback().await.map_err(registry_error)?;
                Err(error)
            }
        }
    }

    async fn revoke_organization_keys(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<SigningKey>, Self::Error> {
        let (tx, state) = self
            .begin_lifecycle(
                &TrustScope::Organization,
                Some(organization_id),
                None,
                LifecycleOperation::Revoke,
            )
            .await?;
        let result = async {
            let mut changed = state.keys;
            for key in &mut changed {
                key.status = SigningKeyStatus::Revoked;
                key.updated_at = state.as_of;
            }
            // Reducing recovery path: no publication policy gate, no history deletion.
            persist_changes(&tx, &mut changed).await?;
            if changed.is_empty() {
                // User-approved RP-1 return shape: affected rows; one terminal row for
                // an idempotent repeat. No lifetime response scan or post-commit error.
                if let Some(row) = SigningKeys::find()
                    .filter(scope_rows(&TrustScope::Organization, Some(organization_id)))
                    .filter(signing_keys::Column::Status.eq("revoked"))
                    .order_by_desc(signing_keys::Column::Id)
                    .one(&tx)
                    .await
                    .map_err(registry_error)?
                {
                    changed.push(to_domain(row)?);
                }
            }
            Ok(changed)
        }
        .await;
        match result {
            Ok(keys) => {
                tx.commit().await.map_err(registry_error)?;
                Ok(keys)
            }
            Err(error) => {
                tx.rollback().await.map_err(registry_error)?;
                Err(error)
            }
        }
    }

    async fn find_by_kid(&self, kid: &str) -> Result<Option<SigningKey>, Self::Error> {
        let model = SigningKeys::find()
            .filter(signing_keys::Column::Kid.eq(kid))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        model.map(to_domain).transpose()
    }

    async fn find_active_platform_key(&self) -> Result<Option<SigningKey>, Self::Error> {
        let model = SigningKeys::find()
            .filter(signing_keys::Column::TrustScope.eq("platform"))
            .filter(signing_keys::Column::Status.eq("active"))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        model.map(to_domain).transpose()
    }

    async fn list_jwks_keys(&self) -> Result<Vec<SigningKey>, Self::Error> {
        let models = SigningKeys::find()
            .filter(signing_keys::Column::Status.is_in(["pending", "active", "retiring"]))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        models.into_iter().map(to_domain).collect()
    }

    async fn update(&self, candidate: &SigningKey) -> Result<(), Self::Error> {
        let (tx, state) = self
            .begin_lifecycle(
                &candidate.trust_scope,
                candidate.organization_id,
                Some(&candidate.issuer),
                LifecycleOperation::Update(candidate),
            )
            .await?;
        let result = async {
            let current = state
                .keys
                .iter()
                .find(|key| key.id == candidate.id)
                .ok_or_else(conflict)?;
            let mut unchanged_material = current.clone();
            unchanged_material.status = candidate.status.clone();
            unchanged_material.created_at = candidate.created_at;
            unchanged_material.updated_at = candidate.updated_at;
            if current.kid != candidate.kid
                || !(same_effective_signing_binding(current, candidate)
                    || unchanged_material == *candidate)
            {
                return Err(conflict());
            }
            if current.status == candidate.status {
                return Ok(());
            } // Never renew retirement timestamps.
            let mut proposed = state.keys.clone();
            let mut changes = Vec::new();
            let mut changed = current.clone();
            changed.status = candidate.status.clone();
            changed.updated_at = state.as_of;
            match (&current.status, &candidate.status) {
                (_, SigningKeyStatus::Revoked) => {}
                (SigningKeyStatus::Active, SigningKeyStatus::Retiring) => {}
                (SigningKeyStatus::Pending, SigningKeyStatus::Active)
                    if current.trust_scope == TrustScope::Platform =>
                {
                    for key in proposed.iter_mut().filter(|key| {
                        key.trust_scope == TrustScope::Platform
                            && key.organization_id.is_none()
                            && key.status == SigningKeyStatus::Active
                    }) {
                        key.status = SigningKeyStatus::Retiring;
                        key.updated_at = state.as_of;
                        changes.push(key.clone());
                    }
                }
                _ => return Err(conflict()),
            }
            *proposed
                .iter_mut()
                .find(|key| key.id == changed.id)
                .ok_or_else(conflict)? = changed.clone();
            changes.push(changed);
            // Revocation and drain only reduce publication; promotion must preserve its reservation.
            if candidate.status != SigningKeyStatus::Revoked {
                self.check_plan(&state, &proposed, None)?;
            }
            persist_changes(&tx, &mut changes).await
        }
        .await;
        match result {
            Ok(()) => tx.commit().await.map_err(registry_error),
            Err(error) => {
                tx.rollback().await.map_err(registry_error)?;
                Err(error)
            }
        }
    }

    async fn find_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<SigningKey>, Self::Error> {
        let models = SigningKeys::find()
            .filter(signing_keys::Column::OrganizationId.eq(organization_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        models.into_iter().map(to_domain).collect()
    }

    async fn find_by_issuer(&self, issuer: &str) -> Result<Vec<SigningKey>, Self::Error> {
        let models = SigningKeys::find()
            .filter(signing_keys::Column::Issuer.eq(issuer))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
        models.into_iter().map(to_domain).collect()
    }
}

pub async fn bootstrap_platform_signing_key(
    registry: &SeaOrmSigningKeyRegistry,
    kid: &str,
    issuer: &str,
    public_key_pem: &str,
    provider_key_ref: &str,
    provider_type: SigningProviderType,
) -> Result<SigningKey, DomainError> {
    let now = Utc::now();
    let candidate = SigningKey {
        id: Uuid::new_v4(),
        kid: kid.to_string(),
        algorithm: "RS256".into(),
        trust_scope: TrustScope::Platform,
        issuer: issuer.to_string(),
        provider_type: provider_type.clone(),
        provider_key_ref: provider_key_ref.to_string(),
        credential_ref: None,
        public_key: public_key_pem.to_string(),
        status: SigningKeyStatus::Active,
        organization_id: None,
        created_at: now,
        updated_at: now,
    };
    let candidate_validation = registry.policy.validate_new_key(&candidate);
    let (tx, state) = registry
        .begin_lifecycle(
            &TrustScope::Platform,
            None,
            Some(issuer),
            LifecycleOperation::Bootstrap(&candidate),
        )
        .await?;
    let result = async {
        let platform: Vec<_> = state
            .keys
            .iter()
            .filter(|row| row.trust_scope == TrustScope::Platform && row.organization_id.is_none())
            .collect();
        let active: Vec<_> = platform
            .iter()
            .filter(|row| row.status == SigningKeyStatus::Active)
            .collect();
        if active.len() > 1 {
            return Err(conflict());
        }
        if let Some(existing) = active.first() {
            require_bootstrap_compatible(
                existing,
                kid,
                issuer,
                public_key_pem,
                provider_key_ref,
                &provider_type,
            )?;
            return Ok((***existing).clone());
        }
        if state.platform_epoch_exists {
            return Err(conflict());
        }
        candidate_validation?;
        let mut key = candidate;
        key.created_at = state.as_of;
        key.updated_at = state.as_of;
        check_issuer_owner(&state)?;
        if state.keys.iter().any(|row| row.kid == key.kid) {
            return Err(conflict());
        }
        let mut proposed = state.keys.clone();
        proposed.push(key.clone());
        registry.check_plan(&state, &proposed, Some(&key))?;
        insert_admitted(&tx, &key, state.as_of).await
    }
    .await;
    match result {
        Ok(key) => {
            tx.commit().await.map_err(registry_error)?;
            Ok(key)
        }
        Err(error) => {
            tx.rollback().await.map_err(registry_error)?;
            Err(error)
        }
    }
}

fn require_bootstrap_compatible(
    existing: &SigningKey,
    kid: &str,
    issuer: &str,
    public_key_pem: &str,
    provider_key_ref: &str,
    provider_type: &SigningProviderType,
) -> Result<(), DomainError> {
    use iam_domain::entity::token::Jwk;
    let old = Jwk::from_rsa_pem(&existing.public_key, "compare", "compare").map_err(|_| {
        DomainError::AuthorizationError("invalid registered platform public key".into())
    })?;
    let new = Jwk::from_rsa_pem(public_key_pem, "compare", "compare").map_err(|_| {
        DomainError::AuthorizationError("invalid bootstrap platform public key".into())
    })?;
    if existing.status != SigningKeyStatus::Active
        || existing.trust_scope != TrustScope::Platform
        || existing.organization_id.is_some()
        || existing.algorithm != "RS256"
        || existing.issuer != issuer
        || &existing.provider_type != provider_type
        || existing.provider_key_ref != provider_key_ref
        || old.n != new.n
        || old.e != new.e
        || (*provider_type != SigningProviderType::PemFile && existing.kid != kid)
    {
        return Err(DomainError::AuthorizationError(
            "platform bootstrap conflicts with registered signing epoch; explicit rotation required".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registered() -> SigningKey {
        let now = Utc::now();
        SigningKey {
            id: Uuid::new_v4(),
            kid: "registered-epoch".into(),
            algorithm: "RS256".into(),
            trust_scope: TrustScope::Platform,
            issuer: "https://iam.example/iam".into(),
            provider_type: SigningProviderType::PemFile,
            provider_key_ref: "platform.pem".into(),
            credential_ref: None,
            public_key: include_str!("../../../config/keys/test-platform.pub").into(),
            status: SigningKeyStatus::Active,
            organization_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn same_pem_preserves_exact_registered_epoch_but_mismatches_fail_closed() {
        use rsa::pkcs8::{EncodePublicKey, LineEnding};
        let key = registered();
        assert!(require_bootstrap_compatible(
            &key,
            "proposed-random-kid",
            &key.issuer,
            &key.public_key,
            &key.provider_key_ref,
            &key.provider_type
        )
        .is_ok());
        assert!(require_bootstrap_compatible(
            &key,
            "new",
            &key.issuer,
            "invalid PEM",
            &key.provider_key_ref,
            &key.provider_type
        )
        .is_err());
        assert!(require_bootstrap_compatible(
            &key,
            "new",
            "https://other.example",
            &key.public_key,
            &key.provider_key_ref,
            &key.provider_type
        )
        .is_err());
        assert!(require_bootstrap_compatible(
            &key,
            "new",
            &key.issuer,
            &key.public_key,
            "other.pem",
            &key.provider_type
        )
        .is_err());
        assert!(require_bootstrap_compatible(
            &key,
            &key.kid,
            &key.issuer,
            &key.public_key,
            &key.provider_key_ref,
            &SigningProviderType::RemoteHttp
        )
        .is_err());
        let alternate = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 1024)
            .unwrap()
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        assert!(require_bootstrap_compatible(
            &key,
            &key.kid,
            &key.issuer,
            &alternate,
            &key.provider_key_ref,
            &key.provider_type
        )
        .is_err());
    }

    #[test]
    fn remote_bootstrap_never_mutates_active_kid_in_place() {
        let mut key = registered();
        key.provider_type = SigningProviderType::RemoteHttp;
        assert!(require_bootstrap_compatible(
            &key,
            "changed-kid",
            &key.issuer,
            &key.public_key,
            &key.provider_key_ref,
            &key.provider_type
        )
        .is_err());
        assert!(require_bootstrap_compatible(
            &key,
            &key.kid,
            &key.issuer,
            &key.public_key,
            &key.provider_key_ref,
            &key.provider_type
        )
        .is_ok());
    }
}
