use reqwest::StatusCode;
use rustycog::testing::http::jwt::create_jwt_token;
use serial_test::serial;
use uuid::Uuid;

mod common;
use common::{fixtures::db::DbFixtures, setup_test_server, Permission, ResourceRef, Subject};

#[tokio::test]
#[serial]
async fn roles_are_listed_and_resolved_only_within_the_authorized_organization() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();
    let other_organization = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    let resource_id = Uuid::new_v4().to_string();
    DbFixtures::resource()
        .id(&resource_id)
        .name(&resource_id)
        .commit(fixture.db())
        .await
        .unwrap();
    let custom_role = DbFixtures::role_permission()
        .organization_id(organization.id)
        .permission_id("read")
        .resource_id(&resource_id)
        .name("custom-reader")
        .description(Some("Reader role used by the integration route"))
        .commit(fixture.db())
        .await
        .unwrap();

    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("organization", organization.id),
        )
        .await
        .expect("failed to grant read on the requested organization");

    let list = client
        .get(format!(
            "{server_url}/api/organizations/{}/roles?page=1&page_size=2",
            organization.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let page: serde_json::Value = list.json().await.unwrap();
    assert_eq!(page["roles"].as_array().unwrap().len(), 2);

    let detail = client
        .get(format!(
            "{server_url}/api/organizations/{}/roles/{}",
            organization.id, custom_role.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let role: serde_json::Value = detail.json().await.unwrap();
    assert_eq!(role["organization_id"], organization.id.to_string());
    assert_eq!(role["resource"], resource_id);

    let forbidden = client
        .get(format!(
            "{server_url}/api/organizations/{}/roles",
            other_organization.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn a_role_from_another_organization_is_not_exposed_even_after_route_authorization() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let first = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();
    let second = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();
    let resource_id = Uuid::new_v4().to_string();
    DbFixtures::resource()
        .id(&resource_id)
        .name(&resource_id)
        .commit(fixture.db())
        .await
        .unwrap();
    let role = DbFixtures::role_permission()
        .organization_id(first.id)
        .permission_id("read")
        .resource_id(&resource_id)
        .name("first-org-only")
        .commit(fixture.db())
        .await
        .unwrap();

    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("organization", second.id),
        )
        .await
        .expect("failed to grant route authorization on the second organization");

    let response = client
        .get(format!(
            "{server_url}/api/organizations/{}/roles/{}",
            second.id, role.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_server_error());
}

#[tokio::test]
#[serial]
async fn listing_roles_requires_auth_and_unknown_role_is_not_exposed() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    let unauthenticated = client
        .get(format!(
            "{server_url}/api/organizations/{}/roles",
            organization.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("organization", organization.id),
        )
        .await
        .expect("failed to grant read");

    let missing = client
        .get(format!(
            "{server_url}/api/organizations/{}/roles/{}",
            organization.id,
            Uuid::new_v4()
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert!(missing.status().is_client_error() || missing.status().is_server_error());
}
