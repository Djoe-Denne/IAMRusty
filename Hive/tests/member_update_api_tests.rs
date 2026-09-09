use reqwest::StatusCode;
use rustycog::testing::http::jwt::create_jwt_token;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;
use uuid::Uuid;

use hive_application::dto::member::{AddMemberRequest, MemberResponse, UpdateMemberRolesRequest};
use hive_application::dto::role::{MemberRole, MemberRolePermission};
use hive_infra::repository::entity::organization_members;

mod common;
use common::{fixtures::db::DbFixtures, setup_test_server, Permission, ResourceRef, Subject};

fn write_roles(organization_id: Uuid) -> UpdateMemberRolesRequest {
    UpdateMemberRolesRequest {
        roles: vec![MemberRole {
            organization_id,
            resource: "organization".to_string(),
            permissions: MemberRolePermission::Write,
        }],
    }
}

#[tokio::test]
#[serial]
async fn member_role_update_happy_path_keeps_the_membership() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let member_user_id = Uuid::new_v4();
    let owner_token = create_jwt_token(owner_id);
    let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", organization.id),
        )
        .await
        .expect("failed to grant owner write");

    let added = client
        .post(format!(
            "{server_url}/api/organizations/{}/members",
            organization.id
        ))
        .header("Authorization", format!("Bearer {owner_token}"))
        .json(&AddMemberRequest {
            user_id: member_user_id,
            roles: vec![MemberRole {
                organization_id: organization.id,
                resource: "organization".to_string(),
                permissions: MemberRolePermission::Read,
            }],
        })
        .send()
        .await
        .unwrap();
    assert_eq!(added.status(), StatusCode::OK);
    let member: MemberResponse = added.json().await.unwrap();

    let response = client
        .patch(format!(
            "{server_url}/api/organizations/{}/members/{member_user_id}",
            organization.id
        ))
        .header("Authorization", format!("Bearer {owner_token}"))
        .json(&write_roles(organization.id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated: MemberResponse = response.json().await.unwrap();
    assert_eq!(updated.id, member.id);
    assert_eq!(updated.user_id, member_user_id);

    let stored = organization_members::Entity::find()
        .filter(organization_members::Column::OrganizationId.eq(organization.id))
        .filter(organization_members::Column::UserId.eq(member_user_id))
        .one(fixture.db().as_ref())
        .await
        .unwrap();
    assert!(
        stored.is_some(),
        "the update route must retain the membership"
    );
}

#[tokio::test]
#[serial]
async fn invalid_member_role_does_not_mutate_the_existing_member() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let member_id = Uuid::new_v4();
    let owner_token = create_jwt_token(owner_id);
    let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", organization.id),
        )
        .await
        .unwrap();
    let added = client
        .post(format!(
            "{server_url}/api/organizations/{}/members",
            organization.id
        ))
        .header("Authorization", format!("Bearer {owner_token}"))
        .json(&AddMemberRequest {
            user_id: member_id,
            roles: vec![MemberRole {
                organization_id: organization.id,
                resource: "organization".to_string(),
                permissions: MemberRolePermission::Read,
            }],
        })
        .send()
        .await
        .unwrap();
    assert_eq!(added.status(), StatusCode::OK);

    let bad_request = serde_json::json!({
        "roles": [{
            "organization_id": organization.id,
            "resource": "organization",
            "permissions": "Delete"
        }]
    });
    let response = client
        .patch(format!(
            "{server_url}/api/organizations/{}/members/{member_id}",
            organization.id
        ))
        .header("Authorization", format!("Bearer {owner_token}"))
        .json(&bad_request)
        .send()
        .await
        .unwrap();
    assert!(response.status().is_server_error());

    let member = organization_members::Entity::find()
        .filter(organization_members::Column::OrganizationId.eq(organization.id))
        .filter(organization_members::Column::UserId.eq(member_id))
        .one(fixture.db().as_ref())
        .await
        .unwrap();
    assert!(
        member.is_some(),
        "a failed conversion must leave the original row intact"
    );
}
