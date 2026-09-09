use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use apparatus_events::{ApparatusDomainEvent, ComponentStatusChangedEvent};
use async_trait::async_trait;
use chrono::Utc;
use manifesto_domain::entity::ProjectComponent;
use manifesto_domain::service::ComponentService;
use manifesto_domain::DomainError;
use manifesto_infra::{
    create_apparatus_event_consumer, ApparatusEventHandler, ComponentStatusProcessor,
};
use rustycog::core::error::ServiceError;
use rustycog::events::{DomainEvent, EventHandler};
use uuid::Uuid;

#[derive(Debug)]
struct DummyEvent {
    payload: String,
}

impl DomainEvent for DummyEvent {
    fn event_type(&self) -> &'static str {
        "unknown_event"
    }

    fn event_id(&self) -> Uuid {
        Uuid::nil()
    }

    fn aggregate_id(&self) -> Uuid {
        Uuid::nil()
    }

    fn occurred_at(&self) -> chrono::DateTime<Utc> {
        Utc::now()
    }

    fn version(&self) -> u32 {
        1
    }

    fn to_json(&self) -> Result<String, ServiceError> {
        Ok(self.payload.clone())
    }

    fn metadata(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

struct InMemoryComponentService {
    component: Mutex<ProjectComponent>,
}

impl InMemoryComponentService {
    const fn new(component: ProjectComponent) -> Self {
        Self {
            component: Mutex::new(component),
        }
    }
}

#[async_trait]
impl ComponentService for InMemoryComponentService {
    async fn get_component(&self, id: &Uuid) -> Result<ProjectComponent, DomainError> {
        let component = self
            .component
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
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
        let component = self
            .component
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
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
        *self
            .component
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = component.clone();
        Ok(component)
    }

    async fn update_component(
        &self,
        component: ProjectComponent,
    ) -> Result<ProjectComponent, DomainError> {
        *self
            .component
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = component.clone();
        Ok(component)
    }

    async fn remove_component(&self, _id: &Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn list_components(
        &self,
        project_id: &Uuid,
    ) -> Result<Vec<ProjectComponent>, DomainError> {
        let component = self
            .component
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
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

#[tokio::test]
async fn apparatus_handler_accepts_status_changes_and_rejects_unknown_payloads() {
    let project_id = Uuid::new_v4();
    let component_service = Arc::new(InMemoryComponentService::new(
        ProjectComponent::new(project_id, "taskboard".to_string()).unwrap(),
    ));
    let processor = Arc::new(ComponentStatusProcessor::new(component_service));
    let handler = ApparatusEventHandler::new(processor);

    assert!(handler.supports_event_type("component_status_changed"));
    assert!(handler.supports_event_type("ComponentStatusChanged"));
    assert!(!handler.supports_event_type("project_created"));

    let event = ApparatusDomainEvent::ComponentStatusChanged(ComponentStatusChangedEvent::new(
        project_id,
        "taskboard".to_string(),
        "pending".to_string(),
        "configured".to_string(),
        Utc::now(),
    ));
    handler
        .handle_event(event.into())
        .await
        .expect("valid apparatus event should be processed");

    assert!(handler
        .handle_event(Box::new(DummyEvent {
            payload: "{not-json".to_string(),
        }))
        .await
        .is_err());
    assert!(handler
        .handle_event(Box::new(DummyEvent {
            payload: serde_json::json!({
                "event_type": "not_an_apparatus_event",
                "data": {}
            })
            .to_string(),
        }))
        .await
        .is_err());

    let _ = create_apparatus_event_consumer(Arc::new(ComponentStatusProcessor::new(Arc::new(
        InMemoryComponentService::new(
            ProjectComponent::new(project_id, "taskboard".to_string()).unwrap(),
        ),
    ))));
}
