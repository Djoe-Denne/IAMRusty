use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::service::RoleService;

use crate::{
    dto::{
        role::{CreateMemberRoleRequest, MemberRole, UpdateMemberRoleRequest},
        PaginationRequest,
    },
    ApplicationError,
};

#[async_trait]
pub trait RoleUseCase: Send + Sync {
    async fn create_role(
        &self,
        organization_id: Uuid,
        request: &CreateMemberRoleRequest,
        user_id: Uuid,
    ) -> Result<MemberRole, ApplicationError>;

    async fn list_roles(
        &self,
        organization_id: Uuid,
        pagination: &PaginationRequest,
    ) -> Result<Vec<MemberRole>, ApplicationError>;

    async fn get_role(
        &self,
        organization_id: Uuid,
        role_id: Uuid,
    ) -> Result<MemberRole, ApplicationError>;

    async fn update_role(
        &self,
        organization_id: Uuid,
        role_id: Uuid,
        request: &UpdateMemberRoleRequest,
        user_id: Uuid,
    ) -> Result<MemberRole, ApplicationError>;

    async fn delete_role(
        &self,
        organization_id: Uuid,
        role_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError>;
}

pub struct RoleUseCaseImpl {
    role_service: Arc<dyn RoleService>,
}

impl RoleUseCaseImpl {
    pub fn new(role_service: Arc<dyn RoleService>) -> Self {
        Self { role_service }
    }
}

#[async_trait]
impl RoleUseCase for RoleUseCaseImpl {
    async fn create_role(
        &self,
        _organization_id: Uuid,
        _request: &CreateMemberRoleRequest,
        _user_id: Uuid,
    ) -> Result<MemberRole, ApplicationError> {
        Err(ApplicationError::Internal {
            message: "create_role is not part of the live Hive command surface".to_string(),
        })
    }

    async fn list_roles(
        &self,
        organization_id: Uuid,
        pagination: &PaginationRequest,
    ) -> Result<Vec<MemberRole>, ApplicationError> {
        let roles = self
            .role_service
            .list_organization_roles(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?;

        let page = pagination.page().max(1);
        let page_size = usize::try_from(pagination.page_size()).unwrap_or(usize::MAX);
        let start = usize::try_from(page.saturating_sub(1))
            .unwrap_or(0)
            .saturating_mul(page_size);

        Ok(roles
            .into_iter()
            .map(MemberRole::from)
            .skip(start)
            .take(page_size)
            .collect())
    }

    async fn get_role(
        &self,
        organization_id: Uuid,
        role_id: Uuid,
    ) -> Result<MemberRole, ApplicationError> {
        let role = self
            .role_service
            .get_organization_role(&organization_id, &role_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(MemberRole::from(role))
    }

    async fn update_role(
        &self,
        _organization_id: Uuid,
        _role_id: Uuid,
        _request: &UpdateMemberRoleRequest,
        _user_id: Uuid,
    ) -> Result<MemberRole, ApplicationError> {
        Err(ApplicationError::Internal {
            message: "update_role is not part of the live Hive command surface".to_string(),
        })
    }

    async fn delete_role(
        &self,
        _organization_id: Uuid,
        _role_id: Uuid,
        _user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        Err(ApplicationError::Internal {
            message: "delete_role is not part of the live Hive command surface".to_string(),
        })
    }
}
