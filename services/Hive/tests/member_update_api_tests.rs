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
        issuer: None,
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
            issuer: None,
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
    assert_eq!(updated.issuer, member.issuer);

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
            issuer: None,
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

async fn last_principal_event(
    fixture: &common::HiveTestFixture,
    organization: Uuid,
    event_type: &str,
) -> hive_events::HiveDomainEvent {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    let payload: serde_json::Value = fixture.db().query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT payload_json FROM rustycog_outbox_events WHERE aggregate_id=$1 AND event_type=$2 ORDER BY created_at DESC LIMIT 1",
        [organization.into(), event_type.to_owned().into()],
    )).await.unwrap().unwrap().try_get("", "payload_json").unwrap();
    serde_json::from_value(payload).expect("typed Hive event persisted in real outbox")
}

#[tokio::test]
#[serial]
async fn hive_membership_and_projection_preserve_target_issuer() {
    use futures::FutureExt;
    use rustycog::testing::http::jwt::TEST_JWT_ISSUER;
    use std::panic::AssertUnwindSafe;
    let (fixture, base, client, openfga) = Box::pin(setup_test_server()).await.unwrap();
    let result = AssertUnwindSafe(async {
        let same_user = Uuid::new_v4();
        let token = create_jwt_token(same_user);
        let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), same_user).await.unwrap();
        let resource = ResourceRef::new("organization", organization.id);
        for permission in [Permission::Write, Permission::Read] {
            openfga.allow(Subject::new(same_user), permission, resource).await.unwrap();
        }
        let alpha = "https://alpha.example/iam";
        let beta = "https://beta.example/iam";
        let mut created = Vec::new();
        for issuer in [alpha, beta] {
            let request = AddMemberRequest {
                user_id: same_user, issuer: Some(issuer.into()),
                roles: vec![MemberRole { organization_id: organization.id, resource: "organization".into(), permissions: MemberRolePermission::Read }],
            };
            let response = client.post(format!("{base}/api/organizations/{}/members", organization.id))
                .bearer_auth(&token).json(&request).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let member: MemberResponse = response.json().await.unwrap();
            assert_eq!(member.issuer, issuer);
            assert_eq!(member.user_id, same_user);
            match last_principal_event(&fixture, organization.id, "member_joined").await {
                hive_events::HiveDomainEvent::MemberJoined(event) => assert_eq!(event.issuer.as_deref(), Some(issuer)),
                _ => panic!("expected membership-created outbox event"),
            }
            created.push(member.id);
        }
        assert_ne!(created[0], created[1]);
        let ambiguous = hive_infra::repository::OrganizationMemberReadRepositoryImpl::find_by_organization_and_user_with_connection(
            fixture.db().as_ref(), &organization.id, &same_user,
        ).await.expect_err("UUID-only lookup must not choose one authority arbitrarily");
        assert!(ambiguous.to_string().contains("MEMBER_PRINCIPAL_AMBIGUOUS"));
        let beta_before = organization_members::Entity::find_by_id(created[1]).one(fixture.db().as_ref()).await.unwrap().unwrap();
        let mut request = write_roles(organization.id);
        request.issuer = Some(alpha.into());
        let response = client.patch(format!("{base}/api/organizations/{}/members/{same_user}", organization.id))
            .bearer_auth(&token).json(&request).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let updated: MemberResponse = response.json().await.unwrap();
        assert_eq!(updated.id, created[0]);
        assert_eq!(updated.issuer, alpha);
        assert!(organization_members::Entity::find_by_id(created[1]).one(fixture.db().as_ref()).await.unwrap().unwrap() == beta_before,
            "updating alpha must not mutate beta at the same UUID");
        match last_principal_event(&fixture, organization.id, "member_roles_updated").await {
            hive_events::HiveDomainEvent::MemberRolesUpdated(event) => assert_eq!(event.issuer.as_deref(), Some(alpha)),
            _ => panic!("expected membership-role outbox event"),
        }
        let response = client.delete(format!("{base}/api/organizations/{}/members/{same_user}", organization.id))
            .query(&[("issuer", alpha)]).bearer_auth(&token).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(organization_members::Entity::find_by_id(created[0]).one(fixture.db().as_ref()).await.unwrap().is_none());
        let remaining = organization_members::Entity::find()
            .filter(organization_members::Column::OrganizationId.eq(organization.id))
            .filter(organization_members::Column::UserId.eq(same_user))
            .all(fixture.db().as_ref()).await.unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|member| member.issuer == TEST_JWT_ISSUER));
        assert!(remaining.iter().any(|member| member.issuer == beta));
        match last_principal_event(&fixture, organization.id, "member_removed").await {
            hive_events::HiveDomainEvent::MemberRemoved(event) => {
                assert_eq!(event.issuer.as_deref(), Some(alpha));
                assert_eq!(event.removed_by_issuer.as_deref(), Some(TEST_JWT_ISSUER));
            }
            _ => panic!("expected membership-deleted outbox event"),
        }
    }).catch_unwind().await;
    let cleanup = common::cleanup_test_servers(&fixture).await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
    assert!(
        cleanup.is_ok(),
        "owned Hive listeners must join before releasing the parent lease"
    );
}
