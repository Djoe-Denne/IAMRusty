use async_trait::async_trait;
use chrono::Utc;
use manifesto_configuration::BusinessConfig;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use manifesto_domain::{
    entity::ProjectMember,
    service::{MemberService, PermissionService, ProjectService},
    value_objects::{MemberSource, PermissionLevel},
};
use manifesto_events::{
    ManifestoDomainEvent, MemberAddedEvent, MemberPermissionsUpdatedEvent, MemberRemovedEvent,
    PermissionGrantedEvent, PermissionRevokedEvent, ProjectOwnershipTransferredEvent,
    ResourcePermission,
};
use rustycog::core::error::DomainError;
use rustycog::events::{DomainEvent, EventPublisher};

use crate::{
    dto::{
        AddMemberRequest, GrantPermissionRequest, MemberListResponse, MemberResponse,
        PaginationRequest, PaginationResponse, ResourcePermissionResponse,
        TransferOwnershipRequest, UpdateMemberPermissionsRequest,
    },
    usecase::project::ProjectAuthorizationUnitOfWork,
    usecase::world_read::{allows_world_read, member_is_project_admin},
    ApplicationError,
};

/// Application use cases for project membership.
#[async_trait]
pub trait MemberUseCase: Send + Sync {
    /// Add a member to a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the member cannot be added.
    async fn add_member(
        &self,
        project_id: Uuid,
        request: &AddMemberRequest,
        added_by: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    /// Self-join a public live project as an authenticated caller.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the project is not world-readable, the
    /// caller is already a member, or persistence fails.
    async fn join_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    /// Get one project member.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the member is missing.
    async fn get_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    /// List members of a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the listing fails.
    async fn list_members(
        &self,
        project_id: Uuid,
        pagination: &PaginationRequest,
        requester_id: Uuid,
    ) -> Result<MemberListResponse, ApplicationError>;

    /// Replace a member's permissions.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the update is forbidden or persistence fails.
    async fn update_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        request: &UpdateMemberPermissionsRequest,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    /// Remove a member from a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the removal is forbidden or persistence fails.
    async fn remove_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        requester_id: Uuid,
    ) -> Result<(), ApplicationError>;

    /// Grant one permission to a member.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the grant is forbidden or persistence fails.
    async fn grant_permission(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        request: &GrantPermissionRequest,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;

    /// Revoke one permission from a member.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the revoke is forbidden or persistence fails.
    async fn revoke_permission(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        resource: &str,
        requester_id: Uuid,
    ) -> Result<(), ApplicationError>;

    /// Transfer project ownership to another active member.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the transfer is forbidden or persistence fails.
    async fn transfer_ownership(
        &self,
        project_id: Uuid,
        request: &TransferOwnershipRequest,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError>;
}

/// Default [`MemberUseCase`] implementation.
pub struct MemberUseCaseImpl {
    member_service: Arc<dyn MemberService>,
    project_service: Arc<dyn ProjectService>,
    permission_service: Arc<dyn PermissionService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
    business_config: BusinessConfig,
    authorization_uow: Option<Arc<dyn ProjectAuthorizationUnitOfWork>>,
}

impl MemberUseCaseImpl {
    /// Create a member use case with its domain collaborators.
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
            authorization_uow: None,
        }
    }

    /// Persist member writes through the same `AuthZ` outbox unit of work as project creation.
    #[must_use]
    pub fn with_authorization_uow(
        mut self,
        authorization_uow: Arc<dyn ProjectAuthorizationUnitOfWork>,
    ) -> Self {
        self.authorization_uow = Some(authorization_uow);
        self
    }

