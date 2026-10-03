//! Manifesto Domain Events
//!
//! This crate contains all domain events for the Manifesto project management service.
//! Events are used for inter-service communication, particularly with the Telegraph
//! notification service.

pub mod authz;
pub mod component;
pub mod member;
pub mod project;

pub use authz::*;
pub use component::*;
pub use member::*;
pub use project::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use rustycog::core::error::ServiceError;
use rustycog::events::DomainEvent;

/// Main enum containing all Manifesto domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum ManifestoDomainEvent {
    // Project events
    #[serde(rename = "project_created")]
    ProjectCreated(ProjectCreatedEvent),
    #[serde(rename = "project_updated")]
    ProjectUpdated(ProjectUpdatedEvent),
    #[serde(rename = "project_deleted")]
    ProjectDeleted(ProjectDeletedEvent),
    #[serde(rename = "project_published")]
    ProjectPublished(ProjectPublishedEvent),
    #[serde(rename = "project_visibility_changed")]
    ProjectVisibilityChanged(ProjectVisibilityChangedEvent),
    #[serde(rename = "project_archived")]
    ProjectArchived(ProjectArchivedEvent),
    #[serde(rename = "project_suspended")]
    ProjectSuspended(ProjectSuspendedEvent),
    #[serde(rename = "project_resumed")]
    ProjectResumed(ProjectResumedEvent),
    #[serde(rename = "project_ownership_transferred")]
    ProjectOwnershipTransferred(ProjectOwnershipTransferredEvent),

    // Component events
    #[serde(rename = "component_added")]
    ComponentAdded(ComponentAddedEvent),
    #[serde(rename = "component_status_changed")]
    ComponentStatusChanged(ComponentStatusChangedEvent),
    #[serde(rename = "component_removed")]
    ComponentRemoved(ComponentRemovedEvent),

    // Member events
    #[serde(rename = "member_added")]
    MemberAdded(MemberAddedEvent),
    #[serde(rename = "member_permissions_updated")]
    MemberPermissionsUpdated(MemberPermissionsUpdatedEvent),
    #[serde(rename = "member_removed")]
    MemberRemoved(MemberRemovedEvent),

    // Permission events
    #[serde(rename = "permission_granted")]
    PermissionGranted(PermissionGrantedEvent),
    #[serde(rename = "permission_revoked")]
    PermissionRevoked(PermissionRevokedEvent),
}

impl ManifestoDomainEvent {
    fn base(&self) -> &rustycog::events::BaseEvent {
        match self {
            Self::ProjectCreated(event) => &event.base,
            Self::ProjectUpdated(event) => &event.base,
            Self::ProjectDeleted(event) => &event.base,
            Self::ProjectPublished(event) => &event.base,
            Self::ProjectVisibilityChanged(event) => &event.base,
            Self::ProjectArchived(event) => &event.base,
            Self::ProjectSuspended(event) => &event.base,
            Self::ProjectResumed(event) => &event.base,
            Self::ProjectOwnershipTransferred(event) => &event.base,
            Self::ComponentAdded(event) => &event.base,
            Self::ComponentStatusChanged(event) => &event.base,
            Self::ComponentRemoved(event) => &event.base,
            Self::MemberAdded(event) => &event.base,
            Self::MemberPermissionsUpdated(event) => &event.base,
            Self::MemberRemoved(event) => &event.base,
            Self::PermissionGranted(event) => &event.base,
            Self::PermissionRevoked(event) => &event.base,
        }
    }
}

impl DomainEvent for ManifestoDomainEvent {
    fn event_type(&self) -> &str {
        self.base().event_type.as_str()
    }

    fn event_id(&self) -> Uuid {
        self.base().event_id
    }

    fn aggregate_id(&self) -> Uuid {
        self.base().aggregate_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.base().occurred_at
    }

    fn version(&self) -> u32 {
        self.base().version
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.base().metadata.clone()
    }
}

impl From<ManifestoDomainEvent> for Box<dyn DomainEvent + 'static> {
    fn from(event: ManifestoDomainEvent) -> Self {
        Box::new(event)
    }
}
