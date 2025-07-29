use uuid::Uuid;
use serde_json::Value;

use crate::{
    entity::*,
    error::DomainError,
    port::*,
};

/// Domain service for organization management
pub struct OrganizationServiceImpl<OR, MR, RR>
where
    OR: OrganizationRepository,
    MR: OrganizationMemberRepository,
    RR: RolePermissionRepository,
{
    organization_repo: OR,
    member_repo: MR,
    role_repo: RR,
}

#[async_trait::async_trait]
pub trait OrganizationService {
    async fn create_organization(&self, organization: &Organization) -> Result<Organization, DomainError>;
    async fn update_organization(&self, id: &Uuid, name: Option<String>, description: Option<String>, avatar_url: Option<String>, settings: Option<Value>, requesting_user_id: &Uuid) -> Result<Organization, DomainError>;
    async fn delete_organization(&self, id: &Uuid, requesting_user_id: &Uuid) -> Result<(), DomainError>;
    async fn get_organization(&self, id: &Uuid) -> Result<Organization, DomainError>;
    async fn get_organization_by_slug(&self, slug: &str) -> Result<Organization, DomainError>;
    async fn list_user_organizations(&self, user_id: &Uuid, page: u32, page_size: u32) -> Result<Vec<Organization>, DomainError>;
    async fn search_organizations(&self, name_pattern: &str, _user_id: Option<Uuid>, page: u32, page_size: u32) -> Result<Vec<Organization>, DomainError>;
    async fn check_admin_permission(&self, organization_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError>;
    async fn check_read_permission(&self, organization_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError>;
    async fn create_default_roles(&self, organization_id: &Uuid) -> Result<(), DomainError>;
    async fn add_owner_as_member(&self, organization: &Organization, owner_user_id: Uuid) -> Result<(), DomainError>;
}

impl<OR, MR, RR> OrganizationServiceImpl<OR, MR, RR>
where
    OR: OrganizationRepository,
    MR: OrganizationMemberRepository,
    RR: RolePermissionRepository,
{
    /// Create a new organization service
    pub fn new(
        organization_repo: OR,
        member_repo: MR,
        role_repo: RR,
    ) -> Self {
        Self {
            organization_repo,
            member_repo,
            role_repo,
        }
    }
}

#[async_trait::async_trait]
impl<OR, MR, RR> OrganizationService for OrganizationServiceImpl<OR, MR, RR>
where
    OR: OrganizationRepository,
    MR: OrganizationMemberRepository,
    RR: RolePermissionRepository,
{
    /// Create a new organization with default roles
    async fn create_organization(
        &self,
        organization: &Organization,
    ) -> Result<Organization, DomainError> {
        // Business rule: Check if organization with same slug already exists
        if self.organization_repo.exists_by_slug(&organization.slug).await? {
            return Err(DomainError::resource_already_exists(
                "Organization",
                &format!("slug={}", organization.slug),
            ));
        }

        let saved_org = self.organization_repo.save(&organization).await?;

        // Create default system roles for the organization
        self.create_default_roles(&saved_org.id).await?;

        // Add owner as the first member with Owner role
        self.add_owner_as_member(&saved_org, organization.owner_user_id).await?;

        Ok(saved_org)
    }

    /// Update organization details
    async fn update_organization(
        &self,
        id: &Uuid,
        name: Option<String>,
        description: Option<String>,
        avatar_url: Option<String>,
        settings: Option<Value>,
        requesting_user_id: &Uuid,
    ) -> Result<Organization, DomainError> {
        // Find the organization
        let mut organization = self
            .organization_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &id.to_string()))?;

        // Business rule: Only owner or admin can update organization
        self.check_admin_permission(id, requesting_user_id).await?;

        // Apply updates
        if let Some(new_name) = name {
            organization.update_name(new_name)?;
        }

        if let Some(new_description) = description {
            organization.update_description(Some(new_description));
        }

        if let Some(new_avatar_url) = avatar_url {
            organization.update_avatar_url(Some(new_avatar_url));
        }

        if let Some(new_settings) = settings {
            organization.update_settings(new_settings);
        }

        // Save updated organization
        let updated_organization = self.organization_repo.save(&organization).await?;

        Ok(updated_organization)
    }