    fn member_to_response(member: &ProjectMember) -> MemberResponse {
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

    async fn reject_suspended_non_admin(
        &self,
        project_id: Uuid,
        requester_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        if project.status != manifesto_domain::value_objects::ProjectStatus::Suspended {
            return Ok(());
        }
        match self
            .member_service
            .get_member(project_id, requester_id)
            .await
        {
            Ok(member) if member_is_project_admin(&member) => Ok(()),
            Ok(_) | Err(DomainError::EntityNotFound { .. }) => Err(Self::permission_denied(
                "Suspended projects are only visible to owners and admins",
            )),
            Err(error) => Err(ApplicationError::from(error)),
        }
    }

    async fn enforce_member_quota(&self, project_id: &Uuid) -> Result<(), ApplicationError> {
        let active_members = self.member_service.count_active_members(project_id).await?;
        if active_members >= i64::from(self.business_config.max_members_per_project) {
            return Err(ApplicationError::Validation(format!(
                "Project {} has reached the maximum number of members ({})",
                project_id, self.business_config.max_members_per_project
            )));
        }

        Ok(())
    }

    fn specific_resource_scope(resource: &str) -> Option<&str> {
        if uuid::Uuid::parse_str(resource).is_ok() {
            Some("component")
        } else {
            resource
                .split_once(':')
                .map(|(resource_type, _)| resource_type)
        }
    }

    async fn persist_member_with_permission_and_event(
        &self,
        member: ProjectMember,
        resource_name: &str,
        permission: &str,
        event: Box<dyn DomainEvent>,
    ) -> Result<ProjectMember, ApplicationError> {
        if let Some(uow) = &self.authorization_uow {
            let saved = uow
                .save_member_with_permission_and_event(
                    member,
                    resource_name,
                    permission,
                    self.business_config.max_members_per_project,
                    event,
                )
                .await?;
            return self
                .member_service
                .get_member(saved.project_id, saved.user_id)
                .await
                .map_err(ApplicationError::from);
        }

        let mut created = self.member_service.add_member(member).await?;
        let role_perm = self
            .permission_service
            .get_or_create_role_permission(created.project_id, resource_name, permission)
            .await?;
        let member_role_perm = self
            .permission_service
            .grant_permission_to_member(
                &created.id,
                &role_perm.id.ok_or_else(|| {
                    DomainError::internal_error("role permission missing id after persist")
                })?,
            )
            .await?;
        created.role_permissions = vec![member_role_perm];
        self.event_publisher.publish(event.as_ref()).await?;
        Ok(created)
    }

    async fn persist_restore_or_insert(
        &self,
        member: ProjectMember,
        resource_name: &str,
        permission: &str,
        added_by: Uuid,
    ) -> Result<ProjectMember, ApplicationError> {
        if let Some(uow) = &self.authorization_uow {
            let saved = uow
                .restore_or_insert_member_with_permission_and_event(
                    member,
                    resource_name,
                    permission,
                    self.business_config.max_members_per_project,
                    added_by,
                )
                .await?;
            return self
                .member_service
                .get_member(saved.project_id, saved.user_id)
                .await
                .map_err(ApplicationError::from);
        }
        let event = ManifestoDomainEvent::MemberAdded(MemberAddedEvent::new(
            member.project_id,
            member.id,
            member.user_id,
            permission.to_string(),
            resource_name.to_string(),
            added_by,
            member.added_at,
        ));
        self.persist_member_with_permission_and_event(
            member,
            resource_name,
            permission,
            event.into(),
        )
        .await
    }

    fn permission_denied(message: &str) -> ApplicationError {
        ApplicationError::from(DomainError::permission_denied(message))
    }

    fn reject_owner_level(level: PermissionLevel) -> Result<(), ApplicationError> {
        if level == PermissionLevel::Owner {
            return Err(Self::permission_denied(
                "Ownership can only be changed via transfer-ownership",
            ));
        }
        Ok(())
    }

    fn grants_from(member: &ProjectMember) -> Vec<ResourcePermission> {
        member
            .role_permissions
            .iter()
            .map(|rp| {
                ResourcePermission::new(
                    rp.role_permission.resource.name.clone(),
                    rp.role_permission.permission.level.to_str().to_string(),
                    member.project_id,
                )
            })
            .collect()
    }

    fn removed_event(
        project_id: Uuid,
        member: &ProjectMember,
        requester_id: Uuid,
    ) -> MemberRemovedEvent {
        let grants = Self::grants_from(member);
        let tuples = grants
            .iter()
            .filter_map(|grant| grant.to_tuple(project_id, member.user_id))
            .collect();
        let component_ids = grants
            .iter()
            .filter_map(|grant| {
                Uuid::parse_str(&grant.resource).ok().or_else(|| {
                    grant
                        .object_id
                        .filter(|_| grant.object_type.as_deref() == Some("component"))
                })
            })
            .collect();
        MemberRemovedEvent::with_tuples(
            project_id,
            member.id,
            member.user_id,
            requester_id,
            Utc::now(),
            tuples,
            component_ids,
        )
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
        self.reject_suspended_non_admin(project_id, added_by)
            .await?;

        // Get requester to validate they can add members
        let requester = self.member_service.get_member(project_id, added_by).await?;

        // Determine resource (defaults to "project")
        let resource_name = request.resource.as_deref().unwrap_or("project");

        self.enforce_member_quota(&project_id).await?;

        // Validate permission level
        let permission_level =
            PermissionLevel::from_str(&request.permission).map_err(ApplicationError::from)?;
        Self::reject_owner_level(permission_level)?;

        // Requester must have at least the permission level they're trying to grant
        if !requester.has_permission(resource_name, &permission_level) {
            return Err(Self::permission_denied(&format!(
                "Cannot grant {} permission on {} - you don't have it yourself",
                request.permission, resource_name
            )));
        }

        // Create member (without role)
        let member = ProjectMember::new(
            project_id,
            request.user_id,
            MemberSource::Direct,
            Some(added_by),
        );

        let created = self
            .persist_restore_or_insert(member, resource_name, &request.permission, added_by)
            .await?;

        Ok(Self::member_to_response(&created))
    }

    async fn join_project(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        if !allows_world_read(&project) {
            return Err(ApplicationError::from(DomainError::permission_denied(
                "Only public active projects can be joined",
            )));
        }

        if self
            .member_service
            .check_member_exists(&project_id, &user_id)
            .await?
        {
            return Err(ApplicationError::AlreadyExists(format!(
                "User {user_id} is already a member of project {project_id}"
            )));
        }

        self.enforce_member_quota(&project_id).await?;

        let member = ProjectMember::new(project_id, user_id, MemberSource::Direct, Some(user_id));
        let created = self
            .persist_restore_or_insert(member, "project", "read", user_id)
            .await?;

        Ok(Self::member_to_response(&created))
    }

    async fn get_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        self.reject_suspended_non_admin(project_id, requester_id)
            .await?;
        let member = self.member_service.get_member(project_id, user_id).await?;
        Ok(Self::member_to_response(&member))
    }

