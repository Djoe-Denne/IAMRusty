use async_trait::async_trait;
use chrono::Utc;
use manifesto_configuration::BusinessConfig;
use std::sync::Arc;
use uuid::Uuid;

use manifesto_domain::{
    entity::ProjectMember,
    service::{MemberService, PermissionService, ProjectService},
    value_objects::{MemberSource, PermissionLevel},
};
use manifesto_events::{
    ManifestoDomainEvent, MemberAddedEvent, MemberPermissionsUpdatedEvent, MemberRemovedEvent,
    PermissionGrantedEvent, PermissionRevokedEvent, ResourcePermission,
};
use rustycog_core::error::DomainError;
use rustycog_events::EventPublisher;

use crate::{
    dto::{
        AddMemberRequest, GrantPermissionRequest, MemberListResponse, MemberResponse,
        PaginationRequest, PaginationResponse, ResourcePermissionResponse,
        UpdateMemberPermissionsRequest,
    },
    ApplicationError,
};

#[async_trait]
pub trait MemberUseCase: Send + Sync {
    async fn add_member(
        &self,
        project_id: Uuid,
        request: &AddMemberRequest,
        added_by: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    async fn get_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    async fn list_members(
        &self,
        project_id: Uuid,
        pagination: &PaginationRequest,
    ) -> Result<MemberListResponse, ApplicationError>;

    async fn update_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        request: &UpdateMemberPermissionsRequest,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    async fn remove_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        requester_id: Uuid,
    ) -> Result<(), ApplicationError>;

    async fn grant_permission(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        request: &GrantPermissionRequest,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    async fn revoke_permission(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        resource: &str,
        requester_id: Uuid,
    ) -> Result<(), ApplicationError>;
}

pub struct MemberUseCaseImpl {
    member_service: Arc<dyn MemberService>,
    project_service: Arc<dyn ProjectService>,
    permission_service: Arc<dyn PermissionService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
    business_config: BusinessConfig,
}

impl MemberUseCaseImpl {
    pub fn new(
        member_service: Arc<dyn MemberService>,
        project_service: Arc<dyn ProjectService>,
        permission_service: Arc<dyn PermissionService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
        business_config: BusinessConfig,
    ) -> Self {
        Self {
            member_service,
            project_service,
            permission_service,
            event_publisher,
            business_config,
        }
    }

    fn member_to_response(&self, member: &ProjectMember) -> MemberResponse {
        let permissions: Vec<ResourcePermissionResponse> = member
            .role_permissions
            .iter()
            .map(|rp| ResourcePermissionResponse {
                resource: rp.role_permission.resource.name.clone(),
                permission: rp.role_permission.permission.level.to_str().to_string(),
            })
            .collect();

        MemberResponse {
            id: member.id,
            user_id: member.user_id,
            permissions,
            source: member.source.as_str().to_string(),
            added_by: member.added_by,
            added_at: member.added_at,
            removed_at: member.removed_at,
            removal_reason: member.removal_reason.clone(),
            grace_period_ends_at: member.grace_period_ends_at,
            last_access_at: member.last_access_at,
        }
    }

    fn configured_page_size(&self, pagination: &PaginationRequest) -> u32 {
        pagination.page_size_with_defaults(
            self.business_config.default_page_size,
            self.business_config.max_page_size,
        )
    }

    async fn enforce_member_quota(&self, project_id: &Uuid) -> Result<(), ApplicationError> {
        let active_members = self.member_service.count_active_members(project_id).await?;
        if active_members >= self.business_config.max_members_per_project as i64 {
            return Err(ApplicationError::Validation(format!(
                "Project {} has reached the maximum number of members ({})",
                project_id,
                self.business_config.max_members_per_project
            )));
        }

        Ok(())
    }

