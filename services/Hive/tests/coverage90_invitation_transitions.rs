//! Coverage90 QA: invitation transitions and organization isolation.
//! Contract: docs/functional/organisation.md, invitation secret and org-scoped Write.
//! These scenarios create invitations through HTTP before inspecting real PG state.

mod common;

use common::{fixtures::db::DbFixtures, Permission, ResourceRef, Subject};
use hive_infra::repository::entity::{organization_invitations, organization_members};
use reqwest::{Client, StatusCode};
use rustycog::testing::http::jwt::create_jwt_token;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

async fn allow_write(openfga: &common::TestOpenFga, user: Uuid, org: Uuid) {
    openfga
        .allow(
            Subject::new(user),
            Permission::Write,
            ResourceRef::new("organization", org),
        )
        .await
        .expect("grant organization write in real OpenFGA");
}

async fn invite(client: &Client, base: &str, token: &str, org: Uuid) -> Value {
    let email = format!("cv90-{}@example.test", Uuid::new_v4());
    let response = client
        .post(format!("{base}/api/organizations/{org}/invitations"))
        .bearer_auth(token)
        .json(&json!({
            "email": email,
            "roles": [{"organization_id": org, "resource": "organization", "permissions": "Read"}],
            "message": "Coverage90 invitation lifecycle"
        }))
        .send()
        .await
        .expect("POST invitation");
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.expect("created invitation JSON")
}

fn invitation_id(created: &Value) -> Uuid {
    Uuid::parse_str(created["id"].as_str().expect("invitation id")).expect("invitation UUID")
}

#[tokio::test]
#[serial]
async fn cancellation_of_an_http_created_invitation_prevents_membership_and_preserves_cancelled_state(
) {
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let invitee = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    allow_write(&openfga, owner, org.id).await;
    let owner_token = create_jwt_token(owner);
    let created = invite(&client, &base, &owner_token, org.id).await;
    let id = invitation_id(&created);
    let secret = created["token"]
        .as_str()
        .expect("create returns invitation secret");
    let cancel_url = format!("{base}/api/organizations/{}/invitations/{id}", org.id);
    let cancelled = client
        .delete(&cancel_url)
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(cancelled.status(), StatusCode::OK);
    let before = organization_invitations::Entity::find_by_id(id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(before.status, "Cancelled");
    assert!(before.accepted_at.is_none());

    let accept = client
        .post(format!("{base}/api/invitations/{secret}/accept"))
        .bearer_auth(create_jwt_token(invitee))
        .send()
        .await
        .unwrap();
    // Current Hive command errors map to HTTP 500. Persistence is the primary
    // oracle: a cancelled bearer secret cannot consume an invitation or add a user.
    assert_eq!(accept.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let after = organization_invitations::Entity::find_by_id(id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after, before,
        "failed acceptance must leave the cancelled row unchanged"
    );
    let memberships = organization_members::Entity::find()
        .filter(organization_members::Column::OrganizationId.eq(org.id))
        .filter(organization_members::Column::UserId.eq(invitee))
        .all(fixture.db().as_ref())
        .await
        .unwrap();
    assert!(
        memberships.is_empty(),
        "cancelled invitation must never create membership"
    );
}

#[tokio::test]
#[serial]
async fn accepted_invitation_cannot_be_cancelled_and_keeps_the_original_membership() {
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let invitee = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    allow_write(&openfga, owner, org.id).await;
    let token = create_jwt_token(owner);
    let created = invite(&client, &base, &token, org.id).await;
    let id = invitation_id(&created);
    let secret = created["token"].as_str().expect("invitation secret");
    let accepted = client
        .post(format!("{base}/api/invitations/{secret}/accept"))
        .bearer_auth(create_jwt_token(invitee))
        .send()
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
    let before = organization_invitations::Entity::find_by_id(id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(before.status, "Accepted");
    assert!(before.accepted_at.is_some());
    let member_before = organization_members::Entity::find()
        .filter(organization_members::Column::OrganizationId.eq(org.id))
        .filter(organization_members::Column::UserId.eq(invitee))
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .expect("accepted membership");

    let cancel = client
        .delete(format!(
            "{base}/api/organizations/{}/invitations/{id}",
            org.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(cancel.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let after = organization_invitations::Entity::find_by_id(id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after, before, "cancel cannot rewrite a consumed invitation");
    let members = organization_members::Entity::find()
        .filter(organization_members::Column::OrganizationId.eq(org.id))
        .filter(organization_members::Column::UserId.eq(invitee))
        .all(fixture.db().as_ref())
        .await
        .unwrap();
    assert_eq!(
        members,
        vec![member_before],
        "rejection must preserve the exact accepted membership"
    );
}

#[tokio::test]
#[serial]
async fn write_on_another_organization_cannot_cancel_a_foreign_invitation() {
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let outsider = Uuid::new_v4();
    let victim_org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    let own_org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), outsider)
        .await
        .unwrap();
    allow_write(&openfga, owner, victim_org.id).await;
    allow_write(&openfga, outsider, own_org.id).await;
    let created = invite(&client, &base, &create_jwt_token(owner), victim_org.id).await;
    let id = invitation_id(&created);
    let before = organization_invitations::Entity::find_by_id(id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();

    let response = client
        .delete(format!(
            "{base}/api/organizations/{}/invitations/{id}",
            own_org.id
        ))
        .bearer_auth(create_jwt_token(outsider))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let after = organization_invitations::Entity::find_by_id(id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after, before,
        "organization-scoped Write must not mutate a foreign invitation (HTTP {status})"
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
    let mismatch: Value = response.json().await.unwrap();
    assert_eq!(mismatch["error_type"], "not_found");
    assert_eq!(mismatch["message"], "Resource not found");
    assert!(mismatch["details"].is_null());

    let absent = client
        .delete(format!(
            "{base}/api/organizations/{}/invitations/{}",
            own_org.id,
            Uuid::new_v4()
        ))
        .bearer_auth(create_jwt_token(outsider))
        .send()
        .await
        .unwrap();
    assert_eq!(absent.status(), StatusCode::NOT_FOUND);
    let absent: Value = absent.json().await.unwrap();
    // Timestamp/request correlation may differ; the public error must not reveal
    // whether the supplied ID exists in a different organization.
    for field in ["error_type", "message", "details", "validation_errors"] {
        assert_eq!(absent[field], mismatch[field], "generic not-found {field}");
    }
    let public_error = mismatch.to_string();
    for secret in [
        before.aggregate_id.as_str(),
        before.token.as_str(),
        victim_org.name.as_str(),
    ] {
        assert!(
            !public_error.contains(secret),
            "foreign invitation data must not leak"
        );
    }
}
