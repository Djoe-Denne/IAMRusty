//! Coverage90 QA: persisted RBAC replacement, failure atomicity and scoped pages.
//! Contract: docs/contracts/coverage90-hive-corrections.md, sections 3/4/6.
//! HTTP covers the public boundary. Rollback/concurrency call the real trusted
//! writer `UoW`; those cases alone do not prove HTTP authorization or FGA delivery.

mod common;

use common::{fixtures::db::DbFixtures, Permission, ResourceRef, Subject};
use hive_application::dto::role::{MemberRole, MemberRolePermission};
use hive_application::HiveOutboxUnitOfWork;
use hive_domain::port::repository::OrganizationMemberReadRepository;
use hive_domain::RolePermission;
use hive_events::{HiveDomainEvent, MemberRolesUpdatedEvent, Role};
use hive_infra::repository::entity::{
    organization_member_role_permissions, organization_members, role_permissions,
};
use hive_infra::{repository::OrganizationMemberReadRepositoryImpl, HiveOutboxUnitOfWorkImpl};
use reqwest::{Client, StatusCode};
use rustycog::core::error::ServiceError;
use rustycog::events::DomainEvent;
use rustycog::outbox::{
    entity::{Column as OutboxColumn, Model as OutboxModel, OutboxEvents, STATUS_PENDING},
    OutboxRecorder,
};
use rustycog::testing::http::jwt::{create_jwt_token, TEST_JWT_ISSUER};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serde_json::{json, Value};
use serial_test::serial;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use uuid::Uuid;

async fn authorize(openfga: &common::TestOpenFga, user: Uuid, org: Uuid) {
    for permission in [Permission::Read, Permission::Write] {
        openfga
            .allow(
                Subject::new(user),
                permission,
                ResourceRef::new("organization", org),
            )
            .await
            .expect("grant real OpenFGA organization permission");
    }
}

fn roles(org: Uuid, level: &str) -> Value {
    json!([{"organization_id": org, "resource": "organization", "permissions": level}])
}

async fn add_read_member(client: &Client, url: &str, token: &str, org: Uuid, user: Uuid) -> Uuid {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(&json!({"user_id": user, "roles": roles(org, "Read")}))
        .send()
        .await
        .expect("POST organization member");
    assert_eq!(response.status(), StatusCode::OK);
    let member: Value = response.json().await.expect("created member JSON");
    Uuid::parse_str(member["id"].as_str().expect("member id")).expect("member UUID")
}

async fn stored_grants(db: &DatabaseConnection, member: Uuid) -> BTreeSet<(Uuid, String, String)> {
    let links = organization_member_role_permissions::Entity::find()
        .filter(organization_member_role_permissions::Column::MemberId.eq(member))
        .all(db)
        .await
        .expect("persisted member grants");
    let mut grants = BTreeSet::new();
    for link in links {
        let grant = role_permissions::Entity::find_by_id(link.role_permission_id)
            .one(db)
            .await
            .expect("role permission lookup")
            .expect("linked role permission");
        assert!(
            grants.insert((
                grant.organization_id,
                grant.resource_id,
                grant.permission_id
            )),
            "duplicate persisted grant"
        );
    }
    grants
}

async fn patch_roles(client: &Client, url: &str, token: &str, request: &Value) -> Value {
    let response = client
        .patch(url)
        .bearer_auth(token)
        .json(request)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.unwrap()
}

async fn outbox_rows(db: &DatabaseConnection, org: Uuid) -> Vec<OutboxModel> {
    OutboxEvents::find()
        .filter(OutboxColumn::AggregateId.eq(org))
        .order_by_asc(OutboxColumn::Id)
        .all(db)
        .await
        .unwrap()
}