    fn specific_resource_scope(resource: &str) -> Option<&str> {
        if uuid::Uuid::parse_str(resource).is_ok() {
            Some("component")
        } else {
            resource.split_once(':').map(|(resource_type, _)| resource_type)
        }
    }

}

#[async_trait]
impl MemberUseCase for MemberUseCaseImpl {
    async fn add_member(
        &self,
        project_id: Uuid,
        request: &AddMemberRequest,
        added_by: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        // Ensure project exists
        let _project = self.project_service.get_project(&project_id).await?;

        // Get requester to validate they can add members
        let requester = self.member_service.get_member(project_id, added_by).await?;
        

        // Determine resource (defaults to "project")
        let resource_name = request.resource.as_deref().unwrap_or("project");

        self.enforce_member_quota(&project_id).await?;

        // Validate permission level
        let permission_level = PermissionLevel::from_str(&request.permission)
            .map_err(ApplicationError::from)?;

        // Requester must have at least the permission level they're trying to grant
        if !requester.has_permission(resource_name, &permission_level) {
            return Err(ApplicationError::Validation(
                format!("Cannot grant {} permission on {} - you don't have it yourself", 
                    request.permission, resource_name)
            ));
        }

        // Create member (without role)
        let member = ProjectMember::new(
            project_id,
            request.user_id,
            MemberSource::Direct,
            Some(added_by),
        );

        // Add through service
        let mut created = self.member_service.add_member(member).await?;

        // Grant initial permission
        let role_perm = self
            .permission_service
            .get_or_create_role_permission(project_id, resource_name, &request.permission)
            .await?;

        let member_role_perm = self
            .permission_service
            .grant_permission_to_member(&created.id, &role_perm.id.unwrap())
            .await?;

        created.role_permissions = vec![member_role_perm];

        // Publish MemberAdded event
        let event = ManifestoDomainEvent::MemberAdded(MemberAddedEvent::new(
            project_id,
            created.id,
            created.user_id,
            request.permission.clone(),
            resource_name.to_string(),
            added_by,
            created.added_at,
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish MemberAdded event: {:?}", e);
        }

        Ok(self.member_to_response(&created))
    }

    async fn get_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        let member = self.member_service.get_member(project_id, user_id).await?;
        Ok(self.member_to_response(&member))
    }

    async fn list_members(
        &self,
        project_id: Uuid,
        pagination: &PaginationRequest,
    ) -> Result<MemberListResponse, ApplicationError> {
        let page = pagination.page();
        let page_size = self.configured_page_size(pagination);

        let members = self
            .member_service
            .list_members(&project_id, None, true, page, page_size)
            .await?;

        let total_count = self.member_service.count_active_members(&project_id).await?;

        let data: Vec<MemberResponse> = members
            .iter()
            .map(|m| self.member_to_response(m))
            .collect();

        let has_more = (page + 1) * page_size < total_count as u32;
        let next_cursor = if has_more {
            Some((page + 1).to_string())
        } else {
            None
        };

        let pagination_response = PaginationResponse::new(next_cursor, has_more, Some(total_count));

        Ok(MemberListResponse {
            data,
            pagination: pagination_response,
        })
    }

    async fn update_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        request: &UpdateMemberPermissionsRequest,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        let mut member = self.member_service.get_member(project_id, user_id).await?;

        // Get requester
        let requester = self.member_service.get_member(project_id, requester_id).await?;

        // Requester needs admin permission on "member" resource
        if !requester.has_permission("member", &PermissionLevel::Admin) {
            return Err(ApplicationError::Validation(
                "Insufficient permissions to update member permissions".into(),
            ));
        }

        // Revoke all existing permissions
        self.permission_service
            .revoke_all_permissions_from_member(&member.id)
            .await?;

        // Grant new permissions
        let mut new_role_permissions = Vec::new();
        for perm_req in &request.permissions {
            // Validate permission level
            let _permission_level = PermissionLevel::from_str(&perm_req.permission)
                .map_err(ApplicationError::from)?;

            // Requester must have the permission they're trying to grant
            if !requester.has_permission(&perm_req.resource, &_permission_level) {
                return Err(ApplicationError::Validation(
                    format!("Cannot grant {} permission on {} - you don't have it yourself", 
                        perm_req.permission, perm_req.resource)
                ));
            }

            let role_perm = self
                .permission_service
                .get_or_create_role_permission(project_id, &perm_req.resource, &perm_req.permission)
                .await?;

            let member_role_perm = self
                .permission_service
                .grant_permission_to_member(&member.id, &role_perm.id.unwrap())
                .await?;

            new_role_permissions.push(member_role_perm);
        }

        member.role_permissions = new_role_permissions;

        // Update through service
        let updated = self.member_service.update_member(member).await?;

        // Publish MemberPermissionsUpdated event
        let permissions: Vec<ResourcePermission> = request
            .permissions
            .iter()
            .map(|p| ResourcePermission {
                resource: p.resource.clone(),
                permission: p.permission.clone(),
            })
            .collect();
        let event = ManifestoDomainEvent::MemberPermissionsUpdated(MemberPermissionsUpdatedEvent::new(
            project_id,
            updated.id,
            updated.user_id,
            permissions,
            requester_id,
            Utc::now(),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish MemberPermissionsUpdated event: {:?}", e);
        }

        Ok(self.member_to_response(&updated))
    }

