//! Apparatus P2 — T3 persist desired_generation + cleanup job + outbox (TDD).
//!
//! Harness unique `common::setup_test_server`. `#[serial]` sur le live.
//! `cache_ttl_seconds=0` via `Manifesto/config/test.toml`.
//! Le helper `FakeBroker` est le minimal test-only pour le rollback atomique.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use manifesto_infra::apparatus_outbox::{
    persist_binding_atomically, persist_cleanup_job_atomically,
};
use rustycog::permission::{Permission, ResourceRef, Subject};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use std::sync::Mutex;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Faux broker in-memory (test-only, même esprit T4)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct FakeEnvelope {
    topic: String,
    payload: String,
}

#[derive(Debug, Default)]
struct FakeBroker {
    published: Mutex<Vec<FakeEnvelope>>,
    fail_next: Mutex<bool>,
}

impl FakeBroker {
    fn new() -> Self {
        Self::default()
    }

    fn publish(&self, topic: &str, payload: &str) -> Result<(), &'static str> {
        if *self.fail_next.lock().expect("verrou") {
            *self.fail_next.lock().expect("verrou") = false;
            return Err("fake broker: injected failure");
        }
        self.published.lock().expect("verrou").push(FakeEnvelope {
            topic: topic.to_owned(),
            payload: payload.to_owned(),
        });
        Ok(())
    }

    fn published(&self) -> Vec<FakeEnvelope> {
        self.published.lock().expect("verrou").clone()
    }

    fn fail_next(&self) {
        *self.fail_next.lock().expect("verrou") = true;
    }
}

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

async fn count_i64(
    db: &sea_orm::DatabaseConnection,
    sql: &str,
    values: impl IntoIterator<Item = sea_orm::Value>,
) -> i64 {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            values,
        ))
        .await
        .expect("requête COUNT")
        .expect("ligne COUNT");
    row.try_get("", "n").expect("n")
}

#[test]
fn t3_openfga_model_has_no_apparatus_type() {
    let model = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../openfga/model.fga");
    let content = std::fs::read_to_string(&model).expect("model.fga lisible");
    assert!(
        !content.to_lowercase().contains("apparatus"),
        "T3 : 0 occurrence apparatus dans openfga/model.fga (pas de nouveau type FGA)"
    );
}

#[tokio::test]
#[serial]
async fn t3_create_managed_sets_desired_generation_one_same_txn_as_outbox() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
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
        .json(&serde_json::json!({"component_type": "wiki"}))
        .send()
        .await
        .expect("POST component");
    assert_eq!(resp.status(), 201);
    let created: serde_json::Value = resp.json().await.expect("JSON");
    let cid = created["id"].as_str().expect("id composant");
    let component_id = Uuid::parse_str(cid).expect("uuid composant");

    let extra_event_tables = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT table_name FROM information_schema.tables \
             WHERE table_schema = 'public' \
               AND (table_name LIKE '%apparatus%event%' \
                    OR table_name = 'apparatus_events')"
                .to_owned(),
        ))
        .await
        .expect("information_schema");
    assert!(
        extra_event_tables.is_empty(),
        "T3 : pas de nouvelle table event Apparatus"
    );

    let binding = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT source, desired_generation, \
                    (next_retry_at IS NOT NULL) AS retry_set \
             FROM apparatus_bindings WHERE component_id = $1",
            [component_id.into()],
        ))
        .await
        .expect("lecture binding")
        .expect("T3 RED : ligne binding managed absente");
    let source: String = binding.try_get("", "source").expect("source");
    let generation: i64 = binding
        .try_get("", "desired_generation")
        .expect("desired_generation");
    let retry_set: bool = binding.try_get("", "retry_set").expect("retry_set");
    assert_eq!(source, "managed");
    assert_eq!(
        generation, 1,
        "T3 : create managed doit poser desired_generation=1"
    );
    assert!(retry_set, "T3 : next_retry_at doit être NOT NULL");

    let outbox_n = count_i64(
        db.as_ref(),
        "SELECT COUNT(*) AS n FROM rustycog_outbox_events \
         WHERE event_type = $1 AND payload_json::text LIKE '%' || $2 || '%'",
        ["component_added".into(), cid.to_owned().into()],
    )
    .await;
    assert_eq!(
        outbox_n, 1,
        "T3 : exactement 1 event ownership component_added dans rustycog_outbox_events"
    );
}

