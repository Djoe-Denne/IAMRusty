//! Fixed-slot writer accounting and durable, validated public publication.
//! Only SQL/local public operations: never provider I/O or PEM parsing here.

use super::{conflict, registry_error, to_domain};
use chrono::{DateTime, Utc};
use iam_domain::entity::signing_key::{
    SigningKey, SigningKeyPublicationSnapshot, SigningKeyStatus, JWKS_RETIRE_SKEW_SECONDS,
};
use iam_domain::entity::signing_publication::{
    PreparedSigningPublicKey, SigningSlotCounts, ValidatedJwksPublication,
};
use iam_domain::error::DomainError;
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, DbBackend, FromQueryResult, QueryResult, Statement,
};
use std::collections::BTreeMap;
use uuid::Uuid;

// Both COUNT and assembly use this exact predicate and the same bound DB clock.
// Inverse cutoff is indexable and avoids arithmetic on legacy row timestamps.
pub(super) const PUBLISHABLE: &str = "(k.status IN ('pending','active') OR (k.status='retiring' AND k.updated_at>$1::timestamp AND k.updated_at<=$2::timestamp))";

#[derive(Clone)]
pub(super) struct StoredPublic {
    n: String,
    e: String,
    fingerprint: Vec<u8>,
    longest_entry_bytes: usize,
}
impl StoredPublic {
    pub(super) fn restore(
        &self,
        key: &SigningKey,
    ) -> Result<PreparedSigningPublicKey, DomainError> {
        PreparedSigningPublicKey::from_persisted(
            key,
            self.n.clone(),
            self.e.clone(),
            &self.fingerprint,
            self.longest_entry_bytes,
        )
    }
}

#[derive(Clone)]
pub(super) struct StoredPublication {
    pub(super) revision: i64,
    publication: ValidatedJwksPublication,
    as_of: DateTime<Utc>,
    next_expiration: Option<DateTime<Utc>>,
    ttl: u64,
    skew: i64,
    dirty: bool,
}

