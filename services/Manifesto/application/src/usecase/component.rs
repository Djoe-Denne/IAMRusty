use async_trait::async_trait;
use chrono::Utc;
use manifesto_configuration::BusinessConfig;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use manifesto_domain::{
    entity::ProjectComponent,
    service::{ComponentService, MemberService, PermissionService, ProjectService},
    value_objects::ComponentStatus,
    ComponentInfo,
};
use manifesto_events::{
    ComponentAddedEvent, ComponentRemovedEvent, ComponentStatusChangedEvent, ManifestoDomainEvent,
};
use rustycog::core::error::DomainError;
use rustycog::events::{DomainEvent, EventPublisher};
use rustycog::permission::PermissionChecker;

use crate::{
    dto::{
        AddComponentRequest, BindingGrantSnapshotResponse, ComponentListResponse,
        ComponentResponse, UpdateComponentRequest, UpsertBindingConsentRequest,
    },
    usecase::binding_grant::{BindingConsentWriter, BindingGrantSnapshotReader},
    usecase::project::{ManagedBindingAttach, ProjectAuthorizationUnitOfWork},
    usecase::world_read::{
        caller_can_read_component, enforce_world_read_or_principal, require_project_mutation_actor,
    },
    ApplicationError,
};

/// Application use cases for project components.
#[async_trait]
pub trait ComponentUseCase: Send + Sync {
    /// Add a component to a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the quota is exceeded or persistence fails.
    async fn add_component(
        &self,
        project_id: Uuid,
        request: &AddComponentRequest,
        user_id: Uuid,
    ) -> Result<ComponentResponse, ApplicationError>;

