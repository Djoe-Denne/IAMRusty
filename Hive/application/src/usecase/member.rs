use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{
    entity::RolePermission,
    service::{MemberService, OrganizationService},
    OrganizationMember,
};
use hive_events::{HiveDomainEvent, MemberJoinedEvent, MemberRemovedEvent, Role};
use rustycog_core::error::DomainError;
use rustycog_events::{DomainEvent, EventPublisher};

use crate::{
    dto::{
        AddMemberRequest, MemberListResponse, MemberResponse, PaginationRequest,
        UpdateMemberRolesRequest,
    },
    ApplicationError, HiveOutboxUnitOfWork,
};

/// Use case trait for member operations
#[async_trait]
pub trait MemberUseCase: Send + Sync {
    /// Add a member to an organization
    async fn add_member(
        &self,
        organization_id: Uuid,
        request: &AddMemberRequest,
        user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    /// Remove a member from an organization
    async fn remove_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError>;

    /// Update a member's role
    async fn update_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        request: &UpdateMemberRolesRequest,
    ) -> Result<MemberResponse, ApplicationError>;

    /// List organization members
    async fn list_members(
        &self,
        organization_id: Uuid,
        pagination: &PaginationRequest,
    ) -> Result<MemberListResponse, ApplicationError>;

    /// Get a specific member
    async fn get_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;
}

/// Implementation of member use case
pub struct MemberUseCaseImpl {
    member_service: Arc<dyn MemberService>,
    organization_service: Arc<dyn OrganizationService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
    outbox_unit_of_work: Option<Arc<dyn HiveOutboxUnitOfWork>>,
}

impl MemberUseCaseImpl {
    /// Create a new member use case instance
    pub fn new(
        member_service: Arc<dyn MemberService>,
        organization_service: Arc<dyn OrganizationService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
    ) -> Self {
        Self {
            member_service,
            organization_service,
            event_publisher,
            outbox_unit_of_work: None,
        }
    }

    pub fn new_with_outbox_unit_of_work(
        member_service: Arc<dyn MemberService>,
        organization_service: Arc<dyn OrganizationService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
        outbox_unit_of_work: Arc<dyn HiveOutboxUnitOfWork>,
    ) -> Self {
        Self {
            member_service,
            organization_service,
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
                .publish(&event)
                .await
                .map_err(ApplicationError::Domain)
        }
    }

    /// Convert domain `OrganizationMember` to response DTO
    fn member_to_response(&self, member: &OrganizationMember) -> MemberResponse {
        MemberResponse {
            id: member.id.unwrap(),
            organization_id: member.organization_id,
            user_id: member.user_id,
            status: member.status.clone().into(),
            joined_at: member.joined_at,
            invited_by_user_id: member.invited_by_user_id,
            invited_at: member.invited_at,
            created_at: member.created_at,
            updated_at: member.updated_at,
        }
    }

    /// Publish member joined event
    async fn publish_member_joined_event(
        &self,
        member: &OrganizationMember,
        organization_name: &str,
        roles: &Vec<RolePermission>,
    ) -> Result<(), ApplicationError> {
        let roles = roles
            .iter()
            .map(|role| {
                Role::new(
                    role.permission.level.to_str().to_string(),
                    role.resource.name.clone(),
                )
            })
            .collect();
        let event = HiveDomainEvent::MemberJoined(MemberJoinedEvent::new(
            member.organization_id,
            organization_name.to_string(),
            member.user_id,
            roles,
            member.joined_at.unwrap_or_else(Utc::now),
        ));

        self.record_or_publish_event(event.into()).await
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
        let event = HiveDomainEvent::MemberRemoved(MemberRemovedEvent::new(
            organization_id,
            organization_name.to_string(),
            user_id,
            user_email.to_string(),
            removed_by_user_id,
            Utc::now(),
        ));

        self.record_or_publish_event(event.into()).await
    }
}

#[async_trait]
impl MemberUseCase for MemberUseCaseImpl {
    async fn add_member(
        &self,
        organization_id: Uuid,
        request: &AddMemberRequest,
        user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        // Get organization for validation and events
        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?;

        let role_permissions: Vec<RolePermission> =
            request.roles.iter().map(std::convert::Into::into).collect();

        // Use domain service to add member
        let member = self
            .member_service
            .add_member(
                organization_id,
                request.user_id,
                role_permissions.clone(),
                Some(user_id),
            )
            .await
            .map_err(ApplicationError::Domain)?;

        // Publish domain event
        // TODO: Get role name from role repository
        self.publish_member_joined_event(&member, &organization.name, &role_permissions)
            .await?;

        Ok(self.member_to_response(&member))
    }

    async fn remove_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        // Get organization for validation and events
        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Use domain service to remove member
        self.member_service
            .remove_member(organization_id, user_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Publish domain event
        // TODO: Get user email from IAM service
        self.publish_member_removed_event(
            organization_id,
            &organization.name,
            user_id,
            "user@example.com", // Placeholder
            user_id,
        )
        .await?;

        Ok(())
    }

    async fn list_members(
        &self,
        organization_id: Uuid,
        pagination: &PaginationRequest,
    ) -> Result<MemberListResponse, ApplicationError> {
        // TODO: Add permission check

        let members = self
            .member_service
            .list_members(organization_id, pagination.page(), pagination.page_size())
            .await
            .map_err(ApplicationError::Domain)?;

        let members: Vec<MemberResponse> = members
            .iter()
            .map(|member| self.member_to_response(member))
            .collect();

        let total_count = members.len() as i64;
        let total_pages = (total_count as f64 / f64::from(pagination.page_size())).ceil() as u32;
        let has_next = pagination.page() < total_pages;

        Ok(MemberListResponse {
            members,
            pagination: crate::dto::PaginationResponse {
                current_page: pagination.page(),
                total_items: Some(total_count),
                has_next,
                has_previous: pagination.page() > 1,
                next_cursor: if has_next {
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
            },
        })
    }

    async fn get_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        // TODO: Add permission check

        let member = self
            .member_service
            .get_member(organization_id, user_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(self.member_to_response(&member))
    }

    async fn update_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        request: &UpdateMemberRolesRequest,
    ) -> Result<MemberResponse, ApplicationError> {
        let role_permissions: Vec<RolePermission> =
            request.roles.iter().map(std::convert::Into::into).collect();

        let member = self
            .member_service
            .update_member_roles(organization_id, user_id, role_permissions)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(self.member_to_response(&member))
    }
}
