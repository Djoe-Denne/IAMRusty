use crate::entity::oauth_transaction::{
    ConsumeOAuthTransaction, OAuthTransaction, OAuthTransactionError,
};
use async_trait::async_trait;

/// Shared authoritative writer; no replica read authorizes an OAuth callback.
#[async_trait]
pub trait OAuthTransactionWriteRepository: Send + Sync {
    /// Delete at most `batch_size` expired rows (consumed or abandoned), using
    /// the primary clock and deterministic expiration/id ordering with SKIP LOCKED.
    /// Reject sizes outside 1..=1000 before SQL. Never return verifiers or rows.
    async fn purge_expired(&self, batch_size: u32) -> Result<u64, OAuthTransactionError>;

    async fn create(&self, transaction: &OAuthTransaction) -> Result<(), OAuthTransactionError>;
    /// Consume one row atomically and return the original verifier, erasing it from
    /// persisted storage before commit. None uniformly denotes an invalid callback.
    async fn consume(
        &self,
        input: &ConsumeOAuthTransaction,
    ) -> Result<Option<OAuthTransaction>, OAuthTransactionError>;
}
