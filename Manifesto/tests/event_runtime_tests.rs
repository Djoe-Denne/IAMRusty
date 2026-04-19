//! Event runtime tests for Manifesto's apparatus integration

use std::sync::{Arc, Mutex};

use apparatus_events::ComponentStatusChangedEvent;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use manifesto_domain::{
    entity::ProjectComponent,
    service::ComponentService,
    value_objects::ComponentStatus,
};
use manifesto_infra::{ApparatusEventConsumer, ComponentStatusProcessor};
use rustycog_config::{KafkaConfig, QueueConfig};
use rustycog_core::error::DomainError;
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
        let mut current = self
            .component
            .lock()
            .expect("component state mutex should not be poisoned");
        *current = component.clone();
        Ok(component)
    }

    async fn update_component(
        &self,
        component: ProjectComponent,
    ) -> Result<ProjectComponent, DomainError> {
        let mut current = self
            .component
            .lock()
            .expect("component state mutex should not be poisoned");
        *current = component.clone();
        Ok(component)
    }

    async fn remove_component(&self, _id: &Uuid) -> Result<(), DomainError> {
        Err(DomainError::internal_error("remove_component not used in this test"))
    }

    async fn list_components(&self, project_id: &Uuid) -> Result<Vec<ProjectComponent>, DomainError> {
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

fn build_pending_component(project_id: Uuid, component_type: &str) -> ProjectComponent {
    ProjectComponent::new(project_id, component_type.to_string())
        .expect("test component should be valid")
}

#[tokio::test]
async fn test_apparatus_consumer_is_noop_when_queue_is_disabled() {
    let project_id = Uuid::new_v4();
    let component_service = Arc::new(InMemoryComponentService::new(build_pending_component(
        project_id,
        "taskboard",
    )));
    let processor = Arc::new(ComponentStatusProcessor::new(component_service));

    let consumer = ApparatusEventConsumer::new(&QueueConfig::Disabled, processor)
        .await
        .expect("Disabled queue should still build a consumer");

    assert!(consumer.is_noop());
}

#[tokio::test]
async fn test_apparatus_consumer_bootstraps_safely_with_enabled_kafka_config() {
    let project_id = Uuid::new_v4();
    let component_service = Arc::new(InMemoryComponentService::new(build_pending_component(
        project_id,
        "taskboard",
    )));
    let processor = Arc::new(ComponentStatusProcessor::new(component_service));

    let mut kafka_config = KafkaConfig::default();
    kafka_config.enabled = true;

    let consumer = ApparatusEventConsumer::new(&QueueConfig::Kafka(kafka_config), processor)
        .await
        .expect("Enabled queue config should not crash consumer bootstrap");

    assert!(
        consumer.is_noop(),
        "Tests should fall back to a safe no-op consumer when no broker is available"
    );
}

#[tokio::test]
async fn test_component_status_processor_applies_incoming_status_changes() {
    let project_id = Uuid::new_v4();
    let component_service = Arc::new(InMemoryComponentService::new(build_pending_component(
        project_id,
        "taskboard",
    )));
    let processor = ComponentStatusProcessor::new(component_service.clone());
    let changed_at = Utc::now() - Duration::minutes(5);

    processor
        .process(ComponentStatusChangedEvent::new(
            project_id,
            "taskboard".to_string(),
            "pending".to_string(),
            "configured".to_string(),
            changed_at,
        ))
        .await
        .expect("Processor should apply a valid status change");

    let updated_component = component_service.snapshot();
    assert_eq!(updated_component.status, ComponentStatus::Configured);
    assert_eq!(updated_component.configured_at, Some(changed_at));
}

#[tokio::test]
async fn test_component_status_processor_ignores_stale_events() {
    let project_id = Uuid::new_v4();
    let mut component = build_pending_component(project_id, "taskboard");
    component
        .transition_status(ComponentStatus::Configured)
        .expect("pending -> configured should be valid");
    component
        .transition_status(ComponentStatus::Active)
        .expect("configured -> active should be valid");

    let original_configured_at = component.configured_at;
    let original_activated_at = component.activated_at;

    let component_service = Arc::new(InMemoryComponentService::new(component));
    let processor = ComponentStatusProcessor::new(component_service.clone());

    processor
        .process(ComponentStatusChangedEvent::new(
            project_id,
            "taskboard".to_string(),
            "pending".to_string(),
            "configured".to_string(),
            Utc::now() - Duration::hours(1),
        ))
        .await
        .expect("Stale events should be ignored rather than failing");

    let updated_component = component_service.snapshot();
    assert_eq!(updated_component.status, ComponentStatus::Active);
    assert_eq!(updated_component.configured_at, original_configured_at);
    assert_eq!(updated_component.activated_at, original_activated_at);
}
