use reqwest::StatusCode;
use rustycog::testing::http::jwt::create_jwt_token;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;
use std::collections::HashMap;
use uuid::Uuid;

use hive_infra::repository::entity::organization_members;

mod common;
use common::{fixtures::db::DbFixtures, setup_test_server, Permission, ResourceRef, Subject};

fn read_role(organization_id: Uuid) -> serde_json::Value {
    serde_json::json!({
        "roles": [{
            "organization_id": organization_id,
            "resource": "organization",
            "permissions": "Read"
        }]
    })
}

#[tokio::test]
#[serial]
async fn member_role_update_failure_is_exposed_without_losing_the_membership() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let member_id = Uuid::new_v4();
    let owner_token = create_jwt_token(owner_id);
    let organization = DbFixtures::create_org(
        fixture.db().as_ref(),
        owner_id,
        HashMap::from([
            (owner_id.to_string(), "owner".to_string()),
            (member_id.to_string(), "read".to_string()),
        ]),
    )
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

    let response = client
        .patch(format!(
            "{server_url}/api/organizations/{}/members/{member_id}",
            organization.id
        ))
        .header("Authorization", format!("Bearer {owner_token}"))
        .json(&read_role(organization.id))
        .send()
        .await
        .unwrap();
    // The live route currently delegates the user id as a membership id to
    // the role service, which rejects the operation. This characterization
    // test protects the externally visible failure and its atomicity until
    // that production behavior is changed in a dedicated feature.
    assert!(response.status().is_server_error());

    let member = organization_members::Entity::find()
        .filter(organization_members::Column::OrganizationId.eq(organization.id))
        .filter(organization_members::Column::UserId.eq(member_id))
        .one(fixture.db().as_ref())
        .await
        .unwrap();
    assert!(
        member.is_some(),
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
    let organization = DbFixtures::create_org(
        fixture.db().as_ref(),
        owner_id,
        HashMap::from([
            (owner_id.to_string(), "owner".to_string()),
            (member_id.to_string(), "read".to_string()),
        ]),
    )
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
