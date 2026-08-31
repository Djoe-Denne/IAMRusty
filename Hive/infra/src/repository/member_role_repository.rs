//! `RolePermissionRepository` `SeaORM` implementation

use async_trait::async_trait;
use hive_domain::port::repository::{
    MemberRoleReadRepository, MemberRoleRepository, MemberRoleWriteRepository,
};
use hive_domain::{
    entity::role_permission::RolePermission,
    entity::{
        organization_member_role_permission::OrganizationMemberRolePermission,
        permission::{Permission, PermissionLevel},
        resource::Resource,
    },
};
use rustycog::core::error::DomainError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QuerySelect, QueryTrait,
};
use std::str::FromStr;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use super::entity::{
    organization_member_role_permissions, organization_members,
    prelude::{OrganizationMemberRolePermissions, RolePermissions as OrganizationRolePermissions},
    role_permissions,
};

pub struct MemberRoleMapper;

impl MemberRoleMapper {
    /// Maps persisted member-role and role-permission rows to the domain entity.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `permission_id` is not a valid [`PermissionLevel`].
    pub fn to_domain(
        model: &organization_member_role_permissions::Model,
        role_permission: role_permissions::Model,
    ) -> Result<OrganizationMemberRolePermission, DomainError> {
        let level = PermissionLevel::from_str(role_permission.permission_id.as_str())?;
        Ok(OrganizationMemberRolePermission {
            id: Some(model.id),
            organization_id: role_permission.organization_id,
            member_id: model.member_id,
            role_permission: RolePermission::new(
                Some(role_permission.id),
                Some(format!(
                    "{}:{}",
                    role_permission.resource_id, role_permission.permission_id
                )),
                role_permission.organization_id,
                &Permission::new(level, None, Some(role_permission.created_at)),
                &Resource::new(
                    role_permission.resource_id,
                    None,
                    Some(role_permission.created_at),
                ),
                Some(model.created_at),
            ),
            created_at: model.created_at,
        })
    }

    /// Builds a `SeaORM` active model from a domain member-role.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `role_permission.id` is missing after persist.
    pub fn to_active_model(
        member_role: &OrganizationMemberRolePermission,
    ) -> Result<organization_member_role_permissions::ActiveModel, DomainError> {
        let role_permission_id = member_role.role_permission.id.ok_or_else(|| {
            DomainError::internal_error("role_permission missing id after persist")
        })?;
        Ok(organization_member_role_permissions::ActiveModel {
            id: ActiveValue::Set(member_role.id.unwrap_or_else(Uuid::new_v4)),
            member_id: ActiveValue::Set(member_role.member_id),
            role_permission_id: ActiveValue::Set(role_permission_id),
            created_at: ActiveValue::Set(member_role.created_at),
        })
    }
}

/// Read repository
#[derive(Clone)]
pub struct MemberRoleReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl MemberRoleReadRepositoryImpl {
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MemberRoleReadRepository for MemberRoleReadRepositoryImpl {
    async fn find_by_organization_member(
        &self,
        member_id: &Uuid,
    ) -> Result<Vec<OrganizationMemberRolePermission>, DomainError> {
        debug!("Finding member roles by member ID: {}", member_id);

        let member_roles = OrganizationMemberRolePermissions::find()
            .filter(organization_member_role_permissions::Column::MemberId.eq(*member_id))
            .find_also_related(OrganizationRolePermissions)
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        member_roles
            .into_iter()
            .map(|(member_role, role_permission)| {
                let role_permission = role_permission.ok_or_else(|| {
                    DomainError::internal_error("member role missing related role_permission")
                })?;
                MemberRoleMapper::to_domain(&member_role, role_permission)
            })
            .collect()
    }
}

/// Write repository
#[derive(Clone)]
pub struct MemberRoleWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl MemberRoleWriteRepositoryImpl {
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Inserts or updates a member-role assignment on `db`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the lookup, insert, or update fails.
    ///
    /// # Panics
    ///
    /// Panics if a required identifier or column is [`None`], or if the related role-permission
    /// row is missing.
    pub async fn save_with_connection<C>(
        db: &C,
        member_role: &OrganizationMemberRolePermission,
    ) -> Result<OrganizationMemberRolePermission, DomainError>
    where
        C: ConnectionTrait,
    {
        let exists = if let Some(id) = member_role.id {
            OrganizationMemberRolePermissions::find_by_id(id)
                .one(db)
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?
                .is_some()
        } else {
            false
        };

        let role_permission_id = member_role.role_permission.id.ok_or_else(|| {
            DomainError::internal_error("role_permission missing id after persist")
        })?;
        let role_permission = OrganizationRolePermissions::find_by_id(role_permission_id)
            .one(db)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .ok_or_else(|| {
                DomainError::internal_error("role_permission row missing after persist")
            })?;

        let active_model = MemberRoleMapper::to_active_model(member_role)?;
        if exists {
            let result = active_model
                .save(db)
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;

            let saved_model = organization_member_role_permissions::Model {
                id: result.id.unwrap(),
                member_id: result.member_id.unwrap(),
                role_permission_id: result.role_permission_id.unwrap(),
                created_at: result.created_at.unwrap(),
            };

            MemberRoleMapper::to_domain(&saved_model, role_permission)
        } else {
            let result = active_model
                .insert(db)
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;

            MemberRoleMapper::to_domain(&result, role_permission)
        }
    }

    /// Deletes member-role assignments for all members of `organization_id`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the delete fails.
    pub async fn delete_by_organization_with_connection<C>(
        db: &C,
        organization_id: &Uuid,
    ) -> Result<(), DomainError>
    where
        C: ConnectionTrait,
    {
        OrganizationMemberRolePermissions::delete_many()
            .filter(
                organization_member_role_permissions::Column::MemberId.in_subquery(
                    organization_members::Entity::find()
                        .filter(organization_members::Column::OrganizationId.eq(*organization_id))
                        .select_only()
                        .column(organization_members::Column::Id)
                        .into_query(),
                ),
            )
            .exec(db)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl MemberRoleWriteRepository for MemberRoleWriteRepositoryImpl {
    async fn save(
        &self,
        member_role: &OrganizationMemberRolePermission,
    ) -> Result<OrganizationMemberRolePermission, DomainError> {
        debug!(
            "Saving member role for member id: {:?} and org id: {:?}",
            member_role.member_id, member_role.organization_id
        );
        Self::save_with_connection(self.db.as_ref(), member_role).await
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
        debug!(
            "Deleting member roles by organization ID: {}",
            organization_id
        );
        Self::delete_by_organization_with_connection(self.db.as_ref(), organization_id).await
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
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl MemberRoleReadRepository for MemberRoleRepositoryImpl {
    async fn find_by_organization_member(
        &self,
        member_id: &Uuid,
    ) -> Result<Vec<OrganizationMemberRolePermission>, DomainError> {
        self.read_repo.find_by_organization_member(member_id).await
    }
}

#[async_trait]
impl MemberRoleWriteRepository for MemberRoleRepositoryImpl {
    async fn save(
        &self,
        member_role: &OrganizationMemberRolePermission,
    ) -> Result<OrganizationMemberRolePermission, DomainError> {
        self.write_repo.save(member_role).await
    }

    async fn delete_by_organization_member(&self, member_id: &Uuid) -> Result<(), DomainError> {
        self.write_repo
            .delete_by_organization_member(member_id)
            .await
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        self.write_repo
            .delete_by_organization(organization_id)
            .await
    }
}

impl MemberRoleRepository for MemberRoleRepositoryImpl {}
