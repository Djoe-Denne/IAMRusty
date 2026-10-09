//! Browser-bound, writer-consumed OAuth authorization capability (ADR-0412).

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::provider::Provider;
use crate::port::repository::OAuthTransactionWriteRepository;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum OAuthOperation {
    #[serde(rename = "login")]
    Login,
    #[serde(rename = "link")]
    Link { user_id: Uuid },
    #[serde(rename = "relink")]
    Relink { user_id: Uuid },
}

impl OAuthOperation {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Link { .. } => "link",
            Self::Relink { .. } => "relink",
        }
    }

    #[must_use]
    pub const fn target_user_id(&self) -> Option<Uuid> {
        match self {
            Self::Login => None,
            Self::Link { user_id } | Self::Relink { user_id } => Some(*user_id),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OAuthTransactionError {
    #[error("invalid OAuth transaction")]
    InvalidTransaction,
    #[error("OAuth transaction storage failed")]
    Storage,
}

/// No Debug/Serialize: hashes and temporary verifier never enter diagnostic logs.
#[derive(Clone)]
pub struct BeginOAuthTransaction {
    pub nonce_hash: [u8; 32],
    pub state_hash: [u8; 32],
    pub browser_nonce_hash: [u8; 32],
    pub provider: Provider,
    pub operation: OAuthOperation,
    pub redirect_uri: String,
    pub expires_at: i64,
    pub pkce_required: bool,
}

#[derive(Debug, Clone)]
pub struct BegunOAuthTransaction {
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

#[derive(Clone)]
pub struct ConsumeOAuthTransaction {
    pub nonce_hash: [u8; 32],
    pub state_hash: [u8; 32],
    pub browser_nonce_hash: [u8; 32],
    pub provider: Provider,
    pub operation: OAuthOperation,
    pub redirect_uri: String,
    pub expires_at: i64,
}

/// Writer record; never use a replica read of this row as an OAuth capability.
pub struct OAuthTransaction {
    pub id: Uuid,
    pub nonce_hash: [u8; 32],
    pub state_hash: [u8; 32],
    pub browser_nonce_hash: [u8; 32],
    pub provider: Provider,
    pub operation: OAuthOperation,
    pub redirect_uri: String,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
    pub pkce_verifier: Option<String>,
}

/// Only constructed after a committed consume. Fields are private; neither a query
/// string nor an HTTP handler can manufacture a callback authorization context.
pub struct ConsumedOAuthTransaction {
    transaction: OAuthTransaction,
}

impl std::fmt::Debug for ConsumedOAuthTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConsumedOAuthTransaction([redacted])")
    }
}

impl ConsumedOAuthTransaction {
    #[must_use]
    pub const fn provider(&self) -> &Provider {
        &self.transaction.provider
    }
    #[must_use]
    pub const fn operation(&self) -> &OAuthOperation {
        &self.transaction.operation
    }
    #[must_use]
    pub const fn target_user_id(&self) -> Option<Uuid> {
        self.operation().target_user_id()
    }
    #[must_use]
    pub fn redirect_uri(&self) -> &str {
        &self.transaction.redirect_uri
    }
    #[must_use]
    pub const fn expires_at(&self) -> i64 {
        self.transaction.expires_at
    }

    /// Only domain orchestration may forward this secret to a token exchange.
    pub(crate) fn pkce_verifier(&self) -> Option<&str> {
        self.transaction.pkce_verifier.as_deref()
    }

    /// Exchange only with this consumed context's redirect and private verifier.
    ///
    /// # Errors
    /// Returns a generic OAuth error for a failed/unsupported PKCE exchange.
    pub async fn exchange_code(
        &self,
        client: &dyn crate::port::service::FederatedOAuthClient,
        code: &str,
    ) -> Result<super::provider::ProviderTokens, crate::error::DomainError> {
        client
            .exchange_code_with_pkce(code, self.redirect_uri(), self.pkce_verifier())
            .await
            .map_err(|_| {
                crate::error::DomainError::OAuth2Error("OAuth token exchange failed".into())
            })
    }

    /// Obtain a callback capability only after committed writer consumption.
    ///
    /// # Errors
    /// Returns a generic invalid-transaction error for absence/mismatch/replay,
    /// or a generic storage error when the writer operation fails.
    pub async fn consume(
        repository: &dyn OAuthTransactionWriteRepository,
        input: ConsumeOAuthTransaction,
    ) -> Result<Self, OAuthTransactionError> {
        let tx = repository
            .consume(&input)
            .await?
            .ok_or(OAuthTransactionError::InvalidTransaction)?;
        if tx.consumed_at.is_none() || !tx.matches(&input) {
            return Err(OAuthTransactionError::InvalidTransaction);
        }
        Ok(Self { transaction: tx })
    }
}

impl OAuthTransaction {
    /// Persist the authorization transaction before returning public PKCE data.
    ///
    /// # Errors
    /// Rejects invalid lifetimes/redirects or propagates a generic writer failure.
    pub async fn begin(
        repository: &dyn OAuthTransactionWriteRepository,
        input: BeginOAuthTransaction,
    ) -> Result<BegunOAuthTransaction, OAuthTransactionError> {
        let now = Utc::now().timestamp();
        if input.expires_at <= now
            || input.expires_at > now.saturating_add(600)
            || input.redirect_uri.is_empty()
        {
            return Err(OAuthTransactionError::InvalidTransaction);
        }
        let verifier = input.pkce_required.then(|| {
            let mut bytes = [0_u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut bytes);
            URL_SAFE_NO_PAD.encode(bytes)
        });
        let challenge = verifier
            .as_ref()
            .map(|v| URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes())));
        let transaction = Self {
            id: Uuid::new_v4(),
            nonce_hash: input.nonce_hash,
            state_hash: input.state_hash,
            browser_nonce_hash: input.browser_nonce_hash,
            provider: input.provider,
            operation: input.operation,
            redirect_uri: input.redirect_uri,
            expires_at: input.expires_at,
            consumed_at: None,
            pkce_verifier: verifier,
        };
        repository.create(&transaction).await?;
        Ok(BegunOAuthTransaction {
            code_challenge: challenge,
            code_challenge_method: input.pkce_required.then(|| "S256".to_string()),
        })
    }

    /// Defense against incorrect adapters: equality of hashes in application code
    /// is constant-time; the SQL writer checks these same bindings atomically.
    #[must_use]
    pub fn matches(&self, input: &ConsumeOAuthTransaction) -> bool {
        let hash_equal = constant_time_equal(&self.nonce_hash, &input.nonce_hash)
            & constant_time_equal(&self.state_hash, &input.state_hash)
            & constant_time_equal(&self.browser_nonce_hash, &input.browser_nonce_hash);
        hash_equal
            && self.provider == input.provider
            && self.operation == input.operation
            && self.redirect_uri == input.redirect_uri
            && self.expires_at == input.expires_at
            && self.expires_at > Utc::now().timestamp()
    }
}

