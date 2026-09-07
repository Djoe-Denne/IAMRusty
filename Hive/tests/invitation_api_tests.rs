use reqwest::StatusCode;
use rustycog::testing::http::jwt::create_jwt_token;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;
use uuid::Uuid;

use hive_infra::repository::entity::{organization_invitations, organization_members};

mod common;
use common::{fixtures::db::DbFixtures, setup_test_server, Permission, ResourceRef, Subject};

fn invitation_body(organization_id: Uuid, email: &str) -> serde_json::Value {
    serde_json::json!({
        "email": email,
        "roles": [{
            "organization_id": organization_id,
            "resource": "organization",
            "permissions": "Read"
        }],
        "message": "Welcome to the organization"
    })
}

async fn allow_organization_write(
    openfga: &common::TestOpenFga,
    user_id: Uuid,
    organization_id: Uuid,
) {
    openfga
        .allow(
            Subject::new(user_id),
            Permission::Write,
            ResourceRef::new("organization", organization_id),
        )
        .await
        .expect("failed to grant organization write");
}

#[tokio::test]
#[serial]
async fn cancel_pending_invitation_persists_cancelled_state_through_live_route() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let owner_token = create_jwt_token(owner_id);
    let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();
    allow_organization_write(&openfga, owner_id, organization.id).await;

    let invitation = DbFixtures::organization_invitation()
        .organization_id(organization.id)
        .invited_by_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();
    assert_eq!(invitation.status, "Pending");

    let cancelled = client
        .delete(format!(
            "{server_url}/api/organizations/{}/invitations/{}",
            organization.id, invitation.id
        ))
        .header("Authorization", format!("Bearer {owner_token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(cancelled.status(), StatusCode::OK);

    let stored = organization_invitations::Entity::find_by_id(invitation.id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_ne!(stored.status, "Pending");
}

#[tokio::test]
#[serial]
async fn accepting_a_pending_invitation_creates_a_member_and_rejects_replay() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let invitee_id = Uuid::new_v4();
    let invitee_token = create_jwt_token(invitee_id);
    let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();
    let invitation = DbFixtures::organization_invitation()
        .organization_id(organization.id)
        .invited_by_user_id(owner_id)
        .aggregate_id("member@example.test")
        .commit(fixture.db())
        .await
        .unwrap();

    let accepted = client
        .post(format!(
            "{server_url}/api/invitations/{}/accept",
            invitation.token
        ))
        .header("Authorization", format!("Bearer {invitee_token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);

    let invitation_row = organization_invitations::Entity::find_by_id(invitation.id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_ne!(invitation_row.status, "Pending");
    assert!(invitation_row.accepted_at.is_some());

    let member = organization_members::Entity::find()
        .filter(organization_members::Column::OrganizationId.eq(organization.id))
        .filter(organization_members::Column::UserId.eq(invitee_id))
        .one(fixture.db().as_ref())
        .await
        .unwrap();
    assert!(
        member.is_some(),
        "accepting must create the organization membership"
    );

    let replay = client
        .post(format!(
            "{server_url}/api/invitations/{}/accept",
            invitation.token
        ))
        .header("Authorization", format!("Bearer {invitee_token}"))
        .send()
        .await
        .unwrap();
    assert!(replay.status().is_server_error());
}

#[tokio::test]
#[serial]
async fn invitation_validation_and_invalid_role_do_not_persist_rows() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let owner_token = create_jwt_token(owner_id);
    let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();
    allow_organization_write(&openfga, owner_id, organization.id).await;

    let invalid_email = client
        .post(format!(
            "{server_url}/api/organizations/{}/invitations",
            organization.id
        ))
        .header("Authorization", format!("Bearer {owner_token}"))
        .json(&invitation_body(organization.id, "not-an-email"))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid_email.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let invalid_role = serde_json::json!({
        "email": "invalid-role@example.test",
        "roles": [{
            "organization_id": organization.id,
            "resource": "organization",
            "permissions": "Delete"
        }]
    });
    let invalid_role_response = client
        .post(format!(
            "{server_url}/api/organizations/{}/invitations",
            organization.id
        ))
        .header("Authorization", format!("Bearer {owner_token}"))
        .json(&invalid_role)
        .send()
        .await
        .unwrap();
    assert!(invalid_role_response.status().is_server_error());

    let invitations = organization_invitations::Entity::find()
        .filter(organization_invitations::Column::OrganizationId.eq(organization.id))
        .all(fixture.db().as_ref())
        .await
        .unwrap();
    assert!(
        invitations.is_empty(),
        "invalid requests must remain atomic"
    );
}

#[tokio::test]
#[serial]
async fn cancel_requires_write_permission_on_the_parent_organization() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let outsider_id = Uuid::new_v4();
    let outsider_token = create_jwt_token(outsider_id);
    let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();
    let invitation = DbFixtures::organization_invitation()
        .organization_id(organization.id)
        .invited_by_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    let forbidden = client
        .delete(format!(
            "{server_url}/api/organizations/{}/invitations/{}",
            organization.id, invitation.id
        ))
        .header("Authorization", format!("Bearer {outsider_token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn expired_cancelled_and_unknown_invitations_never_create_a_membership() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let invitee_id = Uuid::new_v4();
    let invitee_token = create_jwt_token(invitee_id);
    let organization = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    let expired = DbFixtures::organization_invitation()
        .organization_id(organization.id)
        .invited_by_user_id(owner_id)
        .expired()
        .commit(fixture.db())
        .await
        .unwrap();
    let cancelled = DbFixtures::organization_invitation()
        .organization_id(organization.id)
        .invited_by_user_id(owner_id)
        .cancelled()
        .commit(fixture.db())
        .await
        .unwrap();

    for token in [
        expired.token.as_str(),
        cancelled.token.as_str(),
        "unknown-token",
    ] {
        let response = client
            .post(format!("{server_url}/api/invitations/{token}/accept"))
            .header("Authorization", format!("Bearer {invitee_token}"))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_client_error() || response.status().is_server_error());
    }

    let member = organization_members::Entity::find()
        .filter(organization_members::Column::OrganizationId.eq(organization.id))
        .filter(organization_members::Column::UserId.eq(invitee_id))
        .one(fixture.db().as_ref())
        .await
        .unwrap();
    assert!(member.is_none());

    for invitation_id in [expired.id, cancelled.id] {
        let row = organization_invitations::Entity::find_by_id(invitation_id)
            .one(fixture.db().as_ref())
            .await
            .unwrap()
            .unwrap();
        assert!(row.accepted_at.is_none());
    }
}
