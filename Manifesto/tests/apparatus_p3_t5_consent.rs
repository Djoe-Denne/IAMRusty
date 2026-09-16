//! Apparatus P3 — T5 consent write/revoke (domain HTTP, close-at-commit).

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rustycog::permission::{Permission, ResourceRef, Subject};
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

async fn seed_binding(
    db: &sea_orm::DatabaseConnection,
    component_id: Uuid,
    digest: &str,
    desired_generation: i64,
    grant_revision: i64,
) {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_bindings (component_id, source, digest, desired_generation, grant_revision, declared_capabilities) \
         VALUES ($1, 'managed', $2, $3, $4, '[\"project.read\"]'::jsonb)",
        [
            component_id.into(),
            digest.into(),
            desired_generation.into(),
            grant_revision.into(),
        ],
    ))
    .await
    .expect("INSERT apparatus_bindings");
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

fn consents_url(base: &str, project_id: Uuid, component_id: Uuid) -> String {
    format!("{base}/api/projects/{project_id}/bindings/{component_id}/consents")
}

async fn grant_project_admin(openfga: &TestOpenFga, user_id: Uuid, project_id: Uuid) {
    openfga
        .allow(
            Subject::new(user_id),
            Permission::Admin,
            ResourceRef::new("project", project_id),
        )
        .await
        .expect("OpenFGA admin on project");
}

#[tokio::test]
#[serial]
async fn t5_put_consent_then_revoke_bumps_revision_and_closes() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    seed_binding(&db, component.id(), "release-1", 1, 0).await;
    grant_project_admin(&openfga, owner_id, project.id()).await;

    let jwt = create_test_jwt_token(owner_id);
    let snapshot_jwt = create_platform_grant_snapshot_jwt();
    let put_consented = client
        .put(consents_url(&base_url, project.id(), component.id()))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "capability": "project.read",
            "status": "consented"
        }))
        .send()
        .await
        .expect("PUT consented");
    assert_eq!(put_consented.status(), 200, "admin PUT consented");

    let after_consent: serde_json::Value = client
        .get(snapshot_url(&base_url, project.id(), component.id(), None))
        .header("Authorization", format!("Bearer {snapshot_jwt}"))
        .send()
        .await
        .expect("GET after consent")
        .json()
        .await
        .expect("JSON");
    assert_eq!(after_consent["grant_revision"], 1);
    assert_eq!(after_consent["consents"][0]["capability"], "project.read");
    assert_eq!(after_consent["consents"][0]["status"], "consented");
    assert_eq!(after_consent["consents"][0]["grant_revision"], 1);

    let put_revoked = client
        .put(consents_url(&base_url, project.id(), component.id()))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "capability": "project.read",
            "status": "revoked"
        }))
        .send()
        .await
        .expect("PUT revoked");
    assert_eq!(put_revoked.status(), 200, "admin PUT revoked");

    let after_revoke: serde_json::Value = client
        .get(snapshot_url(&base_url, project.id(), component.id(), None))
        .header("Authorization", format!("Bearer {snapshot_jwt}"))
        .send()
        .await
        .expect("GET after revoke")
        .json()
        .await
        .expect("JSON");
    assert_eq!(after_revoke["grant_revision"], 2);
    assert_eq!(after_revoke["consents"][0]["status"], "revoked");
    assert_eq!(after_revoke["consents"][0]["grant_revision"], 2);
    assert_ne!(
        after_revoke["grant_revision"], after_consent["grant_revision"],
        "revoke must bump grant_revision; old revision cannot be reused"
    );
}

#[tokio::test]
#[serial]
async fn t5_two_consents_share_the_bumped_grant_revision() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    seed_binding(&db, component.id(), "release-1", 1, 0).await;
    grant_project_admin(&openfga, owner_id, project.id()).await;

    let jwt = create_test_jwt_token(owner_id);
    let snapshot_jwt = create_platform_grant_snapshot_jwt();
    let put_read = client
        .put(consents_url(&base_url, project.id(), component.id()))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "capability": "project.read",
            "status": "consented"
        }))
        .send()
        .await
        .expect("PUT project.read");
    assert_eq!(put_read.status(), 200);

    let put_kv = client
        .put(consents_url(&base_url, project.id(), component.id()))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "capability": "storage.kv.read",
            "status": "consented"
        }))
        .send()
        .await
        .expect("PUT storage.kv.read");
    assert_eq!(put_kv.status(), 200);

    let snapshot: serde_json::Value = client
        .get(snapshot_url(&base_url, project.id(), component.id(), None))
        .header("Authorization", format!("Bearer {snapshot_jwt}"))
        .send()
        .await
        .expect("GET snapshot")
        .json()
        .await
        .expect("JSON");
    assert_eq!(snapshot["grant_revision"], 2);
    let consents = snapshot["consents"].as_array().expect("consents");
    assert_eq!(consents.len(), 2);
    for consent in consents {
        assert_eq!(
            consent["grant_revision"], snapshot["grant_revision"],
            "every consent row must match apparatus_bindings.grant_revision"
        );
        assert_eq!(consent["status"], "consented");
    }
    let names: Vec<&str> = consents
        .iter()
        .map(|row| row["capability"].as_str().expect("capability"))
        .collect();
    assert!(names.contains(&"project.read"));
    assert!(names.contains(&"storage.kv.read"));
}

