//! Hive Domain Events
//! 
//! This crate contains all domain events for the Hive organization management service.
//! Events are used for inter-service communication, particularly with the Telegraph
//! notification service.

pub mod organization;
pub mod member;
pub mod invitation;
pub mod external_link;
pub mod sync;

pub use organization::*;
pub use member::*;
pub use invitation::*;
pub use external_link::*;
pub use sync::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

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
            HiveDomainEvent::OrganizationCreated(event) => event.base.event_type.as_str(),
            HiveDomainEvent::OrganizationUpdated(event) => event.base.event_type.as_str(),
            HiveDomainEvent::OrganizationDeleted(event) => event.base.event_type.as_str(),
            HiveDomainEvent::MemberInvited(event) => event.base.event_type.as_str(),
            HiveDomainEvent::MemberJoined(event) => event.base.event_type.as_str(),
            HiveDomainEvent::MemberRemoved(event) => event.base.event_type.as_str(),
            HiveDomainEvent::InvitationCreated(event) => event.base.event_type.as_str(),
            HiveDomainEvent::InvitationAccepted(event) => event.base.event_type.as_str(),
            HiveDomainEvent::InvitationExpired(event) => event.base.event_type.as_str(),
            HiveDomainEvent::ExternalLinkCreated(event) => event.base.event_type.as_str(),
            HiveDomainEvent::SyncJobStarted(event) => event.base.event_type.as_str(),
            HiveDomainEvent::SyncJobCompleted(event) => event.base.event_type.as_str(),
        }
    }

    fn event_id(&self) -> Uuid {
        match self {
            HiveDomainEvent::OrganizationCreated(event) => event.base.event_id,
            HiveDomainEvent::OrganizationUpdated(event) => event.base.event_id,
            HiveDomainEvent::OrganizationDeleted(event) => event.base.event_id,
            HiveDomainEvent::MemberInvited(event) => event.base.event_id,
            HiveDomainEvent::MemberJoined(event) => event.base.event_id,
            HiveDomainEvent::MemberRemoved(event) => event.base.event_id,
            HiveDomainEvent::InvitationCreated(event) => event.base.event_id,
            HiveDomainEvent::InvitationAccepted(event) => event.base.event_id,
            HiveDomainEvent::InvitationExpired(event) => event.base.event_id,
            HiveDomainEvent::ExternalLinkCreated(event) => event.base.event_id,
            HiveDomainEvent::SyncJobStarted(event) => event.base.event_id,
            HiveDomainEvent::SyncJobCompleted(event) => event.base.event_id,
        }
    }

    fn aggregate_id(&self) -> Uuid {
        match self {
            HiveDomainEvent::OrganizationCreated(event) => event.base.aggregate_id,
            HiveDomainEvent::OrganizationUpdated(event) => event.base.aggregate_id,
            HiveDomainEvent::OrganizationDeleted(event) => event.base.aggregate_id,
            HiveDomainEvent::MemberInvited(event) => event.base.aggregate_id,
            HiveDomainEvent::MemberJoined(event) => event.base.aggregate_id,
            HiveDomainEvent::MemberRemoved(event) => event.base.aggregate_id,
            HiveDomainEvent::InvitationCreated(event) => event.base.aggregate_id,
            HiveDomainEvent::InvitationAccepted(event) => event.base.aggregate_id,
            HiveDomainEvent::InvitationExpired(event) => event.base.aggregate_id,
            HiveDomainEvent::ExternalLinkCreated(event) => event.base.aggregate_id,
            HiveDomainEvent::SyncJobStarted(event) => event.base.aggregate_id,
            HiveDomainEvent::SyncJobCompleted(event) => event.base.aggregate_id,
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            HiveDomainEvent::OrganizationCreated(event) => event.base.occurred_at,
            HiveDomainEvent::OrganizationUpdated(event) => event.base.occurred_at,
            HiveDomainEvent::OrganizationDeleted(event) => event.base.occurred_at,
            HiveDomainEvent::MemberInvited(event) => event.base.occurred_at,
            HiveDomainEvent::MemberJoined(event) => event.base.occurred_at,
            HiveDomainEvent::MemberRemoved(event) => event.base.occurred_at,
            HiveDomainEvent::InvitationCreated(event) => event.base.occurred_at,
            HiveDomainEvent::InvitationAccepted(event) => event.base.occurred_at,
            HiveDomainEvent::InvitationExpired(event) => event.base.occurred_at,
            HiveDomainEvent::ExternalLinkCreated(event) => event.base.occurred_at,
            HiveDomainEvent::SyncJobStarted(event) => event.base.occurred_at,
            HiveDomainEvent::SyncJobCompleted(event) => event.base.occurred_at,
        }
    }

    fn version(&self) -> u32 {
        match self {
            HiveDomainEvent::OrganizationCreated(event) => event.base.version,
            HiveDomainEvent::OrganizationUpdated(event) => event.base.version,
            HiveDomainEvent::OrganizationDeleted(event) => event.base.version,
            HiveDomainEvent::MemberInvited(event) => event.base.version,
            HiveDomainEvent::MemberJoined(event) => event.base.version,
            HiveDomainEvent::MemberRemoved(event) => event.base.version,
            HiveDomainEvent::InvitationCreated(event) => event.base.version,
            HiveDomainEvent::InvitationAccepted(event) => event.base.version,
            HiveDomainEvent::InvitationExpired(event) => event.base.version,
            HiveDomainEvent::ExternalLinkCreated(event) => event.base.version,
            HiveDomainEvent::SyncJobStarted(event) => event.base.version,
            HiveDomainEvent::SyncJobCompleted(event) => event.base.version,
        }
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(&format!("Failed to serialize event: {}", e)))
    }

    fn metadata(&self) -> HashMap<String, String> {
        match self {
            HiveDomainEvent::OrganizationCreated(event) => event.base.metadata.clone(),
            HiveDomainEvent::OrganizationUpdated(event) => event.base.metadata.clone(),
            HiveDomainEvent::OrganizationDeleted(event) => event.base.metadata.clone(),
            HiveDomainEvent::MemberInvited(event) => event.base.metadata.clone(),
            HiveDomainEvent::MemberJoined(event) => event.base.metadata.clone(),
            HiveDomainEvent::MemberRemoved(event) => event.base.metadata.clone(),
            HiveDomainEvent::InvitationCreated(event) => event.base.metadata.clone(),
            HiveDomainEvent::InvitationAccepted(event) => event.base.metadata.clone(),
            HiveDomainEvent::InvitationExpired(event) => event.base.metadata.clone(),
            HiveDomainEvent::ExternalLinkCreated(event) => event.base.metadata.clone(),
            HiveDomainEvent::SyncJobStarted(event) => event.base.metadata.clone(),
            HiveDomainEvent::SyncJobCompleted(event) => event.base.metadata.clone(),
        }
    }
}

impl From<HiveDomainEvent> for Box<dyn DomainEvent + 'static> {
    fn from(event: HiveDomainEvent) -> Self {
        Box::new(event)
    }
}