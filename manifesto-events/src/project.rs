//! Project domain events for Manifesto service

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rustycog::events::BaseEvent;

// =============================================================================
// Project Events
// =============================================================================

/// Event published when a new project is created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCreatedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub project_name: String,
    pub owner_type: String,
    pub owner_id: Uuid,
    pub created_by: Uuid,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
}

impl ProjectCreatedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        project_name: String,
        owner_type: String,
        owner_id: Uuid,
        created_by: Uuid,
        visibility: String,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("project_created".to_string(), project_id),
            project_id,
            project_name,
            owner_type,
            owner_id,
            created_by,
            visibility,
            created_at,
        }
    }
}

/// Event published when a project is updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectUpdatedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub project_name: String,
    pub updated_fields: Vec<String>,
    pub updated_by: Uuid,
    pub updated_at: DateTime<Utc>,
}

impl ProjectUpdatedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        project_name: String,
        updated_fields: Vec<String>,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("project_updated".to_string(), project_id),
            project_id,
            project_name,
            updated_fields,
            updated_by,
            updated_at,
        }
    }
}

/// Event published when a project is deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDeletedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub project_name: String,
    pub deleted_by: Uuid,
    pub deleted_at: DateTime<Utc>,
    #[serde(default)]
    pub member_user_ids: Vec<Uuid>,
    #[serde(default)]
    pub component_ids: Vec<Uuid>,
    #[serde(default)]
    pub owner_type: Option<String>,
    #[serde(default)]
    pub owner_id: Option<Uuid>,
}

impl ProjectDeletedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        project_name: String,
        deleted_by: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Self {
        Self::with_authz(
            project_id,
            project_name,
            deleted_by,
            deleted_at,
            Vec::new(),
            Vec::new(),
            None,
            None,
        )
    }

    #[must_use]
    pub fn with_authz(
        project_id: Uuid,
        project_name: String,
        deleted_by: Uuid,
        deleted_at: DateTime<Utc>,
        member_user_ids: Vec<Uuid>,
        component_ids: Vec<Uuid>,
        owner_type: Option<String>,
        owner_id: Option<Uuid>,
    ) -> Self {
        Self {
            base: BaseEvent::new("project_deleted".to_string(), project_id).with_version(2),
            project_id,
            project_name,
            deleted_by,
            deleted_at,
            member_user_ids,
            component_ids,
            owner_type,
            owner_id,
        }
    }
}

/// Event published when a project is published (status changed to active)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPublishedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub project_name: String,
    pub published_by: Uuid,
    pub published_at: DateTime<Utc>,
}

impl ProjectPublishedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        project_name: String,
        published_by: Uuid,
        published_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("project_published".to_string(), project_id),
            project_id,
            project_name,
            published_by,
            published_at,
        }
    }
}

/// Event published when a project's visibility actually flips.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectVisibilityChangedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub owner_type: String,
    pub owner_id: Uuid,
    pub old_visibility: String,
    pub new_visibility: String,
    /// Monotonic source revision for ordering visibility changes per project.
    pub visibility_revision: i64,
    pub changed_by: Uuid,
    pub changed_at: DateTime<Utc>,
}

impl ProjectVisibilityChangedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        owner_type: String,
        owner_id: Uuid,
        old_visibility: String,
        new_visibility: String,
        changed_by: Uuid,
        changed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("project_visibility_changed".to_string(), project_id),
            project_id,
            owner_type,
            owner_id,
            old_visibility,
            new_visibility,
            visibility_revision: 1,
            changed_by,
            changed_at,
        }
    }

    /// Set the durable source revision used to order visibility changes.
    #[must_use]
    pub const fn with_visibility_revision(mut self, visibility_revision: i64) -> Self {
        self.visibility_revision = visibility_revision;
        self
    }
}

/// Event published when a project is archived
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectArchivedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub project_name: String,
    pub owner_type: String,
    pub owner_id: Uuid,
    pub archived_by: Uuid,
    pub archived_at: DateTime<Utc>,
}

impl ProjectArchivedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        project_name: String,
        owner_type: String,
        owner_id: Uuid,
        archived_by: Uuid,
        archived_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("project_archived".to_string(), project_id),
            project_id,
            project_name,
            owner_type,
            owner_id,
            archived_by,
            archived_at,
        }
    }
}

/// Event published when a project is suspended
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSuspendedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub project_name: String,
    pub owner_type: String,
    pub owner_id: Uuid,
    pub visibility: String,
    pub suspended_by: Uuid,
    pub suspended_at: DateTime<Utc>,
}

impl ProjectSuspendedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        project_name: String,
        owner_type: String,
        owner_id: Uuid,
        visibility: String,
        suspended_by: Uuid,
        suspended_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("project_suspended".to_string(), project_id).with_version(2),
            project_id,
            project_name,
            owner_type,
            owner_id,
            visibility,
            suspended_by,
            suspended_at,
        }
    }
}

/// Event published when a suspended project is resumed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResumedEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub project_id: Uuid,
    pub project_name: String,
    pub owner_type: String,
    pub owner_id: Uuid,
    pub visibility: String,
    pub resumed_by: Uuid,
    pub resumed_at: DateTime<Utc>,
}

impl ProjectResumedEvent {
    #[must_use]
    pub fn new(
        project_id: Uuid,
        project_name: String,
        owner_type: String,
        owner_id: Uuid,
        visibility: String,
        resumed_by: Uuid,
        resumed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            base: BaseEvent::new("project_resumed".to_string(), project_id).with_version(2),
            project_id,
            project_name,
            owner_type,
            owner_id,
            visibility,
            resumed_by,
            resumed_at,
        }
    }
}
