//! RolePermissionRepository SeaORM implementation

use async_trait::async_trait;
use hive_domain::{entity::{organization_member_role_permission::OrganizationMemberRolePermission, permission::{Permission, PermissionLevel}, resource::Resource}, entity::role_permission::RolePermission};
use rustycog_core::error::DomainError;
use hive_domain::port::repository::{
    MemberRoleReadRepository, MemberRoleRepository, MemberRoleWriteRepository,
};
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

pub struct MemberRoleMapper;

impl MemberRoleMapper {
    pub fn to_domain(model: organization_member_role_permissions::Model, role_permission: role_permissions::Model) -> OrganizationMemberRolePermission {
        OrganizationMemberRolePermission {
            id: Some(model.id),
            organization_id: role_permission.organization_id,
            member_id: model.member_id,
            role_permission: RolePermission::new(
                Some(role_permission.id),
                Some(format!("{}:{}", role_permission.resource_id, role_permission.permission_id)),
                role_permission.organization_id,
                &Permission::new(PermissionLevel::from_str(role_permission.permission_id.as_str()).unwrap(), None, Some(role_permission.created_at)),
                &Resource::new(role_permission.resource_id.into(), None, Some(role_permission.created_at)),
                Some(model.created_at)
            ),
            created_at: model.created_at,
        }
    }

    pub fn to_active_model(member_role: &OrganizationMemberRolePermission) -> organization_member_role_permissions::ActiveModel {
        organization_member_role_permissions::ActiveModel {
            id: ActiveValue::Set(member_role.id.unwrap_or(Uuid::new_v4())),
            member_id: ActiveValue::Set(member_role.member_id),
            role_permission_id: ActiveValue::Set(member_role.role_permission.id.unwrap()),
            created_at: ActiveValue::Set(member_role.created_at),
        }
    }
}

/// Read repository
#[derive(Clone)]
pub struct MemberRoleReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl MemberRoleReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }
}

#[async_trait]
impl MemberRoleReadRepository for MemberRoleReadRepositoryImpl {
    async fn find_by_organization_member(&self, member_id: &Uuid) -> Result<Vec<OrganizationMemberRolePermission>, DomainError> {
        debug!("Finding member roles by member ID: {}", member_id);

        let member_roles = OrganizationMemberRolePermissions::find()
            .filter(organization_member_role_permissions::Column::MemberId.eq(*member_id))
            .find_also_related(OrganizationRolePermissions)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(member_roles.into_iter().map(|(member_role, role_permission)| MemberRoleMapper::to_domain(member_role, role_permission.unwrap())).collect())
    }

}

/// Write repository
#[derive(Clone)]
pub struct MemberRoleWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl MemberRoleWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self { Self { db } }
}

#[async_trait]
impl MemberRoleWriteRepository for MemberRoleWriteRepositoryImpl {
    async fn save(&self, member_role: &OrganizationMemberRolePermission) -> Result<OrganizationMemberRolePermission, DomainError> {
        debug!("Saving member role for member id: {:?} and org id: {:?}", member_role.member_id, member_role.organization_id);

        let exists = member_role.id.is_some() && OrganizationMemberRolePermissions::find_by_id(member_role.id.unwrap())
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .is_some();

        let role_permission = OrganizationRolePermissions::find_by_id(member_role.role_permission.id.unwrap())
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .unwrap();

        if exists {
            // Update
            let active_model = MemberRoleMapper::to_active_model(member_role);
            let result = active_model.save(self.db.as_ref()).await.map_err(|e| DomainError::internal_error(&e.to_string()))?;

            let saved_model = organization_member_role_permissions::Model {
                id: result.id.unwrap(),
                member_id: result.member_id.unwrap(),
                role_permission_id: result.role_permission_id.unwrap(),
                created_at: result.created_at.unwrap(),
            };

            Ok(MemberRoleMapper::to_domain(saved_model, role_permission))
        } else {
            // Insert
            let active_model = MemberRoleMapper::to_active_model(member_role);
            let result = active_model.insert(self.db.as_ref()).await.map_err(|e| DomainError::internal_error(&e.to_string()))?;

            Ok(MemberRoleMapper::to_domain(result, role_permission))
        }
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

/// Combined delegator
#[derive(Clone)]
pub struct MemberRoleRepositoryImpl {
    read_repo: Arc<dyn MemberRoleReadRepository>,
    write_repo: Arc<dyn MemberRoleWriteRepository>,
}

impl MemberRoleRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn MemberRoleReadRepository>,
        write_repo: Arc<dyn MemberRoleWriteRepository>,
    ) -> Self {
        Self { read_repo, write_repo }
    }
}

#[async_trait]
impl MemberRoleReadRepository for MemberRoleRepositoryImpl {
    async fn find_by_organization_member(&self, member_id: &Uuid) -> Result<Vec<OrganizationMemberRolePermission>, DomainError> {
        self.read_repo.find_by_organization_member(member_id).await
    }
}

#[async_trait]
impl MemberRoleWriteRepository for MemberRoleRepositoryImpl {
    async fn save(&self, member_role: &OrganizationMemberRolePermission) -> Result<OrganizationMemberRolePermission, DomainError> {
        self.write_repo.save(member_role).await
    } 

    async fn delete_by_organization_member(&self, member_id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete_by_organization_member(member_id).await
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete_by_organization(organization_id).await
    }
}

impl MemberRoleRepository for MemberRoleRepositoryImpl {}