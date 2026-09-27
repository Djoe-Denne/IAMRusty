//! Identity entity — authentifiable principal within a trust domain (ADR-0305).
//!
//! `HumanAccount` remains the `users` table; do not rename it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::user::User;

/// UX person record (preferences, avatar, recovery). Persistence: table `users`.
///
/// Not an AuthZ principal — that is [`Identity`] `(issuer, subject)`.
pub type HumanAccount = User;

/// Kind of identity within a trust domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityKind {
    Platform,
    OrganizationManaged,
}

impl From<&IdentityKind> for String {
    fn from(kind: &IdentityKind) -> Self {
        match kind {
            IdentityKind::Platform => "platform".to_string(),
            IdentityKind::OrganizationManaged => "organization_managed".to_string(),
        }
    }
}

impl std::str::FromStr for IdentityKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "platform" => Ok(Self::Platform),
            "organization_managed" => Ok(Self::OrganizationManaged),
            other => Err(format!("unknown identity kind: {other}")),
        }
    }
}

/// Authentifiable identity: principal is `(issuer, subject)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub id: Uuid,
    /// FK to `users` (HumanAccount).
    pub user_id: Uuid,
    pub issuer: String,
    pub subject: String,
    pub kind: IdentityKind,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Identity {
    /// Build a platform identity whose subject is the user id string.
    #[must_use]
    pub fn platform(user_id: Uuid, issuer: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            issuer: issuer.into(),
            subject: user_id.to_string(),
            kind: IdentityKind::Platform,
            created_at: now,
            updated_at: now,
        }
    }

    /// Build an organization-managed identity with a fresh opaque subject (≠ `user_id`).
    #[must_use]
    pub fn organization_managed(user_id: Uuid, issuer: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            issuer: issuer.into(),
            subject: Uuid::new_v4().to_string(),
            kind: IdentityKind::OrganizationManaged,
            created_at: now,
            updated_at: now,
        }
    }
}
