//! Redis KV adapter. Layout: HASH `lazaret:kv:{binding}` + HASH `lazaret:cas:{binding}`.

use apparatus_contracts::{
    ApparatusError, BindingId, KvStore, MAX_KV_ENTRIES_PER_BINDING, MAX_KV_KEY_LEN,
    MAX_KV_VALUE_BYTES,
};
use async_trait::async_trait;
use lazaret_domain::AsyncKvStore;
use redis::Commands;

/// Redis KV namespaced by `binding_id`. Caller still only passes `key`.
#[derive(Clone)]
pub struct RedisKvStore {
    client: redis::Client,
}

impl RedisKvStore {
    /// Connect using `redis://host:port`.
    ///
    /// # Errors
    ///
    /// Returns [`ApparatusError::InvalidOperation`] if the URL is unusable.
    pub fn connect(url: &str) -> Result<Self, ApparatusError> {
        let client = redis::Client::open(url).map_err(|_| store_failed())?;
        Ok(Self { client })
    }

    fn connection(&self) -> Result<redis::Connection, ApparatusError> {
        self.client.get_connection().map_err(|_| store_failed())
    }

    /// Delete both hashes for one binding.
    ///
    /// # Errors
    ///
    /// Returns [`ApparatusError`] if the Redis connection or script fails.
    fn purge_namespace(&self, binding: &BindingId) -> Result<(), ApparatusError> {
        let mut con = self.connection()?;
        let script = redis::Script::new(
            r"
redis.call('DEL', KEYS[1])
redis.call('DEL', KEYS[2])
return 1
",
        );
        let _: i32 = script
            .key(kv_hash(binding))
            .key(cas_hash(binding))
            .invoke(&mut con)
            .map_err(|_| store_failed())?;
        Ok(())
    }
}

fn store_failed() -> ApparatusError {
    ApparatusError::InvalidOperation {
        reason: "kv store failed".to_owned(),
    }
}

fn redis_error_text(error: &redis::RedisError) -> String {
    format!(
        "{} {} {}",
        error,
        error.code().unwrap_or(""),
        error.detail().unwrap_or("")
    )
}

fn check_key(key: &str) -> Result<(), ApparatusError> {
    if key.is_empty() || key.chars().count() > MAX_KV_KEY_LEN {
        return Err(ApparatusError::InvalidOperation {
            reason: "kv key length out of bounds".to_owned(),
        });
    }
    Ok(())
}

fn kv_hash(binding: &BindingId) -> String {
    format!("lazaret:kv:{}", binding.as_str())
}

fn cas_hash(binding: &BindingId) -> String {
    format!("lazaret:cas:{}", binding.as_str())
}

impl KvStore for RedisKvStore {
    fn kv_get(&self, binding: &BindingId, key: &str) -> Result<Option<Vec<u8>>, ApparatusError> {
        check_key(key)?;
        let mut con = self.connection()?;
        let found: Option<Vec<u8>> = con
            .hget(kv_hash(binding), key)
            .map_err(|_| store_failed())?;
        Ok(found)
    }

    fn kv_put(
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
        let mut con = self.connection()?;
        let script = redis::Script::new(
            r"
local cas_hash = KEYS[1]
local kv_hash = KEYS[2]
local field = ARGV[1]
local expected = ARGV[2]
local value = ARGV[3]
local quota = tonumber(ARGV[4])
local current = redis.call('HGET', cas_hash, field)
local cur = 0
if current then cur = tonumber(current) end
if expected ~= '' then
  if cur ~= tonumber(expected) then return redis.error_reply('cas mismatch') end
end
if not current then
  local n = redis.call('HLEN', kv_hash)
  if n >= quota then return redis.error_reply('kv entry quota exceeded') end
end
local nxt = cur + 1
redis.call('HSET', cas_hash, field, nxt)
redis.call('HSET', kv_hash, field, value)
return nxt
",
        );
        let expected = expected_cas.map(|n| n.to_string()).unwrap_or_default();
        let next: Result<i64, redis::RedisError> = script
            .key(cas_hash(binding))
            .key(kv_hash(binding))
            .arg(key)
            .arg(expected)
            .arg(value)
            .arg(MAX_KV_ENTRIES_PER_BINDING as i64)
            .invoke(&mut con);
        let next = next.map_err(|error| {
            let msg = redis_error_text(&error);
            if msg.contains("cas mismatch") {
                ApparatusError::InvalidOperation {
                    reason: "cas mismatch".to_owned(),
                }
            } else if msg.contains("quota") {
                ApparatusError::InvalidOperation {
                    reason: "kv entry quota exceeded".to_owned(),
                }
            } else {
                ApparatusError::InvalidOperation {
                    reason: format!("kv store failed: {msg}"),
                }
            }
        })?;
        Ok(next)
    }

    fn kv_delete(&self, binding: &BindingId, key: &str) -> Result<bool, ApparatusError> {
        check_key(key)?;
        let mut con = self.connection()?;
        let script = redis::Script::new(
            r"
local kv_hash = KEYS[1]
local cas_hash = KEYS[2]
local field = ARGV[1]
local removed = redis.call('HDEL', kv_hash, field)
redis.call('HDEL', cas_hash, field)
return removed
",
        );
        let removed: i32 = script
            .key(kv_hash(binding))
            .key(cas_hash(binding))
            .arg(key)
            .invoke(&mut con)
            .map_err(|_| store_failed())?;
        Ok(removed > 0)
    }

    fn kv_purge(&self, binding: &BindingId) {
        let _ = self.purge_namespace(binding);
    }
}

#[async_trait]
impl AsyncKvStore for RedisKvStore {
    async fn get(&self, binding: &BindingId, key: &str) -> Result<Option<Vec<u8>>, ApparatusError> {
        let store = self.clone();
        let binding = binding.clone();
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || KvStore::kv_get(&store, &binding, &key))
            .await
            .map_err(|_| store_failed())?
    }

    async fn put(
        &self,
        binding: &BindingId,
        key: &str,
        value: &[u8],
        expected_cas: Option<i64>,
    ) -> Result<i64, ApparatusError> {
        let store = self.clone();
        let binding = binding.clone();
        let key = key.to_owned();
        let value = value.to_vec();
        tokio::task::spawn_blocking(move || {
            KvStore::kv_put(&store, &binding, &key, &value, expected_cas)
        })
        .await
        .map_err(|_| store_failed())?
    }

    async fn delete(&self, binding: &BindingId, key: &str) -> Result<bool, ApparatusError> {
        let store = self.clone();
        let binding = binding.clone();
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || KvStore::kv_delete(&store, &binding, &key))
            .await
            .map_err(|_| store_failed())?
    }

    async fn purge(&self, binding: &BindingId) -> Result<(), ApparatusError> {
        let store = self.clone();
        let binding = binding.clone();
        tokio::task::spawn_blocking(move || store.purge_namespace(&binding))
            .await
            .map_err(|_| store_failed())?
    }
}