fn assert_pending_role_event(row: &OutboxModel, org: Uuid, user: Uuid, issuer: &str, level: &str) {
    assert_eq!(row.status, STATUS_PENDING);
    assert_eq!(row.aggregate_id, org);
    assert_eq!(row.event_type, "member_roles_updated");
    let event: HiveDomainEvent = serde_json::from_value(row.payload_json.clone()).unwrap();
    let HiveDomainEvent::MemberRolesUpdated(event) = event else {
        panic!("replacement event required")
    };
    assert_eq!(event.organization_id, org);
    assert_eq!(event.user_id, user);
    assert_eq!(event.issuer.as_deref(), Some(issuer));
    assert_eq!(event.roles.len(), 1);
    assert_eq!(
        (
            event.roles[0].resource.as_str(),
            event.roles[0].permission.as_str()
        ),
        ("organization", level)
    );
}

#[derive(Debug, PartialEq)]
struct MemberSnapshot {
    member: organization_members::Model,
    links: Vec<organization_member_role_permissions::Model>,
    outbox: Vec<OutboxModel>,
}

async fn member_snapshot(db: &DatabaseConnection, org: Uuid, id: Uuid) -> MemberSnapshot {
    MemberSnapshot {
        member: organization_members::Entity::find_by_id(id)
            .one(db)
            .await
            .unwrap()
            .unwrap(),
        links: organization_member_role_permissions::Entity::find()
            .filter(organization_member_role_permissions::Column::MemberId.eq(id))
            .order_by_asc(organization_member_role_permissions::Column::Id)
            .all(db)
            .await
            .unwrap(),
        outbox: outbox_rows(db, org).await,
    }
}

fn domain_roles(org: Uuid, permission: MemberRolePermission) -> Vec<RolePermission> {
    vec![RolePermission::try_from(&MemberRole {
        organization_id: org,
        resource: "organization".into(),
        permissions: permission,
    })
    .expect("valid existing organization role")]
}

fn roles_event(org: Uuid, user: Uuid, permission: &str) -> HiveDomainEvent {
    let mut event = MemberRolesUpdatedEvent::new(
        org,
        "Coverage90".into(),
        user,
        vec![Role::new(permission.into(), "organization".into())],
        chrono::Utc::now(),
    );
    event.issuer = Some(TEST_JWT_ISSUER.to_owned());
    HiveDomainEvent::MemberRolesUpdated(event)
}

// Same serialization-only failure seam as outbox_tests.rs. PG, the UoW and
// outbox recording are real; no use-case/DB double or production failpoint.
#[derive(Debug)]
struct BadEvent {
    event_id: Uuid,
    org: Uuid,
    serialized: Arc<AtomicBool>,
}

