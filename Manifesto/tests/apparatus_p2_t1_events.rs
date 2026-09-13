//! Apparatus T1 — un événement sans `binding_id` n'affecte jamais un binding managed.
//!
//! Unitaires, sans Docker : même esprit que `event_runtime_tests.rs`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use apparatus_events::{ApparatusDomainEvent, ComponentStatusChangedEvent};
use async_trait::async_trait;
use chrono::Utc;
use manifesto_domain::{
    entity::ProjectComponent, service::ComponentService, value_objects::ComponentStatus,
};
use manifesto_infra::{
    ApparatusBindingSource, ApparatusBindingSourceLookup, ApparatusEventHandler,
    ComponentStatusProcessor,
};
use rustycog::core::error::{DomainError, ServiceError};
use rustycog::events::EventHandler;
use uuid::Uuid;

#[derive(Clone)]
struct InMemoryComponentService {
    component: Arc<Mutex<ProjectComponent>>,
}

impl InMemoryComponentService {
    fn new(component: ProjectComponent) -> Self {
        Self {
            component: Arc::new(Mutex::new(component)),
        }
    }

    fn snapshot(&self) -> ProjectComponent {
        self.component
            .lock()
            .expect("component state mutex should not be poisoned")
            .clone()
    }
}

#[async_trait]
impl ComponentService for InMemoryComponentService {
    async fn get_component(&self, id: &Uuid) -> Result<ProjectComponent, DomainError> {
        let component = self.snapshot();
        if &component.id == id {
            Ok(component)
        } else {
            Err(DomainError::entity_not_found(
                "ProjectComponent",
                &id.to_string(),
            ))
        }
    }

    async fn get_component_by_type(
        &self,
        project_id: &Uuid,
        component_type: &str,
    ) -> Result<ProjectComponent, DomainError> {
        let component = self.snapshot();
        if &component.project_id == project_id && component.component_type == component_type {
            Ok(component)
        } else {
            Err(DomainError::entity_not_found(
                "ProjectComponent",
                &format!("{project_id}/{component_type}"),
            ))
        }
    }

    async fn add_component(
        &self,
        component: ProjectComponent,
    ) -> Result<ProjectComponent, DomainError> {
        {
            let mut current = self
                .component
                .lock()
                .expect("component state mutex should not be poisoned");
            *current = component.clone();
        }
        Ok(component)
    }

    async fn update_component(
        &self,
        component: ProjectComponent,
    ) -> Result<ProjectComponent, DomainError> {
        {
            let mut current = self
                .component
                .lock()
                .expect("component state mutex should not be poisoned");
            *current = component.clone();
        }
        Ok(component)
    }

    async fn remove_component(&self, _id: &Uuid) -> Result<(), DomainError> {
        Err(DomainError::internal_error(
            "remove_component not used in this test",
        ))
    }

    async fn list_components(
        &self,
        project_id: &Uuid,
    ) -> Result<Vec<ProjectComponent>, DomainError> {
        let component = self.snapshot();
        if &component.project_id == project_id {
            Ok(vec![component])
        } else {
            Ok(vec![])
        }
    }

    async fn validate_component_type(&self, _component_type: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn validate_unique_component(
        &self,
        _project_id: &Uuid,
        _component_type: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

struct InMemoryBindingSource {
    sources: HashMap<Uuid, ApparatusBindingSource>,
}

impl InMemoryBindingSource {
    fn none() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    fn one(component_id: Uuid, source: ApparatusBindingSource) -> Self {
        let mut sources = HashMap::new();
        sources.insert(component_id, source);
        Self { sources }
    }
}

#[async_trait]
impl ApparatusBindingSourceLookup for InMemoryBindingSource {
    async fn source_for_component(
        &self,
        component_id: Uuid,
    ) -> Result<Option<ApparatusBindingSource>, ServiceError> {
        Ok(self.sources.get(&component_id).copied())
    }
}

fn build_pending_component(project_id: Uuid, component_type: &str) -> ProjectComponent {
    ProjectComponent::new(project_id, component_type.to_string())
        .expect("test component should be valid")
}

fn pending_to_configured(project_id: Uuid) -> ComponentStatusChangedEvent {
    ComponentStatusChangedEvent::new(
        project_id,
        "taskboard".to_string(),
        "pending".to_string(),
        "configured".to_string(),
        Utc::now(),
    )
}

#[tokio::test]
async fn t1_managed_status_event_without_binding_id_is_ignored() {
    let project_id = Uuid::new_v4();
    let component = build_pending_component(project_id, "taskboard");
    let original_configured_at = component.configured_at;
    let lookup = InMemoryBindingSource::one(component.id, ApparatusBindingSource::Managed);
    let component_service = Arc::new(InMemoryComponentService::new(component));
    let processor = ComponentStatusProcessor::new(component_service.clone(), Arc::new(lookup));

    processor
        .process(pending_to_configured(project_id))
        .await
        .expect("managed event without binding_id must not fail the queue");

    let snapshot = component_service.snapshot();
    assert_eq!(snapshot.status, ComponentStatus::Pending);
    assert_eq!(snapshot.configured_at, original_configured_at);
}

#[tokio::test]
async fn t1_legacy_status_event_without_binding_id_is_applied() {
    let project_id = Uuid::new_v4();
    let component = build_pending_component(project_id, "taskboard");
    let lookup = InMemoryBindingSource::one(component.id, ApparatusBindingSource::Legacy);
    let component_service = Arc::new(InMemoryComponentService::new(component));
    let processor = ComponentStatusProcessor::new(component_service.clone(), Arc::new(lookup));

    processor
        .process(pending_to_configured(project_id))
        .await
        .expect("legacy event without binding_id follows the current path");

    let snapshot = component_service.snapshot();
    assert_eq!(snapshot.status, ComponentStatus::Configured);
}

#[tokio::test]
async fn t1_missing_binding_row_still_applies_legacy_path() {
    let project_id = Uuid::new_v4();
    let component_service = Arc::new(InMemoryComponentService::new(build_pending_component(
        project_id,
        "taskboard",
    )));
    let processor = ComponentStatusProcessor::new(
        component_service.clone(),
        Arc::new(InMemoryBindingSource::none()),
    );

    processor
        .process(pending_to_configured(project_id))
        .await
        .expect("absent binding row must keep applying the legacy path");

    let snapshot = component_service.snapshot();
    assert_eq!(snapshot.status, ComponentStatus::Configured);
}

#[tokio::test]
async fn t1_handler_managed_event_without_binding_id_returns_ok() {
    let project_id = Uuid::new_v4();
    let component = build_pending_component(project_id, "taskboard");
    let original_status = component.status;
    let original_configured_at = component.configured_at;
    let lookup = InMemoryBindingSource::one(component.id, ApparatusBindingSource::Managed);
    let component_service = Arc::new(InMemoryComponentService::new(component));
    let processor = Arc::new(ComponentStatusProcessor::new(
        component_service.clone(),
        Arc::new(lookup),
    ));
    let handler = ApparatusEventHandler::new(processor);

    let event = ApparatusDomainEvent::ComponentStatusChanged(pending_to_configured(project_id));
    let json = serde_json::to_string(&event).expect("event JSON");
    assert!(
        !json.contains("binding_id"),
        "legacy JSON must omit binding_id: {json}"
    );

    handler
        .handle_event(event.into())
        .await
        .expect("handler must return Ok for managed event without binding_id");

    let snapshot = component_service.snapshot();
    assert_eq!(snapshot.status, original_status);
    assert_eq!(snapshot.configured_at, original_configured_at);
}
