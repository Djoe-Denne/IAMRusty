//! Apparatus P2 — T7 cleanup relançable (DB).

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use std::sync::atomic::{AtomicUsize, Ordering};

use apparatus_contracts::{
    ApparatusError, ApparatusRuntime, BindRequest, BindResponse, BindingId, ConfigureRequest,
    ConfigureResponse, RuntimeObservation, UnbindRequest, UnbindResponse,
};
use chrono::{Duration, Utc};
use common::*;
use fixtures::DbFixtures;
use manifesto_infra::apparatus_runtime::{run_once, InProcessApparatusRuntime};
use rustycog::permission::{Permission, ResourceRef, Subject};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serial_test::serial;
use uuid::Uuid;

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

async fn create_and_delete_managed(
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
    let created = client
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
        .expect("POST");
    assert_eq!(created.status(), 201);
    let body: serde_json::Value = created.json().await.expect("JSON");
    let component_id = Uuid::parse_str(body["id"].as_str().expect("id")).expect("uuid");

    let delete = client
        .delete(format!(
            "{}/api/projects/{}/components/{component_id}",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("DELETE");
    assert_eq!(delete.status(), 204);

    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE apparatus_cleanup_jobs \
         SET next_retry_at = NOW() + INTERVAL '1 hour' \
         WHERE component_id = $1 AND completed_at IS NULL",
        [component_id.into()],
    ))
    .await
    .expect("park job");
    component_id
}

async fn job_completed_at(
    db: &DatabaseConnection,
    component_id: Uuid,
) -> Option<chrono::DateTime<Utc>> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT completed_at FROM apparatus_cleanup_jobs WHERE component_id = $1",
            [component_id.into()],
        ))
        .await
        .expect("query")
        .expect("job");
    row.try_get("", "completed_at").expect("completed_at")
}

async fn job_retry_state(
    db: &DatabaseConnection,
    component_id: Uuid,
) -> (
    i32,
    Option<String>,
    Option<chrono::DateTime<Utc>>,
    Option<chrono::DateTime<Utc>>,
) {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT retry_count, last_error_code, next_retry_at, completed_at \
             FROM apparatus_cleanup_jobs WHERE component_id = $1",
            [component_id.into()],
        ))
        .await
        .expect("query")
        .expect("job");
    (
        row.try_get("", "retry_count").expect("retry_count"),
        row.try_get("", "last_error_code").expect("last_error_code"),
        row.try_get("", "next_retry_at").expect("next_retry_at"),
        row.try_get("", "completed_at").expect("completed_at"),
    )
}

struct AlwaysFailTeardownRuntime {
    inner: InProcessApparatusRuntime,
    teardowns: AtomicUsize,
}

impl AlwaysFailTeardownRuntime {
    fn new() -> Self {
        Self {
            inner: InProcessApparatusRuntime::new(),
            teardowns: AtomicUsize::new(0),
        }
    }

    fn teardown_attempts(&self) -> usize {
        self.teardowns.load(Ordering::SeqCst)
    }
}

impl ApparatusRuntime for AlwaysFailTeardownRuntime {
    fn bind(&self, req: &BindRequest) -> Result<BindResponse, ApparatusError> {
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

    fn teardown(&self, _binding: &BindingId) -> Result<(), ApparatusError> {
        self.teardowns.fetch_add(1, Ordering::SeqCst);
        Err(ApparatusError::InvalidOperation {
            reason: "always fail teardown".to_owned(),
        })
    }
}

#[tokio::test]
#[serial]
async fn t7_delete_then_run_once_completes_job() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_and_delete_managed(db.as_ref(), &base_url, &client, &openfga).await;
    assert!(
        job_completed_at(db.as_ref(), component_id).await.is_none(),
        "T7 : job ouvert"
    );

    let runtime = InProcessApparatusRuntime::new();
    let now = Utc::now() + Duration::hours(2);
    run_once(db.as_ref(), &runtime, "t7-owner", now)
        .await
        .expect("run_once");

