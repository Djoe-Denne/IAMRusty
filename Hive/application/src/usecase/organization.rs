use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{service::OrganizationService, DomainError, Organization};
use hive_events::{
    HiveDomainEvent, OrganizationCreatedEvent, OrganizationDeletedEvent, OrganizationUpdatedEvent,
};
use rustycog_events::{EventPublisher, MultiQueueEventPublisher};

use crate::{
    dto::{
        CreateOrganizationRequest, OrganizationListResponse, OrganizationResponse,
        OrganizationSearchRequest, PaginationRequest, PaginationResponse,
        UpdateOrganizationRequest,
    },
    ApplicationError,
};

pub struct OrganizationUseCase {
    organization_service: Arc<dyn OrganizationService>,
    role_service: Arc<dyn RoleService>,
    event_publisher: Arc<MultiQueueEventPublisher<DomainError>>,
}

impl OrganizationUseCase {
    /// Create a new organization use case instance
    pub fn new(
        organization_service: Arc<dyn OrganizationService>,
        role_service: Arc<dyn RoleService>,
        event_publisher: Arc<MultiQueueEventPublisher<DomainError>>,
    ) -> Self {
        Self {
            organization_service,
            role_service,
            event_publisher,
        }
    }

    /// Convert domain Organization to response DTO
    fn organization_to_response(&self, org: &Organization) -> OrganizationResponse {
        OrganizationResponse {
            id: org.id(),
            name: org.name().to_string(),
            slug: org.slug().to_string(),
            description: org.description().map(|d| d.to_string()),
            avatar_url: org.avatar_url().map(|url| url.to_string()),
            owner_user_id: org.owner_user_id(),
            settings: org.settings().clone(),
            created_at: org.created_at(),
            updated_at: org.updated_at(),
            member_count: org.member_count(),
            role_count: org.role_count(),
            is_owner: org.is_owner(),
            user_role: org.user_role(),
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
        request: UpdateOrganizationRequest,
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
            organization.id,
            organization.name.clone(),
            updated_fields,
            user_id,
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
            organization.id,
            organization.name.clone(),
            user_id,
            Utc::now(),
        ));

        self.event_publisher
            .publish(&event.into())
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }

    pub async fn create_organization(
        &self,
        request: CreateOrganizationRequest,
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

    pub async fn get_organization(
        &self,
        organization_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<OrganizationResponse, ApplicationError> {
        let _authorized = self
            .role_service
            .check_read_permission(&organization_id, &user_id)
            .await
            .map_err(ApplicationError::Domain)?;

        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(self.organization_to_response(&organization))
    }

    pub async fn update_organization(
        &self,
        organization_id: Uuid,
        request: UpdateOrganizationRequest,
        user_id: Uuid,
    ) -> Result<OrganizationResponse, ApplicationError> {
        let _authorized = self
            .role_service
            .check_admin_permission(&organization_id, &user_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Get existing organization
        let mut organization: Organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Save the updated organization
        let updated_organization = self
            .organization_service
            .update_organization(
                &organization_id,
                request.name,
                request.description,
                request.avatar_url,
                request.settings,
                &user_id,
            )
            .await
            .map_err(ApplicationError::Domain)?;

        // Publish domain event
        self.publish_organization_updated_event(&updated_organization, request, user_id)
            .await?;

        Ok(self.organization_to_response(&updated_organization))
    }

    pub async fn delete_organization(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        // Get organization for event
        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| {
                ApplicationError::Domain(DomainError::EntityNotFound {
                    entity_type: "Organization".to_string(),
                    id: organization_id.to_string(),
                })
            })?;

        // Use domain service to delete organization
        self.organization_service
            .delete_organization(organization_id, user_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Publish domain event
        self.publish_organization_deleted_event(&organization, user_id)
            .await?;

        Ok(())
    }

    // TODO: check if both list and search are using the same permission check
    pub async fn list_organizations(
        &self,
        user_id: Uuid,
        pagination: PaginationRequest,
    ) -> Result<OrganizationListResponse, ApplicationError> {
        let organizations = self
            .organization_service
            .list_user_organizations(user_id, pagination.page(), pagination.page_size())
            .await
            .map_err(ApplicationError::Domain)?;

        let total_count = organizations.len();
        let total_pages = (total_count as f64 / pagination.page_size() as f64).ceil() as u32;
        let pagination_response = PaginationResponse {
            current_page: pagination.page(),
            total_items: total_count,
            has_next: total_pages > pagination.page(),
            has_previous: pagination.page() > 1,
            next_cursor: if total_pages > pagination.page() {
                Some(pagination.page() + 1)
            } else {
                None
            },
            previous_cursor: if pagination.page() > 1 {
                Some(pagination.page() - 1)
            } else {
                None
            },
            page_size: pagination.page_size(),
            total_pages,
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

    pub async fn search_organizations(
        &self,
        request: OrganizationSearchRequest,
        user_id: Option<Uuid>,
    ) -> Result<OrganizationListResponse, ApplicationError> {
        let organizations = self
            .organization_service
            .search_organizations(
                request.query.as_deref(),
                user_id,
                request.pagination.page(),
                request.pagination.page_size(),
            )
            .await
            .map_err(ApplicationError::Domain)?;

        let total_count = organizations.len();
        let total_pages =
            (total_count as f64 / request.pagination.page_size() as f64).ceil() as u32;
        let pagination_response = PaginationResponse {
            current_page: request.pagination.page(),
            total_items: total_count,
            has_next: total_pages > request.pagination.page(),
            has_previous: request.pagination.page() > 1,
            next_cursor: if total_pages > request.pagination.page() {
                Some(request.pagination.page() + 1)
            } else {
                None
            },
            previous_cursor: if request.pagination.page() > 1 {
                Some(request.pagination.page() - 1)
            } else {
                None
            },
            page_size: request.pagination.page_size(),
            total_pages,
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
