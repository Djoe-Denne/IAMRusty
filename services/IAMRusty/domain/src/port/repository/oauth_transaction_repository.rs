use crate::entity::oauth_transaction::{
    ConsumeOAuthTransaction, OAuthTransaction, OAuthTransactionError,
};
use async_trait::async_trait;

/// Shared authoritative writer; no replica read authorizes an OAuth callback.
#[async_trait]
pub trait OAuthTransactionWriteRepository: Send + Sync {
    /// Delete at most `batch_size` expired rows (consumed or abandoned), using
    /// the primary clock and deterministic expiration/id ordering with SKIP LOCKED.
    /// Reject sizes outside 1..=1000 before SQL. Never return nonces, verifiers or rows.
    async fn purge_expired(&self, batch_size: u32) -> Result<u64, OAuthTransactionError>;

    async fn create(&self, transaction: &OAuthTransaction) -> Result<(), OAuthTransactionError>;
    /// Consume one matching, unexpired row atomically on the authoritative writer.
    /// Return its original authentication nonce and verifier with a consumed marker
    /// only after committing both consumption and erasure of both persisted secrets.
    /// A write/commit failure returns a generic storage error, never a capability.
    ///
    /// Do not filter out legacy/corrupt nonces: matching rows with a NULL, empty,
    /// malformed or noncanonical nonce must still be consumed and both secrets
    /// erased before commit/return. The domain then refuses the original record.
    /// Invalid bindings leave the row intact; expired rows remain for cleanup.
    /// `None` uniformly denotes no consumable row, including replay.
    async fn consume(
        &self,
        input: &ConsumeOAuthTransaction,
    ) -> Result<Option<OAuthTransaction>, OAuthTransactionError>;
}
