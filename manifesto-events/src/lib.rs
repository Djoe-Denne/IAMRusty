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
            ManifestoDomainEvent::ProjectCreated(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::ProjectUpdated(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::ProjectDeleted(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::ProjectPublished(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::ProjectArchived(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::ComponentAdded(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::ComponentStatusChanged(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::ComponentRemoved(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::MemberAdded(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::MemberPermissionsUpdated(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::MemberRemoved(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::PermissionGranted(event) => event.base.event_type.as_str(),
            ManifestoDomainEvent::PermissionRevoked(event) => event.base.event_type.as_str(),
        }
    }

    fn event_id(&self) -> Uuid {
        match self {
            ManifestoDomainEvent::ProjectCreated(event) => event.base.event_id,
            ManifestoDomainEvent::ProjectUpdated(event) => event.base.event_id,
            ManifestoDomainEvent::ProjectDeleted(event) => event.base.event_id,
            ManifestoDomainEvent::ProjectPublished(event) => event.base.event_id,
            ManifestoDomainEvent::ProjectArchived(event) => event.base.event_id,
            ManifestoDomainEvent::ComponentAdded(event) => event.base.event_id,
            ManifestoDomainEvent::ComponentStatusChanged(event) => event.base.event_id,
            ManifestoDomainEvent::ComponentRemoved(event) => event.base.event_id,
            ManifestoDomainEvent::MemberAdded(event) => event.base.event_id,
            ManifestoDomainEvent::MemberPermissionsUpdated(event) => event.base.event_id,
            ManifestoDomainEvent::MemberRemoved(event) => event.base.event_id,
            ManifestoDomainEvent::PermissionGranted(event) => event.base.event_id,
            ManifestoDomainEvent::PermissionRevoked(event) => event.base.event_id,
        }
    }

    fn aggregate_id(&self) -> Uuid {
        match self {
            ManifestoDomainEvent::ProjectCreated(event) => event.base.aggregate_id,
            ManifestoDomainEvent::ProjectUpdated(event) => event.base.aggregate_id,
            ManifestoDomainEvent::ProjectDeleted(event) => event.base.aggregate_id,
            ManifestoDomainEvent::ProjectPublished(event) => event.base.aggregate_id,
            ManifestoDomainEvent::ProjectArchived(event) => event.base.aggregate_id,
            ManifestoDomainEvent::ComponentAdded(event) => event.base.aggregate_id,
            ManifestoDomainEvent::ComponentStatusChanged(event) => event.base.aggregate_id,
            ManifestoDomainEvent::ComponentRemoved(event) => event.base.aggregate_id,
            ManifestoDomainEvent::MemberAdded(event) => event.base.aggregate_id,
            ManifestoDomainEvent::MemberPermissionsUpdated(event) => event.base.aggregate_id,
            ManifestoDomainEvent::MemberRemoved(event) => event.base.aggregate_id,
            ManifestoDomainEvent::PermissionGranted(event) => event.base.aggregate_id,
            ManifestoDomainEvent::PermissionRevoked(event) => event.base.aggregate_id,
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            ManifestoDomainEvent::ProjectCreated(event) => event.base.occurred_at,
            ManifestoDomainEvent::ProjectUpdated(event) => event.base.occurred_at,
            ManifestoDomainEvent::ProjectDeleted(event) => event.base.occurred_at,
            ManifestoDomainEvent::ProjectPublished(event) => event.base.occurred_at,
            ManifestoDomainEvent::ProjectArchived(event) => event.base.occurred_at,
            ManifestoDomainEvent::ComponentAdded(event) => event.base.occurred_at,
            ManifestoDomainEvent::ComponentStatusChanged(event) => event.base.occurred_at,
            ManifestoDomainEvent::ComponentRemoved(event) => event.base.occurred_at,
            ManifestoDomainEvent::MemberAdded(event) => event.base.occurred_at,
            ManifestoDomainEvent::MemberPermissionsUpdated(event) => event.base.occurred_at,
            ManifestoDomainEvent::MemberRemoved(event) => event.base.occurred_at,
            ManifestoDomainEvent::PermissionGranted(event) => event.base.occurred_at,
            ManifestoDomainEvent::PermissionRevoked(event) => event.base.occurred_at,
        }
    }

    fn version(&self) -> u32 {
        match self {
            ManifestoDomainEvent::ProjectCreated(event) => event.base.version,
            ManifestoDomainEvent::ProjectUpdated(event) => event.base.version,
            ManifestoDomainEvent::ProjectDeleted(event) => event.base.version,
            ManifestoDomainEvent::ProjectPublished(event) => event.base.version,
            ManifestoDomainEvent::ProjectArchived(event) => event.base.version,
            ManifestoDomainEvent::ComponentAdded(event) => event.base.version,
            ManifestoDomainEvent::ComponentStatusChanged(event) => event.base.version,
            ManifestoDomainEvent::ComponentRemoved(event) => event.base.version,
            ManifestoDomainEvent::MemberAdded(event) => event.base.version,
            ManifestoDomainEvent::MemberPermissionsUpdated(event) => event.base.version,
            ManifestoDomainEvent::MemberRemoved(event) => event.base.version,
            ManifestoDomainEvent::PermissionGranted(event) => event.base.version,
            ManifestoDomainEvent::PermissionRevoked(event) => event.base.version,
        }
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        serde_json::to_string(self)
            .map_err(|e| ServiceError::internal(&format!("Failed to serialize event: {}", e)))
    }

    fn metadata(&self) -> HashMap<String, String> {
        match self {
            ManifestoDomainEvent::ProjectCreated(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::ProjectUpdated(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::ProjectDeleted(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::ProjectPublished(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::ProjectArchived(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::ComponentAdded(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::ComponentStatusChanged(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::ComponentRemoved(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::MemberAdded(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::MemberPermissionsUpdated(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::MemberRemoved(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::PermissionGranted(event) => event.base.metadata.clone(),
            ManifestoDomainEvent::PermissionRevoked(event) => event.base.metadata.clone(),
        }
    }
}

impl From<ManifestoDomainEvent> for Box<dyn DomainEvent + 'static> {
    fn from(event: ManifestoDomainEvent) -> Self {
        Box::new(event)
    }
}
