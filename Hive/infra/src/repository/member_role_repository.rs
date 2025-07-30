//! RolePermissionRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::{entity::organization_member_role_permission::OrganizationMemberRolePermission};
use hive_domain::error::DomainError;
use hive_domain::port::repository::MemberRoleRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, 
    QueryFilter, Set
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::MemberRoles,
    member_roles,
};

/// SeaORM implementation of RolePermissionRepository
#[derive(Clone)]
pub struct MemberRoleRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl MemberRoleRepositoryImpl {
    /// Create a new MemberRoleRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert a database model to a domain role permission
    fn to_domain(model: member_roles::Model) -> OrganizationMemberRolePermission {
        OrganizationMemberRolePermission {
            id: model.id,
            organization_id: model.organization_id,
            member_id: model.member_id,
            role_permission: RolePermission::new(model.role_id, model.permission_id, model.resource_id, model.created_at),
            created_at: model.created_at,
        }
    }

    /// Convert a domain role permission to a database active model
    fn to_active_model(member_role: &OrganizationMemberRole) -> member_roles::ActiveModel {
        member_roles::ActiveModel {
            id: ActiveValue::Set(member_role.id),
            organization_id: ActiveValue::Set(member_role.organization_id),
            member_id: ActiveValue::Set(member_role.member_id),
            role_id: ActiveValue::Set(member_role.role_id),
            created_at: ActiveValue::Set(member_role.created_at),
        }
    }
}

#[async_trait]
impl MemberRoleRepository for MemberRoleRepositoryImpl {
    async fn find_by_organization_member(&self, organization_id: &Uuid, member_id: &Uuid) -> Result<Vec<OrganizationMemberRole>, DomainError> {
        debug!("Finding member roles by organization ID: {} and member ID: {}", organization_id, member_id);

        let member_roles = MemberRoles::find()
            .filter(member_roles::Column::OrganizationId.eq(*organization_id))
            .filter(member_roles::Column::MemberId.eq(*member_id))
            .all(self.db.as_ref())
            .await
            .map_err(DomainError::from)?;
    }

    async fn save(&self, member_role: &OrganizationMemberRole) -> Result<OrganizationMemberRole, DomainError> {
        debug!("Saving member role with ID: {}", member_role.id);

        let active_model = Self::to_active_model(member_role);

        let result = active_model.save(self.db.as_ref()).await.map_err(DomainError::from)?;

        Ok(Self::to_domain(result))
    } 

    async fn delete_by_organization_member(&self, organization_id: &Uuid, member_id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting member roles by organization ID: {} and member ID: {}", organization_id, member_id);

        let result = MemberRoles::delete_many().filter(member_roles::Column::OrganizationId.eq(*organization_id)).filter(member_roles::Column::MemberId.eq(*member_id)).exec(self.db.as_ref()).await.map_err(DomainError::from)?;
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting member roles by organization ID: {}", organization_id);

        let result = MemberRoles::delete_many().filter(member_roles::Column::OrganizationId.eq(*organization_id)).exec(self.db.as_ref()).await.map_err(DomainError::from)?;
    }
} 