impl DomainEvent for BadEvent {
    fn event_type(&self) -> &'static str {
        "bad_hive_test_event"
    }
    fn event_id(&self) -> Uuid {
        self.event_id
    }
    fn aggregate_id(&self) -> Uuid {
        self.org
    }
    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
    fn version(&self) -> u32 {
        1
    }
    fn to_json(&self) -> Result<String, ServiceError> {
        self.serialized.store(true, Ordering::SeqCst);
        Err(ServiceError::internal("forced serialization failure"))
    }
    fn metadata(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

#[tokio::test]
#[serial]
async fn reducing_duplicate_and_empty_replacements_preserve_identity_without_residual_grants() {
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let user = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    authorize(&openfga, owner, org.id).await;
    let token = create_jwt_token(owner);
    let url = format!("{base}/api/organizations/{}/members", org.id);
    let id = add_read_member(&client, &url, &token, org.id, user).await;
    let original = organization_members::Entity::find_by_id(id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    let target = format!("{url}/{user}");
    for (level, stored, duplicate) in [
        ("Write", "write", false),
        ("Read", "read", false),
        ("Read", "read", true),
    ] {
        let mut requested = roles(org.id, level);
        if duplicate {
            let same = requested[0].clone();
            requested.as_array_mut().unwrap().push(same);
        }
        let updated = patch_roles(&client, &target, &token, &json!({"roles": requested})).await;
        assert_eq!(updated["id"], id.to_string());
        assert_eq!(
            stored_grants(fixture.db().as_ref(), id).await,
            BTreeSet::from([(org.id, "organization".into(), stored.into())])
        );
    }
    let emptied = patch_roles(&client, &target, &token, &json!({"roles": []})).await;
    assert_eq!(emptied["id"], id.to_string());
    assert!(stored_grants(fixture.db().as_ref(), id).await.is_empty());
    let after = organization_members::Entity::find_by_id(id)
        .one(fixture.db().as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            after.id,
            after.organization_id,
            after.user_id,
            after.issuer,
            after.status,
            after.created_at,
            after.joined_at,
            after.invited_at,
            after.invited_by_user_id
        ),
        (
            original.id,
            original.organization_id,
            original.user_id,
            original.issuer,
            original.status,
            original.created_at,
            original.joined_at,
            original.invited_at,
            original.invited_by_user_id
        )
    );
}

#[tokio::test]
#[serial]
async fn roles_event_is_pending_and_carries_the_exact_target_issuer_and_new_set() {
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let user = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    authorize(&openfga, owner, org.id).await;
    let token = create_jwt_token(owner);
    let url = format!("{base}/api/organizations/{}/members", org.id);
    let local_id = add_read_member(&client, &url, &token, org.id, user).await;
    let issuer = "https://coverage90-target.example/iam";
    let created = client
        .post(&url)
        .bearer_auth(&token)
        .json(&json!({"user_id": user, "issuer": issuer, "roles": roles(org.id, "Read")}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created: Value = created.json().await.unwrap();
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    assert_ne!(id, local_id);
    let local_before = member_snapshot(fixture.db().as_ref(), org.id, local_id).await;
    let before = outbox_rows(fixture.db().as_ref(), org.id).await;
    let updated = patch_roles(
        &client,
        &format!("{url}/{user}"),
        &token,
        &json!({"issuer": issuer, "roles": roles(org.id, "Write")}),
    )
    .await;
    assert_eq!(updated["id"], id.to_string());
    assert_eq!(updated["issuer"], issuer);
    assert_eq!(
        stored_grants(fixture.db().as_ref(), id).await,
        BTreeSet::from([(org.id, "organization".into(), "write".into())])
    );
    let after = outbox_rows(fixture.db().as_ref(), org.id).await;
    let before_ids: BTreeSet<_> = before.iter().map(|row| row.id).collect();
    let added: Vec<_> = after
        .iter()
        .filter(|row| !before_ids.contains(&row.id))
        .collect();
    assert_eq!(
        added.len(),
        1,
        "one transaction must record one replacement intention"
    );
    assert_pending_role_event(added[0], org.id, user, issuer, "write");
    let local_after = member_snapshot(fixture.db().as_ref(), org.id, local_id).await;
    assert_eq!(
        local_after.member, local_before.member,
        "same UUID at another issuer is not the target"
    );
    assert_eq!(local_after.links, local_before.links);
}

#[tokio::test]
#[serial]
async fn real_role_uow_rolls_back_links_metadata_and_outbox_when_record_serialization_fails() {
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let user = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    authorize(&openfga, owner, org.id).await;
    let token = create_jwt_token(owner);
    let url = format!("{base}/api/organizations/{}/members", org.id);
    let id = add_read_member(&client, &url, &token, org.id, user).await;
    let before = member_snapshot(fixture.db().as_ref(), org.id, id).await;
    assert_eq!(before.links.len(), 1);
    let serialized = Arc::new(AtomicBool::new(false));
    let bad = BadEvent {
        event_id: Uuid::new_v4(),
        org: org.id,
        serialized: Arc::clone(&serialized),
    };
    let pool = fixture.fixture.database.as_ref().unwrap().pool.clone();
    let error = HiveOutboxUnitOfWorkImpl::new(pool, OutboxRecorder::new())
        .replace_member_roles(
            org.id,
            user,
            TEST_JWT_ISSUER.to_owned(),
            domain_roles(org.id, MemberRolePermission::Write),
            Box::new(bad),
        )
        .await
        .expect_err("serialization failure must abort the real writer transaction");
    assert!(
        serialized.load(Ordering::SeqCst),
        "the late outbox serialization seam must be reached"
    );
    assert!(
        error.to_string().contains("forced serialization failure"),
        "unexpected UoW error: {error}"
    );
    assert_eq!(
        member_snapshot(fixture.db().as_ref(), org.id, id).await,
        before,
        "rollback must restore exact link IDs/timestamps, member metadata and all outbox rows"
    );
    assert_eq!(
        stored_grants(fixture.db().as_ref(), id).await,
        BTreeSet::from([(org.id, "organization".into(), "read".into())])
    );
}

async fn blocked_writer_pids(db: &DatabaseConnection, holder: i32) -> BTreeSet<i32> {
    loop {
        // Include indirect waiters: the second row-lock contender may queue
        // behind the first contender rather than directly behind the holder.
        let rows = db
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "WITH RECURSIVE waiting(pid) AS (
                SELECT $1::int
                UNION
                SELECT a.pid FROM pg_stat_activity a
                JOIN waiting w ON w.pid = ANY(pg_blocking_pids(a.pid))
             ) SELECT pid FROM waiting WHERE pid <> $1",
                [holder.into()],
            ))
            .await
            .unwrap();
        let pids: BTreeSet<i32> = rows
            .iter()
            .map(|row| row.try_get("", "pid").unwrap())
            .collect();
        assert!(
            pids.len() <= 2,
            "unexpected extra writer waiting on this exact parent lock: {pids:?}"
        );
        if pids.len() == 2 {
            return pids;
        }
        tokio::task::yield_now().await;
    }
}