#[tokio::test]
#[serial]
async fn t3_external_failure_rolls_back_desired_and_outbox() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (_project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");

    let broker = FakeBroker::new();
    broker.fail_next();
    let outcome = persist_binding_atomically(db.as_ref(), component.id(), || {
        broker.publish("apparatus", "binding")
    })
    .await;
    assert!(
        outcome.is_err(),
        "T3 : échec externe ⇒ erreur, insertion annulée"
    );

    let n = count_i64(
        db.as_ref(),
        "SELECT COUNT(*) AS n FROM apparatus_bindings WHERE component_id = $1",
        [component.id().into()],
    )
    .await;
    assert_eq!(n, 0, "T3 : échec ⇒ 0 ligne binding (desired non persisté)");
    assert!(
        broker.published().is_empty(),
        "T3 : échec externe ⇒ rien publié"
    );
}

#[tokio::test]
#[serial]
async fn t3_delete_managed_inserts_cleanup_job_then_cascades() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member) = DbFixtures::create_project_with_owner(&db, owner_id)
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
    let created_resp = client
        .post(format!(
            "{}/api/projects/{}/components",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"component_type": "wiki"}))
        .send()
        .await
        .expect("POST component");
    assert_eq!(created_resp.status(), 201);
    let created: serde_json::Value = created_resp.json().await.expect("JSON");
    let cid = created["id"].as_str().expect("id composant");
    let component_id = Uuid::parse_str(cid).expect("uuid composant");

    let delete_resp = client
        .delete(format!(
            "{}/api/projects/{}/components/{}",
            base_url,
            project.id(),
            component_id
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("DELETE component");
    assert_eq!(delete_resp.status(), 204);

    let job = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT component_id, project_id, \
                    (completed_at IS NULL) AS open_job \
             FROM apparatus_cleanup_jobs WHERE component_id = $1",
            [component_id.into()],
        ))
        .await
        .expect("lecture job")
        .expect("T3 RED : job cleanup absent");
    let job_component: Uuid = job.try_get("", "component_id").expect("component_id");
    let job_project: Uuid = job.try_get("", "project_id").expect("project_id");
    let open_job: bool = job.try_get("", "open_job").expect("open_job");
    assert_eq!(job_component, component_id);
    assert_eq!(job_project, project.id());
    assert!(open_job, "T3 : completed_at doit rester NULL");

    let job_n = count_i64(
        db.as_ref(),
        "SELECT COUNT(*) AS n FROM apparatus_cleanup_jobs WHERE component_id = $1",
        [component_id.into()],
    )
    .await;
    assert_eq!(job_n, 1, "T3 : exactement 1 job cleanup");

    let binding_n = count_i64(
        db.as_ref(),
        "SELECT COUNT(*) AS n FROM apparatus_bindings WHERE component_id = $1",
        [component_id.into()],
    )
    .await;
    assert_eq!(
        binding_n, 0,
        "T3 : CASCADE doit effacer le binding après delete"
    );

    let removed_n = count_i64(
        db.as_ref(),
        "SELECT COUNT(*) AS n FROM rustycog_outbox_events \
         WHERE event_type = $1 AND payload_json::text LIKE '%' || $2 || '%'",
        ["component_removed".into(), cid.to_owned().into()],
    )
    .await;
    assert_eq!(
        removed_n, 1,
        "T3 : event component_removed dans rustycog_outbox_events"
    );
}

#[tokio::test]
#[serial]
async fn t3_cleanup_publish_failure_rolls_back_job() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = Uuid::new_v4();
    let project_id = Uuid::new_v4();
    let broker = FakeBroker::new();
    broker.fail_next();

    let outcome =
        persist_cleanup_job_atomically(db.as_ref(), component_id, project_id, 1, None, || {
            broker.publish("apparatus", "cleanup")
        })
        .await;
    assert!(
        outcome.is_err(),
        "T3 : échec publish ⇒ helper atomique en erreur"
    );

    let n = count_i64(
        db.as_ref(),
        "SELECT COUNT(*) AS n FROM apparatus_cleanup_jobs WHERE component_id = $1",
        [component_id.into()],
    )
    .await;
    assert_eq!(n, 0, "T3 : échec publish ⇒ 0 job persisté");
    assert!(broker.published().is_empty(), "T3 : rien publié");
}
