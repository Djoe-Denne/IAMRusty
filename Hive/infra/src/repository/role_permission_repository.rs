//! RolePermissionRepository SeaORM implementation

use async_trait::async_trait;
use hive_domain::entity::{
    permission::{Permission, PermissionLevel},
    resource::Resource,
    RolePermission,
};
use hive_domain::port::repository::{
    RolePermissionReadRepository, RolePermissionRepository, RolePermissionWriteRepository,
};
use rustycog_core::error::DomainError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    TryIntoModel,
};
use serde::de;
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use super::entity::{prelude::RolePermissions as OrganizationRolePermissions, role_permissions};

pub struct RolePermissionMapper;

impl RolePermissionMapper {
    pub fn to_domain(model: role_permissions::Model) -> RolePermission {
        RolePermission::new(
            Some(model.id),
            Some(model.name.clone()),
            model.organization_id,
            &Permission::new(
                PermissionLevel::from_str(model.permission_id.as_str()).unwrap(),
                None,
                Some(model.created_at),
            ),
            &Resource::new(model.resource_id.into(), None, Some(model.created_at)),
            Some(model.created_at),
        )
    }

    pub fn to_active_model(role_permission: &RolePermission) -> role_permissions::ActiveModel {
        role_permissions::ActiveModel {
            id: ActiveValue::Set(role_permission.id.unwrap_or(Uuid::new_v4())),
            organization_id: ActiveValue::Set(role_permission.organization_id),
            permission_id: ActiveValue::Set(role_permission.permission.level.to_str().to_string()),
            resource_id: ActiveValue::Set(role_permission.resource.name.clone()),
            description: ActiveValue::Set(role_permission.resource.description.clone()),
            created_at: ActiveValue::Set(role_permission.created_at.unwrap()),
            name: ActiveValue::Set(role_permission.name.clone().unwrap_or_default()),
        }
    }
}

/// Read repository
#[derive(Clone)]
pub struct RolePermissionReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl RolePermissionReadRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RolePermissionReadRepository for RolePermissionReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<RolePermission>, DomainError> {
        debug!("Finding role permission by ID: {}", id);

        let role_permission = OrganizationRolePermissions::find()
            .filter(role_permissions::Column::Id.eq(*id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(role_permission.map(RolePermissionMapper::to_domain))
    }

    async fn find_by_organization_role(
        &self,
        organization_id: &Uuid,
        resource_type: &str,
        permission: &str,
    ) -> Result<Option<RolePermission>, DomainError> {
        debug!("Finding role permissions by organization ID: {} and resource type: {} and permission ID: {}", organization_id, resource_type, permission);

        let role_permissions = OrganizationRolePermissions::find()
            .filter(role_permissions::Column::OrganizationId.eq(*organization_id))
            .filter(role_permissions::Column::ResourceId.eq(resource_type))
            .filter(role_permissions::Column::PermissionId.eq(permission))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(role_permissions
            .into_iter()
            .map(RolePermissionMapper::to_domain)
            .next())
    }

    async fn find_by_organization_roles(
        &self,
        organization_id: &Uuid,
        role_permissions: &Vec<RolePermission>,
    ) -> Result<Vec<RolePermission>, DomainError> {
        debug!(
            "Finding role permissions by organization ID: {} and role permissions: {:?}",
            organization_id, role_permissions
        );

        let role_permissions = OrganizationRolePermissions::find()
            .filter(role_permissions::Column::OrganizationId.eq(*organization_id))
            .filter(
                role_permissions::Column::ResourceId.is_in(
                    role_permissions
                        .iter()
                        .map(|role| role.resource.name.clone())
                        .collect::<Vec<_>>(),
                ),
            )
            .filter(
                role_permissions::Column::PermissionId.is_in(
                    role_permissions
                        .iter()
                        .map(|role| role.permission.level.to_str().to_string())
                        .collect::<Vec<_>>(),
                ),
            )
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(role_permissions
            .into_iter()
            .map(RolePermissionMapper::to_domain)
            .collect())
    }
}

/// Write repository
#[derive(Clone)]
pub struct RolePermissionWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl RolePermissionWriteRepositoryImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RolePermissionWriteRepository for RolePermissionWriteRepositoryImpl {
    async fn save(
        &self,
        organization_id: &Uuid,
        role_permission: &RolePermission,
    ) -> Result<RolePermission, DomainError> {
        debug!(
            "Saving role permission with {} for organization {}",
            role_permission
                .name
                .as_ref()
                .map(|n| n.as_str())
                .unwrap_or("Unknown"),
            organization_id
        );

        let exists = role_permission.id.is_some()
            && OrganizationRolePermissions::find_by_id(role_permission.id.unwrap())
                .one(self.db.as_ref())
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?
                .is_some();

        if exists {
            // Update
            let active_model = RolePermissionMapper::to_active_model(role_permission);
            let result = active_model
                .save(self.db.as_ref())
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;

            let saved_model = role_permissions::Model {
                id: result.id.unwrap(),
                organization_id: result.organization_id.unwrap(),
                permission_id: result.permission_id.unwrap(),
                resource_id: result.resource_id.unwrap(),
                description: result.description.unwrap(),
                created_at: result.created_at.unwrap(),
                name: result.name.unwrap(),
            };

            Ok(RolePermissionMapper::to_domain(saved_model))
        } else {
            // Insert
            let active_model = RolePermissionMapper::to_active_model(role_permission);
            let result = active_model
                .insert(self.db.as_ref())
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;

            Ok(RolePermissionMapper::to_domain(result))
        }
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        debug!(
            "Deleting role permissions by organization ID: {}",
            organization_id
        );

        let _result = OrganizationRolePermissions::delete_many()
            .filter(role_permissions::Column::OrganizationId.eq(*organization_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(())
    }
}

/// Combined delegator
#[derive(Clone)]
pub struct RolePermissionRepositoryImpl {
    read_repo: Arc<dyn RolePermissionReadRepository>,
    write_repo: Arc<dyn RolePermissionWriteRepository>,
}

impl RolePermissionRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn RolePermissionReadRepository>,
        write_repo: Arc<dyn RolePermissionWriteRepository>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl RolePermissionReadRepository for RolePermissionRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<RolePermission>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_organization_role(
        &self,
        organization_id: &Uuid,
        resource_type: &str,
        permission: &str,
    ) -> Result<Option<RolePermission>, DomainError> {
        self.read_repo
            .find_by_organization_role(organization_id, resource_type, permission)
            .await
    }

    async fn find_by_organization_roles(
        &self,
        organization_id: &Uuid,
        role_permissions: &Vec<RolePermission>,
    ) -> Result<Vec<RolePermission>, DomainError> {
        self.read_repo
            .find_by_organization_roles(organization_id, role_permissions)
            .await
    }
}

#[async_trait]
impl RolePermissionWriteRepository for RolePermissionRepositoryImpl {
    async fn save(
        &self,
        organization_id: &Uuid,
        role_permission: &RolePermission,
    ) -> Result<RolePermission, DomainError> {
        self.write_repo.save(organization_id, role_permission).await
    }

    async fn delete_by_organization(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        self.write_repo
            .delete_by_organization(organization_id)
            .await
    }
}

impl RolePermissionRepository for RolePermissionRepositoryImpl {}
