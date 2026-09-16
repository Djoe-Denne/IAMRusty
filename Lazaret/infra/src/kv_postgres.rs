//! Postgres `apparatus_kv_entries` adapter (Lazaret DB).

use std::sync::Arc;

use apparatus_contracts::{
    ApparatusError, BindingId, KvStore, MAX_KV_ENTRIES_PER_BINDING, MAX_KV_KEY_LEN,
    MAX_KV_VALUE_BYTES,
};
use async_trait::async_trait;
use lazaret_domain::AsyncKvStore;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, QueryResult, Statement};
use uuid::Uuid;

/// Postgres KV namespaced by `binding_id`.
#[derive(Clone)]
pub struct PostgresKvStore {
    db: DatabaseConnection,
}

impl PostgresKvStore {
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

fn binding_uuid(binding: &BindingId) -> Result<Uuid, ApparatusError> {
    Uuid::parse_str(binding.as_str()).map_err(|_| ApparatusError::InvalidId {
        reason: "binding id must be a UUID".to_owned(),
    })
}

fn check_key(key: &str) -> Result<(), ApparatusError> {
    if key.is_empty() || key.chars().count() > MAX_KV_KEY_LEN {
        return Err(ApparatusError::InvalidOperation {
            reason: "kv key length out of bounds".to_owned(),
        });
    }
    Ok(())
}

fn cas_mismatch() -> ApparatusError {
    ApparatusError::InvalidOperation {
        reason: "cas mismatch".to_owned(),
    }
}

fn quota_exceeded() -> ApparatusError {
    ApparatusError::InvalidOperation {
        reason: "kv entry quota exceeded".to_owned(),
    }
}

fn store_failed() -> ApparatusError {
    ApparatusError::InvalidOperation {
        reason: "kv store failed".to_owned(),
    }
}

fn map_put_cas_row(row: QueryResult) -> Result<i64, ApparatusError> {
    let outcome: String = row.try_get("", "outcome").map_err(|_| store_failed())?;
    match outcome.as_str() {
        "ok" => row.try_get("", "cas_version").map_err(|_| store_failed()),
        "cas mismatch" => Err(cas_mismatch()),
        "kv entry quota exceeded" => Err(quota_exceeded()),
        _ => Err(store_failed()),
    }
}

/// `$4` = expected CAS. `0` on a missing key inserts version 1 when under quota.
/// `_lock` is referenced via `FROM _lock` so the xact advisory lock is evaluated.
const PUT_CAS: &str = r"
WITH _lock AS MATERIALIZED (
  SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0)) IS NULL AS taken
),
upd AS (
  UPDATE apparatus_kv_entries
  SET value = $3, cas_version = cas_version + 1
  FROM _lock
  WHERE apparatus_kv_entries.binding_id = $1
    AND apparatus_kv_entries.key = $2
    AND apparatus_kv_entries.cas_version = $4
  RETURNING apparatus_kv_entries.cas_version
),
ins AS (
  INSERT INTO apparatus_kv_entries (binding_id, key, value, cas_version)
  SELECT $1, $2, $3, 1
  FROM _lock
  WHERE $4 = 0
    AND NOT EXISTS (SELECT 1 FROM upd)
    AND NOT EXISTS (
      SELECT 1 FROM apparatus_kv_entries
      WHERE binding_id = $1 AND key = $2
    )
    AND (SELECT COUNT(*) FROM apparatus_kv_entries WHERE binding_id = $1) < $5
  RETURNING cas_version
)
SELECT cas_version, 'ok'::text AS outcome FROM upd
UNION ALL
SELECT cas_version, 'ok' FROM ins
UNION ALL
SELECT NULL::bigint,
  CASE
    WHEN $4 <> 0
      OR EXISTS (
        SELECT 1 FROM apparatus_kv_entries WHERE binding_id = $1 AND key = $2
      )
    THEN 'cas mismatch'
    ELSE 'kv entry quota exceeded'
  END
WHERE NOT EXISTS (SELECT 1 FROM upd)
  AND NOT EXISTS (SELECT 1 FROM ins)
";

const PUT_UPSERT: &str = r"
WITH _lock AS MATERIALIZED (
  SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0)) IS NULL AS taken
),
existing AS (
  SELECT e.cas_version
  FROM apparatus_kv_entries e
  CROSS JOIN _lock
  WHERE e.binding_id = $1 AND e.key = $2
),
ins AS (
  INSERT INTO apparatus_kv_entries (binding_id, key, value, cas_version)
  SELECT $1, $2, $3, 1
  FROM _lock
  WHERE NOT EXISTS (SELECT 1 FROM existing)
    AND (SELECT COUNT(*) FROM apparatus_kv_entries WHERE binding_id = $1) < $4
  RETURNING cas_version
),
upd AS (
  UPDATE apparatus_kv_entries
  SET value = $3, cas_version = cas_version + 1
  FROM _lock
  WHERE apparatus_kv_entries.binding_id = $1
    AND apparatus_kv_entries.key = $2
    AND EXISTS (SELECT 1 FROM existing)
  RETURNING apparatus_kv_entries.cas_version
)
SELECT cas_version FROM ins
UNION ALL
SELECT cas_version FROM upd
";

