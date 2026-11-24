use std::sync::Arc;

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use serde::Serialize;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

#[derive(Serialize)]
pub struct SavedAddress {
    pub id: Uuid,
    pub email: String,
    pub is_primary: bool,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ProfileVerificationError {
    NotEligible,
    Cooldown,
    Unavailable,
    Internal,
}

pub type VerificationResult<T> = Result<T, ProfileVerificationError>;

/// All methods use the primary database. Issuance includes the outbox insert
/// and confirmation checks and consumes the current capability atomically.
#[async_trait]
pub trait ProfileEmailVerificationRepository: Send + Sync {
    async fn list(&self, user_id: Uuid) -> VerificationResult<Vec<SavedAddress>>;
    async fn issue(
        &self,
        user_id: Uuid,
        address_id: Uuid,
        digest: [u8; 32],
        verification_url: String,
    ) -> VerificationResult<()>;
    async fn confirm(&self, user_id: Uuid, digest: [u8; 32]) -> VerificationResult<()>;
}

pub struct ProfileEmailVerification {
    repository: Arc<dyn ProfileEmailVerificationRepository>,
    landing_url: Option<Url>,
}

impl ProfileEmailVerification {
    /// Validate the deployment-selected destination without any network access.
    ///
    /// # Errors
    ///
    /// Returns a constant error for a non-HTTPS or otherwise invalid landing URL.
    pub fn new(
        repository: Arc<dyn ProfileEmailVerificationRepository>,
        landing_url: Option<&str>,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            repository,
            landing_url: landing_url.map(parse_landing).transpose()?,
        })
    }

    /// List only the signed-in account's current saved addresses.
    ///
    /// # Errors
    ///
    /// Returns an internal error if the writer cannot read the addresses.
    pub async fn list(&self, user_id: Uuid) -> VerificationResult<Vec<SavedAddress>> {
        self.repository.list(user_id).await
    }

    /// Issue, or explicitly replace, a capability for a saved address.
    ///
    /// # Errors
    ///
    /// Fails if issuance is disabled, the address is ineligible, the cooldown
    /// has not elapsed, or the writer cannot commit an intent.
    pub async fn issue(&self, user_id: Uuid, address_id: Uuid) -> VerificationResult<()> {
        let mut url = self
            .landing_url
            .clone()
            .ok_or(ProfileVerificationError::Unavailable)?;
        let mut random = [0_u8; 32];
        OsRng
            .try_fill_bytes(&mut random)
            .map_err(|_| ProfileVerificationError::Internal)?;
        let token = URL_SAFE_NO_PAD.encode(random);
        url.set_fragment(Some(&token));
        self.repository
            .issue(user_id, address_id, digest(&token), url.to_string())
            .await
    }

    /// Confirm only through the explicit authenticated action.
    ///
    /// # Errors
    ///
    /// Returns the same ineligible error for malformed, unknown, foreign,
    /// stale, expired and used capabilities, or an internal writer error.
    pub async fn confirm(&self, user_id: Uuid, token: &str) -> VerificationResult<()> {
        if token.len() != 43
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(ProfileVerificationError::NotEligible);
        }
        self.repository.confirm(user_id, digest(token)).await
    }
}

fn digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

fn parse_landing(raw: &str) -> Result<Url, &'static str> {
    const INVALID: &str = "Invalid profile email verification landing URL";
    // Require an explicit authority and reject normalization of backslashes,
    // whitespace or empty userinfo before the WHATWG parser can discard them.
    let authority = raw
        .strip_prefix("https://")
        .ok_or(INVALID)?
        .split('/')
        .next()
        .unwrap_or_default();
    let url = Url::parse(raw).map_err(|_| INVALID)?;
    if authority.is_empty()
        || authority.contains('@')
        || raw.contains('\\')
        || raw.chars().any(char::is_whitespace)
        || url.scheme() != "https"
        || !url.has_host()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(INVALID);
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::parse_landing;

    #[test]
    fn profile_landing_requires_an_explicit_clean_https_destination() {
        assert!(parse_landing("https://profile.example.invalid/confirm").is_ok());
        for url in [
            "http://profile.example.invalid/confirm",
            "//profile.example.invalid/confirm",
            "https:///profile.example.invalid/confirm",
            "https://user:pass@profile.example.invalid/confirm",
            "https://@profile.example.invalid/confirm",
            "https://profile.example.invalid/confirm?next=x",
            "https://profile.example.invalid/confirm#fragment",
            " https://profile.example.invalid/confirm",
            "https://profile.example.invalid\\confirm",
        ] {
            assert!(parse_landing(url).is_err(), "invalid landing was accepted");
        }
    }
}