    async fn list_members(
        &self,
        project_id: Uuid,
        pagination: &PaginationRequest,
        requester_id: Uuid,
    ) -> Result<MemberListResponse, ApplicationError> {
        self.reject_suspended_non_admin(project_id, requester_id)
            .await?;
        let page = pagination.page();
        let page_size = self.configured_page_size(pagination);

        let members = self
            .member_service
            .list_members(&project_id, None, true, page, page_size)
            .await?;

        let total_count = self
            .member_service
            .count_active_members(&project_id)
            .await?;

        let data: Vec<MemberResponse> = members.iter().map(Self::member_to_response).collect();

        let consumed = i64::from(page.saturating_add(1)).saturating_mul(i64::from(page_size));
        let has_more = consumed < total_count;
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
        self.reject_suspended_non_admin(project_id, requester_id)
            .await?;
        let member = self.member_service.get_member(project_id, user_id).await?;

        // Get requester
        let requester = self
            .member_service
            .get_member(project_id, requester_id)
            .await?;

        if member.is_project_owner() {
            return Err(Self::permission_denied(
                "Cannot change the owner's permissions; transfer ownership instead",
            ));
        }

        // Requester needs admin permission on "member" resource
        if !requester.has_permission("member", &PermissionLevel::Admin)
            && !requester.is_project_owner()
        {
            return Err(Self::permission_denied(
                "Insufficient permissions to update member permissions",
            ));
        }

        let previous = Self::grants_from(&member);
        let mut grants = Vec::new();
        for perm_req in &request.permissions {
            let permission_level =
                PermissionLevel::from_str(&perm_req.permission).map_err(ApplicationError::from)?;
            Self::reject_owner_level(permission_level)?;
            if !requester.has_permission(&perm_req.resource, &permission_level) {
                return Err(Self::permission_denied(&format!(
                    "Cannot grant {} permission on {} - you don't have it yourself",
                    perm_req.permission, perm_req.resource
                )));
            }
            grants.push((perm_req.resource.clone(), perm_req.permission.clone()));
        }

        let permissions: Vec<ResourcePermission> = request
            .permissions
            .iter()
            .map(|p| ResourcePermission::new(p.resource.clone(), p.permission.clone(), project_id))
            .collect();
        let event = ManifestoDomainEvent::MemberPermissionsUpdated(
            MemberPermissionsUpdatedEvent::with_previous(
                project_id,
                member.id,
                member.user_id,
                previous,
                permissions,
                requester_id,
                Utc::now(),
            ),
        );

        let updated = if let Some(uow) = &self.authorization_uow {
            uow.replace_member_permissions_with_events(member, &grants, vec![event.into()])
                .await?
        } else {
            self.permission_service
                .revoke_all_permissions_from_member(&member.id)
                .await?;
            let mut new_role_permissions = Vec::new();
            for perm_req in &request.permissions {
                let role_perm = self
                    .permission_service
                    .get_or_create_role_permission(
                        project_id,
                        &perm_req.resource,
                        &perm_req.permission,
                    )
                    .await?;
                let member_role_perm = self
                    .permission_service
                    .grant_permission_to_member(
                        &member.id,
                        &role_perm.id.ok_or_else(|| {
                            DomainError::internal_error("role permission missing id after persist")
                        })?,
                    )
                    .await?;
                new_role_permissions.push(member_role_perm);
            }
            let mut member = member;
            member.role_permissions = new_role_permissions;
            let updated = self.member_service.update_member(member).await?;
            let domain_ev: Box<dyn DomainEvent> = event.into();
            self.event_publisher.publish(domain_ev.as_ref()).await?;
            updated
        };

        Ok(Self::member_to_response(&updated))
    }