#[tokio::test]
#[serial]
async fn t5_consent_write_requires_jwt_and_project_admin() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _member, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    seed_binding(&db, component.id(), "release-1", 1, 0).await;

    let url = consents_url(&base_url, project.id(), component.id());
    let body = serde_json::json!({
        "capability": "project.read",
        "status": "consented"
    });

    let unauth = client
        .put(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("PUT sans JWT");
    assert_eq!(unauth.status(), 401);

    let stranger = Uuid::new_v4();
    let world_read = create_test_jwt_token(stranger);
    openfga
        .allow_wildcard(Permission::Read, ResourceRef::new("project", project.id()))
        .await
        .expect("world-read wildcard");
    let public_write = client
        .put(&url)
        .header("Authorization", format!("Bearer {world_read}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("PUT world-read");
    assert_eq!(
        public_write.status(),
        403,
        "world-read / public Read must not grant consent write"
    );
}

#[tokio::test]
#[serial]
async fn t5_member_removal_closes_principal_in_db_even_if_fga_still_allows() {
    let (fixture, base_url, client, openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let member_id = Uuid::new_v4();
    let (project, _owner, component) =
        DbFixtures::create_project_with_component(&db, owner_id, "taskboard")
            .await
            .expect("composant");
    DbFixtures::member()
        .direct(project.id(), member_id, owner_id)
        .commit(db.clone())
        .await
        .expect("membre");
    seed_binding(&db, component.id(), "release-1", 1, 0).await;
    grant_project_admin(&openfga, owner_id, project.id()).await;
    openfga
        .allow(
            Subject::new(member_id),
            Permission::Read,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("stale FGA allow for member — left in place after DELETE");

    let jwt = create_test_jwt_token(owner_id);
    let snapshot_jwt = create_platform_grant_snapshot_jwt();
    let put = client
        .put(consents_url(&base_url, project.id(), component.id()))
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "capability": "project.read",
            "status": "consented"
        }))
        .send()
        .await
        .expect("PUT consent");
    assert_eq!(put.status(), 200);

    let before: serde_json::Value = client
        .get(snapshot_url(
            &base_url,
            project.id(),
            component.id(),
            Some(member_id),
        ))
        .header("Authorization", format!("Bearer {snapshot_jwt}"))
        .send()
        .await
        .expect("GET before remove")
        .json()
        .await
        .expect("JSON");
    assert_eq!(before["principal"]["user_id"], member_id.to_string());
    assert_eq!(before["principal"]["active"], true);

    let deleted = client
        .delete(format!(
            "{base_url}/api/projects/{}/members/{member_id}",
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .expect("DELETE member");
    assert_eq!(deleted.status(), 204);

    let after: serde_json::Value = client
        .get(snapshot_url(
            &base_url,
            project.id(),
            component.id(),
            Some(member_id),
        ))
        .header("Authorization", format!("Bearer {snapshot_jwt}"))
        .send()
        .await
        .expect("GET after remove")
        .json()
        .await
        .expect("JSON");
    assert_eq!(after["principal"]["user_id"], member_id.to_string());
    assert_eq!(
        after["principal"]["active"], false,
        "DB membership is the source; OpenFGA tuples were left allowing"
    );
}

#[test]
fn t5_openfga_model_has_no_apparatus() {
    let path = workspace_root().join("openfga/model.fga");
    let content = std::fs::read_to_string(path).expect("model.fga");
    assert!(
        !content.to_ascii_lowercase().contains("apparatus"),
        "grants must not add an apparatus OpenFGA type"
    );
}

#[test]
fn t5_handler_does_not_call_world_read() {
    let handler = include_str!("../http/src/handlers/bindings.rs");
    assert!(
        !handler.contains("enforce_world_read_or_principal"),
        "consent write must not call enforce_world_read_or_principal"
    );
    let usecase = include_str!("../application/src/usecase/component.rs");
    assert!(
        !usecase.contains("upsert_binding_consent")
            || !method_contains_world_read(usecase, "upsert_binding_consent"),
        "upsert_binding_consent must not call world-read helpers"
    );
}

fn method_contains_world_read(src: &str, name: &str) -> bool {
    let needle = format!("async fn {name}");
    let Some(start) = src.rfind(&needle) else {
        return true;
    };
    let from = &src[start..];
    let open = from.find('{').unwrap_or(0);
    let mut depth = 0_i32;
    let mut end = from.len();
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
    let body = &from[..end];
    body.contains("enforce_world_read_or_principal") || body.contains("caller_can_read_component")
}

#[test]
fn t5_prod_files_forbid_gateway_lazaret_and_skip() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = [
        "http/src/handlers/bindings.rs",
        "application/src/dto/binding.rs",
        "application/src/usecase/binding_grant.rs",
        "infra/src/binding_grant_snapshot.rs",
    ];
    for rel in files {
        let content = std::fs::read_to_string(root.join(rel)).expect("prod file");
        for token in ["gateway", "lazaret", "trusted_skip_gateway"] {
            assert!(
                !content.to_ascii_lowercase().contains(token),
                "{rel} must not contain {token}"
            );
        }
    }
}
