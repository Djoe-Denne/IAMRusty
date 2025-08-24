use uuid::Uuid;

use crate::{entity::*, port::*, service::role_service::RoleService};
use rustycog_core::error::DomainError;
use std::sync::Arc;
use tracing::debug;

/// Domain service for organization member management
pub struct MemberServiceImpl<MR, OR, RS>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    RS: RoleService,
{
    member_repo: Arc<MR>,
    organization_repo: Arc<OR>,
    role_service: Arc<RS>,
}

#[async_trait::async_trait]
pub trait MemberService: Send + Sync {
    /**
     * Add a member to an organization
     * 
     * @param organization_id - The ID of the organization to add the member to
     * @param user_id - The ID of the user to add as a member
     * @param roles - The roles to assign to the member
     * @param added_by_user_id - The ID of the user who added the member. If Option empty, bypass permission check, used for system operations such as owner creation
     */
    async fn add_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        roles: Vec<RolePermission>,
        added_by_user_id: Option<Uuid>,
    ) -> Result<OrganizationMember, DomainError>;

    /**
     * Remove a member from an organization
     * 
     * @param organization_id - The ID of the organization to remove the member from
     * @param user_id - The ID of the user to remove as a member
     * @param removed_by_user_id - The ID of the user who removed the member. If Option empty, bypass permission check, used for system operations such as owner removal
     */
    async fn remove_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError>;

    /**
     * Remove all members from an organization
     * 
     * @param organization_id - The ID of the organization to remove the members from
     */
    async fn remove_organization_members(&self, organization_id: Uuid) -> Result<(), DomainError>;

    /**
     * Get a member by organization and user ID
     * 
     * @param organization_id - The ID of the organization to get the member from
     * @param user_id - The ID of the user to get as a member
     */
    async fn get_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<OrganizationMember, DomainError>;

    /**
     * List all members of an organization
     * 
     * @param organization_id - The ID of the organization to list the members of
     */
    async fn list_members(
        &self,
        organization_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<OrganizationMember>, DomainError>;

    /**
     * List active members of an organization
     * 
     * @param organization_id - The ID of the organization to list the active members of
     */
    async fn list_active_members(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationMember>, DomainError>;

    /**
     * Update a member's role
     * 
     * @param organization_id - The ID of the organization to update the member's role in
     * @param member_id - The ID of the member to update the role of
     * @param roles - The roles to assign to the member
     */
    async fn update_member_roles(
        &self,
        organization_id: Uuid,
        member_id: Uuid,
        roles: Vec<RolePermission>,
    ) -> Result<OrganizationMember, DomainError>;
}

impl<MR, OR, RS> MemberServiceImpl<MR, OR, RS>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    RS: RoleService,
{
    /// Create a new member service
    pub fn new(member_repo: Arc<MR>, organization_repo: Arc<OR>, role_service: Arc<RS>) -> Self {
        Self {
            member_repo,
            organization_repo,
            role_service,
        }
    }

    
    async fn update_member_roles(
        &self,
        member: &mut OrganizationMember,
        roles: Vec<RolePermission>,
    ) -> Result<OrganizationMember, DomainError> {
        if member.id.is_none() {
            return Err(DomainError::invalid_input("Member ID is required"));
        }

        let new_roles = self.role_service.add_roles(&member.organization_id, &member.id.unwrap(), roles).await?;
        member.update_roles(new_roles);
        self.member_repo.save(member).await
    }
}

#[async_trait::async_trait]
impl<MR, OR, RS> MemberService for MemberServiceImpl<MR, OR, RS>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    RS: RoleService,
{
    /// Add a member to an organization
    async fn add_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        roles: Vec<RolePermission>,
        added_by_user_id: Option<Uuid>,
    ) -> Result<OrganizationMember, DomainError> {
        debug!("Adding user {} as member to organization: {:?}", user_id, organization_id);
        // Validate organization exists
        let _ = self
            .organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found("Organization", &organization_id.to_string())
            })?;

        // Business rule: Check if user is already a member
        if let Some(_) = self
            .member_repo
            .find_by_organization_and_user(&organization_id, &user_id)
            .await?
        {
            return Err(DomainError::resource_already_exists(
                "OrganizationMember",
                &format!("user_id={}, organization_id={}", user_id, organization_id),
            ));
        }

        // Create new member
        let member = OrganizationMember::new(organization_id, user_id, added_by_user_id);
        let mut saved_member = self.member_repo.save(&member).await?;

        let roles = self.role_service.find_role_permissions_by_organization(&organization_id, &roles).await?;

        // Add roles to member
        let updated_member = self.update_member_roles(&mut saved_member, roles).await?;

        Ok(updated_member)
    }

    /// Remove a member from an organization
    async fn remove_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        // Validate organization exists
        let organization = self
            .organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found("Organization", &organization_id.to_string())
            })?;

        // Find the member
        let member = self
            .member_repo
            .find_by_organization_and_user(&organization_id, &user_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "OrganizationMember",
                    &format!("user_id={}, organization_id={}", user_id, organization_id),
                )
            })?;

        // Remove the member
        self.member_repo.delete_by_id(&member.id.unwrap()).await?;

        Ok(())
    }

    /// Remove all members from an organization

    async fn remove_organization_members(&self, organization_id: Uuid) -> Result<(), DomainError> {
        self.member_repo.delete_by_organization(&organization_id).await?;
        Ok(())
    }

    /// Update a member's role
    async fn update_member_roles(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        roles: Vec<RolePermission>
    ) -> Result<OrganizationMember, DomainError> {
        let mut member = self.get_member(organization_id, user_id).await?;
        self.update_member_roles(&mut member, roles).await
    }

    /// Get a member by organization and user ID
    async fn get_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<OrganizationMember, DomainError> {
        self.member_repo
            .find_by_organization_and_user(&organization_id, &user_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "OrganizationMember",
                    &format!("user_id={}, organization_id={}", user_id, organization_id),
                )
            })
    }

    /// List all members of an organization
    async fn list_members(
        &self,
        organization_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<OrganizationMember>, DomainError> {
        self.member_repo
            .find_by_organization(&organization_id, page, page_size)
            .await
    }

    /// List active members of an organization
    async fn list_active_members(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationMember>, DomainError> {
        // Validate organization exists
        self.organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found("Organization", &organization_id.to_string())
            })?;

        self.member_repo
            .find_by_organization_and_status(&organization_id, &MemberStatus::Active)
            .await
    }
}
