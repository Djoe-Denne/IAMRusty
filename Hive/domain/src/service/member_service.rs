use uuid::Uuid;

use crate::{
    entity::*,
    error::DomainError,
    port::*,
};

/// Domain service for organization member management
pub struct MemberServiceImpl<MR, OR, RR>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    RR: OrganizationRoleRepository,
{
    member_repo: MR,
    organization_repo: OR,
    role_repo: RR,
}

#[async_trait::async_trait]
pub trait MemberService {
    async fn add_member(&self, organization_id: Uuid, user_id: Uuid, role_id: Uuid, added_by_user_id: Uuid) -> Result<OrganizationMember, DomainError>;
    async fn remove_member(&self, organization_id: Uuid, user_id: Uuid, removed_by_user_id: Uuid) -> Result<(), DomainError>;
    async fn update_member_role(&self, organization_id: Uuid, user_id: Uuid, new_role_id: Uuid, updated_by_user_id: Uuid) -> Result<OrganizationMember, DomainError>;
    async fn get_member(&self, organization_id: Uuid, user_id: Uuid) -> Result<OrganizationMember, DomainError>;
    async fn list_members(&self, organization_id: Uuid) -> Result<Vec<OrganizationMember>, DomainError>;
    async fn list_active_members(&self, organization_id: Uuid) -> Result<Vec<OrganizationMember>, DomainError>;
    async fn check_member_management_permission(&self, organization_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError>;
}

impl<MR, OR, RR> MemberServiceImpl<MR, OR, RR>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    RR: OrganizationRoleRepository,
{
    /// Create a new member service
    pub fn new(
        member_repo: MR,
        organization_repo: OR,
        role_repo: RR,
    ) -> Self {
        Self {
            member_repo,
            organization_repo,
            role_repo,
        }
    }
}

#[async_trait::async_trait]
impl<MR, OR, RR> MemberService for MemberServiceImpl<MR, OR, RR>
where
    MR: OrganizationMemberRepository,
    OR: OrganizationRepository,
    RR: OrganizationRoleRepository,
{
    /// Add a member to an organization
    async fn add_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        role_id: Uuid,
        added_by_user_id: Uuid,
    ) -> Result<OrganizationMember, DomainError> {
        // Validate organization exists
        let organization = self
            .organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        // Validate role exists and belongs to organization
        let role = self
            .role_repo
            .find_by_id(&role_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("OrganizationRole", &role_id.to_string()))?;

        if role.organization_id != organization_id {
            return Err(DomainError::business_rule_violation(
                "Role does not belong to the specified organization",
            ));
        }

        // Business rule: Check if user is already a member
        if let Some(existing_member) = self
            .member_repo
            .find_by_organization_and_user(&organization_id, &user_id)
            .await?
        {
            if existing_member.is_active() {
                return Err(DomainError::resource_already_exists(
                    "OrganizationMember",
                    &format!("user_id={}, organization_id={}", user_id, organization_id),
                ));
            }
            // If member exists but is not active, we can reactivate them
            let mut updated_member = existing_member;
            updated_member.reactivate()?;
            updated_member.update_role(role_id);
            return self.member_repo.save(&updated_member).await;
        }

        // Business rule: Check permission to add members
        self.check_member_management_permission(&organization_id, &added_by_user_id).await?;

        // Business rule: Cannot add owner to same organization again
        if organization.is_owned_by(&user_id) {
            return Err(DomainError::business_rule_violation(
                "Organization owner is automatically a member",
            ));
        }

        // Create new member
        let member = OrganizationMember::new_from_invitation(organization_id, user_id, role_id, added_by_user_id);
        let saved_member = self.member_repo.save(&member).await?;

        Ok(saved_member)
    }

    /// Remove a member from an organization
    async fn remove_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        removed_by_user_id: Uuid,
    ) -> Result<(), DomainError> {
        // Validate organization exists
        let organization = self
            .organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        // Business rule: Cannot remove organization owner
        if organization.is_owned_by(&user_id) {
            return Err(DomainError::business_rule_violation(
                "Cannot remove organization owner from membership",
            ));
        }

        // Find the member
        let member = self
            .member_repo
            .find_by_organization_and_user(&organization_id, &user_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found(
                "OrganizationMember",
                &format!("user_id={}, organization_id={}", user_id, organization_id),
            ))?;

        // Business rule: Check permission to remove members
        self.check_member_management_permission(&organization_id, &removed_by_user_id).await?;

        // Business rule: Cannot remove yourself unless you're the owner
        if user_id == removed_by_user_id && !organization.is_owned_by(&removed_by_user_id) {
            return Err(DomainError::business_rule_violation(
                "Members cannot remove themselves (contact admin)",
            ));
        }

        // Remove the member
        self.member_repo.delete_by_id(&member.id).await?;

        Ok(())
    }

    /// Update a member's role
    async fn update_member_role(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        new_role_id: Uuid,
        updated_by_user_id: Uuid,
    ) -> Result<OrganizationMember, DomainError> {
        // Validate organization exists
        let organization = self
            .organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        // Validate new role exists and belongs to organization
        let role = self
            .role_repo
            .find_by_id(&new_role_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("OrganizationRole", &new_role_id.to_string()))?;

        if role.organization_id != organization_id {
            return Err(DomainError::business_rule_violation(
                "Role does not belong to the specified organization",
            ));
        }

        // Find the member
        let mut member = self
            .member_repo
            .find_by_organization_and_user(&organization_id, &user_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found(
                "OrganizationMember",
                &format!("user_id={}, organization_id={}", user_id, organization_id),
            ))?;

        // Business rule: Cannot change owner's role
        if organization.is_owned_by(&user_id) {
            return Err(DomainError::business_rule_violation(
                "Cannot change organization owner's role",
            ));
        }

        // Business rule: Check permission to update member roles
        self.check_member_management_permission(&organization_id, &updated_by_user_id).await?;

        // Update member role
        member.update_role(new_role_id);
        let updated_member = self.member_repo.save(&member).await?;

        Ok(updated_member)
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
            .ok_or_else(|| DomainError::entity_not_found(
                "OrganizationMember",
                &format!("user_id={}, organization_id={}", user_id, organization_id),
            ))
    }

    /// List all members of an organization
    async fn list_members(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationMember>, DomainError> {
        // Validate organization exists
        self.organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        self.member_repo
            .find_by_organization(&organization_id)
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
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        self.member_repo
            .find_by_organization_and_status(&organization_id, &MemberStatus::Active)
            .await
    }

    /// Check if user has permission to manage members (add/remove/update)
    async fn check_member_management_permission(
        &self,
        organization_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<(), DomainError> {
        // Find the organization
        let organization = self
            .organization_repo
            .find_by_id(organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        // Owner always has permission
        if organization.is_owned_by(user_id) {
            return Ok(());
        }

        // Check if user is a member with admin role
        let member = self
            .member_repo
            .find_by_organization_and_user(organization_id, user_id)
            .await?;

        if let Some(member) = member {
            if member.is_active() {
                // Get the role and check if it has admin permissions
                let role = self
                    .role_repo
                    .find_by_id(&member.role_id)
                    .await?
                    .ok_or_else(|| DomainError::entity_not_found("OrganizationRole", &member.role_id.to_string()))?;

                if role.is_admin() {
                    return Ok(());
                }
            }
        }

        Err(DomainError::permission_denied(
            "User does not have permission to manage members in this organization",
        ))
    }
} 