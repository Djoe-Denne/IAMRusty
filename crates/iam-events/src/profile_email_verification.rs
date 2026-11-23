use std::collections::HashMap;

use rustycog::core::error::ServiceError;
use rustycog::events::event::{BaseEvent, DomainEvent};
use serde::Serialize;
use uuid::Uuid;

pub const PROFILE_EMAIL_VERIFICATION_REQUESTED: &str = "profile_email_verification_requested";

/// Private delivery intent for proof of a saved profile address (v1).
#[derive(Serialize)]
pub struct ProfileEmailVerificationRequested {
    #[serde(flatten)]
    base: BaseEvent,
    user_id: Uuid,
    email: String,
    verification_url: String,
}

impl ProfileEmailVerificationRequested {
    #[must_use]
    pub fn new(user_id: Uuid, email: String, verification_url: String) -> Self {
        Self {
            base: BaseEvent::new(PROFILE_EMAIL_VERIFICATION_REQUESTED.to_owned(), user_id),
            user_id,
            email,
            verification_url,
        }
    }
}

impl std::fmt::Debug for ProfileEmailVerificationRequested {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileEmailVerificationRequested")
            .field("event_id", &self.base.event_id)
            .finish_non_exhaustive()
    }
}

impl DomainEvent for ProfileEmailVerificationRequested {
    fn event_type(&self) -> &str {
        PROFILE_EMAIL_VERIFICATION_REQUESTED
    }

    fn event_id(&self) -> Uuid {
        self.base.event_id
    }

    fn aggregate_id(&self) -> Uuid {
        self.base.aggregate_id
    }

    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.base.occurred_at
    }

    fn version(&self) -> u32 {
        self.base.version
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|_| ServiceError::internal("Unable to serialize profile verification intent"))
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.base.metadata.clone()
    }
}
