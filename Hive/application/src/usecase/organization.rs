use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{service::OrganizationService, Organization};
use rustycog_core::error::DomainError;
use hive_events::{
    HiveDomainEvent, OrganizationCreatedEvent, OrganizationDeletedEvent, OrganizationUpdatedEvent,
};
use rustycog_events::EventPublisher;

use crate::{
    dto::{
        CreateOrganizationRequest, OrganizationListResponse, OrganizationResponse,
        OrganizationSearchRequest, PaginationRequest, PaginationResponse,
        UpdateOrganizationRequest,
    },
    ApplicationError,
};

#[async_trait::async_trait]
pub trait OrganizationUseCase: Send + Sync {

    /**
     * Create a new organization
     * 
     * @param request - The request to create the organization
     * @param user_id - The ID of the user creating the organization
     */
    async fn create_organization(
        &self,
        request: &CreateOrganizationRequest,
        user_id: Uuid,
    ) -> Result<OrganizationResponse, ApplicationError>;

    /**
     * Get an organization
     * 
     * @param organization_id - The ID of the organization
     * @param user_id - The ID of the user requesting the organization
     */
    async fn get_organization(
        &self,
        organization_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<OrganizationResponse, ApplicationError>;

    /**
     * Update an organization
     * 
     * @param organization_id - The ID of the organization
     * @param request - The request to update the organization
     * @param user_id - The ID of the user updating the organization
     */
    async fn update_organization(
        &self,
        organization_id: Uuid,
        request: &UpdateOrganizationRequest,
        user_id: Uuid,
    ) -> Result<OrganizationResponse, ApplicationError>;

    /**
     * Delete an organization
     * 
     * @param organization_id - The ID of the organization
     * @param user_id - The ID of the user deleting the organization
     */
    async fn delete_organization(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError>;

    /**
     * List organizations
     * 
     * @param user_id - The ID of the user listing the organizations
     * @param pagination - The pagination request
     */
    async fn list_organizations(
        &self,
        user_id: Uuid,
        pagination: &PaginationRequest,
    ) -> Result<OrganizationListResponse, ApplicationError>;

    /**
     * Search organizations
     * 
     * @param request - The request to search the organizations
     * @param user_id - The ID of the user searching the organizations
     */
    async fn search_organizations(
        &self,
        request: &OrganizationSearchRequest,
        user_id: Option<Uuid>,
    ) -> Result<OrganizationListResponse, ApplicationError>;
}

pub struct OrganizationUseCaseImpl {
    organization_service: Arc<dyn OrganizationService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
}

impl OrganizationUseCaseImpl {
    /// Create a new organization use case instance
    pub fn new(
        organization_service: Arc<dyn OrganizationService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            organization_service,
            event_publisher,
        }
    }

    /// Convert domain Organization to response DTO
    fn organization_to_response(&self, org: &Organization) -> OrganizationResponse {
        OrganizationResponse {
            id: org.id,
            name: org.name.clone(),
            slug: org.slug.clone(),
            description: org.description.clone(),
            avatar_url: org.avatar_url.clone(),
            owner_user_id: org.owner_user_id,
            settings: org.settings.clone(),
            created_at: org.created_at,
            updated_at: org.updated_at,
            member_count: Some(0),
            role_count: Some(0),
            is_owner: Some(false),
            user_role: None,
        }
    }

    /// Publish organization created event
    async fn publish_organization_created_event(
        &self,
        organization: &Organization,
    ) -> Result<(), ApplicationError> {
        let event = HiveDomainEvent::OrganizationCreated(OrganizationCreatedEvent::new(
            organization.id,
            organization.name.clone(),
            organization.slug.clone(),
            organization.owner_user_id,
            organization.created_at,
        ));

        self.event_publisher
            .publish(&event.into())
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }

    /// Publish organization updated event
    async fn publish_organization_updated_event(
        &self,
        organization: &Organization,
        request: &UpdateOrganizationRequest,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let updated_fields = vec![
            if request.name.is_some() {
                "name".to_string()
            } else {
                "".to_string()
            },
            if request.description.is_some() {
                "description".to_string()
            } else {
                "".to_string()
            },
            if request.avatar_url.is_some() {
                "avatar_url".to_string()
            } else {
                "".to_string()
            },
            if request.settings.is_some() {
                "settings".to_string()
            } else {
                "".to_string()
            },
        ]
        .into_iter()
        .filter(|field| *field != "".to_string())
        .collect::<Vec<String>>();

        let event = HiveDomainEvent::OrganizationUpdated(OrganizationUpdatedEvent::new(
            organization.id.clone(),
            organization.name.clone(),
            updated_fields,
            user_id.clone(),
            Utc::now(),
        ));

        self.event_publisher
            .publish(&event.into())
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }

    /// Publish organization deleted event
    async fn publish_organization_deleted_event(
        &self,
        organization: &Organization,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let event = HiveDomainEvent::OrganizationDeleted(OrganizationDeletedEvent::new(
            organization.id.clone(),
            organization.name.clone(),
            user_id.clone(),
            Utc::now(),
        ));

        self.event_publisher
            .publish(&event.into())
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl OrganizationUseCase for OrganizationUseCaseImpl {
    async fn create_organization(
        &self,
        request: &CreateOrganizationRequest,
        user_id: Uuid,
    ) -> Result<OrganizationResponse, ApplicationError> {
        // Create the organization
        let organization = Organization::new(
            request.name.clone(),
            request.slug.clone(),
            request.description.clone(),
            user_id,
        )
        .map_err(ApplicationError::Domain)?;

        let saved_org = self
            .organization_service
            .create_organization(&organization)
            .await
            .map_err(ApplicationError::Domain)?;

        // Publish domain event
        self.publish_organization_created_event(&saved_org).await?;

        Ok(self.organization_to_response(&saved_org))
    }

    async fn get_organization(
        &self,
        organization_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<OrganizationResponse, ApplicationError> {
        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(self.organization_to_response(&organization))
    }

    async fn update_organization(
        &self,
        organization_id: Uuid,
        request: &UpdateOrganizationRequest,
        user_id: Uuid,
    ) -> Result<OrganizationResponse, ApplicationError> {
        // Save the updated organization
        let updated_organization = self
            .organization_service
            .update_organization(
                organization_id.clone(),
                request.name.clone(),
                request.description.clone(),
                request.avatar_url.clone(),
                request.settings.clone(),
            )
            .await
            .map_err(ApplicationError::Domain)?;

        // Publish domain event
        self.publish_organization_updated_event(&updated_organization, request, user_id)
            .await?;

        Ok(self.organization_to_response(&updated_organization))
    }

    async fn delete_organization(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        // Get organization for event
        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Use domain service to delete organization
        self.organization_service
            .delete_organization(organization_id.clone())
            .await
            .map_err(ApplicationError::Domain)?;

        // Publish domain event
        self.publish_organization_deleted_event(&organization, user_id)
            .await?;

        Ok(())
    }

    async fn list_organizations(
        &self,
        user_id: Uuid,
        pagination: &PaginationRequest,
    ) -> Result<OrganizationListResponse, ApplicationError> {
        let organizations = self
            .organization_service
            .list_user_organizations(&user_id, pagination.page(), pagination.page_size())
            .await
            .map_err(ApplicationError::Domain)?;

        let total_count = organizations.len() as i64;
        let total_pages = (total_count as f64 / pagination.page_size() as f64).ceil() as u32;
        let pagination_response = PaginationResponse {
            current_page: pagination.page(),
            total_items: Some(total_count),
            has_next: total_pages > pagination.page(),
            has_previous: pagination.page() > 1,
            next_cursor: if total_pages > pagination.page() {
                Some((pagination.page() + 1).to_string())
            } else {
                None
            },
            previous_cursor: if pagination.page() > 1 {
                Some((pagination.page() - 1).to_string())
            } else {
                None
            },
            page_size: pagination.page_size(),
            total_pages: Some(total_pages),
        };

        let organizations: Vec<OrganizationResponse> = organizations
            .iter()
            .map(|org| self.organization_to_response(org))
            .collect();

        Ok(OrganizationListResponse {
            organizations,
            pagination: pagination_response,
        })
    }

    async fn search_organizations(
        &self,
        request: &OrganizationSearchRequest,
        user_id: Option<Uuid>,
    ) -> Result<OrganizationListResponse, ApplicationError> {
        let organizations = self
            .organization_service
            .search_organizations(
                &request.query,
                user_id,
                request.page.unwrap_or(1) as u32,
                request.page_size.unwrap_or(10) as u32,
            )
            .await
            .map_err(ApplicationError::Domain)?;

        let total_count = organizations.len() as i64;
        let total_pages =
            (total_count as f64 / request.page_size.unwrap_or(10) as f64).ceil() as u32;
        let pagination_response = PaginationResponse {
            current_page: request.page.unwrap_or(1),
            total_items: Some(total_count),
            has_next: total_pages > request.page.unwrap_or(1),
            has_previous: request.page.unwrap_or(1) > 1,
            next_cursor: if total_pages > request.page.unwrap_or(1) {
                Some(((request.page.unwrap_or(1) + 1) as i64).to_string())
            } else {
                None
            },
            previous_cursor: if request.page.unwrap_or(1) > 1 {
                Some(((request.page.unwrap_or(1) - 1) as i64).to_string())
            } else {
                None
            },
            page_size: request.page_size.unwrap_or(10),
            total_pages: Some(total_pages),
        };
        let organizations: Vec<OrganizationResponse> = organizations
            .iter()
            .map(|org| self.organization_to_response(org))
            .collect();

        Ok(OrganizationListResponse {
            organizations,
            pagination: pagination_response,
        })
    }
}