impl StoredPublication {
    pub(super) fn decode(row: &QueryResult) -> Result<Self, DomainError> {
        let revision: i64 = row.try_get("", "revision").map_err(registry_error)?;
        let ttl: i64 = row
            .try_get("", "access_token_ttl")
            .map_err(registry_error)?;
        let payload: String = row.try_get("", "payload").map_err(registry_error)?;
        let as_of: DateTime<chrono::FixedOffset> =
            row.try_get("", "as_of").map_err(registry_error)?;
        let next: Option<DateTime<chrono::FixedOffset>> =
            row.try_get("", "next_expiration").map_err(registry_error)?;
        if revision < 0 || ttl < 0 || (revision > 0 && ttl == 0) {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        let stored = Self {
            revision,
            publication: ValidatedJwksPublication::from_compact_payload(&payload)?,
            as_of: as_of.with_timezone(&Utc),
            next_expiration: next.map(|time| time.with_timezone(&Utc)),
            ttl: u64::try_from(ttl).map_err(|_| conflict())?,
            skew: row.try_get("", "retire_skew").map_err(registry_error)?,
            dirty: row.try_get("", "dirty").map_err(registry_error)?,
        };
        if stored
            .next_expiration
            .is_some_and(|time| time <= stored.as_of)
        {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        Ok(stored)
    }

    pub(super) fn needs_refresh(&self, now: DateTime<Utc>, ttl: u64) -> Result<bool, DomainError> {
        if self.as_of > now {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        Ok(self.dirty
            || self.revision == 0
            || self.ttl != ttl
            || self.skew != JWKS_RETIRE_SKEW_SECONDS
            || self.next_expiration.is_some_and(|deadline| deadline <= now))
    }
    pub(super) fn snapshot(&self) -> Result<SigningKeyPublicationSnapshot, DomainError> {
        Ok(SigningKeyPublicationSnapshot {
            publication: self.publication.clone(),
            as_of: self.as_of,
            revision: u64::try_from(self.revision).map_err(|_| conflict())?,
            access_token_expiration_seconds: self.ttl,
            next_expiration: self.next_expiration,
        })
    }
}

pub(super) async fn lock_snapshot(
    tx: &DatabaseTransaction,
) -> Result<StoredPublication, DomainError> {
    let row=tx.query_one(Statement::from_string(DbBackend::Postgres,
        "SELECT revision,dirty,payload,as_of,next_expiration,access_token_ttl,retire_skew FROM signing_jwks_publication WHERE singleton=1 FOR UPDATE"))
        .await.map_err(registry_error)?.ok_or_else(conflict)?;
    StoredPublication::decode(&row)
}

pub(super) async fn counts(
    tx: &DatabaseTransaction,
    cutoff: DateTime<Utc>,
    now: DateTime<Utc>,
    org: Option<Uuid>,
) -> Result<SigningSlotCounts, DomainError> {
    let row=tx.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
        format!("SELECT count(*)::bigint AS global, count(*) FILTER(WHERE k.trust_scope='organization')::bigint AS global_organization, count(*) FILTER(WHERE k.trust_scope='platform' AND k.organization_id IS NULL)::bigint AS platform, count(*) FILTER(WHERE k.trust_scope='organization' AND k.organization_id=$3::uuid)::bigint AS organization FROM signing_keys k WHERE {PUBLISHABLE}"),
        vec![cutoff.naive_utc().into(),now.naive_utc().into(),org.into()])).await.map_err(registry_error)?.ok_or_else(conflict)?;
    let count = |field| -> Result<usize, DomainError> {
        let n: i64 = row.try_get("", field).map_err(registry_error)?;
        usize::try_from(n).map_err(|_| conflict())
    };
    Ok(SigningSlotCounts {
        global: count("global")?,
        global_organization: count("global_organization")?,
        platform: count("platform")?,
        organization: count("organization")?,
    })
}

pub(super) async fn load_view(
    tx: &DatabaseTransaction,
    cutoff: DateTime<Utc>,
    now: DateTime<Utc>,
    affected: &[SigningKey],
) -> Result<(Vec<SigningKey>, BTreeMap<Uuid, StoredPublic>, bool), DomainError> {
    let mut values = vec![cutoff.naive_utc().into(), now.naive_utc().into()];
    let identities = affected
        .iter()
        .enumerate()
        .map(|(index, key)| {
            values.push(key.id.into());
            format!("${}::uuid", index + 3)
        })
        .collect::<Vec<_>>();
    let extra = if identities.is_empty() {
        String::new()
    } else {
        format!(" OR k.id IN ({})", identities.join(","))
    };
    let rows=tx.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
        format!("SELECT k.*,p.public_n,p.public_e,p.binding_fingerprint,p.longest_entry_bytes FROM signing_keys k LEFT JOIN signing_public_entries p ON p.signing_key_id=k.id WHERE {PUBLISHABLE} OR (k.status='retiring' AND k.updated_at>$2::timestamp){extra} ORDER BY k.id"),values)).await.map_err(registry_error)?;
    let mut keys = Vec::with_capacity(rows.len());
    let mut publics = BTreeMap::new();
    let mut invalid_retirement = false;
    for row in rows {
        let key = to_domain(
            super::signing_keys::Model::from_query_result(&row, "").map_err(registry_error)?,
        )?;
        let n: Option<String> = row.try_get("", "public_n").map_err(registry_error)?;
        invalid_retirement |= key.status == SigningKeyStatus::Retiring && key.updated_at > now;
        if let Some(n) = n {
            let length: i32 = row
                .try_get("", "longest_entry_bytes")
                .map_err(registry_error)?;
            publics.insert(
                key.id,
                StoredPublic {
                    n,
                    e: row.try_get("", "public_e").map_err(registry_error)?,
                    fingerprint: row
                        .try_get("", "binding_fingerprint")
                        .map_err(registry_error)?,
                    longest_entry_bytes: usize::try_from(length).map_err(|_| conflict())?,
                },
            );
        }
        keys.push(key);
    }
    Ok((keys, publics, invalid_retirement))
}

pub(super) fn occupies_slot(key: &SigningKey, cutoff: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    match key.status {
        SigningKeyStatus::Pending | SigningKeyStatus::Active => true,
        SigningKeyStatus::Retiring => key.updated_at > cutoff && key.updated_at <= now,
        SigningKeyStatus::Revoked => false,
    }
}

