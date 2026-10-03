use reqwest::StatusCode;
use rustycog::testing::http::jwt::create_jwt_token;
use serial_test::serial;
use uuid::Uuid;

mod common;
use common::{fixtures::db::DbFixtures, setup_test_server, Permission, ResourceRef, Subject};

#[tokio::test]
#[serial]
async fn create_external_link_happy_path() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);

    // Arrange: seed org owned by owner_id
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Route guard: `with_permission_on(Permission::Admin, "organization")`
    // on `POST /api/organizations/{org_id}/external-links`.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization admin");

    // Seed external provider via fixtures
    let provider = DbFixtures::external_provider()
        .provider_source("github")
        .name("GitHub")
        .commit(fixture.db())
        .await
        .unwrap();

    // Prepare request body
    let body = serde_json::json!({
        "provider_id": provider.id,
        "provider_config": {"org": "example"},
        "sync_enabled": false,
        "sync_settings": {}
    });

    // Act - call API
    let res = client
        .post(format!(
            "{server_url}/api/organizations/{}/external-links",
            org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let response_json: serde_json::Value = res.json().await.unwrap();
    assert_eq!(
        response_json["organization_id"].as_str().unwrap(),
        org.id.to_string()
    );
    assert_eq!(
        response_json["provider_id"].as_str().unwrap(),
        provider.id.to_string()
    );
    assert_eq!(
        response_json["provider_name"]
            .as_str()
            .unwrap()
            .to_lowercase(),
        "github"
    );
}

#[tokio::test]
#[serial]
async fn create_external_link_requires_auth() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    let body = serde_json::json!({
        "provider_id": Uuid::new_v4(),
        "provider_config": {"org": "example"}
    });

    let res = client
        .post(format!(
            "{server_url}/api/organizations/{}/external-links",
            org.id
        ))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn create_external_link_forbidden_for_read_only_member() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let read_user_id = Uuid::new_v4();
    let token = create_jwt_token(read_user_id);

    // Owner + read member
    let org = DbFixtures::create_org(
        fixture.db().as_ref(),
        owner_id,
        std::collections::HashMap::from([
            (owner_id.to_string(), "owner".to_string()),
            (read_user_id.to_string(), "read".to_string()),
        ]),
    )
    .await
    .unwrap();

    // Default-deny: the route guard
    // `with_permission_on(Permission::Admin, "organization")` will Check
    // `(read_user_id, admin, organization:<org_id>)`. Real OpenFGA
    // returns false because no tuple has been written for the read user,
    // so the request 403s before the handler runs.

    let body = serde_json::json!({
        "provider_id": Uuid::new_v4(),
        "provider_config": {"org": "example"}
    });

    let res = client
        .post(format!(
            "{server_url}/api/organizations/{}/external-links",
            org.id
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
async fn create_external_link_rejects_invalid_payload_and_unknown_provider() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization admin");

    let invalid = client
        .post(format!(
            "{server_url}/api/organizations/{}/external-links",
            org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"provider_config": {"org": "example"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let _provider_mock = common::fixtures::ExternalProviderFixtures::service().await;
    let unknown = client
        .post(format!(
            "{server_url}/api/organizations/{}/external-links",
            org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "provider_id": Uuid::new_v4(),
            "provider_config": {"org": "example"},
            "sync_enabled": true,
            "sync_settings": {}
        }))
        .send()
        .await
        .unwrap();
    assert!(unknown.status().is_client_error() || unknown.status().is_server_error());
}
