//! Apparatus P3 — T4 call-time grants (Manifesto domain GET, live DB).
//!
//! Filename keeps "gateway" for the gate checklist. Production src stays
//! domain language (binding / consent / grants / privileged caller).

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Serialize;
use serial_test::serial;
use uuid::Uuid;

const GRANT_SNAPSHOT_TEST_HS256_SECRET: &str = "manifesto-grant-snapshot-test-hs256";
const GRANT_SNAPSHOT_ISSUER: &str = "aiforall-platform";
const GRANT_SNAPSHOT_AUDIENCE: &str = "manifesto-bindings";

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

#[derive(Serialize)]
struct PlatformGrantSnapshotClaims {
    sub: String,
    iss: String,
    aud: String,
    exp: i64,
    iat: i64,
    jti: String,
}

fn create_platform_grant_snapshot_jwt() -> String {
    let now = chrono::Utc::now();
    let claims = PlatformGrantSnapshotClaims {
        sub: Uuid::new_v4().to_string(),
        iss: GRANT_SNAPSHOT_ISSUER.to_owned(),
        aud: GRANT_SNAPSHOT_AUDIENCE.to_owned(),
        exp: (now + chrono::Duration::hours(1)).timestamp(),
        iat: now.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(GRANT_SNAPSHOT_TEST_HS256_SECRET.as_bytes()),
    )
    .expect("platform grant snapshot JWT")
}

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("workspace lisible")
}

async fn seed_binding_consent_revisions(
    db: &sea_orm::DatabaseConnection,
    component_id: Uuid,
    digest: &str,
    desired_generation: i64,
    binding_revision: i64,
    capability: &str,
    consent_status: &str,
    consent_revision: i64,
) {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_bindings (component_id, source, digest, desired_generation, grant_revision, declared_capabilities) \
         VALUES ($1, 'managed', $2, $3, $4, jsonb_build_array($5))",
        [
            component_id.into(),
            digest.into(),
            desired_generation.into(),
            binding_revision.into(),
            capability.into(),
        ],
    ))
    .await
    .expect("INSERT apparatus_bindings");
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_capability_consents (component_id, capability, status, grant_revision) \
         VALUES ($1, $2, $3, $4)",
        [
            component_id.into(),
            capability.into(),
            consent_status.into(),
            consent_revision.into(),
        ],
    ))
    .await
    .expect("INSERT apparatus_capability_consents");
}

fn snapshot_url(
    base: &str,
    project_id: Uuid,
    component_id: Uuid,
    principal: Option<Uuid>,
) -> String {
    match principal {
        Some(user) => {
            format!("{base}/api/projects/{project_id}/bindings/{component_id}?principal={user}")
        }
        None => format!("{base}/api/projects/{project_id}/bindings/{component_id}"),
    }
}