/// Apply only affected rows' differences to authoritative SQL counts. This does
/// not price/scan all keys, and replacing/promoting a slot does not double-charge.
#[cfg(test)]
pub(super) fn adjusted_counts(
    mut count: SigningSlotCounts,
    before: &[SigningKey],
    after: &[SigningKey],
    cutoff: DateTime<Utc>,
    now: DateTime<Utc>,
    org: Option<Uuid>,
) -> Result<SigningSlotCounts, DomainError> {
    let before = before
        .iter()
        .map(|key| (key.id, key))
        .collect::<BTreeMap<_, _>>();
    for key in after {
        let old = before.get(&key.id).copied();
        let was = old.is_some_and(|key| occupies_slot(key, cutoff, now));
        let will = occupies_slot(key, cutoff, now);
        if was == will {
            continue;
        }
        let adjust = |n: &mut usize| -> Result<(), DomainError> {
            *n = if will {
                n.checked_add(1)
            } else {
                n.checked_sub(1)
            }
            .ok_or_else(conflict)?;
            Ok(())
        };
        adjust(&mut count.global)?;
        if key.organization_id.is_some() {
            adjust(&mut count.global_organization)?;
            if key.organization_id == org {
                adjust(&mut count.organization)?;
            }
        } else {
            adjust(&mut count.platform)?;
        }
    }
    count.validate()?;
    Ok(count)
}

pub(super) struct PublicationPlan {
    pub(super) publication: ValidatedJwksPublication,
    pub(super) next_expiration: Option<DateTime<Utc>>,
}

pub(super) fn plan(
    keys: &[SigningKey],
    publics: &BTreeMap<Uuid, StoredPublic>,
    new: Option<(&SigningKey, &PreparedSigningPublicKey)>,
    now: DateTime<Utc>,
    retention: chrono::Duration,
) -> Result<PublicationPlan, DomainError> {
    let cutoff = now
        .checked_sub_signed(retention)
        .ok_or(DomainError::InvalidSigningKeyMaterial)?;
    let mut entries = Vec::new();
    let mut next_expiration = None;
    for key in keys {
        if key.status == SigningKeyStatus::Retiring && key.updated_at > now {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        if !occupies_slot(key, cutoff, now) {
            continue;
        }
        let prepared = if let Some((candidate, prepared)) =
            new.filter(|(candidate, _)| candidate.id == key.id)
        {
            let _ = candidate;
            prepared.clone()
        } else {
            publics
                .get(&key.id)
                .ok_or(DomainError::InvalidSigningKeyMaterial)?
                .restore(key)?
        };
        entries.push(prepared.project(key)?);
        if key.status == SigningKeyStatus::Retiring {
            let deadline = key
                .updated_at
                .checked_add_signed(retention)
                .ok_or(DomainError::InvalidSigningKeyMaterial)?;
            next_expiration = Some(
                next_expiration.map_or(deadline, |current: DateTime<Utc>| current.min(deadline)),
            );
        }
    }
    Ok(PublicationPlan {
        publication: ValidatedJwksPublication::from_entries(entries)?,
        next_expiration,
    })
}

pub(super) async fn persist_prepared(
    tx: &DatabaseTransaction,
    key: &SigningKey,
    prepared: &PreparedSigningPublicKey,
) -> Result<(), DomainError> {
    prepared.project(key)?;
    tx.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO signing_public_entries(signing_key_id,public_n,public_e,binding_fingerprint,longest_entry_bytes) VALUES($1,$2,$3,$4,$5)",
        vec![key.id.into(),prepared.n().to_string().into(),prepared.e().to_string().into(),prepared.binding_fingerprint().to_vec().into(),i32::try_from(prepared.longest_entry_bytes()).map_err(|_|conflict())?.into()])).await.map_err(registry_error)?;
    Ok(())
}

