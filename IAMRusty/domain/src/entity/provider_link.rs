use super::provider::Provider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a link between a user and an OAuth provider
///
/// This captures the relationship between a user in our system and their
/// account on a specific OAuth provider (GitHub, GitLab, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderLink {
    /// The user ID in our system
    pub user_id: Uuid,

    /// The OAuth provider (GitHub, GitLab, etc.)
    pub provider: Provider,

    /// The user ID from the OAuth provider (e.g., GitHub user ID)
    pub provider_user_id: String,

    /// When this provider was first linked
    pub linked_at: DateTime<Utc>,
}

impl ProviderLink {
    /// Creates a new provider link
    pub fn new(user_id: Uuid, provider: Provider, provider_user_id: String) -> Self {
        Self {
            user_id,
            provider,
            provider_user_id,
            linked_at: Utc::now(),
        }
    }
}