    /// Delete organization
    async fn delete_organization(
        &self,
        id: &Uuid,
        requesting_user_id: &Uuid,
    ) -> Result<(), DomainError> {
        // Find the organization
        let organization = self
            .organization_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &id.to_string()))?;

        // Business rule: Only owner can delete organization
        if !organization.is_owned_by(requesting_user_id) {
            return Err(DomainError::permission_denied(
                "Only organization owner can delete the organization",
            ));
        }

        // Business rule: Cannot delete organization with active members (other than owner)
        let member_count = self.member_repo.count_active_by_organization(id).await?;
        if member_count > 1 {
            return Err(DomainError::business_rule_violation(
                "Cannot delete organization with active members. Remove all members first.",
            ));
        }

        // Delete the organization (cascade will handle related entities)
        self.organization_repo.delete_by_id(id).await?;

        Ok(())
    }

    /// Get organization by ID
    async fn get_organization(&self, id: &Uuid) -> Result<Organization, DomainError> {
        self.organization_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &id.to_string()))
    }

    /// Get organization by slug
    async fn get_organization_by_slug(&self, slug: &str) -> Result<Organization, DomainError> {
        self.organization_repo
            .find_by_slug(slug)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", slug))
    }

    /// List organizations for a user
    async fn list_user_organizations(&self, user_id: &Uuid, page: u32, page_size: u32) -> Result<Vec<Organization>, DomainError> {
        self.organization_repo
            .find_by_user_membership(user_id, page, page_size)
            .await
    }

    /// Search organizations by name
    async fn search_organizations(&self, name_pattern: &str, _user_id: Option<Uuid>, page: u32, page_size: u32) -> Result<Vec<Organization>, DomainError> {
        self.organization_repo
            .search_by_name(name_pattern, page, page_size)
            .await
    }

    /// Check if user has admin permission in organization
    async fn check_admin_permission(
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

        // Owner always has admin permission
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
            "User does not have admin permission in this organization",
        ))
    }

    /// Check if user has read permission in organization
    async fn check_read_permission(
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

        // Owner always has read permission
        if organization.is_owned_by(user_id) {
            return Ok(());
        }

        // Check if user is an active member
        let member = self
            .member_repo
            .find_by_organization_and_user(organization_id, user_id)
            .await?;

        if let Some(member) = member {
            if member.is_active() {
                return Ok(());
            }
        }

        Err(DomainError::permission_denied(
            "User does not have permission to access this organization",
        ))
    }

    /// Create default system roles for a new organization
    async fn create_default_roles(&self, organization_id: &Uuid) -> Result<(), DomainError> {
        let system_roles = vec![
            SystemRole::Owner,
            SystemRole::Admin,
            SystemRole::Member,
            SystemRole::Viewer,
        ];

        for system_role in system_roles {
            let role = OrganizationRole::new_system_role(*organization_id, system_role);
            self.role_repo.save(&role).await?;
        }

        Ok(())
    }

    /// Add organization owner as the first member
    async fn add_owner_as_member(
        &self,
        organization: &Organization,
        owner_user_id: Uuid,
    ) -> Result<(), DomainError> {
        // Find the Owner role
        let owner_role = self
            .role_repo
            .find_by_organization_and_name(&organization.id, "Owner")
            .await?
            .ok_or_else(|| DomainError::internal_error("Owner role not found after creation"))?;

        // Create member record for the owner
        let member = OrganizationMember::new(organization.id, owner_user_id, owner_role.id);
        self.member_repo.save(&member).await?;

        Ok(())
    }
}