#[async_trait]
impl AsyncKvStore for PostgresKvStore {
    async fn get(&self, binding: &BindingId, key: &str) -> Result<Option<Vec<u8>>, ApparatusError> {
        check_key(key)?;
        let binding_id = binding_uuid(binding)?;
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT value FROM apparatus_kv_entries WHERE binding_id = $1 AND key = $2",
                [binding_id.into(), key.into()],
            ))
            .await
            .map_err(|_| store_failed())?;
        match row {
            Some(row) => {
                let value: Vec<u8> = row.try_get("", "value").map_err(|_| store_failed())?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn put(
        &self,
        binding: &BindingId,
        key: &str,
        value: &[u8],
        expected_cas: Option<i64>,
    ) -> Result<i64, ApparatusError> {
        check_key(key)?;
        if value.len() > MAX_KV_VALUE_BYTES {
            return Err(ApparatusError::PayloadTooLarge {
                max: MAX_KV_VALUE_BYTES,
                actual: value.len(),
            });
        }
        let binding_id = binding_uuid(binding)?;
        let quota = MAX_KV_ENTRIES_PER_BINDING as i64;
        if let Some(expected) = expected_cas {
            let row = self
                .db
                .query_one(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    PUT_CAS,
                    [
                        binding_id.into(),
                        key.into(),
                        value.to_vec().into(),
                        expected.into(),
                        quota.into(),
                    ],
                ))
                .await
                .map_err(|_| store_failed())?;
            let Some(row) = row else {
                return Err(cas_mismatch());
            };
            return map_put_cas_row(row);
        }
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                PUT_UPSERT,
                [
                    binding_id.into(),
                    key.into(),
                    value.to_vec().into(),
                    quota.into(),
                ],
            ))
            .await
            .map_err(|_| store_failed())?;
        let Some(row) = row else {
            return Err(quota_exceeded());
        };
        let version: i64 = row.try_get("", "cas_version").map_err(|_| store_failed())?;
        Ok(version)
    }

    async fn delete(&self, binding: &BindingId, key: &str) -> Result<bool, ApparatusError> {
        check_key(key)?;
        let binding_id = binding_uuid(binding)?;
        let result = self
            .db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "DELETE FROM apparatus_kv_entries WHERE binding_id = $1 AND key = $2",
                [binding_id.into(), key.into()],
            ))
            .await
            .map_err(|_| store_failed())?;
        Ok(result.rows_affected() > 0)
    }

    async fn purge(&self, binding: &BindingId) {
        let Ok(binding_id) = binding_uuid(binding) else {
            return;
        };
        let _ = self
            .db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "DELETE FROM apparatus_kv_entries WHERE binding_id = $1",
                [binding_id.into()],
            ))
            .await;
    }
}

impl KvStore for PostgresKvStore {
    fn kv_get(&self, binding: &BindingId, key: &str) -> Result<Option<Vec<u8>>, ApparatusError> {
        run_sync(AsyncKvStore::get(self, binding, key))
    }

    fn kv_put(
        &self,
        binding: &BindingId,
        key: &str,
        value: &[u8],
        expected_cas: Option<i64>,
    ) -> Result<i64, ApparatusError> {
        run_sync(AsyncKvStore::put(self, binding, key, value, expected_cas))
    }

    fn kv_delete(&self, binding: &BindingId, key: &str) -> Result<bool, ApparatusError> {
        run_sync(AsyncKvStore::delete(self, binding, key))
    }

    fn kv_purge(&self, binding: &BindingId) {
        run_sync(async {
            AsyncKvStore::purge(self, binding).await;
        });
    }
}

fn run_sync<T>(fut: impl std::future::Future<Output = T>) -> T {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("kv runtime")
            .block_on(fut),
    }
}

#[cfg(test)]
mod sql_contract {
    use super::{PUT_CAS, PUT_UPSERT};

    #[test]
    fn put_statements_evaluate_advisory_lock_cte() {
        for sql in [PUT_CAS, PUT_UPSERT] {
            assert!(
                sql.contains("hashtextextended($1::text, 0)"),
                "64-bit lock key: {sql}"
            );
            assert!(
                sql.contains("AS MATERIALIZED"),
                "lock CTE must not be inlined away: {sql}"
            );
            assert!(
                sql.contains("FROM _lock"),
                "unreferenced SELECT CTE is skipped; DML must FROM _lock: {sql}"
            );
        }
    }

    #[test]
    fn put_cas_creates_when_expected_is_zero() {
        assert!(PUT_CAS.contains("INSERT INTO apparatus_kv_entries"));
        assert!(PUT_CAS.contains("WHERE $4 = 0"));
        assert!(PUT_CAS.contains("THEN 'cas mismatch'"));
        assert!(PUT_CAS.contains("ELSE 'kv entry quota exceeded'"));
    }
}
