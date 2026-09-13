//! Apparatus P2 — T5 ticker, concurrence, claim, is_live.
//!
//! `#[serial]` pour le live DB. Ticker de test : intervalle long.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use apparatus_contracts::{
    ApparatusError, ApparatusRuntime, BindRequest, BindResponse, BindingId, ConfigureRequest,
    ConfigureResponse, RuntimeObservation, UnbindRequest, UnbindResponse,
};
use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use common::*;
use fixtures::DbFixtures;
use manifesto_domain::{entity::ProjectComponent, service::ComponentService};
use manifesto_infra::apparatus_runtime::{apply_due_once, InProcessApparatusRuntime};
use manifesto_infra::{
    ApparatusBindingSource, ApparatusBindingSourceLookup, ApparatusEventConsumer,
    ComponentStatusProcessor,
};
use manifesto_setup::start_apparatus_runtime;
use rustycog::config::QueueConfig;
use rustycog::core::error::{DomainError, ServiceError};
use rustycog::permission::{Permission, ResourceRef, Subject};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serial_test::serial;
use uuid::Uuid;

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

fn sample_digest() -> String {
    apparatus_contracts::ReleaseDigest::from_bytes(&[0x42; 32])
        .as_str()
        .to_owned()
}

async fn create_managed_with_digest(
    db: &DatabaseConnection,
    base_url: &str,
    client: &reqwest::Client,
    openfga: &TestOpenFga,
) -> Uuid {
    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(db, owner_id)
        .await
        .expect("projet");
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("grant admin");
    let jwt = create_test_jwt_token(owner_id);
    let resp = client
        .post(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"component_type": "taskboard"}))
        .send()
        .await
        .expect("POST component");
    assert_eq!(resp.status(), 201);
    let created: serde_json::Value = resp.json().await.expect("JSON");
    let component_id = Uuid::parse_str(created["id"].as_str().expect("id")).expect("uuid");
    let digest = sample_digest();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE apparatus_bindings \
         SET digest = $1, next_retry_at = NOW() + INTERVAL '1 hour' \
         WHERE component_id = $2",
        [digest.into(), component_id.into()],
    ))
    .await
    .expect("digest");
    component_id
}

async fn lease_row(db: &DatabaseConnection, component_id: Uuid) -> (i64, String) {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT lease_epoch, lease_owner FROM apparatus_bindings WHERE component_id = $1",
            [component_id.into()],
        ))
        .await
        .expect("query")
        .expect("binding");
    (
        row.try_get("", "lease_epoch").expect("epoch"),
        row.try_get("", "lease_owner").expect("owner"),
    )
}

async fn observed_generation(db: &DatabaseConnection, component_id: Uuid) -> i64 {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT observed_generation FROM apparatus_bindings WHERE component_id = $1",
            [component_id.into()],
        ))
        .await
        .expect("query")
        .expect("binding");
    row.try_get("", "observed_generation").expect("observed")
}

struct FailFirstBindRuntime {
    inner: InProcessApparatusRuntime,
    fail_next: AtomicBool,
}

impl FailFirstBindRuntime {
    fn new() -> Self {
        Self {
            inner: InProcessApparatusRuntime::new(),
            fail_next: AtomicBool::new(true),
        }
    }
}

impl ApparatusRuntime for FailFirstBindRuntime {
    fn bind(&self, req: &BindRequest) -> Result<BindResponse, ApparatusError> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(ApparatusError::InvalidOperation {
                reason: "poison bind".to_owned(),
            });
        }
        self.inner.bind(req)
    }

    fn configure(&self, req: &ConfigureRequest) -> Result<ConfigureResponse, ApparatusError> {
        self.inner.configure(req)
    }

    fn unbind(&self, req: &UnbindRequest) -> Result<UnbindResponse, ApparatusError> {
        self.inner.unbind(req)
    }

    fn observe(&self, binding: &BindingId) -> Result<RuntimeObservation, ApparatusError> {
        self.inner.observe(binding)
    }

    fn teardown(&self, binding: &BindingId) -> Result<(), ApparatusError> {
        self.inner.teardown(binding)
    }
}

