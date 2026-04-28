//! Manifesto Domain Events
//!
//! This crate contains all domain events for the Manifesto project management service.
//! Events are used for inter-service communication, particularly with the Telegraph
//! notification service.

pub mod component;
pub mod member;
pub mod project;

pub use component::*;
pub use member::*;
pub use project::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use rustycog_core::error::ServiceError;
use rustycog_events::DomainEvent;

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
    #[serde(rename = "project_archived")]
    ProjectArchived(ProjectArchivedEvent),

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

impl DomainEvent for ManifestoDomainEvent {
    fn event_type(&self) -> &str {
        match self {
            Self::ProjectCreated(event) => event.base.event_type.as_str(),
            Self::ProjectUpdated(event) => event.base.event_type.as_str(),
            Self::ProjectDeleted(event) => event.base.event_type.as_str(),
            Self::ProjectPublished(event) => event.base.event_type.as_str(),
            Self::ProjectArchived(event) => event.base.event_type.as_str(),
            Self::ComponentAdded(event) => event.base.event_type.as_str(),
            Self::ComponentStatusChanged(event) => event.base.event_type.as_str(),
            Self::ComponentRemoved(event) => event.base.event_type.as_str(),
            Self::MemberAdded(event) => event.base.event_type.as_str(),
            Self::MemberPermissionsUpdated(event) => event.base.event_type.as_str(),
            Self::MemberRemoved(event) => event.base.event_type.as_str(),
            Self::PermissionGranted(event) => event.base.event_type.as_str(),
            Self::PermissionRevoked(event) => event.base.event_type.as_str(),
        }
    }

    fn event_id(&self) -> Uuid {
        match self {
            Self::ProjectCreated(event) => event.base.event_id,
            Self::ProjectUpdated(event) => event.base.event_id,
            Self::ProjectDeleted(event) => event.base.event_id,
            Self::ProjectPublished(event) => event.base.event_id,
            Self::ProjectArchived(event) => event.base.event_id,
            Self::ComponentAdded(event) => event.base.event_id,
            Self::ComponentStatusChanged(event) => event.base.event_id,
            Self::ComponentRemoved(event) => event.base.event_id,
            Self::MemberAdded(event) => event.base.event_id,
            Self::MemberPermissionsUpdated(event) => event.base.event_id,
            Self::MemberRemoved(event) => event.base.event_id,
            Self::PermissionGranted(event) => event.base.event_id,
            Self::PermissionRevoked(event) => event.base.event_id,
        }
    }

    fn aggregate_id(&self) -> Uuid {
        match self {
            Self::ProjectCreated(event) => event.base.aggregate_id,
            Self::ProjectUpdated(event) => event.base.aggregate_id,
            Self::ProjectDeleted(event) => event.base.aggregate_id,
            Self::ProjectPublished(event) => event.base.aggregate_id,
            Self::ProjectArchived(event) => event.base.aggregate_id,
            Self::ComponentAdded(event) => event.base.aggregate_id,
            Self::ComponentStatusChanged(event) => event.base.aggregate_id,
            Self::ComponentRemoved(event) => event.base.aggregate_id,
            Self::MemberAdded(event) => event.base.aggregate_id,
            Self::MemberPermissionsUpdated(event) => event.base.aggregate_id,
            Self::MemberRemoved(event) => event.base.aggregate_id,
            Self::PermissionGranted(event) => event.base.aggregate_id,
            Self::PermissionRevoked(event) => event.base.aggregate_id,
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::ProjectCreated(event) => event.base.occurred_at,
            Self::ProjectUpdated(event) => event.base.occurred_at,
            Self::ProjectDeleted(event) => event.base.occurred_at,
            Self::ProjectPublished(event) => event.base.occurred_at,
            Self::ProjectArchived(event) => event.base.occurred_at,
            Self::ComponentAdded(event) => event.base.occurred_at,
            Self::ComponentStatusChanged(event) => event.base.occurred_at,
            Self::ComponentRemoved(event) => event.base.occurred_at,
            Self::MemberAdded(event) => event.base.occurred_at,
            Self::MemberPermissionsUpdated(event) => event.base.occurred_at,
            Self::MemberRemoved(event) => event.base.occurred_at,
            Self::PermissionGranted(event) => event.base.occurred_at,
            Self::PermissionRevoked(event) => event.base.occurred_at,
        }
    }

    fn version(&self) -> u32 {
        match self {
            Self::ProjectCreated(event) => event.base.version,
            Self::ProjectUpdated(event) => event.base.version,
            Self::ProjectDeleted(event) => event.base.version,
            Self::ProjectPublished(event) => event.base.version,
            Self::ProjectArchived(event) => event.base.version,
            Self::ComponentAdded(event) => event.base.version,
            Self::ComponentStatusChanged(event) => event.base.version,
            Self::ComponentRemoved(event) => event.base.version,
            Self::MemberAdded(event) => event.base.version,
            Self::MemberPermissionsUpdated(event) => event.base.version,
            Self::MemberRemoved(event) => event.base.version,
            Self::PermissionGranted(event) => event.base.version,
            Self::PermissionRevoked(event) => event.base.version,
        }
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(format!("Failed to serialize event: {e}")))
    }

    fn metadata(&self) -> HashMap<String, String> {
        match self {
            Self::ProjectCreated(event) => event.base.metadata.clone(),
            Self::ProjectUpdated(event) => event.base.metadata.clone(),
            Self::ProjectDeleted(event) => event.base.metadata.clone(),
            Self::ProjectPublished(event) => event.base.metadata.clone(),
            Self::ProjectArchived(event) => event.base.metadata.clone(),
            Self::ComponentAdded(event) => event.base.metadata.clone(),
            Self::ComponentStatusChanged(event) => event.base.metadata.clone(),
            Self::ComponentRemoved(event) => event.base.metadata.clone(),
            Self::MemberAdded(event) => event.base.metadata.clone(),
            Self::MemberPermissionsUpdated(event) => event.base.metadata.clone(),
            Self::MemberRemoved(event) => event.base.metadata.clone(),
            Self::PermissionGranted(event) => event.base.metadata.clone(),
            Self::PermissionRevoked(event) => event.base.metadata.clone(),
        }
    }
}

impl From<ManifestoDomainEvent> for Box<dyn DomainEvent + 'static> {
    fn from(event: ManifestoDomainEvent) -> Self {
        Box::new(event)
    }
}