fn assert_writer_member(member: &hive_domain::OrganizationMember, id: Uuid, level: &str) {
    assert_eq!(member.id, Some(id));
    assert_eq!(member.issuer, TEST_JWT_ISSUER);
    assert_eq!(
        member.roles.len(),
        1,
        "UoW return must contain the committed replacement"
    );
    assert_eq!(member.roles[0].member_id, id);
    assert_eq!(
        member.roles[0].role_permission.permission.level.to_str(),
        level
    );
}

#[tokio::test]
#[serial]
async fn concurrent_role_writers_block_on_the_same_parent_and_commit_one_complete_set_not_a_union()
{
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let user = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    authorize(&openfga, owner, org.id).await;
    let token = create_jwt_token(owner);
    let url = format!("{base}/api/organizations/{}/members", org.id);
    let id = add_read_member(&client, &url, &token, org.id, user).await;
    patch_roles(
        &client,
        &format!("{url}/{user}"),
        &token,
        &json!({"roles": []}),
    )
    .await;
    let db = fixture.db();
    assert!(
        stored_grants(db.as_ref(), id).await.is_empty(),
        "exercise parent locking even with no child links"
    );
    let pool = fixture.fixture.database.as_ref().unwrap().pool.clone();
    let uow = HiveOutboxUnitOfWorkImpl::new(pool, OutboxRecorder::new());
    let first_event = roles_event(org.id, user, "read");
    let second_event = roles_event(org.id, user, "write");
    let event_ids = [
        (first_event.event_id(), "read"),
        (second_event.event_id(), "write"),
    ];
    let (one, two) = run_overlapping_writers(
        db.as_ref(),
        &uow,
        org.id,
        user,
        id,
        [first_event, second_event],
    )
    .await;
    assert_writer_member(&one, id, "read");
    assert_writer_member(&two, id, "write");
    let final_set = stored_grants(db.as_ref(), id).await;
    assert!(
        [
            BTreeSet::from([(org.id, "organization".into(), "read".into())]),
            BTreeSet::from([(org.id, "organization".into(), "write".into())])
        ]
        .contains(&final_set),
        "the final state must be one complete writer set, never a union: {final_set:?}"
    );
    for (event_id, level) in event_ids {
        let row = OutboxEvents::find()
            .filter(OutboxColumn::EventId.eq(event_id))
            .one(db.as_ref())
            .await
            .unwrap()
            .expect("each committed writer's event");
        assert_pending_role_event(&row, org.id, user, TEST_JWT_ISSUER, level);
    }
}

