use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use hive_domain::{
    entity::{PermissionLevel, RolePermission},
    service::{MemberService, OrganizationService},
    OrganizationMember,
};
use hive_events::{
    HiveDomainEvent, MemberJoinedEvent, MemberRemovedEvent, MemberRolesUpdatedEvent, Role,
};
use rustycog::core::error::DomainError;
use rustycog::events::{DomainEvent, EventPublisher};

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
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    /// List organization members
    async fn list_members(
        &self,
        organization_id: Uuid,
        pagination: &PaginationRequest,
        requester_id: Uuid,
    ) -> Result<MemberListResponse, ApplicationError>;

    /// Get a specific member
    async fn get_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        requester_id: Uuid,
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
                .publish(event.as_ref())
                .await
                .map_err(ApplicationError::Domain)
        }
    }

    fn role_is_privileged(level: &PermissionLevel) -> bool {
        matches!(level, PermissionLevel::Admin | PermissionLevel::Owner)
    }

    async fn require_org_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<OrganizationMember, ApplicationError> {
        self.member_service
            .get_member(organization_id, user_id)
            .await
            .map_err(ApplicationError::Domain)
    }

    async fn require_can_assign_roles(
        &self,
        organization_id: Uuid,
        actor_id: Uuid,
        roles: &[RolePermission],
    ) -> Result<(), ApplicationError> {
        if !roles
            .iter()
            .any(|role| Self::role_is_privileged(&role.permission.level))
        {
            return Ok(());
        }
        let actor = self.require_org_member(organization_id, actor_id).await?;
        let actor_privileged = actor
            .roles
            .iter()
            .any(|role| Self::role_is_privileged(&role.role_permission.permission.level));
        if actor_privileged {
            Ok(())
        } else {
            Err(ApplicationError::Domain(DomainError::permission_denied(
                "Only org admins can assign admin or owner roles",
            )))
        }
    }

    /// Convert domain `OrganizationMember` to response DTO
    fn member_to_response(member: &OrganizationMember) -> Result<MemberResponse, ApplicationError> {
        Ok(MemberResponse {
            id: member
                .id
                .ok_or_else(|| DomainError::internal_error("member missing id after persist"))?,
            organization_id: member.organization_id,
            user_id: member.user_id,
            status: member.status.clone().into(),
            joined_at: member.joined_at,
            invited_by_user_id: member.invited_by_user_id,
            invited_at: member.invited_at,
            created_at: member.created_at,
            updated_at: member.updated_at,
        })
    }

    /// Publish member joined event
    async fn publish_member_joined_event(
        &self,
        member: &OrganizationMember,
        organization_name: &str,
        roles: &[RolePermission],
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

    async fn publish_member_roles_updated_event(
        &self,
        member: &OrganizationMember,
        organization_name: &str,
        roles: &[RolePermission],
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
        let event = HiveDomainEvent::MemberRolesUpdated(MemberRolesUpdatedEvent::new(
            member.organization_id,
            organization_name.to_string(),
            member.user_id,
            roles,
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

        let role_permissions: Vec<RolePermission> = request
            .roles
            .iter()
            .map(RolePermission::try_from)
            .collect::<Result<_, _>>()
            .map_err(ApplicationError::Domain)?;

        self.require_can_assign_roles(organization_id, user_id, &role_permissions)
            .await?;

        let member = if let Some(outbox_unit_of_work) = &self.outbox_unit_of_work {
            let roles = role_permissions
                .iter()
                .map(|role| {
                    Role::new(
                        role.permission.level.to_str().to_string(),
                        role.resource.name.clone(),
                    )
                })
                .collect();
            let event = HiveDomainEvent::MemberJoined(MemberJoinedEvent::new(
                organization_id,
                organization.name.clone(),
                request.user_id,
                roles,
                Utc::now(),
            ));
            outbox_unit_of_work
                .add_member(
                    organization_id,
                    request.user_id,
                    role_permissions.clone(),
                    Some(user_id),
                    event.into(),
                )
                .await?
        } else {
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
            self.publish_member_joined_event(&member, &organization.name, &role_permissions)
                .await?;
            member
        };

        Self::member_to_response(&member)
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

        if let Some(outbox_unit_of_work) = &self.outbox_unit_of_work {
            let event = HiveDomainEvent::MemberRemoved(MemberRemovedEvent::new(
                organization_id,
                organization.name.clone(),
                user_id,
                "user@example.com".to_string(),
                user_id,
                Utc::now(),
            ));
            outbox_unit_of_work
                .remove_member(organization_id, user_id, event.into())
                .await?;
        } else {
            self.member_service
                .remove_member(organization_id, user_id)
                .await
                .map_err(ApplicationError::Domain)?;
            self.publish_member_removed_event(
                organization_id,
                &organization.name,
                user_id,
                "user@example.com",
                user_id,
            )
            .await?;
        }

        Ok(())
    }

    async fn list_members(
        &self,
        organization_id: Uuid,
        pagination: &PaginationRequest,
        requester_id: Uuid,
    ) -> Result<MemberListResponse, ApplicationError> {
        self.require_org_member(organization_id, requester_id)
            .await?;

        let members = self
            .member_service
            .list_members(organization_id, pagination.page(), pagination.page_size())
            .await
            .map_err(ApplicationError::Domain)?;

        let members: Vec<MemberResponse> = members
            .iter()
            .map(Self::member_to_response)
            .collect::<Result<_, _>>()?;

        let total_count = i64::try_from(members.len()).unwrap_or(i64::MAX);
        let page_size = u64::from(pagination.page_size());
        let total_pages = if page_size == 0 {
            0
        } else {
            let tc = u64::try_from(total_count).unwrap_or(u64::MAX);
            u32::try_from(tc.div_ceil(page_size)).unwrap_or(u32::MAX)
        };
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
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        self.require_org_member(organization_id, requester_id)
            .await?;

        let member = self
            .member_service
            .get_member(organization_id, user_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Self::member_to_response(&member)
    }

    async fn update_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        request: &UpdateMemberRolesRequest,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        let organization = self
            .organization_service
            .get_organization(&organization_id)
            .await
            .map_err(ApplicationError::Domain)?;

        let role_permissions: Vec<RolePermission> = request
            .roles
            .iter()
            .map(RolePermission::try_from)
            .collect::<Result<_, _>>()
            .map_err(ApplicationError::Domain)?;

        self.require_can_assign_roles(organization_id, requester_id, &role_permissions)
            .await?;

        let member = self
            .member_service
            .update_member_roles(organization_id, user_id, role_permissions.clone())
            .await
            .map_err(ApplicationError::Domain)?;

        self.publish_member_roles_updated_event(&member, &organization.name, &role_permissions)
            .await?;

        Self::member_to_response(&member)
    }
}
