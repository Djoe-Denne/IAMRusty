use reqwest::StatusCode;
use serial_test::serial;
use rustycog_testing::http::jwt::create_jwt_token;
use uuid::Uuid;
use sea_orm::{EntityTrait, ActiveModelTrait, Set};

use hive_application::dto::member::{AddMemberRequest, MemberResponse};
use hive_application::dto::role::{MemberRole, MemberRolePermission};
use hive_infra::repository::entity::{organization_members, role_permissions, organization_member_role_permissions};

mod common;
use common::{fixtures::db::DbFixtures, setup_test_server, Permission, ResourceRef, Subject};

#[tokio::test]
#[serial]
async fn add_member_requires_auth() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    let new_user = Uuid::new_v4();
    let body = AddMemberRequest {
        user_id: new_user,
        roles: vec![MemberRole { organization_id: org.id, resource: "organization".to_string(), permissions: MemberRolePermission::Read }],
    };

    let res = client
        .post(format!("{}/api/organizations/{}/members", server_url, org.id))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn add_member_forbidden_for_read_only_member() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let read_user_id = Uuid::new_v4();
    let token = create_jwt_token(read_user_id);

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

    // Route guard: `with_permission_on(Permission::Write, "organization")`
    // on `POST /organizations/{org_id}/members`. Trailing UUID = org_id.
    // Reset the harness's permissive default and mount a deny matching
    // the exact tuple the middleware will Check.
    openfga.reset().await;
    openfga
        .mock_check_deny(
            Subject::new(read_user_id),
            Permission::Write,
            ResourceRef::new("organization", org.id),
        )
        .await;

    let new_user = Uuid::new_v4();
    let body = AddMemberRequest {
        user_id: new_user,
        roles: vec![MemberRole { organization_id: org.id, resource: "organization".to_string(), permissions: MemberRolePermission::Read }],
    };

    let res = client
        .post(format!("{}/api/organizations/{}/members", server_url, org.id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn add_member_happy_path_by_owner() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    let new_user = Uuid::new_v4();
    let body = AddMemberRequest {
        user_id: new_user,
        roles: vec![MemberRole { organization_id: org.id, resource: "organization".to_string(), permissions: MemberRolePermission::Read }],
    };

    let res = client
        .post(format!("{}/api/organizations/{}/members", server_url, org.id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let member: MemberResponse = res.json().await.unwrap();
    assert_eq!(member.organization_id, org.id);
    assert_eq!(member.user_id, new_user);

    // Assert in DB
    let got = organization_members::Entity::find_by_id(member.id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.user_id, new_user);
}

#[tokio::test]
#[serial]
async fn list_members_requires_auth_and_forbids_non_member() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Arrange: random non-member user the second sub-assertion will deny.
    // Route guard: `with_permission_on(Permission::Read, "organization")`
    // on `GET /organizations/{org_id}/members`. Trailing UUID = org_id.
    let random_user = Uuid::new_v4();
    openfga.reset().await;
    openfga
        .mock_check_deny(
            Subject::new(random_user),
            Permission::Read,
            ResourceRef::new("organization", org.id),
        )
        .await;

    // No auth — the strict 401 path doesn't touch the checker.
    let res = client
        .get(format!("{}/api/organizations/{}/members?page=0&page_size=10", server_url, org.id))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Random user (non member) — Check returns deny per the stub above.
    let token = create_jwt_token(random_user);
    let res = client
        .get(format!("{}/api/organizations/{}/members?page=0&page_size=10", server_url, org.id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn list_members_happy_path_for_owner() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    let res = client
        .get(format!("{}/api/organizations/{}/members?page=0&page_size=10", server_url, org.id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body["members"].as_array().unwrap().len() >= 1);
}

#[tokio::test]
#[serial]
async fn get_member_requires_auth_and_forbids_non_member() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Route: `GET /organizations/{org_id}/members/{user_id}` with
    // `with_permission_on(Permission::Read, "organization")`. The
    // middleware extracts the **trailing** UUID — `owner_id` from the URL
    // — as the resource id, so the deny tuple is
    // `(random_user, Read, organization:owner_id)`, not
    // `(random_user, Read, organization:org_id)`.
    let random_user = Uuid::new_v4();
    openfga.reset().await;
    openfga
        .mock_check_deny(
            Subject::new(random_user),
            Permission::Read,
            ResourceRef::new("organization", owner_id),
        )
        .await;

    let res = client
        .get(format!("{}/api/organizations/{}/members/{}", server_url, org.id, owner_id))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let token = create_jwt_token(random_user);
    let res = client
        .get(format!("{}/api/organizations/{}/members/{}", server_url, org.id, owner_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn get_member_happy_path_for_owner() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    let res = client
        .get(format!("{}/api/organizations/{}/members/{}", server_url, org.id, owner_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let got: serde_json::Value = res.json().await.unwrap();
    assert_eq!(got["organization_id"].as_str().unwrap(), org.id.to_string());
    assert_eq!(got["user_id"].as_str().unwrap(), owner_id.to_string());
}

#[tokio::test]
#[serial]
async fn remove_member_requires_auth_and_forbids_read_only() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let read_user_id = Uuid::new_v4();
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

    // Route: `DELETE /organizations/{org_id}/members/{user_id}` with
    // `with_permission_on(Permission::Write, "organization")`. The
    // trailing UUID in the URL `/members/{read_user_id}` is
    // `read_user_id`, so the resource id the middleware Checks against
    // is `read_user_id` (not `org.id`).
    openfga.reset().await;
    openfga
        .mock_check_deny(
            Subject::new(read_user_id),
            Permission::Write,
            ResourceRef::new("organization", read_user_id),
        )
        .await;

    // No auth — strict 401 path.
    let res = client
        .delete(format!("{}/api/organizations/{}/members/{}", server_url, org.id, read_user_id))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Read-only cannot remove — Check returns deny per the stub above.
    let token = create_jwt_token(read_user_id);
    let res = client
        .delete(format!("{}/api/organizations/{}/members/{}", server_url, org.id, read_user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn remove_member_happy_path_by_owner() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let read_user_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org(fixture.db().as_ref(), owner_id, std::collections::HashMap::from([
        (owner_id.to_string(), "owner".to_string()),
        (read_user_id.to_string(), "read".to_string()),
    ]))
        .await
        .unwrap();

    let res = client
        .delete(format!("{}/api/organizations/{}/members/{}", server_url, org.id, read_user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}


