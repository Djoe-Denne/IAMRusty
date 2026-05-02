use reqwest::StatusCode;
use rustycog_testing::http::jwt::create_jwt_token;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serial_test::serial;
use std::collections::HashMap;
use uuid::Uuid;

use hive_application::dto::organization::{CreateOrganizationRequest, OrganizationResponse};
use hive_infra::repository::entity::organizations;
use hive_infra::repository::entity::{external_links, external_providers};

mod common;
use common::{fixtures::db::DbFixtures, setup_test_server, Permission, ResourceRef, Subject};

#[tokio::test]
#[serial]
async fn create_organization_happy_path() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);

    let create_body = CreateOrganizationRequest {
        name: "Org A".to_string(),
        slug: format!("org-a-{}", &Uuid::new_v4().to_string()[..8]),
        description: Some("desc".to_string()),
        avatar_url: None,
    };

    let res = client
        .post(format!("{server_url}/api/organizations"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&create_body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let created_org: OrganizationResponse = res.json().await.unwrap();

    // Act - select by sql
    let org = organizations::Entity::find_by_id(created_org.id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(org.name, created_org.name);
}

#[tokio::test]
#[serial]
async fn get_organization_happy_path() {
    // Arrange
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    // Act - get
    let res = client
        .get(format!("{}/api/organizations/{}", server_url, org.id))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let got: serde_json::Value = res.json().await.unwrap();
    assert_eq!(got.get("id").unwrap().as_str().unwrap(), org.id.to_string());
}

#[tokio::test]
#[serial]
async fn list_requires_auth_and_returns_empty_initially() {
    let (_fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();

    let res = client
        .get(format!("{server_url}/api/organizations"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let user_id = Uuid::new_v4();
    let token = create_jwt_token(user_id);
    let res = client
        .get(format!("{server_url}/api/organizations"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    // `GET /api/organizations` is `.authenticated()` only — there is no
    // `.with_permission_on(...)` on this route in `Hive/http/src/lib.rs`,
    // so any authenticated user reaches the handler. The test's stale
    // 403 expectation predated that route shape; the test name's
    // "returns_empty_initially" already encoded the right intent. Assert
    // 200 + empty data array.
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let items = body["data"]
        .as_array()
        .or_else(|| body["organizations"].as_array())
        .or_else(|| body.as_array())
        .expect("list response should expose an array of organizations");
    assert!(items.is_empty(), "fresh org list should be empty initially");
}

#[tokio::test]
#[serial]
async fn update_and_delete_organization_with_permissions() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);

    // Seed an org owned by owner_id with membership and owner role
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // PUT and DELETE on `/api/organizations/{org_id}` both require
    // `Permission::Admin, "organization"`. Trailing UUID = org.id.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization admin");

    // Update
    let update_body = serde_json::json!({
        "name": "Org Updated",
        "description": "new description"
    });
    let res = client
        .put(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {token}"))
        .json(&update_body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let updated: serde_json::Value = res.json().await.unwrap();
    assert_eq!(updated["name"], "Org Updated");

    // Delete
    let res = client
        .delete(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn search_organizations_is_public_and_returns_results() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();

    // Seed a couple orgs
    let _ = DbFixtures::organization()
        .owner_user_id(owner_id)
        .settings(serde_json::json!({
            "visibility": "Public"
        }))
        .commit(fixture.db())
        .await
        .unwrap();

    let res = client
        .get(format!(
            "{server_url}/api/organizations/search?query=Org&page=0&page_size=10"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(!body["organizations"].as_array().unwrap().is_empty());
}

async fn update_and_delete_require_auth() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    let res = client
        .put(format!("{}/api/organizations/{}", server_url, org.id))
        .json(&serde_json::json!({"name":"X"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = client
        .delete(format!("{}/api/organizations/{}", server_url, org.id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn sync_jobs_requires_auth() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    let body = serde_json::json!({
        "external_link_id": Uuid::new_v4(),
        "job_type": "full_sync",
        "options": null
    });
    let res = client
        .post(format!(
            "{}/api/organizations/{}/sync-jobs",
            server_url, org.id
        ))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn roles_endpoints_require_auth() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    let res = client
        .get(format!(
            "{}/api/organizations/{}/roles?page=1&page_size=10",
            server_url, org.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = client
        .get(format!(
            "{}/api/organizations/{}/roles/{}",
            server_url,
            org.id,
            Uuid::new_v4()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn update_delete_forbidden_for_read_only_member() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let read_user_id = Uuid::new_v4();
    let token = create_jwt_token(read_user_id);

    let org = DbFixtures::create_org(
        fixture.db().as_ref(),
        owner_id,
        HashMap::from([
            (owner_id.to_string(), "owner".to_string()),
            (read_user_id.to_string(), "read".to_string()),
        ]),
    )
    .await
    .unwrap();

    // Default-deny: PUT and DELETE on `/api/organizations/{org_id}` both
    // require `Permission::Admin, "organization"`. Trailing UUID = org.id.
    // Real OpenFGA returns false for
    // `Check(read_user, admin, organization:<org_id>)` because no tuple
    // has been written, so both calls 403.

    let res = client
        .put(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"name":"New"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = client
        .delete(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn sync_jobs_forbidden_for_read_only_member() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let read_user_id = Uuid::new_v4();
    let token = create_jwt_token(read_user_id);

    let org = DbFixtures::create_org(
        fixture.db().as_ref(),
        owner_id,
        HashMap::from([
            (owner_id.to_string(), "owner".to_string()),
            (read_user_id.to_string(), "read".to_string()),
        ]),
    )
    .await
    .unwrap();

    // Default-deny: `POST /api/organizations/{org_id}/sync-jobs` requires
    // `Permission::Write, "organization"`. The path's trailing UUID is
    // `org.id` (`sync-jobs` is a string segment). Real OpenFGA returns
    // false because no tuple has been written, so the call 403s.

    let body = serde_json::json!({
        "external_link_id": Uuid::new_v4(),
        "job_type": "full_sync",
        "options": null
    });
    let res = client
        .post(format!(
            "{}/api/organizations/{}/sync-jobs",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn get_nonexistent_organization_returns_404() {
    let (_fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let res = client
        .get(format!(
            "{}/api/organizations/{}",
            server_url,
            Uuid::new_v4()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn sync_jobs_nonexistent_external_link_returns_404() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Route guard must pass so this test reaches the handler's
    // nonexistent-external-link branch.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization write");

    let body = serde_json::json!({
        "external_link_id": Uuid::new_v4(),
        "job_type": "full_sync",
        "options": null
    });
    let res = client
        .post(format!(
            "{}/api/organizations/{}/sync-jobs",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn create_validation_errors() {
    let (_fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let user_id = Uuid::new_v4();
    let token = create_jwt_token(user_id);

    // Create with invalid payload (empty name)
    let bad_create = serde_json::json!({
        "name": "",
        "slug": "abc",
        "description": null,
        "avatar_url": null
    });
    let res = client
        .post(format!("{server_url}/api/organizations"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&bad_create)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 422);
}

#[tokio::test]
#[serial]
async fn start_sync_job_happy_path() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Route guard: `with_permission_on(Permission::Write, "organization")`
    // on `POST /api/organizations/{org_id}/sync-jobs`.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization write");

    // Seed external provider
    let provider = external_providers::ActiveModel {
        id: Set(Uuid::new_v4()),
        provider_type: Set("github".to_string()),
        name: Set("GitHub".to_string()),
        config_schema: Set(None),
        is_active: Set(true),
        created_at: Set(chrono::Utc::now()),
    }
    .insert(fixture.db().as_ref())
    .await
    .unwrap();

    // Seed external link with sync enabled
    let elink = external_links::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(org.id),
        provider_id: Set(provider.id),
        provider_config: Set(serde_json::json!({"org":"dummy"})),
        sync_enabled: Set(true),
        sync_settings: Set(serde_json::json!({})),
        last_sync_at: Set(None),
        last_sync_status: Set(None),
        sync_error: Set(None),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    }
    .insert(fixture.db().as_ref())
    .await
    .unwrap();

    let body = serde_json::json!({
        "external_link_id": elink.id,
        "job_type": "full_sync",
        "options": null
    });
    let res = client
        .post(format!(
            "{}/api/organizations/{}/sync-jobs",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
