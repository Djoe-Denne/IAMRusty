//! Writer authority for initial authenticator enrollment and password sessions.

use crate::entity::token::RefreshToken;
use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

/// Provisioning material is intentionally not `Debug` or a persisted event.
pub struct TotpEnrollment {
    pub id: Uuid,
    pub otpauth_uri: String,
}

/// Internal proof, revalidated against the writer's current state at issuance.
pub enum PasswordFactorProof {
    NotEnrolled,
    Verified { enrollment_id: Uuid, step: i64 },
}

/// Redacted outcomes: never attach passwords, app codes, seeds, or storage errors.
#[derive(Debug, Error)]
pub enum TotpError {
    #[error("An authenticator code is required")]
    Required,
    #[error("Invalid credentials")]
    InvalidProof,
    #[error("Enrollment not found")]
    NotFound,
    #[error("Account is not eligible for initial enrollment")]
    Conflict,
    #[error("Too many authenticator attempts")]
    Throttled,
    #[error("Authenticator service unavailable")]
    Unavailable,
}

#[async_trait]
pub trait TotpStore: Send + Sync {
    async fn begin(&self, user_id: Uuid) -> Result<TotpEnrollment, TotpError>;

    async fn confirm(
        &self,
        user_id: Uuid,
        enrollment_id: Uuid,
        expected_password_hash: &str,
        code: &str,
    ) -> Result<(), TotpError>;

    async fn cancel(&self, user_id: Uuid, enrollment_id: Uuid) -> Result<(), TotpError>;

    async fn verify_login(
        &self,
        user_id: Uuid,
        expected_password_hash: &str,
        code: Option<&str>,
    ) -> Result<PasswordFactorProof, TotpError>;

    /// Check the password snapshot, consume the step and persist the refresh row atomically.
    async fn issue_password_session(
        &self,
        token: &RefreshToken,
        expected_password_hash: &str,
        proof: PasswordFactorProof,
    ) -> Result<(), TotpError>;
}
