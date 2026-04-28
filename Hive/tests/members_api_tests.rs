use reqwest::StatusCode;
use rustycog_testing::http::jwt::create_jwt_token;
use sea_orm::{ActiveModelTrait, EntityTrait};
use serial_test::serial;
use uuid::Uuid;

use hive_application::dto::member::{AddMemberRequest, MemberResponse};
use hive_application::dto::role::{MemberRole, MemberRolePermission};
use hive_infra::repository::entity::organization_members;

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
        roles: vec![MemberRole {
            organization_id: org.id,
            resource: "organization".to_string(),
            permissions: MemberRolePermission::Read,
        }],
    };

    let res = client
        .post(format!(
            "{}/api/organizations/{}/members",
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
async fn add_member_forbidden_for_read_only_member() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
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

    // Default-deny: route guard
    // `with_permission_on(Permission::Write, "organization")` on
    // `POST /organizations/{org_id}/members` will issue
    // `Check(read_user, write, organization:<org_id>)`. Real OpenFGA
    // returns false because no tuple has been written, so the request
    // 403s without any explicit arrange.

    let new_user = Uuid::new_v4();
    let body = AddMemberRequest {
        user_id: new_user,
        roles: vec![MemberRole {
            organization_id: org.id,
            resource: "organization".to_string(),
            permissions: MemberRolePermission::Read,
        }],
    };

    let res = client
        .post(format!(
            "{}/api/organizations/{}/members",
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
async fn add_member_happy_path_by_owner() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Route guard: `with_permission_on(Permission::Write, "organization")`.
    // POST /organizations/{org_id}/members trailing UUID = org.id.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization write");

    let new_user = Uuid::new_v4();
    let body = AddMemberRequest {
        user_id: new_user,
        roles: vec![MemberRole {
            organization_id: org.id,
            resource: "organization".to_string(),
            permissions: MemberRolePermission::Read,
        }],
    };

    let res = client
        .post(format!(
            "{}/api/organizations/{}/members",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
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
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Default-deny: route guard
    // `with_permission_on(Permission::Read, "organization")` on
    // `GET /organizations/{org_id}/members` will issue
    // `Check(random_user, read, organization:<org_id>)`. Real OpenFGA
    // returns false because no tuple has been written for that subject,
    // so the second sub-assertion 403s without any explicit arrange.
    let random_user = Uuid::new_v4();

    // No auth — the strict 401 path doesn't touch the checker.
    let res = client
        .get(format!(
            "{}/api/organizations/{}/members?page=0&page_size=10",
            server_url, org.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Random user (non member) — Check returns deny per the stub above.
    let token = create_jwt_token(random_user);
    let res = client
        .get(format!(
            "{}/api/organizations/{}/members?page=0&page_size=10",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn list_members_happy_path_for_owner() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Route guard: `with_permission_on(Permission::Read, "organization")`.
    // GET /organizations/{org_id}/members trailing UUID = org.id.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization read");

    let res = client
        .get(format!(
            "{}/api/organizations/{}/members?page=0&page_size=10",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(!body["members"].as_array().unwrap().is_empty());
}

#[tokio::test]
#[serial]
async fn get_member_requires_auth_and_forbids_non_member() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Default-deny: route
    // `GET /organizations/{org_id}/members/{user_id}` with guard
    // `with_permission_on(Permission::Read, "organization")` extracts the
    // **trailing** UUID — `owner_id` from the URL — as the resource id.
    // Real OpenFGA returns false for
    // `Check(random_user, read, organization:<owner_id>)` because no
    // tuple has been written for that subject, so the request 403s.
    let random_user = Uuid::new_v4();

    let res = client
        .get(format!(
            "{}/api/organizations/{}/members/{}",
            server_url, org.id, owner_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let token = create_jwt_token(random_user);
    let res = client
        .get(format!(
            "{}/api/organizations/{}/members/{}",
            server_url, org.id, owner_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn get_member_happy_path_for_owner() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Route guard: `with_permission_on(Permission::Read, "organization")`.
    // GET /members/{user_id} trailing UUID = owner_id (the user id).
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("organization", owner_id),
        )
        .await
        .expect("Failed to grant organization read");

    let res = client
        .get(format!(
            "{}/api/organizations/{}/members/{}",
            server_url, org.id, owner_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let got: serde_json::Value = res.json().await.unwrap();
    assert_eq!(got["organization_id"].as_str().unwrap(), org.id.to_string());
    assert_eq!(got["user_id"].as_str().unwrap(), owner_id.to_string());
}

#[tokio::test]
#[serial]
async fn remove_member_requires_auth_and_forbids_read_only() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
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

    // Default-deny: route
    // `DELETE /organizations/{org_id}/members/{user_id}` with guard
    // `with_permission_on(Permission::Write, "organization")`. The
    // trailing UUID in the URL `/members/{read_user_id}` is
    // `read_user_id`, so the middleware Checks
    // `(read_user, write, organization:<read_user_id>)`. Real OpenFGA
    // returns false because no tuple was written.

    // No auth — strict 401 path.
    let res = client
        .delete(format!(
            "{}/api/organizations/{}/members/{}",
            server_url, org.id, read_user_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Read-only cannot remove — Check returns deny per the stub above.
    let token = create_jwt_token(read_user_id);
    let res = client
        .delete(format!(
            "{}/api/organizations/{}/members/{}",
            server_url, org.id, read_user_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn remove_member_happy_path_by_owner() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let read_user_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
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

    // Route guard: `with_permission_on(Permission::Write, "organization")`.
    // DELETE /members/{user_id} trailing UUID = read_user_id.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", read_user_id),
        )
        .await
        .expect("Failed to grant organization write");

    let res = client
        .delete(format!(
            "{}/api/organizations/{}/members/{}",
            server_url, org.id, read_user_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
