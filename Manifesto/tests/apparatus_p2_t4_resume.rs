//! Apparatus P2 — T4 crash / reprise / idempotence (DB).
//!
//! Harness `common::setup_test_server`. `#[serial]`.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use chrono::{Duration, Utc};
use common::*;
use fixtures::DbFixtures;
use manifesto_infra::apparatus_runtime::{
    apply_due_once, apply_due_once_skip_observe, write_observed, InProcessApparatusRuntime,
};
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

#[tokio::test]
#[serial]
async fn t4_apply_due_once_sets_observed_to_desired() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_managed_with_digest(db.as_ref(), &base_url, &client, &openfga).await;
    let runtime = InProcessApparatusRuntime::new();
    let now = Utc::now() + Duration::hours(2);

    apply_due_once(db.as_ref(), &runtime, "t4-owner", now)
        .await
        .expect("apply");

    let observed = observed_generation(db.as_ref(), component_id).await;
    let desired: i64 = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT desired_generation FROM apparatus_bindings WHERE component_id = $1",
            [component_id.into()],
        ))
        .await
        .expect("query")
        .expect("binding")
        .try_get("", "desired_generation")
        .expect("desired");
    assert_eq!(
        observed, desired,
        "T4 RED : observed_generation == desired_generation"
    );
    assert_eq!(runtime.applied_bind_count(), 1);
}

#[tokio::test]
#[serial]
async fn t4_second_apply_does_not_bind_twice() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_managed_with_digest(db.as_ref(), &base_url, &client, &openfga).await;
    let runtime = InProcessApparatusRuntime::new();
    let now = Utc::now() + Duration::hours(2);

    apply_due_once(db.as_ref(), &runtime, "t4-owner", now)
        .await
        .expect("apply 1");
    let observed_after_first = observed_generation(db.as_ref(), component_id).await;

    apply_due_once(db.as_ref(), &runtime, "t4-owner", now)
        .await
        .expect("apply 2");

    assert_eq!(
        runtime.applied_bind_count(),
        1,
        "T4 : pas de 2e bind effectif"
    );
    assert_eq!(
        observed_generation(db.as_ref(), component_id).await,
        observed_after_first
    );
}

#[tokio::test]
#[serial]
async fn t4_crash_after_bind_resumes_without_second_instance() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_managed_with_digest(db.as_ref(), &base_url, &client, &openfga).await;
    let runtime = InProcessApparatusRuntime::new();
    let now = Utc::now() + Duration::hours(2);

    apply_due_once_skip_observe(db.as_ref(), &runtime, "t4-owner", now)
        .await
        .expect("crash après bind");
    assert_eq!(observed_generation(db.as_ref(), component_id).await, 0);
    assert_eq!(runtime.instance_count(), 1);

    apply_due_once(db.as_ref(), &runtime, "t4-owner", now)
        .await
        .expect("reprise");

    assert_eq!(runtime.instance_count(), 1, "T4 RED : pas de 2e instance");
    assert_eq!(runtime.applied_bind_count(), 1);
    assert_eq!(observed_generation(db.as_ref(), component_id).await, 1);
}

#[tokio::test]
#[serial]
async fn t4_stale_observed_write_is_refused() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let component_id = create_managed_with_digest(db.as_ref(), &base_url, &client, &openfga).await;
    let runtime = InProcessApparatusRuntime::new();
    let now = Utc::now() + Duration::hours(2);

    apply_due_once_skip_observe(db.as_ref(), &runtime, "t4-owner", now)
        .await
        .expect("claim");

    let digest = sample_digest();
    let rows = write_observed(db.as_ref(), component_id, 1, &digest, 1, "other", now)
        .await
        .expect("write observed");
    assert_eq!(rows, 0, "T4 RED : fencing refuse (0 row)");
    assert_eq!(observed_generation(db.as_ref(), component_id).await, 0);
}