async fn run_overlapping_writers(
    db: &DatabaseConnection,
    uow: &HiveOutboxUnitOfWorkImpl,
    org: Uuid,
    user: Uuid,
    id: Uuid,
    events: [HiveDomainEvent; 2],
) -> (
    hive_domain::OrganizationMember,
    hive_domain::OrganizationMember,
) {
    let [first_event, second_event] = events;
    let hold = db.begin().await.unwrap();
    hold.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM organization_members WHERE id=$1 FOR UPDATE",
        [id.into()],
    ))
    .await
    .unwrap()
    .expect("lock the exact target parent");
    let holder: i32 = hold
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let barrier = tokio::sync::Barrier::new(3);
    let first = async {
        barrier.wait().await;
        uow.replace_member_roles(
            org,
            user,
            TEST_JWT_ISSUER.to_owned(),
            domain_roles(org, MemberRolePermission::Read),
            first_event.into(),
        )
        .await
    };
    let second = async {
        barrier.wait().await;
        uow.replace_member_roles(
            org,
            user,
            TEST_JWT_ISSUER.to_owned(),
            domain_roles(org, MemberRolePermission::Write),
            second_event.into(),
        )
        .await
    };
    tokio::pin!(first, second);
    let observation = tokio::time::timeout(Duration::from_secs(5), async {
        barrier.wait().await;
        blocked_writer_pids(db, holder).await
    });
    let waiters = tokio::select! {
        observed = observation => observed.expect("both real PG writers must overlap before lock release"),
        result = &mut first => panic!("first writer finished while parent was locked: {result:?}"),
        result = &mut second => panic!("second writer finished while parent was locked: {result:?}"),
    };
    assert_eq!(
        waiters.len(),
        2,
        "two distinct PostgreSQL backend connections must be blocked"
    );
    assert!(!waiters.contains(&holder));
    hold.commit().await.unwrap();
    let (one, two) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(&mut first, &mut second)
    })
    .await
    .expect("both writers must complete after parent-lock release");
    (
        one.expect("first replacement commits"),
        two.expect("second replacement commits"),
    )
}

