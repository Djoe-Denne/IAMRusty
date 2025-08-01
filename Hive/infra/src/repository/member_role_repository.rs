//! RolePermissionRepository SeaORM implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hive_domain::{entity::{organization_member_role_permission::OrganizationMemberRolePermission, permission::Permission, resource::Resource}, entity::role_permission::RolePermission};
use hive_domain::error::DomainError;
use hive_domain::port::repository::MemberRoleRepository;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityOrSelect, EntityTrait, ModelTrait, QueryFilter, QuerySelect, QueryTrait, Set
};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{
    prelude::{OrganizationMemberRolePermissions, RolePermissions as OrganizationRolePermissions},
    organization_member_role_permissions, organization_members, role_permissions
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
    fn to_domain(model: organization_member_role_permissions::Model, role_permission: role_permissions::Model) -> OrganizationMemberRolePermission {
        OrganizationMemberRolePermission {
            id: model.id,
            organization_id: role_permission.organization_id,
            member_id: model.member_id,
            role_permission: RolePermission::new(
                Some(role_permission.id),
                Some(format!("{}:{}", role_permission.resource_id, role_permission.permission_id)),
                role_permission.organization_id,
                &Permission::new(role_permission.permission_id.into(), None, role_permission.created_at),
                &Resource::new(role_permission.resource_id.into(), None, Some(role_permission.created_at)),
                Some(model.created_at)
            ),
            created_at: model.created_at,
        }
    }

    /// Convert a domain role permission to a database active model
    fn to_active_model(member_role: &OrganizationMemberRolePermission) -> organization_member_role_permissions::ActiveModel {
        organization_member_role_permissions::ActiveModel {
            id: ActiveValue::Set(member_role.id),
            member_id: ActiveValue::Set(member_role.member_id),
            role_permission_id: ActiveValue::Set(member_role.role_permission.id.unwrap()),
            created_at: ActiveValue::Set(member_role.created_at),
        }
    }
}

#[async_trait]
impl MemberRoleRepository for MemberRoleRepositoryImpl {
    async fn find_by_organization_member(&self, member_id: &Uuid) -> Result<Vec<OrganizationMemberRolePermission>, DomainError> {
        debug!("Finding member roles by member ID: {}", member_id);

        let member_roles = OrganizationMemberRolePermissions::find()
            .filter(organization_member_role_permissions::Column::MemberId.eq(*member_id))
            .find_also_related(OrganizationRolePermissions)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(member_roles.into_iter().map(|(member_role, role_permission)| Self::to_domain(member_role, role_permission.unwrap())).collect())
    }

    async fn save(&self, member_role: &OrganizationMemberRolePermission) -> Result<OrganizationMemberRolePermission, DomainError> {
        debug!("Saving member role with ID: {}", member_role.id);

        let active_model = Self::to_active_model(member_role);

        let result = active_model.save(self.db.as_ref()).await.map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let member_role = OrganizationMemberRolePermissions::find_by_id(result.id.unwrap())
            .find_also_related(OrganizationRolePermissions)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let (member_role, role_permission) = member_role.unwrap();

        Ok(Self::to_domain(member_role, role_permission.unwrap()))
    } 

    async fn delete_by_organization_member(&self, member_id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting member roles by member ID: {}", member_id);

        let result = OrganizationMemberRolePermissions::delete_many()
        .filter(organization_member_role_permissions::Column::MemberId.eq(*member_id))
        .exec(self.db.as_ref())
        .await
        .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        debug!("Deleted {} member roles", result.rows_affected);
        Ok(())
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        debug!("Deleting member roles by organization ID: {}", organization_id);
    
        let result = OrganizationMemberRolePermissions::delete_many()
            .filter(
                organization_member_role_permissions::Column::MemberId.in_subquery(
                    organization_members::Entity::find()
                        .filter(organization_members::Column::OrganizationId.eq(*organization_id))
                        .select_only()
                        .column(organization_members::Column::Id)
                        .into_query()
                )
            )
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;
    
        debug!("Deleted {} member roles", result.rows_affected);
        Ok(())
    }
} 