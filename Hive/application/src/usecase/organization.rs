use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{
    service::{MemberService, OrganizationService},
    Organization,
};
use hive_events::{
    HiveDomainEvent, OrganizationCreatedEvent, OrganizationDeletedEvent, OrganizationUpdatedEvent,
};
use rustycog::core::error::DomainError;
use rustycog::events::{DomainEvent, EventPublisher};

use crate::{
    dto::{
        CreateOrganizationRequest, OrganizationListResponse, OrganizationResponse,
        OrganizationSearchRequest, PaginationRequest, PaginationResponse,
        UpdateOrganizationRequest,
    },
    ApplicationError, HiveOutboxUnitOfWork,
};

#[async_trait::async_trait]
pub trait OrganizationUseCase: Send + Sync {
    /**
     * Create a new organization
     *
     * @param request - The request to create the organization
     * @param `user_id` - The ID of the user creating the organization
     */
    async fn create_organization(
        &self,
        request: &CreateOrganizationRequest,
        user_id: Uuid,
    ) -> Result<OrganizationResponse, ApplicationError>;

    /**
     * Get an organization
     *
     * @param `organization_id` - The ID of the organization
     * @param `user_id` - The ID of the user requesting the organization
     */
    async fn get_organization(
        &self,
        organization_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<OrganizationResponse, ApplicationError>;

    /**
     * Update an organization
     *
     * @param `organization_id` - The ID of the organization
     * @param request - The request to update the organization
     * @param `user_id` - The ID of the user updating the organization
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
     * @param `organization_id` - The ID of the organization
     * @param `user_id` - The ID of the user deleting the organization
     */
    async fn delete_organization(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError>;

    /**
     * List organizations
     *
     * @param `user_id` - The ID of the user listing the organizations
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
     * @param `user_id` - The ID of the user searching the organizations
     */
    async fn search_organizations(
        &self,
        request: &OrganizationSearchRequest,
        user_id: Option<Uuid>,
    ) -> Result<OrganizationListResponse, ApplicationError>;
}

pub struct OrganizationUseCaseImpl {
    organization_service: Arc<dyn OrganizationService>,
    member_service: Arc<dyn MemberService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
    outbox_unit_of_work: Option<Arc<dyn HiveOutboxUnitOfWork>>,
}

impl OrganizationUseCaseImpl {
    /// Create a new organization use case instance
    pub fn new(
        organization_service: Arc<dyn OrganizationService>,
        member_service: Arc<dyn MemberService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            organization_service,
            member_service,
            event_publisher,
            outbox_unit_of_work: None,
        }
    }

    pub fn new_with_outbox_unit_of_work(
        organization_service: Arc<dyn OrganizationService>,
        member_service: Arc<dyn MemberService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
        outbox_unit_of_work: Arc<dyn HiveOutboxUnitOfWork>,
    ) -> Self {
        Self {
            organization_service,
            member_service,
            event_publisher,
            outbox_unit_of_work: Some(outbox_unit_of_work),
        }
    }

    async fn record_or_publish_event(
        &self,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError> {
        if let Some(outbox_unit_of_work) = &self.outbox_unit_of_work {
            outbox_unit_of_work.record_event(event).await
        } else {
            self.event_publisher
                .publish(event.as_ref())
                .await
                .map_err(ApplicationError::Domain)
        }
    }

    /// Convert domain Organization to response DTO
    fn organization_to_response(org: &Organization) -> OrganizationResponse {
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

        self.record_or_publish_event(event.into()).await
    }

    fn organization_updated_event(
        organization: &Organization,
        request: &UpdateOrganizationRequest,
        user_id: Uuid,
    ) -> HiveDomainEvent {
        let updated_fields = vec![
            if request.name.is_some() {
                "name".to_string()
            } else {
                String::new()
            },
            if request.description.is_some() {
                "description".to_string()
            } else {
                String::new()
            },
            if request.avatar_url.is_some() {
                "avatar_url".to_string()
            } else {
                String::new()
            },
            if request.settings.is_some() {
                "settings".to_string()
            } else {
                String::new()
            },
        ]
        .into_iter()
        .filter(|field| *field != String::new())
        .collect::<Vec<String>>();

        HiveDomainEvent::OrganizationUpdated(OrganizationUpdatedEvent::new(
            organization.id,
            organization.name.clone(),
            updated_fields,
            user_id,
            Utc::now(),
        ))
    }

    /// Publish organization updated event
    async fn publish_organization_updated_event(
        &self,
        organization: &Organization,
        request: &UpdateOrganizationRequest,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.record_or_publish_event(
            Self::organization_updated_event(organization, request, user_id)
                .into(),
        )
        .await
    }

    /// Publish organization deleted event
    async fn publish_organization_deleted_event(
        &self,
        organization: &Organization,
        member_user_ids: Vec<Uuid>,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let event = HiveDomainEvent::OrganizationDeleted(OrganizationDeletedEvent::new(
            organization.id,
            organization.name.clone(),
            organization.owner_user_id,
            member_user_ids,
            user_id,
            Utc::now(),
        ));

        self.record_or_publish_event(event.into()).await
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

        let saved_org = if let Some(outbox_unit_of_work) = &self.outbox_unit_of_work {
            let event = HiveDomainEvent::OrganizationCreated(OrganizationCreatedEvent::new(
                organization.id,
                organization.name.clone(),
                organization.slug.clone(),
                organization.owner_user_id,
                organization.created_at,
            ));
            outbox_unit_of_work
                .create_organization(organization, event.into())
                .await?
        } else {
            let saved_org = self
                .organization_service
                .create_organization(&organization)
                .await
                .map_err(ApplicationError::Domain)?;
            self.publish_organization_created_event(&saved_org).await?;
            saved_org
        };

        Ok(Self::organization_to_response(&saved_org))
    }

    async fn get_organization(
        &self,
        organization_id: Uuid,
        _user_id: Option<Uuid>,
    ) -> Result<OrganizationResponse, ApplicationError> {
        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(Self::organization_to_response(&organization))
    }

    async fn update_organization(
        &self,
        organization_id: Uuid,
        request: &UpdateOrganizationRequest,
        user_id: Uuid,
    ) -> Result<OrganizationResponse, ApplicationError> {
        let updated_organization = if let Some(outbox_unit_of_work) = &self.outbox_unit_of_work {
            let mut organization = self
                .organization_service
                .get_organization(&organization_id)
                .await
                .map_err(ApplicationError::Domain)?;
            if let Some(new_name) = request.name.clone() {
                organization.update_name(new_name)?;
            }
            if let Some(new_description) = request.description.clone() {
                organization.update_description(Some(new_description));
            }
            if let Some(new_avatar_url) = request.avatar_url.clone() {
                organization.update_avatar_url(Some(new_avatar_url));
            }
            if let Some(new_settings) = request.settings.clone() {
                organization.update_settings(new_settings);
            }
            let event = Self::organization_updated_event(&organization, request, user_id);
            outbox_unit_of_work
                .update_organization(organization, event.into())
                .await?
        } else {
            let updated_organization = self
                .organization_service
                .update_organization(
                    organization_id,
                    request.name.clone(),
                    request.description.clone(),
                    request.avatar_url.clone(),
                    request.settings.clone(),
                )
                .await
                .map_err(ApplicationError::Domain)?;
            self.publish_organization_updated_event(&updated_organization, request, user_id)
                .await?;
            updated_organization
        };

        Ok(Self::organization_to_response(&updated_organization))
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

        let members = self
            .member_service
            .list_active_members(organization_id)
            .await
            .map_err(ApplicationError::Domain)?;
        let member_user_ids = members.iter().map(|member| member.user_id).collect();

        if let Some(outbox_unit_of_work) = &self.outbox_unit_of_work {
            let event = HiveDomainEvent::OrganizationDeleted(OrganizationDeletedEvent::new(
                organization.id,
                organization.name.clone(),
                organization.owner_user_id,
                member_user_ids,
                user_id,
                Utc::now(),
            ));
            outbox_unit_of_work
                .delete_organization(organization_id, event.into())
                .await?;
        } else {
            self.organization_service
                .delete_organization(organization_id)
                .await
                .map_err(ApplicationError::Domain)?;
            self.publish_organization_deleted_event(&organization, member_user_ids, user_id)
                .await?;
        }

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

        let total_count = i64::try_from(organizations.len()).unwrap_or(i64::MAX);
        let page = pagination.page();
        let mut pagination_response =
            PaginationResponse::new(page, pagination.page_size(), Some(total_count));
        if pagination_response.has_next {
            pagination_response.next_cursor = Some((page + 1).to_string());
        }
        if pagination_response.has_previous {
            pagination_response.previous_cursor = Some((page - 1).to_string());
        }

        let organizations: Vec<OrganizationResponse> = organizations
            .iter()
            .map(Self::organization_to_response)
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
                request.page.unwrap_or(1),
                request.page_size.unwrap_or(10),
            )
            .await
            .map_err(ApplicationError::Domain)?;

        let total_count = i64::try_from(organizations.len()).unwrap_or(i64::MAX);
        let page = request.page.unwrap_or(1);
        let mut pagination_response =
            PaginationResponse::new(page, request.page_size.unwrap_or(10), Some(total_count));
        if pagination_response.has_next {
            pagination_response.next_cursor = Some((page + 1).to_string());
        }
        if pagination_response.has_previous {
            pagination_response.previous_cursor = Some((page - 1).to_string());
        }
        let organizations: Vec<OrganizationResponse> = organizations
            .iter()
            .map(Self::organization_to_response)
            .collect();

        Ok(OrganizationListResponse {
            organizations,
            pagination: pagination_response,
        })
    }
}
