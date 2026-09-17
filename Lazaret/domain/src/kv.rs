//! Platform KV port (async I/O adapters behind [`apparatus_contracts::KvStore`]).

use apparatus_contracts::{ApparatusError, BindingId};
use async_trait::async_trait;

/// Async surface used by Lazaret invoke. Adapters also implement [`apparatus_contracts::KvStore`].
///
/// Method names differ from [`apparatus_contracts::KvStore`] so UFCS stays unambiguous.
#[async_trait]
pub trait AsyncKvStore: Send + Sync {
    /// Read one key in the binding namespace.
    async fn get(&self, binding: &BindingId, key: &str) -> Result<Option<Vec<u8>>, ApparatusError>;

    /// Write one key. See [`apparatus_contracts::KvStore::kv_put`] for CAS.
    async fn put(
        &self,
        binding: &BindingId,
        key: &str,
        value: &[u8],
        expected_cas: Option<i64>,
    ) -> Result<i64, ApparatusError>;

    /// Delete one key.
    async fn delete(&self, binding: &BindingId, key: &str) -> Result<bool, ApparatusError>;

    /// Wipe one binding namespace only.
    ///
    /// # Errors
    ///
    /// Returns [`ApparatusError`] if the backing store fails to purge.
    async fn purge(&self, binding: &BindingId) -> Result<(), ApparatusError>;
}