    async fn remove_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        requester_id: Uuid,
    ) -> Result<(), ApplicationError> {
        // Get target member
        let target = self.member_service.get_member(project_id, user_id).await?;

        let member_id = target.id;

        // Remove through service
        self.member_service
            .remove_member(
                &project_id,
                &user_id,
                Some(self.business_config.member_removal_grace_period_days as i64),
            )
            .await?;

        // Publish MemberRemoved event
        let event = ManifestoDomainEvent::MemberRemoved(MemberRemovedEvent::new(
            project_id,
            member_id,
            user_id,
            requester_id,
            Utc::now(),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish MemberRemoved event: {:?}", e);
        }

        Ok(())
    }

    async fn grant_permission(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        request: &GrantPermissionRequest,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        // Get member
        let mut member = self.member_service.get_member(project_id, user_id).await?;

        // Get requester
        let requester = self.member_service.get_member(project_id, requester_id).await?;

        // Validate permission level
        let permission_level = PermissionLevel::from_str(&request.permission)
            .map_err(ApplicationError::from)?;

        // Requester needs to have the permission they're trying to grant
        // For specific resources (UUIDs like component instances), also check generic "component" permission
        let has_permission = if let Some(resource_scope) =
            Self::specific_resource_scope(&request.resource)
        {
            // Specific resource - check both the exact resource and its generic scope.
            requester.has_permission(&request.resource, &permission_level)
                || requester.has_permission(resource_scope, &permission_level)
        } else {
            // Generic resource - check direct permission
            requester.has_permission(&request.resource, &permission_level)
        };
        
        if !has_permission {
            return Err(ApplicationError::Validation(
                format!("Cannot grant {} permission on {} - you don't have it yourself", 
                    request.permission, request.resource)
            ));
        }

        // Check if member already has this exact permission
        // Use case-insensitive comparison since resource names in DB may be capitalized
        if member
            .role_permissions
            .iter()
            .any(|rp| {
                rp.role_permission.resource.name.eq_ignore_ascii_case(&request.resource)
                    && rp.role_permission.permission.level == permission_level
            })
        {
            return Err(ApplicationError::Validation(
                "Member already has this permission".into(),
            ));
        }

        // Get or create role_permission
        let role_perm = self
            .permission_service
            .get_or_create_role_permission(project_id, &request.resource, &request.permission)
            .await?;

        // Grant permission
        let member_role_perm = self
            .permission_service
            .grant_permission_to_member(&member.id, &role_perm.id.unwrap())
            .await?;

        member.role_permissions.push(member_role_perm);

        // Update through service (to refresh the full member with permissions)
        let updated = self.member_service.update_member(member).await?;

        // Publish PermissionGranted event
        let event = ManifestoDomainEvent::PermissionGranted(PermissionGrantedEvent::new(
            project_id,
            updated.id,
            updated.user_id,
            request.resource.clone(),
            request.permission.clone(),
            requester_id,
            Utc::now(),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish PermissionGranted event: {:?}", e);
        }

        Ok(self.member_to_response(&updated))
    }

    async fn revoke_permission(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        resource: &str,
        requester_id: Uuid,
    ) -> Result<(), ApplicationError> {
        // Get member
        let member = self.member_service.get_member(project_id, user_id).await?;

        // Get requester
        let _ = self.member_service.get_member(project_id, requester_id).await?;

        // Find the role_permission to revoke
        // Use case-insensitive comparison since resource names in DB may be capitalized
        let role_perm_to_revoke = member
            .role_permissions
            .iter()
            .find(|rp| rp.role_permission.resource.name.eq_ignore_ascii_case(resource))
            .ok_or_else(|| {
                ApplicationError::Validation("Member does not have this permission".into())
            })?;

        // Revoke permission
        self.permission_service
            .revoke_permission_from_member(&member.id, &role_perm_to_revoke.role_permission.id.unwrap())
            .await?;

        // Publish PermissionRevoked event
        let event = ManifestoDomainEvent::PermissionRevoked(PermissionRevokedEvent::new(
            project_id,
            member.id,
            user_id,
            resource.to_string(),
            requester_id,
            Utc::now(),
        ));
        if let Err(e) = self.event_publisher.publish(&event.into()).await {
            tracing::warn!("Failed to publish PermissionRevoked event: {:?}", e);
        }

        Ok(())
    }
}