    /// Get one project component.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the component is missing.
    async fn get_component(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ComponentResponse, ApplicationError>;

    /// List components of a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the listing fails.
    async fn list_components(
        &self,
        project_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ComponentListResponse, ApplicationError>;

    /// Update a component status.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the transition is forbidden or persistence fails.
    async fn update_component_status(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        request: &UpdateComponentRequest,
        user_id: Uuid,
    ) -> Result<ComponentResponse, ApplicationError>;

    /// Remove a component from a project.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the removal is forbidden or persistence fails.
    async fn remove_component(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError>;

    /// Privileged read of binding consents and grants.
    ///
    /// `principal` is the intersection user when present. The caller JWT is
    /// not used as that principal.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::NotFound`] when the binding is missing, or
    /// [`ApplicationError::Internal`] when the reader is unwired or the read fails.
    async fn get_binding_grant_snapshot(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshotResponse, ApplicationError>;

    /// Write or revoke a consented capability. Closes access at commit.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::NotFound`] when the binding is missing,
    /// [`ApplicationError::Validation`] when the body is invalid, or
    /// [`ApplicationError::Internal`] when the writer is unwired or the write fails.
    async fn upsert_binding_consent(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        request: &UpsertBindingConsentRequest,
    ) -> Result<BindingGrantSnapshotResponse, ApplicationError>;
}

/// Default [`ComponentUseCase`] implementation.
pub struct ComponentUseCaseImpl {
    component_service: Arc<dyn ComponentService>,
    project_service: Arc<dyn ProjectService>,
    member_service: Arc<dyn MemberService>,
    permission_service: Arc<dyn PermissionService>,
    event_publisher: Arc<dyn EventPublisher<DomainError>>,
    business_config: BusinessConfig,
    org_permission_checker: Arc<dyn PermissionChecker>,
    authorization_uow: Option<Arc<dyn ProjectAuthorizationUnitOfWork>>,
    binding_grant_reader: Option<Arc<dyn BindingGrantSnapshotReader>>,
    binding_consent_writer: Option<Arc<dyn BindingConsentWriter>>,
}

impl ComponentUseCaseImpl {
    /// Create a component use case with its domain collaborators.
    pub fn new(
        component_service: Arc<dyn ComponentService>,
        project_service: Arc<dyn ProjectService>,
        member_service: Arc<dyn MemberService>,
        permission_service: Arc<dyn PermissionService>,
        event_publisher: Arc<dyn EventPublisher<DomainError>>,
        business_config: BusinessConfig,
        org_permission_checker: Arc<dyn PermissionChecker>,
    ) -> Self {
        Self {
            component_service,
            project_service,
            member_service,
            permission_service,
            event_publisher,
            business_config,
            org_permission_checker,
            authorization_uow: None,
            binding_grant_reader: None,
            binding_consent_writer: None,
        }
    }

    /// Persist component writes through the project AuthZ unit of work.
    #[must_use]
    pub fn with_authorization_uow(
        mut self,
        authorization_uow: Arc<dyn ProjectAuthorizationUnitOfWork>,
    ) -> Self {
        self.authorization_uow = Some(authorization_uow);
        self
    }

    /// Wire the privileged binding grant snapshot reader.
    #[must_use]
    pub fn with_binding_grant_reader(
        mut self,
        binding_grant_reader: Arc<dyn BindingGrantSnapshotReader>,
    ) -> Self {
        self.binding_grant_reader = Some(binding_grant_reader);
        self
    }

    /// Wire the consent writer (same transaction as `grant_revision` bump).
    #[must_use]
    pub fn with_binding_consent_writer(
        mut self,
        binding_consent_writer: Arc<dyn BindingConsentWriter>,
    ) -> Self {
        self.binding_consent_writer = Some(binding_consent_writer);
        self
    }

    fn component_to_response(component: &ProjectComponent) -> ComponentResponse {
        ComponentResponse {
            id: component.id,
            component_type: component.component_type.clone(),
            status: component.status.as_str().to_string(),
            added_at: component.added_at,
            configured_at: component.configured_at,
            activated_at: component.activated_at,
            disabled_at: component.disabled_at,
        }
    }

    async fn enforce_component_quota(&self, project_id: &Uuid) -> Result<(), ApplicationError> {
        let existing_components = self.component_service.list_components(project_id).await?;
        if existing_components.len() >= self.business_config.max_components_per_project as usize {
            return Err(ApplicationError::Validation(format!(
                "Project {} has reached the maximum number of components ({})",
                project_id, self.business_config.max_components_per_project
            )));
        }

        Ok(())
    }
}

/// `io.aiforall.reference-kv` (gold path) exige un digest catalogue. Autres types : pin optionnel.
const REFERENCE_KV_TYPE: &str = "io.aiforall.reference-kv";

/// Résout digest + declared depuis le catalogue. Fail-closed si reference-kv sans digest.
///
/// # Errors
///
/// [`ApplicationError::Validation`] when a managed Apparatus type has no catalog digest.
pub fn attach_from_catalog(
    component_type: &str,
    info: Option<&ComponentInfo>,
) -> Result<ManagedBindingAttach, ApplicationError> {
    let digest = info
        .and_then(|row| row.digest.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let declared_capabilities = info
        .and_then(|row| row.declared_capabilities.clone())
        .unwrap_or_default();
    if component_type == REFERENCE_KV_TYPE && digest.is_none() {
        return Err(ApplicationError::Validation(format!(
            "catalog digest required for managed type {component_type}"
        )));
    }
    Ok(ManagedBindingAttach {
        digest,
        declared_capabilities,
    })
}

/// Attach managed (`digest` ou `io.aiforall.reference-kv`) exige le UoW
/// qui appelle `insert_managed_binding`. Sans UoW : fail-closed, pas de persist.
fn require_uow_for_managed_attach(
    has_uow: bool,
    attach: &ManagedBindingAttach,
    component_type: &str,
) -> Result<(), ApplicationError> {
    if has_uow {
        return Ok(());
    }
    if attach.digest.is_some() || component_type == REFERENCE_KV_TYPE {
        return Err(ApplicationError::Validation(
            "managed attach requires authorization unit of work for insert_managed_binding"
                .to_owned(),
        ));
    }
    Ok(())
}

#[async_trait]
impl ComponentUseCase for ComponentUseCaseImpl {
    async fn add_component(
        &self,
        project_id: Uuid,
        request: &AddComponentRequest,
        user_id: Uuid,
    ) -> Result<ComponentResponse, ApplicationError> {
        // Ensure project exists
        let project = self.project_service.get_project(&project_id).await?;
        require_project_mutation_actor(
            &project,
            user_id,
            &self.member_service,
            &self.org_permission_checker,
        )
        .await?;

        // Validate component type exists in component service
        self.component_service
            .validate_component_type(&request.component_type)
            .await?;

        let catalog = self
            .component_service
            .catalog_info(&request.component_type)
            .await?;
        let attach = attach_from_catalog(&request.component_type, catalog.as_ref())?;
        require_uow_for_managed_attach(
            self.authorization_uow.is_some(),
            &attach,
            &request.component_type,
        )?;

        self.enforce_component_quota(&project_id).await?;

        // Check uniqueness
        self.component_service
            .validate_unique_component(&project_id, &request.component_type)
            .await?;

        // Create component so we have a stable UUID for the matching component-instance ACL
        // resource before anything is persisted.
        let component = ProjectComponent::new(project_id, request.component_type.clone())?;
        let event = ManifestoDomainEvent::ComponentAdded(ComponentAddedEvent::new(
            project_id,
            component.id,
            component.component_type.clone(),
            user_id,
            component.added_at,
        ));
        let created = if let Some(uow) = &self.authorization_uow {
            uow.save_component_with_events(project_id, component, true, attach, vec![event.into()])
                .await?
        } else {
            self.permission_service
                .create_component_instance_resource(&component.id)
                .await?;
            match self
                .component_service
                .add_component(component.clone())
                .await
            {
                Ok(created) => {
                    let domain_ev: Box<dyn DomainEvent> = event.into();
                    self.event_publisher.publish(domain_ev.as_ref()).await?;
                    created
                }
                Err(error) => {
                    if let Err(cleanup_error) = self
                        .permission_service
                        .delete_component_instance_resource(&component.id)
                        .await
                    {
                        tracing::error!(
                            "Failed to clean up component ACL resource {} after add failure: {:?}",
                            component.id,
                            cleanup_error
                        );
                    }
                    return Err(error.into());
                }
            }
        };

        Ok(Self::component_to_response(&created))
    }

    async fn get_component(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ComponentResponse, ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        enforce_world_read_or_principal(
            &project,
            user_id,
            &self.member_service,
            &self.org_permission_checker,
        )
        .await?;

        let component = self.component_service.get_component(&component_id).await?;

        if component.project_id != project_id {
            return Err(ApplicationError::NotFound(format!(
                "ProjectComponent not found for project {project_id}"
            )));
        }
        if !caller_can_read_component(
            &project,
            component_id,
            user_id,
            &self.member_service,
            &self.org_permission_checker,
        )
        .await?
        {
            return Err(ApplicationError::from(DomainError::permission_denied(
                "Insufficient permissions to read this component",
            )));
        }

        Ok(Self::component_to_response(&component))
    }

    async fn list_components(
        &self,
        project_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<ComponentListResponse, ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        enforce_world_read_or_principal(
            &project,
            user_id,
            &self.member_service,
            &self.org_permission_checker,
        )
        .await?;

        let components = self.component_service.list_components(&project_id).await?;
        let mut data = Vec::new();
        for component in components {
            if caller_can_read_component(
                &project,
                component.id,
                user_id,
                &self.member_service,
                &self.org_permission_checker,
            )
            .await?
            {
                data.push(Self::component_to_response(&component));
            }
        }

        Ok(ComponentListResponse { data })
    }

    async fn update_component_status(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        request: &UpdateComponentRequest,
        user_id: Uuid,
    ) -> Result<ComponentResponse, ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        require_project_mutation_actor(
            &project,
            user_id,
            &self.member_service,
            &self.org_permission_checker,
        )
        .await?;
        let mut component = self.component_service.get_component(&component_id).await?;

        if component.project_id != project_id {
            return Err(ApplicationError::NotFound(format!(
                "ProjectComponent not found for project {project_id}"
            )));
        }

        let new_status =
            ComponentStatus::from_str(&request.status).map_err(ApplicationError::from)?;

        let old_status = component.status;

        // Transition status (validates transition)
        component
            .transition_status(new_status)
            .map_err(ApplicationError::from)?;

        let event = ManifestoDomainEvent::ComponentStatusChanged(ComponentStatusChangedEvent::new(
            project_id,
            component.id,
            component.component_type.clone(),
            old_status.as_str().to_string(),
            component.status.as_str().to_string(),
            user_id,
            Utc::now(),
        ));
        let updated = if let Some(uow) = &self.authorization_uow {
            uow.save_component_with_events(
                project_id,
                component,
                false,
                ManagedBindingAttach::default(),
                vec![event.into()],
            )
            .await?
        } else {
            let updated = self.component_service.update_component(component).await?;
            let domain_ev: Box<dyn DomainEvent> = event.into();
            self.event_publisher.publish(domain_ev.as_ref()).await?;
            updated
        };

        Ok(Self::component_to_response(&updated))
    }

    async fn remove_component(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let project = self.project_service.get_project(&project_id).await?;
        require_project_mutation_actor(
            &project,
            user_id,
            &self.member_service,
            &self.org_permission_checker,
        )
        .await?;
        let component = self.component_service.get_component(&component_id).await?;

        if component.project_id != project_id {
            return Err(ApplicationError::NotFound(format!(
                "ProjectComponent not found for project {project_id}"
            )));
        }

        let event = ManifestoDomainEvent::ComponentRemoved(ComponentRemovedEvent::new(
            project_id,
            component_id,
            component.component_type.clone(),
            user_id,
            Utc::now(),
        ));
        if let Some(uow) = &self.authorization_uow {
            uow.delete_component_with_events(project_id, component_id, vec![event.into()])
                .await?;
        } else {
            self.component_service
                .remove_component(&component.id)
                .await?;
            if let Err(error) = self
                .permission_service
                .delete_component_instance_resource(&component_id)
                .await
            {
                if let Err(restore_error) = self
                    .component_service
                    .add_component(component.clone())
                    .await
                {
                    tracing::error!(
                        "Failed to restore component {} after ACL cleanup failure: {:?}",
                        component_id,
                        restore_error
                    );
                    return Err(ApplicationError::Internal(format!(
                        "Removed component {component_id} but failed to delete its ACL resource ({error}); restoring the component also failed ({restore_error})"
                    )));
                }
                return Err(error.into());
            }
            let domain_ev: Box<dyn DomainEvent> = event.into();
            self.event_publisher.publish(domain_ev.as_ref()).await?;
        }

        Ok(())
    }

    async fn get_binding_grant_snapshot(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshotResponse, ApplicationError> {
        let reader = self.binding_grant_reader.as_ref().ok_or_else(|| {
            ApplicationError::Internal("binding grant snapshot reader is not configured".to_owned())
        })?;
        reader.load(project_id, component_id, principal).await
    }

    async fn upsert_binding_consent(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        request: &UpsertBindingConsentRequest,
    ) -> Result<BindingGrantSnapshotResponse, ApplicationError> {
        let writer = self.binding_consent_writer.as_ref().ok_or_else(|| {
            ApplicationError::Internal("binding consent writer is not configured".to_owned())
        })?;
        writer.upsert(project_id, component_id, request).await
    }
}

#[cfg(test)]
mod attach_from_catalog_tests {
    use super::*;
    use manifesto_domain::ComponentInfo;

    fn info(digest: Option<&str>, declared: Option<Vec<&str>>) -> ComponentInfo {
        ComponentInfo {
            component_type: "io.aiforall.reference-kv".to_owned(),
            name: "Reference KV".to_owned(),
            description: None,
            version: "1.0.0".to_owned(),
            endpoint: "http://127.0.0.1:8080/lazaret".to_owned(),
            digest: digest.map(ToOwned::to_owned),
            declared_capabilities: declared
                .map(|caps| caps.into_iter().map(ToOwned::to_owned).collect()),
        }
    }

    #[test]
    fn reference_kv_without_digest_fails_closed() {
        let err = attach_from_catalog(REFERENCE_KV_TYPE, Some(&info(None, None)))
            .expect_err("fail-closed");
        match err {
            ApplicationError::Validation(msg) => {
                assert!(msg.contains("catalog digest required"));
            }
            other => panic!("expected validation, got {other:?}"),
        }
    }

    #[test]
    fn reference_kv_with_digest_writes_declared() {
        let pin = attach_from_catalog(
            REFERENCE_KV_TYPE,
            Some(&info(
                Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                Some(vec!["project.read", "storage.kv.read"]),
            )),
        )
        .expect("pin");
        assert_eq!(
            pin.digest.as_deref(),
            Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(
            pin.declared_capabilities,
            vec!["project.read".to_owned(), "storage.kv.read".to_owned()]
        );
    }

    #[test]
    fn taskboard_without_digest_skips_apparatus_pin() {
        let pin = attach_from_catalog("taskboard", None).expect("legacy type");
        assert!(pin.digest.is_none());
        assert!(pin.declared_capabilities.is_empty());
    }

    #[test]
    fn managed_attach_without_uow_fails_closed() {
        let pin = attach_from_catalog(
            REFERENCE_KV_TYPE,
            Some(&info(
                Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                Some(vec!["storage.kv.read"]),
            )),
        )
        .expect("pin");
        let err = require_uow_for_managed_attach(false, &pin, REFERENCE_KV_TYPE)
            .expect_err("fail-closed");
        match err {
            ApplicationError::Validation(msg) => {
                assert!(msg.contains("insert_managed_binding"));
            }
            other => panic!("expected validation, got {other:?}"),
        }
    }

    #[test]
    fn managed_attach_with_uow_ok() {
        let pin = attach_from_catalog(
            REFERENCE_KV_TYPE,
            Some(&info(
                Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                None,
            )),
        )
        .expect("pin");
        require_uow_for_managed_attach(true, &pin, REFERENCE_KV_TYPE).expect("uow wired");
    }

    #[test]
    fn legacy_type_without_uow_still_allowed() {
        let pin = attach_from_catalog("wiki", None).expect("legacy");
        require_uow_for_managed_attach(false, &pin, "wiki").expect("not managed");
    }
}