#[must_use]
pub fn hash_oauth_bytes(value: &[u8]) -> [u8; 32] {
    Sha256::digest(value).into()
}

fn constant_time_equal(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b).fold(0_u8, |diff, (a, b)| diff | (a ^ b)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Writer(Mutex<Option<OAuthTransaction>>);
    #[async_trait]
    impl OAuthTransactionWriteRepository for Writer {
        async fn purge_expired(&self, batch_size: u32) -> Result<u64, OAuthTransactionError> {
            if !(1..=1000).contains(&batch_size) {
                return Err(OAuthTransactionError::InvalidTransaction);
            }
            let now = Utc::now().timestamp();
            let purged = {
                let mut slot = self.0.lock().unwrap();
                if slot.as_ref().is_some_and(|tx| tx.expires_at <= now) {
                    *slot = None;
                    true
                } else {
                    false
                }
            };
            Ok(u64::from(purged))
        }
        async fn create(&self, tx: &OAuthTransaction) -> Result<(), OAuthTransactionError> {
            let stored = OAuthTransaction {
                id: tx.id,
                nonce_hash: tx.nonce_hash,
                state_hash: tx.state_hash,
                browser_nonce_hash: tx.browser_nonce_hash,
                provider: tx.provider.clone(),
                operation: tx.operation.clone(),
                redirect_uri: tx.redirect_uri.clone(),
                expires_at: tx.expires_at,
                consumed_at: None,
                pkce_verifier: tx.pkce_verifier.clone(),
            };
            let mut slot = self.0.lock().unwrap();
            *slot = Some(stored);
            drop(slot);
            Ok(())
        }
        async fn consume(
            &self,
            input: &ConsumeOAuthTransaction,
        ) -> Result<Option<OAuthTransaction>, OAuthTransactionError> {
            let mut slot = self.0.lock().unwrap();
            if !slot.as_ref().is_some_and(|tx| tx.matches(input)) {
                drop(slot);
                return Ok(None);
            }
            let mut tx = slot.take().unwrap();
            drop(slot);
            tx.consumed_at = Some(Utc::now().timestamp());
            Ok(Some(tx))
        }
    }

    fn begin() -> BeginOAuthTransaction {
        BeginOAuthTransaction {
            nonce_hash: [1; 32],
            state_hash: [2; 32],
            browser_nonce_hash: [3; 32],
            provider: "github".parse().unwrap(),
            operation: OAuthOperation::Login,
            redirect_uri: "https://iam.example/callback".into(),
            expires_at: Utc::now().timestamp() + 300,
            pkce_required: true,
        }
    }
    fn consume(input: &BeginOAuthTransaction) -> ConsumeOAuthTransaction {
        ConsumeOAuthTransaction {
            nonce_hash: input.nonce_hash,
            state_hash: input.state_hash,
            browser_nonce_hash: input.browser_nonce_hash,
            provider: input.provider.clone(),
            operation: input.operation.clone(),
            redirect_uri: input.redirect_uri.clone(),
            expires_at: input.expires_at,
        }
    }
    #[tokio::test]
    async fn wrong_browser_provider_intention_redirect_expiry_denied_then_consume_once() {
        let writer = Writer::default();
        let input = begin();
        let callback = consume(&input);
        let started = OAuthTransaction::begin(&writer, input.clone())
            .await
            .unwrap();
        assert_eq!(started.code_challenge_method.as_deref(), Some("S256"));
        for mismatch in 0..5 {
            let mut bad = callback.clone();
            match mismatch {
                0 => bad.browser_nonce_hash = [4; 32],
                1 => bad.provider = "gitlab".parse().unwrap(),
                2 => {
                    bad.operation = OAuthOperation::Relink {
                        user_id: Uuid::new_v4(),
                    }
                }
                3 => bad.redirect_uri.push_str("/other"),
                _ => bad.expires_at -= 1,
            }
            assert!(ConsumedOAuthTransaction::consume(&writer, bad)
                .await
                .is_err());
        }
        let fresh = ConsumedOAuthTransaction::consume(&writer, callback.clone())
            .await
            .unwrap();
        let verifier = fresh.pkce_verifier().unwrap();
        assert_eq!(
            started.code_challenge.unwrap(),
            URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
        );
        assert!(!format!("{fresh:?}").contains(verifier));
        assert!(ConsumedOAuthTransaction::consume(&writer, callback)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn one_consumed_capability_and_expired_rows_never_authorize() {
        let writer = Writer::default();
        let input = begin();
        let callback = consume(&input);
        OAuthTransaction::begin(&writer, input.clone())
            .await
            .unwrap();
        let (first, second) = tokio::join!(
            ConsumedOAuthTransaction::consume(&writer, callback.clone()),
            ConsumedOAuthTransaction::consume(&writer, callback)
        );
        assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
        OAuthTransaction::begin(&writer, input.clone())
            .await
            .unwrap();
        let expired = Utc::now().timestamp() - 1;
        writer.0.lock().unwrap().as_mut().unwrap().expires_at = expired;
        let mut callback = consume(&input);
        callback.expires_at = expired;
        assert!(ConsumedOAuthTransaction::consume(&writer, callback)
            .await
            .is_err());
    }
}