fn method_body(src: &str, name: &str) -> String {
    let needle = format!("async fn {name}");
    let start = src
        .rfind(&needle)
        .unwrap_or_else(|| panic!("async fn {name} absent"));
    let from = &src[start..];
    let open = from.find('{').expect("accolade");
    let mut depth = 0_i32;
    let mut end = 0;
    for (i, ch) in from.char_indices() {
        if i < open {
            continue;
        }
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    from[..end].to_string()
}

#[tokio::test]
#[serial]
async fn t4_snapshot_returns_consented_cap_from_db() {
    let (fixture, base_url, client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");

    seed_binding_consent_revisions(
        &db,
        component.id(),
        "release-1",
        1,
        0,
        "project.read",
        "consented",
        0,
    )
    .await;

    let jwt = create_platform_grant_snapshot_jwt();
    let response = client
        .get(snapshot_url(
            &base_url,
            project.id(),
            component.id(),
            Some(owner_id),
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("GET snapshot");
    assert_eq!(
        response.status(),
        200,
        "privileged GET must succeed with platform JWT"
    );
    let body: serde_json::Value = response.json().await.expect("JSON");
    assert_eq!(body["component_id"], component.id().to_string());
    assert_eq!(body["project_id"], project.id().to_string());
    assert_eq!(body["digest"], "release-1");
    assert_eq!(body["desired_generation"], 1);
    assert_eq!(body["grant_revision"], 0);
    let consents = body["consents"].as_array().expect("consents");
    assert_eq!(consents.len(), 1);
    assert_eq!(consents[0]["capability"], "project.read");
    assert_eq!(consents[0]["status"], "consented");
    assert_eq!(consents[0]["grant_revision"], 0);
    let declared = body["declared"].as_array().expect("declared");
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0], "project.read");
    assert_eq!(body["principal"]["user_id"], owner_id.to_string());
    assert_eq!(body["principal"]["active"], true);
}

#[tokio::test]
#[serial]
async fn t4_snapshot_requires_jwt() {
    let (fixture, base_url, client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    seed_binding_consent_revisions(
        &db,
        component.id(),
        "release-1",
        1,
        0,
        "project.read",
        "consented",
        0,
    )
    .await;

    let response = client
        .get(snapshot_url(
            &base_url,
            project.id(),
            component.id(),
            Some(owner_id),
        ))
        .send()
        .await
        .expect("GET sans JWT");
    assert_eq!(response.status(), 401);
}

#[tokio::test]
#[serial]
async fn t4_snapshot_rejects_iam_jwt() {
    let (fixture, base_url, client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    seed_binding_consent_revisions(
        &db,
        component.id(),
        "release-1",
        1,
        0,
        "project.read",
        "consented",
        0,
    )
    .await;

    let jwt = create_test_jwt_token(Uuid::new_v4());
    let response = client
        .get(snapshot_url(
            &base_url,
            project.id(),
            component.id(),
            Some(owner_id),
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("GET IAM JWT");
    assert_eq!(response.status(), 401, "IAM JWT must not pass snapshot GET");
}

#[tokio::test]
#[serial]
async fn t4_revoked_consent_is_visible() {
    let (fixture, base_url, client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    seed_binding_consent_revisions(
        &db,
        component.id(),
        "release-1",
        1,
        0,
        "project.read",
        "revoked",
        0,
    )
    .await;

    let jwt = create_platform_grant_snapshot_jwt();
    let response = client
        .get(snapshot_url(&base_url, project.id(), component.id(), None))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("GET revoked");
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("JSON");
    assert_eq!(body["consents"][0]["status"], "revoked");
    assert!(body["principal"].is_null());
}

#[tokio::test]
#[serial]
async fn t4_mismatched_consent_revision_is_visible() {
    let (fixture, base_url, client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    seed_binding_consent_revisions(
        &db,
        component.id(),
        "release-1",
        1,
        0,
        "project.read",
        "consented",
        9,
    )
    .await;

    let jwt = create_platform_grant_snapshot_jwt();
    let response = client
        .get(snapshot_url(&base_url, project.id(), component.id(), None))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("GET mismatch");
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("JSON");
    assert_eq!(body["grant_revision"], 0);
    assert_eq!(body["consents"][0]["grant_revision"], 9);
}

#[test]
fn t4_handler_does_not_call_world_read() {
    let handler = include_str!("../http/src/handlers/bindings.rs");
    assert!(
        !handler.contains("enforce_world_read_or_principal"),
        "privileged binding read must not call enforce_world_read_or_principal"
    );
    assert!(
        !handler.contains("caller_can_read_component"),
        "privileged binding read must not call caller_can_read_component"
    );
    let usecase = include_str!("../application/src/usecase/component.rs");
    let body = method_body(usecase, "get_binding_grant_snapshot");
    assert!(
        !body.contains("enforce_world_read_or_principal"),
        "get_binding_grant_snapshot must not call enforce_world_read_or_principal"
    );
    assert!(
        !body.contains("caller_can_read_component"),
        "get_binding_grant_snapshot must not call caller_can_read_component"
    );
}

#[test]
fn t4_openfga_model_has_no_apparatus() {
    let path = workspace_root().join("openfga/model.fga");
    let content = std::fs::read_to_string(path).expect("model.fga");
    assert!(
        !content.to_ascii_lowercase().contains("apparatus"),
        "grants must not add an apparatus OpenFGA type"
    );
}

#[test]
fn t4_new_prod_files_forbid_skip_and_runtime_tokens() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = [
        "http/src/handlers/bindings.rs",
        "application/src/dto/binding.rs",
        "application/src/usecase/binding_grant.rs",
        "infra/src/binding_grant_snapshot.rs",
    ];
    for rel in files {
        let content = std::fs::read_to_string(root.join(rel)).expect("prod file");
        for token in [
            "Factory",
            "trusted_skip_gateway",
            "kubernetes",
            "wasm",
            "iframe",
            "apparatus_host",
            "ui_host",
        ] {
            assert!(!content.contains(token), "{rel} must not contain {token}");
        }
        assert!(
            !content.to_ascii_lowercase().contains("host"),
            "{rel} must not contain host"
        );
    }
}
