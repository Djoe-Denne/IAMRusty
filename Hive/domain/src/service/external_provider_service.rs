use uuid::Uuid;

use crate::{
    entity::*,
    error::DomainError,
    port::{repository::*, service::*},
    service::{
        organization_service::OrganizationService,
        role_service::RoleService,
        sync_service::{SyncService, SyncResult},
    },
};

/// Domain service for external provider integration
pub struct ExternalProviderServiceImpl<OR, ELR, EPR, OS, RS, SS, PC>
where
    OR: OrganizationRepository,
    ELR: ExternalLinkRepository,
    EPR: ExternalProviderRepository,
    OS: OrganizationService,
    RS: RoleService,
    SS: SyncService,
    PC: ExternalProviderClient,
{
    organization_repo: OR,
    external_link_repo: ELR,
    external_provider_repo: EPR,
    organization_service: OS,
    role_service: RS,
    sync_service: SS,
    provider_client: PC,
}

#[async_trait::async_trait]
pub trait ExternalProviderService: Send + Sync {
    /**
     * Link an organization to an external provider
     * 
     * @param organization_id - The ID of the organization to link
     * @param provider_id - The ID of the external provider
     * @param provider_config - Configuration for the external provider connection
     * @param requesting_user_id - The ID of the user making the request
     */
    async fn link_organization(
        &self,
        organization_id: Uuid,
        provider_id: Uuid,
        provider_config: &serde_json::Value,
        requesting_user_id: Uuid,
    ) -> Result<ExternalLink, DomainError>;

    /**
     * Unlink an organization from an external provider
     * 
     * @param organization_id - The ID of the organization to unlink
     * @param provider_id - The ID of the external provider to unlink
     * @param requesting_user_id - The ID of the user making the request
     */
    async fn unlink_organization(
        &self,
        organization_id: Uuid,
        provider_id: Uuid,
        requesting_user_id: Uuid,
    ) -> Result<(), DomainError>;

    /**
     * Test connection to external provider
     * 
     * @param provider_id - The ID of the external provider
     * @param provider_config - Configuration for the connection
     */
    async fn test_connection(
        &self,
        provider_id: Uuid,
        provider_config: &serde_json::Value,
    ) -> Result<bool, DomainError>;

    /**
     * Get external link by organization and provider
     * 
     * @param organization_id - The ID of the organization
     * @param provider_id - The ID of the provider
     */
    async fn get_external_link(
        &self,
        organization_id: Uuid,
        provider_id: Uuid,
    ) -> Result<ExternalLink, DomainError>;

