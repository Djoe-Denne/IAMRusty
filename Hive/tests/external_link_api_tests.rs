use reqwest::StatusCode;
use serial_test::serial;
use rustycog_testing::http::jwt::create_jwt_token;
use uuid::Uuid;

mod common;
use common::{
    fixtures::{db::DbFixtures, ExternalProviderFixtures},
    setup_test_server, Permission, ResourceRef, Subject,
};

#[tokio::test]
#[serial]
async fn create_external_link_happy_path() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);

    // Arrange: seed org owned by owner_id
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

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
            "{}/api/organizations/{}/external-links",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let response_json: serde_json::Value = res.json().await.unwrap();
    assert_eq!(response_json["organization_id"].as_str().unwrap(), org.id.to_string());
    assert_eq!(response_json["provider_id"].as_str().unwrap(), provider.id.to_string());
    assert_eq!(response_json["provider_name"].as_str().unwrap().to_lowercase(), "github");
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
            "{}/api/organizations/{}/external-links",
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
async fn create_external_link_forbidden_for_read_only_member() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
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

    // We're testing the denial path. The route guard
    // `with_permission_on(Permission::Admin, "organization")` will Check
    // `(read_user_id, Admin, organization:<org_id>)`. The harness's
    // permissive `mock_check_any(true)` default would let this through
    // and the handler would then 500 looking up an absent provider, so
    // wipe the default first and mount a per-tuple deny matching the
    // tuple the middleware will actually issue (wiremock matches in
    // registration order, so a deny mounted on top of the catch-all
    // would never fire).
    openfga.reset().await;
    openfga
        .mock_check_deny(
            Subject::new(read_user_id),
            Permission::Admin,
            ResourceRef::new("organization", org.id),
        )
        .await;

    let body = serde_json::json!({
        "provider_id": Uuid::new_v4(),
        "provider_config": {"org": "example"}
    });

    let res = client
        .post(format!(
            "{}/api/organizations/{}/external-links",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}


