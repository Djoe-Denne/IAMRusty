//! KV namespace lifecycle (purge on binding end-of-life).

use apparatus_contracts::{ApparatusError, BindingId};
use lazaret_domain::AsyncKvStore;

/// Wipe the platform KV namespace for one binding.
///
/// # Errors
///
/// Returns [`ApparatusError`] if the KV adapter fails to purge the namespace.
pub async fn purge_binding_namespace(
    store: &dyn AsyncKvStore,
    binding: BindingId,
) -> Result<(), ApparatusError> {
    store.purge(&binding).await
}
