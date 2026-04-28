use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    entity::Organization,
    port::OrganizationRepository,
    service::{member_service::MemberService, role_service::RoleService},
};
use rustycog_core::error::DomainError;

/// Domain service for organization management
pub struct OrganizationServiceImpl<OR, MS, RS>
where
    OR: OrganizationRepository,
    MS: MemberService,
    RS: RoleService,
{
    organization_repo: Arc<OR>,
    member_service: Arc<MS>,
    role_service: Arc<RS>,
}

#[async_trait::async_trait]
pub trait OrganizationService: Send + Sync {
    /**
     * Create a new organization with default roles
     *
     * @param organization - The organization to create
     */
    async fn create_organization(
        &self,
        organization: &Organization,
    ) -> Result<Organization, DomainError>;

    /**
     * Update an organization
     *
     * @param id - The ID of the organization to update
     */
    async fn update_organization(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        avatar_url: Option<String>,
        settings: Option<Value>,
    ) -> Result<Organization, DomainError>;

    /**
     * Delete an organization
     *
     * @param id - The ID of the organization to delete
     */
    async fn delete_organization(&self, id: Uuid) -> Result<(), DomainError>;

    /**
     * Get an organization by ID
     *
     * @param id - The ID of the organization to get
     */
    async fn get_organization(&self, id: &Uuid) -> Result<Organization, DomainError>;

    /**
     * Get an organization by slug
     *
     * @param slug - The slug of the organization to get
     */
    async fn get_organization_by_slug(&self, slug: &str) -> Result<Organization, DomainError>;

    /**
     * List organizations for a user
     *
     * @param `user_id` - The ID of the user to list the organizations for
     */
    async fn list_user_organizations(
        &self,
        user_id: &Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError>;

    /**
     * Search organizations by name
     *
     * @param `name_pattern` - The pattern to search for
     */
    async fn search_organizations(
        &self,
        name_pattern: &str,
        _user_id: Option<Uuid>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError>;
}

impl<OR, MS, RS> OrganizationServiceImpl<OR, MS, RS>
where
    OR: OrganizationRepository,
    MS: MemberService,
    RS: RoleService,
{
    /// Create a new organization service
    pub const fn new(
        organization_repo: Arc<OR>,
        member_service: Arc<MS>,
        role_service: Arc<RS>,
    ) -> Self {
        Self {
            organization_repo,
            member_service,
            role_service,
        }
    }
}

#[async_trait::async_trait]
impl<OR, MS, RS> OrganizationService for OrganizationServiceImpl<OR, MS, RS>
where
    OR: OrganizationRepository,
    MS: MemberService,
    RS: RoleService,
{
    /// Create a new organization with default roles
    async fn create_organization(
        &self,
        organization: &Organization,
    ) -> Result<Organization, DomainError> {
        // Business rule: Check if organization with same slug already exists
        if self
            .organization_repo
            .exists_by_slug(&organization.slug)
            .await?
        {
            return Err(DomainError::resource_already_exists(
                "Organization",
                &format!("slug={}", organization.slug),
            ));
        }

        let saved_org = self.organization_repo.save(organization).await?;

        // Create default system roles for the organization
        let default_roles = self
            .role_service
            .create_default_roles(&saved_org.id)
            .await?;

        let owner_role_permission = self
            .role_service
            .find_role_permissions("organization", "owner", default_roles)
            .await?;

        // Add owner as the first member with Owner role
        self.member_service
            .add_member(
                saved_org.id,
                organization.owner_user_id,
                vec![owner_role_permission],
                None,
            )
            .await?;

        Ok(saved_org)
    }

    /// Update organization details
    async fn update_organization(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        avatar_url: Option<String>,
        settings: Option<Value>,
    ) -> Result<Organization, DomainError> {
        // Find the organization
        let mut organization = self
            .organization_repo
            .find_by_id(&id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &id.to_string()))?;

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
    async fn delete_organization(&self, id: Uuid) -> Result<(), DomainError> {
        // Find the organization
        let organization = self
            .organization_repo
            .find_by_id(&id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &id.to_string()))?;

        // Delete all members
        self.member_service
            .remove_organization_members(organization.id)
            .await?;

        // Delete all roles
        self.role_service.delete_organization_roles(&id).await?;

        // Delete the organization (cascade will handle related entities)
        self.organization_repo.delete_by_id(&id).await?;

        Ok(())
    }

    /// Get organization by ID
    async fn get_organization(&self, id: &Uuid) -> Result<Organization, DomainError> {
        let organization = self
            .organization_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &id.to_string()))?;

        Ok(organization)
    }

    /// Get organization by slug
    async fn get_organization_by_slug(&self, slug: &str) -> Result<Organization, DomainError> {
        self.organization_repo
            .find_by_slug(slug)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", slug))
    }

    /// List organizations for a user
    async fn list_user_organizations(
        &self,
        user_id: &Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        self.organization_repo
            .find_by_user_membership(user_id, page, page_size)
            .await
    }

    /// Search organizations by name
    async fn search_organizations(
        &self,
        name_pattern: &str,
        user_id: Option<Uuid>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Organization>, DomainError> {
        self.organization_repo
            .search_by_name(user_id, name_pattern, page, page_size)
            .await
    }
}