#[tokio::test]
#[serial]
async fn t5_concurrent_apply_single_claim() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_managed_with_digest(db.as_ref(), &base_url, &client, &openfga).await;
    let runtime = InProcessApparatusRuntime::new();
    let now = Utc::now() + ChronoDuration::hours(2);

    let (a, b) = tokio::join!(
        apply_due_once(db.as_ref(), &runtime, "owner-a", now),
        apply_due_once(db.as_ref(), &runtime, "owner-b", now),
    );
    a.expect("apply a");
    b.expect("apply b");

    let (epoch, owner) = lease_row(db.as_ref(), component_id).await;
    assert_eq!(epoch, 1, "T5 RED : lease_epoch +1 une seule fois");
    assert!(
        owner == "owner-a" || owner == "owner-b",
        "T5 : un seul owner gagnant, got {owner}"
    );
}

#[tokio::test]
#[serial]
async fn t5_expired_claim_is_stolen() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_managed_with_digest(db.as_ref(), &base_url, &client, &openfga).await;
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE apparatus_bindings \
         SET lease_epoch = 4, lease_owner = 'other', \
             lease_expires_at = NOW() - INTERVAL '1 minute' \
         WHERE component_id = $1",
        [component_id.into()],
    ))
    .await
    .expect("expire");

    let runtime = InProcessApparatusRuntime::new();
    let now = Utc::now() + ChronoDuration::hours(2);
    apply_due_once(db.as_ref(), &runtime, "thief", now)
        .await
        .expect("steal");

    let (epoch, owner) = lease_row(db.as_ref(), component_id).await;
    assert_eq!(epoch, 5, "T5 RED : vol => epoch +1");
    assert_eq!(owner, "thief");
}

#[tokio::test]
#[serial]
async fn t5_queue_disabled_runtime_can_be_live() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let project_id = Uuid::new_v4();
    let component_service = Arc::new(InMemoryComponentService::new(build_pending_component(
        project_id,
        "taskboard",
    )));
    let processor = Arc::new(ComponentStatusProcessor::new(
        component_service,
        Arc::new(AbsentBindingSource),
    ));
    let consumer = ApparatusEventConsumer::new(&QueueConfig::Disabled, processor)
        .await
        .expect("consumer");
    assert!(consumer.is_noop());

    let runtime = Arc::new(InProcessApparatusRuntime::new());
    let handle = start_apparatus_runtime(
        fixture.db().as_ref().clone(),
        runtime,
        "t5-live".to_owned(),
        Duration::from_secs(60),
    );
    assert!(
        handle.is_live(),
        "T5 RED : runtime live indépendamment de la queue"
    );
    handle.abort();
}

#[tokio::test]
#[serial]
async fn t5_ticker_start_stop_is_live() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let runtime = Arc::new(InProcessApparatusRuntime::new());
    let handle = start_apparatus_runtime(
        fixture.db().as_ref().clone(),
        runtime,
        "t5-tick".to_owned(),
        Duration::from_secs(60),
    );
    assert!(handle.is_live(), "T5 RED : is_live après start");
    handle.abort();
    assert!(!handle.is_live(), "T5 RED : is_live false après abort");
}

#[tokio::test]
#[serial]
async fn t5_poison_bind_does_not_abort_pass() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let first = create_managed_with_digest(db.as_ref(), &base_url, &client, &openfga).await;
    let second = create_managed_with_digest(db.as_ref(), &base_url, &client, &openfga).await;
    let runtime = FailFirstBindRuntime::new();
    let now = Utc::now() + ChronoDuration::hours(2);

    apply_due_once(db.as_ref(), &runtime, "t5-poison", now)
        .await
        .expect("T5 : poison isolé, apply_due_once Ok");

    let observed = [
        observed_generation(db.as_ref(), first).await,
        observed_generation(db.as_ref(), second).await,
    ];
    assert!(
        observed.contains(&0) && observed.contains(&1),
        "T5 : second binding observé (generation 1), poison non observé, got {observed:?}"
    );
}

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
        *self.component.lock().expect("mutex") = component.clone();
        Ok(component)
    }

    async fn update_component(
        &self,
        component: ProjectComponent,
    ) -> Result<ProjectComponent, DomainError> {
        *self.component.lock().expect("mutex") = component.clone();
        Ok(component)
    }

    async fn remove_component(&self, _id: &Uuid) -> Result<(), DomainError> {
        Ok(())
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

struct AbsentBindingSource;

#[async_trait]
impl ApparatusBindingSourceLookup for AbsentBindingSource {
    async fn source_for_component(
        &self,
        _component_id: Uuid,
    ) -> Result<Option<ApparatusBindingSource>, ServiceError> {
        Ok(None)
    }
}

fn build_pending_component(project_id: Uuid, component_type: &str) -> ProjectComponent {
    ProjectComponent::new(project_id, component_type.to_string())
        .expect("test component should be valid")
}
