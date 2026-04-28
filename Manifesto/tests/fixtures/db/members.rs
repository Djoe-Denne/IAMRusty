//! Member fixtures for testing

use chrono::Utc;
use rustycog_testing::db::CommittedFixture;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
};
use std::sync::Arc;
use uuid::Uuid;

// Import the entity types from infra crate
use manifesto_infra::repository::entity::project_member_role_permissions::ActiveModel as ProjectMemberRolePermissionActiveModel;
use manifesto_infra::repository::entity::project_members::{
    ActiveModel as MemberActiveModel, Model as MemberModel,
};
use manifesto_infra::repository::entity::role_permissions::{
    ActiveModel as RolePermissionActiveModel, Column as RolePermissionsColumn,
    Entity as RolePermissionsEntity,
};

/// Member fixture wrapper
pub struct MemberFixture {
    inner: CommittedFixture<MemberModel>,
}

impl MemberFixture {
    /// Get the member ID
    pub const fn id(&self) -> Uuid {
        self.inner.model.id
    }

    /// Get the project ID
    pub const fn project_id(&self) -> Uuid {
        self.inner.model.project_id
    }

    /// Get the user ID
    pub const fn user_id(&self) -> Uuid {
        self.inner.model.user_id
    }

    /// Get the member source
    pub fn source(&self) -> &str {
        &self.inner.model.source
    }

    /// Check if member is active (not removed)
    pub const fn is_active(&self) -> bool {
        self.inner.model.removed_at.is_none()
    }

    /// Get the inner model
    pub const fn model(&self) -> &MemberModel {
        &self.inner.model
    }
}

/// Member fixture builder with fluent API
#[derive(Debug, Clone)]
pub struct MemberFixtureBuilder {
    id: Option<Uuid>,
    project_id: Option<Uuid>,
    user_id: Option<Uuid>,
    source: Option<String>,
    added_by: Option<Uuid>,
    permission_level: Option<String>,
}

impl MemberFixtureBuilder {
    /// Create a new member fixture builder
    pub const fn new() -> Self {
        Self {
            id: None,
            project_id: None,
            user_id: None,
            source: None,
            added_by: None,
            permission_level: None,
        }
    }

    /// Set the member ID
    pub const fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the project ID
    pub const fn for_project(mut self, project_id: Uuid) -> Self {
        self.project_id = Some(project_id);
        self
    }

    /// Set the user ID
    pub const fn user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Create an owner member for a project
    pub fn owner(mut self, project_id: Uuid, user_id: Uuid) -> Self {
        self.project_id = Some(project_id);
        self.user_id = Some(user_id);
        self.source = Some("direct".to_string());
        self.added_by = Some(user_id);
        self.permission_level = Some("owner".to_string());
        self
    }

    /// Create a direct member
    pub fn direct(mut self, project_id: Uuid, user_id: Uuid, added_by: Uuid) -> Self {
        self.project_id = Some(project_id);
        self.user_id = Some(user_id);
        self.source = Some("direct".to_string());
        self.added_by = Some(added_by);
        self.permission_level = Some("read".to_string());
        self
    }

    /// Set the member source
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set as direct source
    pub fn source_direct(mut self) -> Self {
        self.source = Some("direct".to_string());
        self
    }

    /// Set as `org_cascade` source
    pub fn source_org_cascade(mut self) -> Self {
        self.source = Some("org_cascade".to_string());
        self
    }

    /// Set as invitation source
    pub fn source_invitation(mut self) -> Self {
        self.source = Some("invitation".to_string());
        self
    }

    /// Set who added this member
    pub const fn added_by(mut self, added_by: Uuid) -> Self {
        self.added_by = Some(added_by);
        self
    }

    /// Commit the member to the database
    pub async fn commit(self, db: Arc<DatabaseConnection>) -> Result<MemberFixture, DbErr> {
        let now: DateTimeWithTimeZone = Utc::now().into();
        let id = self.id.unwrap_or_else(Uuid::new_v4);

        let project_id = self.project_id.ok_or_else(|| {
            DbErr::Custom("project_id is required for MemberFixtureBuilder".to_string())
        })?;

        let user_id = self.user_id.ok_or_else(|| {
            DbErr::Custom("user_id is required for MemberFixtureBuilder".to_string())
        })?;

        let active_model = MemberActiveModel {
            id: ActiveValue::Set(id),
            project_id: ActiveValue::Set(project_id),
            user_id: ActiveValue::Set(user_id),
            source: ActiveValue::Set(self.source.unwrap_or_else(|| "direct".to_string())),
            added_by: ActiveValue::Set(self.added_by),
            added_at: ActiveValue::Set(now),
            removed_at: ActiveValue::NotSet,
            removal_reason: ActiveValue::NotSet,
            grace_period_ends_at: ActiveValue::NotSet,
            last_access_at: ActiveValue::NotSet,
        };

        let model = active_model.insert(db.as_ref()).await?;

        // Create role permission for this member if permission level is set
        if let Some(permission_level) = self.permission_level {
            let resources = ["project", "component", "member"];
            let permission_level = permission_level.clone();
            for resource in resources {
                // Check if role_permission already exists for this project/permission/resource
                let existing = RolePermissionsEntity::find()
                    .filter(RolePermissionsColumn::ProjectId.eq(project_id))
                    .filter(RolePermissionsColumn::PermissionId.eq(permission_level.clone()))
                    .filter(RolePermissionsColumn::ResourceId.eq(resource))
                    .one(db.as_ref())
                    .await?;

                let role_permission_id = if let Some(existing_rp) = existing {
                    // Reuse existing role_permission
                    existing_rp.id
                } else {
                    // Create new role_permission entry
                    let new_role_permission_id = Uuid::new_v4();
                    let role_permission = RolePermissionActiveModel {
                        id: ActiveValue::Set(new_role_permission_id),
                        name: ActiveValue::Set(Some(format!("{}_role", permission_level.clone()))),
                        project_id: ActiveValue::Set(project_id),
                        permission_id: ActiveValue::Set(permission_level.clone()),
                        resource_id: ActiveValue::Set(resource.to_string()),
                        created_at: ActiveValue::Set(now),
                    };
                    role_permission.insert(db.as_ref()).await?;
                    new_role_permission_id
                };

                // Create project_member_role_permission entry linking member to role permission
                let member_role_permission = ProjectMemberRolePermissionActiveModel {
                    id: ActiveValue::Set(Uuid::new_v4()),
                    member_id: ActiveValue::Set(id),
                    role_permission_id: ActiveValue::Set(role_permission_id),
                    created_at: ActiveValue::Set(now),
                };
                member_role_permission.insert(db.as_ref()).await?;
            }
        }

        Ok(MemberFixture {
            inner: CommittedFixture::new(model),
        })
    }
}

impl Default for MemberFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}