    /**
     * List external links for an organization
     * 
     * @param organization_id - The ID of the organization
     */
    async fn list_organization_links(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<ExternalLink>, DomainError>;
}

impl<OR, ELR, EPR, OS, RS, SS, PC> ExternalProviderServiceImpl<OR, ELR, EPR, OS, RS, SS, PC>
where
    OR: OrganizationRepository,
    ELR: ExternalLinkRepository,
    EPR: ExternalProviderRepository,
    OS: OrganizationService,
    RS: RoleService,
    SS: SyncService,
    PC: ExternalProviderClient,
{
    /// Create a new external provider service
    pub fn new(
        organization_repo: OR,
        external_link_repo: ELR,
        external_provider_repo: EPR,
        organization_service: OS,
        role_service: RS,
        sync_service: SS,
        provider_client: PC,
    ) -> Self {
        Self {
            organization_repo,
            external_link_repo,
            external_provider_repo,
            organization_service,
            role_service,
            sync_service,
            provider_client,
        }
    }
}

#[async_trait::async_trait]
impl<OR, ELR, EPR, OS, RS, SS, PC> ExternalProviderService for ExternalProviderServiceImpl<OR, ELR, EPR, OS, RS, SS, PC>
where
    OR: OrganizationRepository,
    ELR: ExternalLinkRepository,
    EPR: ExternalProviderRepository,
    OS: OrganizationService,
    RS: RoleService,
    SS: SyncService,
    PC: ExternalProviderClient,
{
    /// Link an organization to an external provider
    async fn link_organization(
        &self,
        organization_id: Uuid,
        provider_id: Uuid,
        provider_config: &serde_json::Value,
        requesting_user_id: Uuid,
    ) -> Result<ExternalLink, DomainError> {
        // Validate organization exists
        let organization = self
            .organization_repo
            .find_by_id(&organization_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        // Check if user has permission to manage external links
        self.role_service
            .check_admin_permission(&organization_id, &requesting_user_id, "organization")
            .await?;

        // Validate provider exists
        let provider = self
            .external_provider_repo
            .find_by_id(&provider_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExternalProvider", &provider_id.to_string()))?;

        // Check if link already exists
        if let Some(_existing_link) = self
            .external_link_repo
            .find_by_organization_and_provider(&organization_id, &provider_id)
            .await?
        {
            return Err(DomainError::resource_already_exists(
                "ExternalLink",
                &format!("organization_id={}, provider_id={}", organization_id, provider_id),
            ));
        }

        // Create new external link
        let external_link = ExternalLink::new(
            organization_id,
            Some(organization.name),
            provider_id,
            Some(provider.name),
            provider_config.clone(),
            Some(serde_json::json!({})), // TODO: Default sync settings
        )?;

        // Save the link
        let saved_link = self.external_link_repo.save(&external_link).await?;

        Ok(saved_link)
    }

    /// Unlink an organization from an external provider
    async fn unlink_organization(
        &self,
        organization_id: Uuid,
        provider_id: Uuid,
        requesting_user_id: Uuid,
    ) -> Result<(), DomainError> {
        // Check if user has permission to manage external links
        self.role_service
            .check_admin_permission(&organization_id, &requesting_user_id, "organization")
            .await?;

        // Find the external link
        let external_link = self
            .external_link_repo
            .find_by_organization_and_provider(&organization_id, &provider_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "ExternalLink",
                    &format!("organization_id={}, provider_id={}", organization_id, provider_id),
                )
            })?;

        // Delete the link
        self.external_link_repo.delete_by_id(&external_link.id).await?;

        Ok(())
    }

    /// Test connection to external provider
    async fn test_connection(
        &self,
        provider_id: Uuid,
        provider_config: &serde_json::Value,
    ) -> Result<bool, DomainError> {
        // Validate provider exists
        let _provider = self
            .external_provider_repo
            .find_by_id(&provider_id)
            .await?
            .ok_or_else(|| DomainError::entity_not_found("ExternalProvider", &provider_id.to_string()))?;

        // Validate configuration
        self.provider_client.validate_config(provider_config).await?;

        // Test connection
        let connection_ok = self.provider_client.test_connection(provider_config).await?;

        Ok(connection_ok)
    }

    /// Get external link by organization and provider
    async fn get_external_link(
        &self,
        organization_id: Uuid,
        provider_id: Uuid,
    ) -> Result<ExternalLink, DomainError> {
        self.external_link_repo
            .find_by_organization_and_provider(&organization_id, &provider_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "ExternalLink",
                    &format!("organization_id={}, provider_id={}", organization_id, provider_id),
                )
            })
    }

    /// List external links for an organization
    async fn list_organization_links(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<ExternalLink>, DomainError> {
        let organization = self.organization_repo.find_by_id(&organization_id).await?
            .ok_or_else(|| DomainError::entity_not_found("Organization", &organization_id.to_string()))?;

        let mut links = self.external_link_repo
            .find_by_organization(&organization_id)
            .await?;

        for link in links.iter_mut() {
            link.set_organization_name(organization.name.clone());
            let provider = self.external_provider_repo.find_by_id(&link.provider_id).await?
                .ok_or_else(|| DomainError::entity_not_found("ExternalProvider", &link.provider_id.to_string()))?;
            link.set_provider_name(provider.name.clone());
        }

        Ok(links)
    }
} 