    async fn remove_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        requester_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.reject_suspended_non_admin(project_id, requester_id)
            .await?;
        let requester = self
            .member_service
            .get_member(project_id, requester_id)
            .await?;
        if !requester.has_permission("member", &PermissionLevel::Admin)
            && !requester.is_project_owner()
        {
            return Err(Self::permission_denied(
                "Insufficient permissions to remove a member",
            ));
        }

        let target = self.member_service.get_member(project_id, user_id).await?;
        if target.is_project_owner() {
            return Err(Self::permission_denied(
                "Cannot remove the project owner; transfer ownership first",
            ));
        }

        let grace_days = i64::from(self.business_config.member_removal_grace_period_days);
        let event = ManifestoDomainEvent::MemberRemoved(Self::removed_event(
            project_id,
            &target,
            requester_id,
        ));
        if let Some(uow) = &self.authorization_uow {
            uow.remove_member_with_events(target, grace_days, vec![event.into()])
                .await?;
        } else {
            self.member_service
                .remove_member(&project_id, &user_id, Some(grace_days))
                .await?;
            let domain_ev: Box<dyn DomainEvent> = event.into();
            self.event_publisher.publish(domain_ev.as_ref()).await?;
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
        self.reject_suspended_non_admin(project_id, requester_id)
            .await?;
        // Get member
        let mut member = self.member_service.get_member(project_id, user_id).await?;

        // Get requester
        let requester = self
            .member_service
            .get_member(project_id, requester_id)
            .await?;

        // Validate permission level
        let permission_level =
            PermissionLevel::from_str(&request.permission).map_err(ApplicationError::from)?;
        Self::reject_owner_level(permission_level)?;

        // Requester needs to have the permission they're trying to grant
        // For specific resources (UUIDs like component instances), also check generic "component" permission
        let has_permission = Self::specific_resource_scope(&request.resource).map_or_else(
            || requester.has_permission(&request.resource, &permission_level),
            |resource_scope| {
                requester.has_permission(&request.resource, &permission_level)
                    || requester.has_permission(resource_scope, &permission_level)
            },
        );

        if !has_permission {
            return Err(Self::permission_denied(&format!(
                "Cannot grant {} permission on {} - you don't have it yourself",
                request.permission, request.resource
            )));
        }

        if member.role_permissions.iter().any(|rp| {
            rp.role_permission
                .resource
                .name
                .eq_ignore_ascii_case(&request.resource)
                && rp.role_permission.permission.level == permission_level
        }) {
            return Err(ApplicationError::AlreadyExists(
                "Member already has this permission".into(),
            ));
        }

        let event = ManifestoDomainEvent::PermissionGranted(PermissionGrantedEvent::new(
            project_id,
            member.id,
            member.user_id,
            request.resource.clone(),
            request.permission.clone(),
            requester_id,
            Utc::now(),
        ));
        let updated = if let Some(uow) = &self.authorization_uow {
            uow.grant_permission_with_events(
                member,
                &request.resource,
                &request.permission,
                vec![event.into()],
            )
            .await?
        } else {
            let role_perm = self
                .permission_service
                .get_or_create_role_permission(project_id, &request.resource, &request.permission)
                .await?;
            let member_role_perm = self
                .permission_service
                .grant_permission_to_member(
                    &member.id,
                    &role_perm.id.ok_or_else(|| {
                        DomainError::internal_error("role permission missing id after persist")
                    })?,
                )
                .await?;
            member.role_permissions.push(member_role_perm);
            let updated = self.member_service.update_member(member).await?;
            let domain_ev: Box<dyn DomainEvent> = event.into();
            self.event_publisher.publish(domain_ev.as_ref()).await?;
            updated
        };

        Ok(Self::member_to_response(&updated))
    }