    assert!(
        job_completed_at(db.as_ref(), component_id).await.is_some(),
        "T7 RED : completed_at NOT NULL"
    );
    assert_eq!(runtime.teardown_call_count(), 1);
}

#[tokio::test]
#[serial]
async fn t7_run_once_replay_is_idempotent() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_and_delete_managed(db.as_ref(), &base_url, &client, &openfga).await;
    let runtime = InProcessApparatusRuntime::new();
    let now = Utc::now() + Duration::hours(2);
    run_once(db.as_ref(), &runtime, "t7-owner", now)
        .await
        .expect("run_once 1");
    let first = job_completed_at(db.as_ref(), component_id)
        .await
        .expect("completed");

    run_once(db.as_ref(), &runtime, "t7-owner", now)
        .await
        .expect("run_once 2");

    let second = job_completed_at(db.as_ref(), component_id)
        .await
        .expect("still completed");
    assert_eq!(first, second, "T7 RED : completed_at inchangé");
    assert_eq!(runtime.teardown_call_count(), 1);
}

#[tokio::test]
#[serial]
async fn t7_teardown_fail_writes_backoff() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_and_delete_managed(db.as_ref(), &base_url, &client, &openfga).await;
    let runtime = AlwaysFailTeardownRuntime::new();
    let now = Utc::now() + Duration::hours(2);

    run_once(db.as_ref(), &runtime, "t7-backoff", now)
        .await
        .expect("run_once fail");

    let (retry, last_error, next_retry, completed) =
        job_retry_state(db.as_ref(), component_id).await;
    assert_eq!(retry, 1);
    assert_eq!(last_error.as_deref(), Some("teardown_failed"));
    assert!(completed.is_none(), "T7 : pas de completed_at");
    let next = next_retry.expect("next_retry_at");
    let expected = now + Duration::seconds(60);
    assert!(
        (next - expected).num_seconds().abs() <= 2,
        "T7 : 1er fail → 60s, expected {expected}, got {next}"
    );
    assert_eq!(runtime.teardown_attempts(), 1);

    run_once(db.as_ref(), &runtime, "t7-backoff", now)
        .await
        .expect("same now not due");
    let (retry2, _, _, completed2) = job_retry_state(db.as_ref(), component_id).await;
    assert_eq!(retry2, 1, "T7 : same now ne ré-incrémente pas");
    assert!(completed2.is_none());
    assert_eq!(runtime.teardown_attempts(), 1);
}

#[tokio::test]
#[serial]
async fn t7_teardown_fails_until_terminal_excluded() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_and_delete_managed(db.as_ref(), &base_url, &client, &openfga).await;
    let runtime = AlwaysFailTeardownRuntime::new();
    let mut now = Utc::now() + Duration::hours(2);

    for expected_retry in 1..=8 {
        run_once(db.as_ref(), &runtime, "t7-terminal", now)
            .await
            .expect("run_once fail");
        let (retry, last_error, next_retry, completed) =
            job_retry_state(db.as_ref(), component_id).await;
        assert_eq!(retry, expected_retry);
        assert_eq!(last_error.as_deref(), Some("teardown_failed"));
        assert!(completed.is_none());
        if expected_retry >= 8 {
            assert!(next_retry.is_none(), "T7 : terminal next_retry_at NULL");
        } else {
            assert!(next_retry.is_some());
            now += Duration::seconds(301);
        }
    }
    assert_eq!(runtime.teardown_attempts(), 8);

    run_once(
        db.as_ref(),
        &runtime,
        "t7-terminal",
        now + Duration::hours(24),
    )
    .await
    .expect("far future");

    let (retry, last_error, next_retry, completed) =
        job_retry_state(db.as_ref(), component_id).await;
    assert_eq!(retry, 8);
    assert_eq!(last_error.as_deref(), Some("teardown_failed"));
    assert!(next_retry.is_none());
    assert!(completed.is_none(), "T7 : terminal non complété");
    assert_eq!(runtime.teardown_attempts(), 8);
}