pub(super) async fn write(
    tx: &DatabaseTransaction,
    plan: &PublicationPlan,
    previous: i64,
    now: DateTime<Utc>,
    ttl: u64,
) -> Result<(), DomainError> {
    previous.checked_add(1).ok_or_else(conflict)?;
    let result=tx.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE signing_jwks_publication SET revision=revision+1,dirty=false,payload=$1,as_of=$2,next_expiration=$3,access_token_ttl=$4,retire_skew=$5 WHERE singleton=1 AND revision=$6",
        vec![plan.publication.payload().to_string().into(),now.into(),plan.next_expiration.into(),i64::try_from(ttl).map_err(|_|conflict())?.into(),JWKS_RETIRE_SKEW_SECONDS.into(),previous.into()])).await.map_err(registry_error)?;
    if result.rows_affected() != 1 {
        return Err(conflict());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{
        admission_tests::{public_projection, row, snapshot_projection},
        SeaOrmSigningKeyRegistry,
    };
    use super::*;
    use iam_domain::{
        entity::signing_key::{
            SigningKeyAdmissionHistory, SigningKeyAdmissionReason, SigningKeyLifecyclePolicy,
        },
        port::repository::SigningKeyRegistry,
    };
    use sea_orm::{DatabaseConnection, MockDatabase, MockExecResult, TransactionTrait, Value};
    use std::sync::Arc;

    fn prepared(key: &SigningKey) -> StoredPublic {
        let value = PreparedSigningPublicKey::prepare(key).unwrap();
        StoredPublic {
            n: value.n().into(),
            e: value.e().into(),
            fingerprint: value.binding_fingerprint().to_vec(),
            longest_entry_bytes: value.longest_entry_bytes(),
        }
    }
    fn clock(now: DateTime<Utc>) -> BTreeMap<String, Value> {
        BTreeMap::from([("as_of".into(), now.fixed_offset().into())])
    }
    fn repository(db: Arc<DatabaseConnection>, ttl: u64) -> SeaOrmSigningKeyRegistry {
        SeaOrmSigningKeyRegistry::new(db, Arc::new(SigningKeyLifecyclePolicy::new().unwrap()), ttl)
            .unwrap()
    }
    fn logs(db: Arc<DatabaseConnection>) -> String {
        format!("{:?}", Arc::try_unwrap(db).unwrap().into_transaction_log()).replace("\\\"", "\"")
    }

    #[test]
    fn expire_and_revoke_free_slots_but_never_erase_immutable_churn_evidence() {
        let now = Utc::now();
        let org = Uuid::new_v4();
        let retention = chrono::Duration::seconds(960);
        let cutoff = now - retention;
        let mut active = to_domain(row(org, "active")).unwrap();
        active.updated_at = now;
        let mut retiring = active.clone();
        retiring.id = Uuid::new_v4();
        retiring.kid = Uuid::new_v4().simple().to_string();
        retiring.status = SigningKeyStatus::Retiring;
        retiring.updated_at = cutoff;
        assert!(!occupies_slot(&retiring, cutoff, now));
        retiring.updated_at += chrono::Duration::nanoseconds(1);
        assert!(occupies_slot(&retiring, cutoff, now));
        let before = vec![active.clone(), retiring.clone()];
        active.status = SigningKeyStatus::Revoked;
        let counts = adjusted_counts(
            SigningSlotCounts {
                global: 2,
                global_organization: 2,
                platform: 0,
                organization: 2,
            },
            &before,
            &[active, retiring],
            cutoff,
            now,
            Some(org),
        )
        .unwrap();
        assert_eq!(
            counts,
            SigningSlotCounts {
                global: 1,
                global_organization: 1,
                platform: 0,
                organization: 1
            }
        );
        let history = vec![
            SigningKeyAdmissionHistory {
                organization_id: Some(org),
                admitted_at: now - chrono::Duration::seconds(1)
            };
            4
        ];
        assert!(matches!(
            SigningKeyLifecyclePolicy::new()
                .unwrap()
                .check_churn(&history, Some(org), now),
            Err(DomainError::SigningKeyAdmissionDenied {
                reason: SigningKeyAdmissionReason::ChurnRate,
                retry_after_seconds: Some(3599)
            })
        ));
    }

    #[test]
    fn prepared_public_corruption_duplicate_kids_and_future_retirement_fail_closed_before_writes() {
        let now = Utc::now();
        let mut key = to_domain(row(Uuid::new_v4(), "active")).unwrap();
        key.updated_at = now;
        let public = prepared(&key);
        let mut records = BTreeMap::from([(key.id, public.clone())]);
        let retention = chrono::Duration::seconds(960);
        let valid = plan(&[key.clone()], &records, None, now, retention).unwrap();
        assert_eq!(valid.publication.counts().global, 1);
        records.get_mut(&key.id).unwrap().fingerprint[0] ^= 1;
        assert!(plan(&[key.clone()], &records, None, now, retention).is_err());
        records.insert(key.id, public.clone());
        records.get_mut(&key.id).unwrap().longest_entry_bytes += 1;
        assert!(plan(&[key.clone()], &records, None, now, retention).is_err());
        records.insert(key.id, public);
        let mut duplicate = key.clone();
        duplicate.id = Uuid::new_v4();
        duplicate.status = SigningKeyStatus::Pending;
        let candidate = PreparedSigningPublicKey::prepare(&duplicate).unwrap();
        assert!(plan(
            &[key.clone(), duplicate.clone()],
            &records,
            Some((&duplicate, &candidate)),
            now,
            retention
        )
        .is_err());
        key.status = SigningKeyStatus::Retiring;
        key.updated_at = now + chrono::Duration::seconds(1);
        assert!(plan(&[key], &records, None, now, retention).is_err());
    }

    #[tokio::test]
    async fn primary_count_and_prepared_view_share_exact_bound_clock_predicate_without_tariffs() {
        let now = Utc::now();
        let cutoff = now - chrono::Duration::seconds(233);
        let org = Uuid::new_v4();
        let model = row(org, "pending");
        let count: BTreeMap<String, Value> = BTreeMap::from([
            ("global".into(), 1i64.into()),
            ("global_organization".into(), 1i64.into()),
            ("platform".into(), 0i64.into()),
            ("organization".into(), 1i64.into()),
        ]);
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![count]])
                .append_query_results([public_projection(&[model])])
                .into_connection(),
        );
        let tx = db.begin().await.unwrap();
        assert_eq!(counts(&tx, cutoff, now, Some(org)).await.unwrap().global, 1);
        let (keys, records, future) = load_view(&tx, cutoff, now, &[]).await.unwrap();
        assert!(!future);
        let value = plan(&keys, &records, None, now, chrono::Duration::seconds(233)).unwrap();
        assert_eq!(value.publication.counts().organization, 1);
        tx.rollback().await.unwrap();
        let log = logs(db);
        assert_eq!(log.matches(PUBLISHABLE).count(), 2);
        assert!(log.contains("count(*) FILTER"));
        assert!(log.contains("signing_public_entries"));
        assert!(!log.contains(" LIMIT "));
        assert!(log.contains(&format!("{:?}", cutoff.naive_utc())));
        assert!(log.contains(&format!("{:?}", now.naive_utc())));
    }

    #[tokio::test]
    async fn snapshot_metadata_requires_refresh_for_dirty_revision0_policy_and_exact_deadline() {
        let now = Utc::now();
        for case in 0..5 {
            let mut value = snapshot_projection(&[], now);
            let deadline = now + chrono::Duration::seconds(960);
            match case {
                0 => {
                    value.insert("dirty".into(), true.into());
                }
                1 => {
                    value.insert("revision".into(), 0i64.into());
                    value.insert("access_token_ttl".into(), 0i64.into());
                }
                2 => {
                    value.insert("access_token_ttl".into(), 173i64.into());
                }
                3 => {
                    value.insert("retire_skew".into(), 65i64.into());
                }
                _ => {
                    value.insert(
                        "next_expiration".into(),
                        Some(deadline.fixed_offset()).into(),
                    );
                }
            }
            let db = MockDatabase::new(DbBackend::Postgres)
                .append_query_results([vec![value]])
                .into_connection();
            let row = db
                .query_one(Statement::from_string(
                    DbBackend::Postgres,
                    "SELECT fixture",
                ))
                .await
                .unwrap()
                .unwrap();
            let stored = StoredPublication::decode(&row).unwrap();
            if case == 4 {
                assert!(!stored
                    .needs_refresh(deadline - chrono::Duration::nanoseconds(1), 900)
                    .unwrap());
                assert!(stored.needs_refresh(deadline, 900).unwrap());
            } else {
                assert!(stored.needs_refresh(now, 900).unwrap());
            }
            assert!(stored
                .needs_refresh(now - chrono::Duration::nanoseconds(1), 900)
                .is_err());
        }
    }

    #[tokio::test]
    async fn expiry_policy_refresh_uses_postlock_primary_clock_and_increments_global_revision() {
        for ttl in [900, 173] {
            let materialized = Utc::now();
            let deadline = materialized + chrono::Duration::seconds(960);
            let read_clock = deadline;
            let postlock = deadline + chrono::Duration::seconds(7);
            let mut model = row(Uuid::new_v4(), "retiring");
            model.updated_at = materialized.naive_utc();
            let mut value = snapshot_projection(&[model], materialized);
            value.insert(
                "next_expiration".into(),
                Some(deadline.fixed_offset()).into(),
            );
            value.insert("db_as_of".into(), read_clock.fixed_offset().into());
            let db = Arc::new(
                MockDatabase::new(DbBackend::Postgres)
                    .append_exec_results((0..2).map(|_| MockExecResult {
                        rows_affected: 1,
                        last_insert_id: 0,
                    }))
                    .append_query_results([
                        vec![value.clone()],
                        vec![value],
                        vec![clock(postlock)],
                        vec![],
                    ])
                    .into_connection(),
            );
            let repository = repository(db.clone(), ttl);
            let result = repository.jwks_publication_snapshot().await.unwrap();
            assert_eq!(result.revision, 2);
            assert_eq!(result.as_of, postlock);
            assert_eq!(result.access_token_expiration_seconds, ttl);
            assert_eq!(result.next_expiration, None);
            assert_eq!(result.publication.payload(), "{\"keys\":[]}");
            drop(repository);
            let log = logs(db);
            let lock = log.find("iam-signing-jwks-admission-v1").unwrap();
            let row_lock = log.find("FOR UPDATE").unwrap();
            let clock = log.find("SELECT statement_timestamp() AS as_of").unwrap();
            let refresh = log.find("UPDATE signing_jwks_publication").unwrap();
            assert!(lock < row_lock && row_lock < clock && clock < refresh);
            assert!(log.contains("revision=revision+1,dirty=false"));
            assert!(log.contains("COMMIT"));
            assert!(!log.contains("INSERT INTO"));
        }
    }

    #[tokio::test]
    async fn corrupt_or_missing_primary_snapshot_never_returns_empty_or_attempts_bootstrap() {
        let now = Utc::now();
        for payload in ["", "{}", "{\"keys\":[],\"private\":true}", "{ \"keys\":[]}"] {
            let mut value = snapshot_projection(&[], now);
            value.insert("payload".into(), payload.into());
            let db = Arc::new(
                MockDatabase::new(DbBackend::Postgres)
                    .append_query_results([vec![value]])
                    .into_connection(),
            );
            let repository = repository(db.clone(), 900);
            assert!(repository.jwks_publication_snapshot().await.is_err());
            drop(repository);
            let log = logs(db);
            assert!(!log.contains("BEGIN"));
            assert!(!log.contains("INSERT INTO"));
            assert!(!log.contains("FROM signing_keys"));
        }
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_query_results([Vec::<BTreeMap<String, Value>>::new()])
                .into_connection(),
        );
        let repository = repository(db.clone(), 900);
        assert!(repository.jwks_publication_snapshot().await.is_err());
        drop(repository);
        assert!(!logs(db).contains("BEGIN"));
    }

    #[tokio::test]
    async fn dirty_prepared_record_corruption_forces_primary_refresh_and_rolls_back_not_empty() {
        let now = Utc::now();
        let model = row(Uuid::new_v4(), "active");
        let mut cached = snapshot_projection(&[model.clone()], now);
        cached.insert("dirty".into(), true.into());
        let mut records = public_projection(&[model]);
        if let Value::Bytes(Some(bytes)) = records[0].get_mut("binding_fingerprint").unwrap() {
            bytes[0] ^= 1;
        } else {
            panic!("fixture fingerprint must be binary");
        }
        let db = Arc::new(
            MockDatabase::new(DbBackend::Postgres)
                .append_exec_results([MockExecResult {
                    rows_affected: 1,
                    last_insert_id: 0,
                }])
                .append_query_results([
                    vec![cached.clone()],
                    vec![cached],
                    vec![clock(now)],
                    records,
                ])
                .into_connection(),
        );
        let repository = repository(db.clone(), 900);
        assert!(matches!(
            repository.jwks_publication_snapshot().await,
            Err(DomainError::InvalidSigningKeyMaterial)
        ));
        drop(repository);
        let log = logs(db);
        assert!(
            log.contains("FROM signing_public_entries")
                || log.contains("JOIN signing_public_entries")
        );
        assert!(log.contains("ROLLBACK"));
        assert!(!log.contains("COMMIT"));
        assert!(!log.contains("UPDATE "));
        assert!(!log.contains("INSERT INTO"));
    }
}
