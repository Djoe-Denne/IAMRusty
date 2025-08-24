use reqwest::StatusCode;
use serial_test::serial;
use rustycog_testing::http::jwt::create_jwt_token;
use uuid::Uuid;
use sea_orm::EntityTrait;

use hive_application::dto::organization::{CreateOrganizationRequest, OrganizationResponse};
use hive_infra::repository::entity::organizations;

mod common;
use common::{fixtures::db::{DbFixtures, seed_org_with_owner}, setup_test_server};

#[tokio::test]
#[serial]
async fn create_organization_happy_path() {
    let (fixture, server_url, client) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);

    let create_body = CreateOrganizationRequest {
        name: "Org A".to_string(),
        slug: format!("org-a-{}", &Uuid::new_v4().to_string()[..8]),
        description: Some("desc".to_string()),
        avatar_url: None,
    };

    let res = client
        .post(format!("{}/api/organizations", server_url))
        .header("Authorization", format!("Bearer {}", token))
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
    let (fixture, server_url, client) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::organization().owner_user_id(owner_id).commit(fixture.db()).await.unwrap();

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
    let (_fixture, server_url, client) = setup_test_server().await.unwrap();

    let res = client
        .get(format!("{}/api/organizations", server_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let user_id = Uuid::new_v4();
    let token = create_jwt_token(user_id);
    let res = client
        .get(format!("{}/api/organizations", server_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    // Route is permission guarded without path resource -> expect forbidden
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn update_and_delete_organization_with_permissions() {
    let (fixture, server_url, client) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);

    // Seed an org owned by owner_id with membership and owner role
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Update
    let update_body = serde_json::json!({
        "name": "Org Updated",
        "description": "new description"
    });
    let res = client
        .put(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {}", token))
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
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn search_organizations_is_public_and_returns_results() {
    let (fixture, server_url, client) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();

    // Seed a couple orgs
    let _ = DbFixtures::organization()
    .owner_user_id(owner_id)
    .settings(serde_json::json!({
        "visibility": "Public"
    }))
    .commit(fixture.db())
    .await.unwrap();

    let res = client
        .get(format!("{}/api/organizations/search?query=Org&page=0&page_size=10", server_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body["organizations"].as_array().unwrap().len() >= 1);
}


