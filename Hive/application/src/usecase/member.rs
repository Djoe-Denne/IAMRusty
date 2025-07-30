use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{
    port::repository::{OrganizationMemberRepository, OrganizationRepository},
    service::MemberService,
    DomainError, Organization, OrganizationMember, OrganizationRole,
};
use hive_events::{event_types, MemberJoinedEvent, MemberRemovedEvent};
use rustycog_events::{DomainEvent, MultiQueueEventPublisher};

use crate::{
    dto::{
        AddMemberRequest, MemberListResponse, MemberResponse, PaginationRequest,
        UpdateMemberRequest,
    },
    ApplicationError,
};

/// Use case trait for member operations
#[async_trait]
pub trait MemberUseCase: Send + Sync {
    /// Add a member to an organization
    async fn add_member(
        &self,
        organization_id: Uuid,
        request: AddMemberRequest,
        added_by_user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    /// Remove a member from an organization
    async fn remove_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        removed_by_user_id: Uuid,
    ) -> Result<(), ApplicationError>;

    /// List organization members
    async fn list_members(
        &self,
        organization_id: Uuid,
        pagination: PaginationRequest,
        requesting_user_id: Uuid,
    ) -> Result<MemberListResponse, ApplicationError>;

    /// Get a specific member
    async fn get_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        requesting_user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;
}

/// Implementation of member use case
pub struct MemberUseCaseImpl {
    member_service: Arc<dyn MemberService>,
    member_repository: Arc<dyn OrganizationMemberRepository>,
    organization_repository: Arc<dyn OrganizationRepository>,
    event_publisher: Arc<MultiQueueEventPublisher<DomainError>>,
}

impl MemberUseCaseImpl {
    /// Create a new member use case instance
    pub fn new(
        member_service: Arc<dyn MemberService>,
        member_repository: Arc<dyn OrganizationMemberRepository>,
        organization_repository: Arc<dyn OrganizationRepository>,
        event_publisher: Arc<MultiQueueEventPublisher<DomainError>>,
    ) -> Self {
        Self {
            member_service,
            member_repository,
            organization_repository,
            event_publisher,
        }
    }

    /// Convert domain OrganizationMember to response DTO
    fn member_to_response(&self, member: &OrganizationMember) -> MemberResponse {
        MemberResponse {
            id: member.id(),
            organization_id: member.organization_id(),
            user_id: member.user_id(),
            role_id: member.role_id(),
            status: member.status().to_string(),
            joined_at: member.joined_at(),
            invited_by_user_id: member.invited_by_user_id(),
            invited_at: member.invited_at(),
            created_at: member.created_at(),
            updated_at: member.updated_at(),
        }
    }

    /// Publish member joined event
    async fn publish_member_joined_event(
        &self,
        member: &OrganizationMember,
        organization_name: &str,
        role_name: &str,
    ) -> Result<(), ApplicationError> {
        let event = MemberJoinedEvent {
            organization_id: member.organization_id(),
            organization_name: organization_name.to_string(),
            user_id: member.user_id(),
            role_name: role_name.to_string(),
            joined_at: member.joined_at().unwrap_or_else(|| Utc::now()),
        };

        let domain_event: Box<dyn DomainEvent> = Box::new(rustycog_events::event::Event::new(
            event_types::MEMBER_JOINED,
            serde_json::to_value(event).map_err(|e| {
                ApplicationError::internal_error(&format!("Failed to serialize event: {}", e))
            })?,
            member.organization_id(),
        ));

        self.event_publisher
            .publish(&domain_event)
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }

    /// Publish member removed event
    async fn publish_member_removed_event(
        &self,
        organization_id: Uuid,
        organization_name: &str,
        user_id: Uuid,
        user_email: &str,
        removed_by_user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let event = MemberRemovedEvent {
            organization_id,
            organization_name: organization_name.to_string(),
            user_id,
            user_email: user_email.to_string(),
            removed_by_user_id,
            removed_at: Utc::now(),
        };

        let domain_event: Box<dyn DomainEvent> = Box::new(rustycog_events::event::Event::new(
            event_types::MEMBER_REMOVED,
            serde_json::to_value(event).map_err(|e| {
                ApplicationError::internal_error(&format!("Failed to serialize event: {}", e))
            })?,
            organization_id,
        ));

        self.event_publisher
            .publish(&domain_event)
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(())
    }
}

#[async_trait]
impl MemberUseCase for MemberUseCaseImpl {
    async fn add_member(
        &self,
        organization_id: Uuid,
        request: AddMemberRequest,
        added_by_user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        // Get organization for validation and events
        let organization = self
            .organization_repository
            .find_by_id(organization_id)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| {
                ApplicationError::Domain(DomainError::EntityNotFound {
                    entity_type: "Organization".to_string(),
                    id: organization_id.to_string(),
                })
            })?;

        // Use domain service to add member
        let member = self
            .member_service
            .add_member(
                organization_id,
                request.user_id,
                request.role_id,
                added_by_user_id,
            )
            .await
            .map_err(ApplicationError::Domain)?;

        // Publish domain event
        // TODO: Get role name from role repository
        self.publish_member_joined_event(&member, organization.name(), "Member")
            .await?;

        Ok(self.member_to_response(&member))
    }

    async fn remove_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        removed_by_user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        // Get organization for validation and events
        let organization = self
            .organization_repository
            .find_by_id(organization_id)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| {
                ApplicationError::Domain(DomainError::EntityNotFound {
                    entity_type: "Organization".to_string(),
                    id: organization_id.to_string(),
                })
            })?;

        // Use domain service to remove member
        self.member_service
            .remove_member(organization_id, user_id, removed_by_user_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Publish domain event
        // TODO: Get user email from IAM service
        self.publish_member_removed_event(
            organization_id,
            organization.name(),
            user_id,
            "user@example.com", // Placeholder
            removed_by_user_id,
        )
        .await?;

        Ok(())
    }

    async fn list_members(
        &self,
        organization_id: Uuid,
        pagination: PaginationRequest,
        requesting_user_id: Uuid,
    ) -> Result<MemberListResponse, ApplicationError> {
        // TODO: Add permission check

        let (members, total_count) = self
            .member_repository
            .find_by_organization_id(organization_id, pagination.page(), pagination.page_size())
            .await
            .map_err(ApplicationError::Domain)?;

        let members: Vec<MemberResponse> = members
            .iter()
            .map(|member| self.member_to_response(member))
            .collect();

        let total_pages =
            (total_count.unwrap_or(0) as f64 / pagination.page_size() as f64).ceil() as u32;
        let has_next = pagination.page() < total_pages;

        Ok(MemberListResponse {
            members,
            pagination: crate::dto::PaginationResponse {
                current_page: pagination.page(),
                total_items: total_count,
                has_next,
                has_previous: pagination.page() > 1,
                next_cursor: if has_next {
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
            },
        })
    }

    async fn get_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        requesting_user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        // TODO: Add permission check

        let member = self
            .member_repository
            .find_by_organization_and_user(organization_id, user_id)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| {
                ApplicationError::Domain(DomainError::EntityNotFound {
                    entity_type: "OrganizationMember".to_string(),
                    id: format!("{}:{}", organization_id, user_id),
                })
            })?;

        Ok(self.member_to_response(&member))
    }
}
