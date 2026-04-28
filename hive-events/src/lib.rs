//! Hive Domain Events
//!
//! This crate contains all domain events for the Hive organization management service.
//! Events are used for inter-service communication, particularly with the Telegraph
//! notification service.

pub mod external_link;
pub mod invitation;
pub mod member;
pub mod organization;
pub mod role;
pub mod sync;

pub use external_link::*;
pub use invitation::*;
pub use member::*;
pub use organization::*;
pub use role::*;
pub use sync::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use rustycog_core::error::ServiceError;
use rustycog_events::DomainEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum HiveDomainEvent {
    #[serde(rename = "organization_created")]
    OrganizationCreated(OrganizationCreatedEvent),
    #[serde(rename = "organization_updated")]
    OrganizationUpdated(OrganizationUpdatedEvent),
    #[serde(rename = "organization_deleted")]
    OrganizationDeleted(OrganizationDeletedEvent),
    #[serde(rename = "member_invited")]
    MemberInvited(MemberInvitedEvent),
    #[serde(rename = "member_joined")]
    MemberJoined(MemberJoinedEvent),
    #[serde(rename = "member_removed")]
    MemberRemoved(MemberRemovedEvent),
    #[serde(rename = "invitation_created")]
    InvitationCreated(InvitationCreatedEvent),
    #[serde(rename = "invitation_accepted")]
    InvitationAccepted(InvitationAcceptedEvent),
    #[serde(rename = "invitation_expired")]
    InvitationExpired(InvitationExpiredEvent),
    #[serde(rename = "external_link_created")]
    ExternalLinkCreated(ExternalLinkCreatedEvent),
    #[serde(rename = "sync_job_started")]
    SyncJobStarted(SyncJobStartedEvent),
    #[serde(rename = "sync_job_completed")]
    SyncJobCompleted(SyncJobCompletedEvent),
}

impl DomainEvent for HiveDomainEvent {
    fn event_type(&self) -> &str {
        match self {
            Self::OrganizationCreated(event) => event.base.event_type.as_str(),
            Self::OrganizationUpdated(event) => event.base.event_type.as_str(),
            Self::OrganizationDeleted(event) => event.base.event_type.as_str(),
            Self::MemberInvited(event) => event.base.event_type.as_str(),
            Self::MemberJoined(event) => event.base.event_type.as_str(),
            Self::MemberRemoved(event) => event.base.event_type.as_str(),
            Self::InvitationCreated(event) => event.base.event_type.as_str(),
            Self::InvitationAccepted(event) => event.base.event_type.as_str(),
            Self::InvitationExpired(event) => event.base.event_type.as_str(),
            Self::ExternalLinkCreated(event) => event.base.event_type.as_str(),
            Self::SyncJobStarted(event) => event.base.event_type.as_str(),
            Self::SyncJobCompleted(event) => event.base.event_type.as_str(),
        }
    }

    fn event_id(&self) -> Uuid {
        match self {
            Self::OrganizationCreated(event) => event.base.event_id,
            Self::OrganizationUpdated(event) => event.base.event_id,
            Self::OrganizationDeleted(event) => event.base.event_id,
            Self::MemberInvited(event) => event.base.event_id,
            Self::MemberJoined(event) => event.base.event_id,
            Self::MemberRemoved(event) => event.base.event_id,
            Self::InvitationCreated(event) => event.base.event_id,
            Self::InvitationAccepted(event) => event.base.event_id,
            Self::InvitationExpired(event) => event.base.event_id,
            Self::ExternalLinkCreated(event) => event.base.event_id,
            Self::SyncJobStarted(event) => event.base.event_id,
            Self::SyncJobCompleted(event) => event.base.event_id,
        }
    }

    fn aggregate_id(&self) -> Uuid {
        match self {
            Self::OrganizationCreated(event) => event.base.aggregate_id,
            Self::OrganizationUpdated(event) => event.base.aggregate_id,
            Self::OrganizationDeleted(event) => event.base.aggregate_id,
            Self::MemberInvited(event) => event.base.aggregate_id,
            Self::MemberJoined(event) => event.base.aggregate_id,
            Self::MemberRemoved(event) => event.base.aggregate_id,
            Self::InvitationCreated(event) => event.base.aggregate_id,
            Self::InvitationAccepted(event) => event.base.aggregate_id,
            Self::InvitationExpired(event) => event.base.aggregate_id,
            Self::ExternalLinkCreated(event) => event.base.aggregate_id,
            Self::SyncJobStarted(event) => event.base.aggregate_id,
            Self::SyncJobCompleted(event) => event.base.aggregate_id,
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::OrganizationCreated(event) => event.base.occurred_at,
            Self::OrganizationUpdated(event) => event.base.occurred_at,
            Self::OrganizationDeleted(event) => event.base.occurred_at,
            Self::MemberInvited(event) => event.base.occurred_at,
            Self::MemberJoined(event) => event.base.occurred_at,
            Self::MemberRemoved(event) => event.base.occurred_at,
            Self::InvitationCreated(event) => event.base.occurred_at,
            Self::InvitationAccepted(event) => event.base.occurred_at,
            Self::InvitationExpired(event) => event.base.occurred_at,
            Self::ExternalLinkCreated(event) => event.base.occurred_at,
            Self::SyncJobStarted(event) => event.base.occurred_at,
            Self::SyncJobCompleted(event) => event.base.occurred_at,
        }
    }

    fn version(&self) -> u32 {
        match self {
            Self::OrganizationCreated(event) => event.base.version,
            Self::OrganizationUpdated(event) => event.base.version,
            Self::OrganizationDeleted(event) => event.base.version,
            Self::MemberInvited(event) => event.base.version,
            Self::MemberJoined(event) => event.base.version,
            Self::MemberRemoved(event) => event.base.version,
            Self::InvitationCreated(event) => event.base.version,
            Self::InvitationAccepted(event) => event.base.version,
            Self::InvitationExpired(event) => event.base.version,
            Self::ExternalLinkCreated(event) => event.base.version,
            Self::SyncJobStarted(event) => event.base.version,
            Self::SyncJobCompleted(event) => event.base.version,
        }
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        match self {
            Self::OrganizationCreated(event) => event.base.metadata.clone(),
            Self::OrganizationUpdated(event) => event.base.metadata.clone(),
            Self::OrganizationDeleted(event) => event.base.metadata.clone(),
            Self::MemberInvited(event) => event.base.metadata.clone(),
            Self::MemberJoined(event) => event.base.metadata.clone(),
            Self::MemberRemoved(event) => event.base.metadata.clone(),
            Self::InvitationCreated(event) => event.base.metadata.clone(),
            Self::InvitationAccepted(event) => event.base.metadata.clone(),
            Self::InvitationExpired(event) => event.base.metadata.clone(),
            Self::ExternalLinkCreated(event) => event.base.metadata.clone(),
            Self::SyncJobStarted(event) => event.base.metadata.clone(),
            Self::SyncJobCompleted(event) => event.base.metadata.clone(),
        }
    }
}

impl From<HiveDomainEvent> for Box<dyn DomainEvent + 'static> {
    fn from(event: HiveDomainEvent) -> Self {
        Box::new(event)
    }
}