#[tokio::test]
#[serial]
async fn replacing_roles_persists_only_the_new_scoped_grant() {
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let user = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    authorize(&openfga, owner, org.id).await;
    let token = create_jwt_token(owner);
    let url = format!("{base}/api/organizations/{}/members", org.id);
    let id = add_read_member(&client, &url, &token, org.id, user).await;
    assert_eq!(
        stored_grants(fixture.db().as_ref(), id).await,
        BTreeSet::from([(org.id, "organization".into(), "read".into())])
    );

    let response = client
        .patch(format!("{url}/{user}"))
        .bearer_auth(&token)
        .json(&json!({"roles": roles(org.id, "Write")}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Value = response.json().await.unwrap();
    assert_eq!(updated["id"], id.to_string());
    assert_eq!(
        stored_grants(fixture.db().as_ref(), id).await,
        BTreeSet::from([(org.id, "organization".into(), "write".into())]),
        "a replacement must remove read rather than merge grants or elevate to admin"
    );

    // MemberRolesUpdated describes a replacement set: replaying the same PATCH
    // must preserve that exact set, not append duplicate persisted links.
    let repeated = client
        .patch(format!("{url}/{user}"))
        .bearer_auth(&token)
        .json(&json!({"roles": roles(org.id, "Write")}))
        .send()
        .await
        .unwrap();
    assert_eq!(repeated.status(), StatusCode::OK);
    let repeated_member: Value = repeated.json().await.unwrap();
    assert_eq!(repeated_member["id"], id.to_string());
    assert_eq!(
        stored_grants(fixture.db().as_ref(), id).await,
        BTreeSet::from([(org.id, "organization".into(), "write".into())]),
        "repeating the replacement must retain exactly write with no duplicate links"
    );
}

#[tokio::test]
#[serial]
async fn a_later_invalid_role_keeps_all_original_grants_unchanged() {
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let user = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    authorize(&openfga, owner, org.id).await;
    let token = create_jwt_token(owner);
    let url = format!("{base}/api/organizations/{}/members", org.id);
    let id = add_read_member(&client, &url, &token, org.id, user).await;
    let before = stored_grants(fixture.db().as_ref(), id).await;
    assert_eq!(
        before,
        BTreeSet::from([(org.id, "organization".into(), "read".into())])
    );

    let response = client
        .patch(format!("{url}/{user}"))
        .bearer_auth(&token)
        .json(&json!({"roles": [
            {"organization_id": org.id, "resource": "organization", "permissions": "Write"},
            {"organization_id": org.id, "resource": "organization", "permissions": "Delete"}
        ]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(stored_grants(fixture.db().as_ref(), id).await, before,
        "conversion failure after a valid role must not revoke or partially replace persisted grants");
    let read = client
        .get(format!("{url}/{user}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status(), StatusCode::OK);
    let stored: Value = read.json().await.unwrap();
    assert_eq!(
        stored["id"],
        id.to_string(),
        "the original membership must still resolve through HTTP"
    );
}

#[tokio::test]
#[serial]
async fn member_page_totals_and_cursors_describe_the_whole_organization_not_the_current_slice() {
    let (fixture, base, client, openfga) = Box::pin(common::setup_test_server()).await.unwrap();
    let owner = Uuid::new_v4();
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner)
        .await
        .unwrap();
    DbFixtures::create_org_with_owner(fixture.db().as_ref(), Uuid::new_v4())
        .await
        .unwrap();
    authorize(&openfga, owner, org.id).await;
    let token = create_jwt_token(owner);
    let url = format!("{base}/api/organizations/{}/members", org.id);
    let mut expected = BTreeSet::from([owner.to_string()]);
    for _ in 0..4 {
        let user = Uuid::new_v4();
        add_read_member(&client, &url, &token, org.id, user).await;
        expected.insert(user.to_string());
    }
    tie_population_with_mixed_statuses(fixture.db().as_ref(), org.id, owner).await;
    assert_scoped_member_pages(&client, &url, &token, org.id, &expected).await;
}

async fn tie_population_with_mixed_statuses(db: &DatabaseConnection, org: Uuid, owner: Uuid) {
    let members = organization_members::Entity::find()
        .filter(organization_members::Column::OrganizationId.eq(org))
        .order_by_asc(organization_members::Column::Id)
        .all(db)
        .await
        .unwrap();
    assert_eq!(members.len(), 5);
    let tied_at = members[0].created_at;
    let mut non_owner = 0;
    for member in members {
        let is_owner = member.user_id == owner;
        let mut member = member.into_active_model();
        member.created_at = Set(tied_at);
        if !is_owner {
            match non_owner {
                0 => member.status = Set("Pending".into()),
                1 => member.status = Set("Suspended".into()),
                2 => member.issuer = Set("https://coverage90-page.example/iam".into()),
                _ => {}
            }
            non_owner += 1;
        }
        member.update(db).await.unwrap();
    }
}

async fn member_page(client: &Client, url: &str, token: &str, page: &str) -> Value {
    let response = client
        .get(url)
        .bearer_auth(token)
        .query(&[("page_size", "2"), ("page", page)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.unwrap()
}

async fn assert_scoped_member_pages(
    client: &Client,
    url: &str,
    token: &str,
    org: Uuid,
    expected: &BTreeSet<String>,
) {
    let mut seen = BTreeSet::new();
    let mut ordered_ids = Vec::new();
    let mut statuses = BTreeSet::new();
    let mut issuers = BTreeSet::new();
    for (index, expected_len) in [2, 2, 1].into_iter().enumerate() {
        let cursor = (index + 1).to_string();
        let page = member_page(client, url, token, &cursor).await;
        let rows = page["members"].as_array().expect("member page");
        assert_eq!(rows.len(), expected_len, "page {cursor}");
        assert_eq!(
            page["pagination"]["total_items"], 5,
            "total must include rows on other pages"
        );
        assert_eq!(page["pagination"]["total_pages"], 3);
        assert_eq!(page["pagination"]["has_next"], index < 2);
        assert_eq!(page["pagination"]["has_previous"], index > 0);
        assert_eq!(page["pagination"]["current_page"], index + 1);
        assert_eq!(page["pagination"]["page_size"], 2);
        if index > 0 {
            assert_eq!(page["pagination"]["previous_cursor"], index.to_string());
        } else {
            assert!(page["pagination"]["previous_cursor"].is_null());
        }
        if index < 2 {
            assert_eq!(page["pagination"]["next_cursor"], (index + 2).to_string());
        } else {
            assert!(page["pagination"]["next_cursor"].is_null());
        }
        for row in rows {
            assert_eq!(row["organization_id"], org.to_string());
            assert!(
                seen.insert(row["user_id"].as_str().expect("member user").to_owned()),
                "no member may appear on two pages"
            );
            ordered_ids.push(Uuid::parse_str(row["id"].as_str().unwrap()).unwrap());
            statuses.insert(row["status"].as_str().unwrap().to_ascii_lowercase());
            issuers.insert(row["issuer"].as_str().unwrap().to_owned());
        }
        let replay = member_page(client, url, token, &cursor).await;
        assert_eq!(
            replay["members"], page["members"],
            "ties must not reshuffle a stable population"
        );
    }
    assert_eq!(&seen, expected);
    // The contract fixes ID as the tie-breaker but does not prescribe its direction.
    assert!(
        ordered_ids.windows(2).all(|pair| pair[0] < pair[1])
            || ordered_ids.windows(2).all(|pair| pair[0] > pair[1]),
        "equal creation timestamps must be ordered by ID"
    );
    assert_eq!(
        statuses,
        BTreeSet::from(["active".into(), "pending".into(), "suspended".into()])
    );
    assert!(issuers.contains("https://coverage90-page.example/iam"));
    let outside = member_page(client, url, token, "4").await;
    assert_eq!(outside["members"], json!([]));
    assert_eq!(outside["pagination"]["total_items"], 5);
    assert_eq!(outside["pagination"]["total_pages"], 3);
    assert_eq!(outside["pagination"]["has_next"], false);
    assert_eq!(outside["pagination"]["previous_cursor"], "3");
    assert!(outside["pagination"]["next_cursor"].is_null());
}

#[tokio::test]
#[serial]
async fn member_repository_counts_and_lists_zero_members_for_an_empty_organization() {
    let (fixture, _, _, _) = Box::pin(common::setup_test_server()).await.unwrap();
    let empty = DbFixtures::organization()
        .commit(fixture.db())
        .await
        .unwrap();
    DbFixtures::create_org_with_owner(fixture.db().as_ref(), Uuid::new_v4())
        .await
        .unwrap();
    let repository = OrganizationMemberReadRepositoryImpl::new(fixture.db());
    assert_eq!(
        repository.count_by_organization(&empty.id).await.unwrap(),
        0
    );
    assert_eq!(
        repository
            .find_by_organization(&empty.id, 0, 2)
            .await
            .unwrap(),
        [] as [hive_domain::OrganizationMember; 0]
    );
}
