use async_trait::async_trait;
use uuid::Uuid;

use crate::{port::*, OrganizationService, MemberService};
use rustycog_permission::{ResourceId, PermissionsFetcher, Permission};
use rustycog_core::error::DomainError;
use std::sync::Arc;
use tracing::{debug, error};

pub struct ResourcePermissionFetcher<OS, MS, MRP> 
where
    OS: OrganizationService,
    MS: MemberService,
    MRP: MemberRoleRepository,
{
    organization_service: Arc<OS>,
    member_service: Arc<MS>,
    member_role_repo: Arc<MRP>,
    resource_name: Vec<String>,
}

impl<OS, MS, MRP> ResourcePermissionFetcher<OS, MS, MRP>
where
    OS: OrganizationService,
    MS: MemberService,
    MRP: MemberRoleRepository,
{
    pub fn new(organization_service: Arc<OS>, member_service: Arc<MS>, member_role_repo: Arc<MRP>, resource_name: Vec<String>) -> Self {
        Self { organization_service, member_service, member_role_repo, resource_name }
    }
}

#[async_trait]
impl<OS, MS, MRP> PermissionsFetcher for ResourcePermissionFetcher<OS, MS, MRP>
where
    OS: OrganizationService + Send + Sync,
    MS: MemberService + Send + Sync,
    MRP: MemberRoleRepository + Send + Sync,
{
    async fn fetch_permissions(
        &self,
        user_id: Uuid,
        resource_ids: Vec<ResourceId>,
    ) -> Result<Vec<Permission>, DomainError> {
        debug!("getting organization with id: {:?}", resource_ids[0].id());
        let organization = self
            .organization_service
            .get_organization(&resource_ids[0].id())
            .await?;
        
        debug!("getting member with user id: {:?}", user_id);
        let member = self
            .member_service
            .get_member(organization.id, user_id)
            .await?;

        debug!("getting member roles with id: {:?}", member.id.unwrap());
        let member_roles = self
            .member_role_repo
            .find_by_organization_member(&member.id.unwrap())
            .await?;
        debug!("member roles: {:?}", member_roles);
        let domain_permissions = member_roles
            .iter()
            .filter(|role| self.resource_name.contains(&role.role_permission.resource.name))
            .map(|role| role.role_permission.permission.level.clone())
            .collect::<Vec<_>>();

        Ok(domain_permissions.into_iter().map(|level| level.into()).collect())
    }
}