    async fn revoke_permission(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        resource: &str,
        requester_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.reject_suspended_non_admin(project_id, requester_id)
            .await?;
        let member = self.member_service.get_member(project_id, user_id).await?;
        let requester = self
            .member_service
            .get_member(project_id, requester_id)
            .await?;
        if !requester.has_permission("member", &PermissionLevel::Admin)
            && !requester.is_project_owner()
        {
            return Err(Self::permission_denied(
                "Insufficient permissions to revoke a permission",
            ));
        }
        if member.is_project_owner() && resource.eq_ignore_ascii_case("project") {
            return Err(Self::permission_denied(
                "Cannot revoke the owner's project permission; transfer ownership first",
            ));
        }

        let role_perm_to_revoke = member
            .role_permissions
            .iter()
            .find(|rp| {
                rp.role_permission
                    .resource
                    .name
                    .eq_ignore_ascii_case(resource)
            })
            .ok_or_else(|| {
                ApplicationError::NotFound("Member does not have this permission".into())
            })?;
        let role_permission_id = role_perm_to_revoke.role_permission.id.ok_or_else(|| {
            DomainError::internal_error("role permission missing id after persist")
        })?;
        let permission = role_perm_to_revoke
            .role_permission
            .permission
            .level
            .to_str()
            .to_string();
        let event = ManifestoDomainEvent::PermissionRevoked(PermissionRevokedEvent::new(
            project_id,
            member.id,
            user_id,
            resource.to_string(),
            permission,
            requester_id,
            Utc::now(),
        ));
        if let Some(uow) = &self.authorization_uow {
            uow.revoke_permission_with_events(member, role_permission_id, vec![event.into()])
                .await?;
        } else {
            self.permission_service
                .revoke_permission_from_member(&member.id, &role_permission_id)
                .await?;
            let domain_ev: Box<dyn DomainEvent> = event.into();
            self.event_publisher.publish(domain_ev.as_ref()).await?;
        }

        Ok(())
    }

    async fn transfer_ownership(
        &self,
        project_id: Uuid,
        request: &TransferOwnershipRequest,
        requester_id: Uuid,
    ) -> Result<MemberResponse, ApplicationError> {
        let mut project = self.project_service.get_project(&project_id).await?;
        let current_owner = self
            .member_service
            .get_member(project_id, requester_id)
            .await?;
        if !current_owner.is_project_owner() {
            return Err(Self::permission_denied(
                "Only the project owner can transfer ownership",
            ));
        }
        if request.user_id == requester_id {
            return Err(ApplicationError::Validation(
                "Cannot transfer ownership to the current owner".into(),
            ));
        }
        let new_owner = self
            .member_service
            .get_member(project_id, request.user_id)
            .await?;
        if project.owner_type == manifesto_domain::value_objects::OwnerType::Personal {
            project.owner_id = request.user_id;
        }
        project.revision = project
            .revision
            .checked_add(1)
            .ok_or_else(|| DomainError::internal_error("project revision overflow"))?;
        project.updated_at = Utc::now();

        let event = ManifestoDomainEvent::ProjectOwnershipTransferred(
            ProjectOwnershipTransferredEvent::new(
                project_id,
                requester_id,
                request.user_id,
                requester_id,
                Utc::now(),
                Some(project.owner_type.as_str().to_string()),
                Some(current_owner.user_id),
                Some(request.user_id),
            ),
        );
        let new_owner = if let Some(uow) = &self.authorization_uow {
            let (_project, _previous, new_owner) = uow
                .transfer_ownership_with_events(
                    project,
                    current_owner,
                    new_owner,
                    self.business_config.max_projects_per_user,
                    vec![event.into()],
                )
                .await?;
            new_owner
        } else {
            return Err(ApplicationError::Internal(
                "ownership transfer requires an authorization unit of work".into(),
            ));
        };
        Ok(Self::member_to_response(&new_owner))
    }
}
