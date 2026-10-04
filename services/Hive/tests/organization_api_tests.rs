use reqwest::StatusCode;
use rustycog::testing::http::jwt::create_jwt_token;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serial_test::serial;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use hive_application::dto::organization::{CreateOrganizationRequest, OrganizationResponse};
use hive_infra::repository::entity::organizations;
use hive_infra::repository::entity::{external_links, external_providers};
use hive_infra::repository::entity::{
    organization_member_role_permissions, organization_members, permissions, resources,
    role_permissions,
};
use hive_migration::{Migrator, MigratorTrait};

mod common;
use common::{fixtures::db::DbFixtures, setup_test_server, Permission, ResourceRef, Subject};

#[tokio::test]
#[serial]
async fn create_organization_happy_path() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);

    let create_body = CreateOrganizationRequest {
        name: "Org A".to_string(),
        slug: format!("org-a-{}", &Uuid::new_v4().to_string()[..8]),
        description: Some("desc".to_string()),
        avatar_url: None,
    };

    let res = client
        .post(format!("{server_url}/api/organizations"))
        .header("Authorization", format!("Bearer {token}"))
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
async fn create_two_organizations_preserves_scoped_default_role_permissions() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let permissions = permissions::Entity::find().all(db.as_ref()).await.unwrap();
    let resources = resources::Entity::find().all(db.as_ref()).await.unwrap();
    let expected: HashSet<_> = permissions
        .iter()
        .flat_map(|permission| {
            resources
                .iter()
                .map(move |resource| (permission.id.clone(), resource.id.clone()))
        })
        .collect();
    assert!(!expected.is_empty(), "Default role catalog must be seeded");

    // The same catalog must be available to two distinct organizations without
    // globally deduplicating their roles or weakening the per-organization key.
    let mut created_ids = Vec::new();
    for label in ["first", "second"] {
        let org = create_scoped_role_test_organization(&client, &server_url, &token, label).await;
        created_ids.push(org.id);
        let roles = role_permissions::Entity::find()
            .filter(role_permissions::Column::OrganizationId.eq(org.id))
            .all(db.as_ref())
            .await
            .unwrap();
        let combinations: HashSet<_> = roles
            .iter()
            .map(|role| (role.permission_id.clone(), role.resource_id.clone()))
            .collect();
        assert_eq!(
            combinations, expected,
            "Complete default catalog for {label}"
        );
        assert_eq!(
            roles.len(),
            expected.len(),
            "No duplicate roles for {label}"
        );

        let member = organization_members::Entity::find()
            .filter(organization_members::Column::OrganizationId.eq(org.id))
            .filter(organization_members::Column::UserId.eq(owner_id))
            .one(db.as_ref())
            .await
            .unwrap()
            .expect("Organization owner must be a member");
        let owner_role = roles
            .iter()
            .find(|role| role.name == "organization:owner")
            .expect("Default organization owner role must remain");
        let assigned = organization_member_role_permissions::Entity::find()
            .filter(organization_member_role_permissions::Column::MemberId.eq(member.id))
            .all(db.as_ref())
            .await
            .unwrap();
        assert_eq!(assigned.len(), 1);
        assert_eq!(assigned[0].role_permission_id, owner_role.id);

        let mut duplicate = owner_role.clone().into_active_model();
        duplicate.id = Set(Uuid::new_v4());
        let error = duplicate.insert(db.as_ref()).await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("idx_role_permissions_org_unique_combo"),
            "Duplicate permission/resource in one organization must be rejected: {error}"
        );
    }
    cleanup_scoped_role_test_organizations(db.as_ref(), created_ids).await;
}

async fn create_scoped_role_test_organization(
    client: &reqwest::Client,
    server_url: &str,
    token: &str,
    label: &str,
) -> OrganizationResponse {
    let body = CreateOrganizationRequest {
        name: format!("Scoped roles {label}"),
        slug: format!("scoped-roles-{}", Uuid::new_v4()),
        description: None,
        avatar_url: None,
    };
    let response = client
        .post(format!("{server_url}/api/organizations"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "Create {label} org");
    response.json().await.unwrap()
}

async fn cleanup_scoped_role_test_organizations(db: &impl ConnectionTrait, ids: Vec<Uuid>) {
    // Only our own test-created rows cascade away. In particular, do not leave
    // cross-org collisions for the harness's next migrations-down reset.
    for id in ids {
        let result = organizations::Entity::delete_by_id(id)
            .exec(db)
            .await
            .unwrap();
        assert_eq!(result.rows_affected, 1);
    }
}

async fn role_permission_scope_state(
    db: &impl ConnectionTrait,
) -> (
    Vec<role_permissions::Model>,
    Vec<organization_member_role_permissions::Model>,
) {
    let roles = role_permissions::Entity::find()
        .order_by_asc(role_permissions::Column::Id)
        .all(db)
        .await
        .unwrap();
    let assignments = organization_member_role_permissions::Entity::find()
        .order_by_asc(organization_member_role_permissions::Column::Id)
        .all(db)
        .await
        .unwrap();
    (roles, assignments)
}

async fn role_permission_scope_indexes(db: &impl ConnectionTrait) -> Vec<String> {
    db.query_all(Statement::from_string(
        DbBackend::Postgres,
        "SELECT indexname FROM pg_indexes WHERE schemaname = current_schema() \
         AND indexname IN ('idx_role_permissions_unique_combo', \
         'idx_role_permissions_org_unique_combo') ORDER BY indexname",
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.try_get("", "indexname").unwrap())
    .collect()
}

#[tokio::test]
#[serial]
async fn initial_schema_round_trip_recreates_scoped_index_and_rollback_preserves_rows() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let db = fixture.db();
    let token = create_jwt_token(Uuid::new_v4());
    let org = create_scoped_role_test_organization(&client, &server_url, &token, "upgrade").await;
    let txn = db.begin().await.unwrap();
    let before = role_permission_scope_state(&txn).await;
    assert!(!before.0.is_empty() && !before.1.is_empty());

    // There is one initial migration, not an incremental index migration:
    // down drops the schema; the outer transaction protects the shared DB.
    Migrator::down(&txn, Some(1)).await.unwrap();
    assert_role_permission_schema_dropped(&txn).await;

    Migrator::up(&txn, Some(1)).await.unwrap();
    let recreated = role_permission_scope_state(&txn).await;
    assert!(recreated.0.is_empty() && recreated.1.is_empty());
    assert_eq!(
        role_permission_scope_indexes(&txn).await,
        vec![String::from("idx_role_permissions_org_unique_combo")]
    );

    // Only rolling back the outer transaction preserves the populated rows.
    txn.rollback().await.unwrap();
    assert_eq!(role_permission_scope_state(db.as_ref()).await, before);
    assert_eq!(
        role_permission_scope_indexes(db.as_ref()).await,
        vec![String::from("idx_role_permissions_org_unique_combo")]
    );
    cleanup_scoped_role_test_organizations(db.as_ref(), vec![org.id]).await;
}

#[tokio::test]
#[serial]
async fn initial_schema_down_drops_cross_org_roles_and_rollback_preserves_rows() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let db = fixture.db();
    let token = create_jwt_token(Uuid::new_v4());
    let first = create_scoped_role_test_organization(&client, &server_url, &token, "first").await;
    let second = create_scoped_role_test_organization(&client, &server_url, &token, "second").await;
    let txn = db.begin().await.unwrap();
    let before = role_permission_scope_state(&txn).await;

    // Flattening removed the downgrade to a global uniqueness index. Even
    // cross-org combinations are dropped by down of the initial schema.
    Migrator::down(&txn, Some(1)).await.unwrap();
    assert_role_permission_schema_dropped(&txn).await;
    txn.rollback().await.unwrap();
    assert_eq!(role_permission_scope_state(db.as_ref()).await, before);
    cleanup_scoped_role_test_organizations(db.as_ref(), vec![first.id, second.id]).await;
}

async fn assert_role_permission_schema_dropped(db: &impl ConnectionTrait) {
    let row = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT to_regclass('role_permissions') IS NULL AND \
             to_regclass('organization_member_role_permissions') IS NULL AS dropped",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(row.try_get::<bool>("", "dropped").unwrap());
}

#[tokio::test]
#[serial]
async fn get_organization_happy_path() {
    // Arrange
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Read,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization read");

    // Act - get
    let res = client
        .get(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {token}"))
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
    let (_fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();

    let res = client
        .get(format!("{server_url}/api/organizations"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let user_id = Uuid::new_v4();
    let token = create_jwt_token(user_id);
    let res = client
        .get(format!("{server_url}/api/organizations"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    // `GET /api/organizations` is `.authenticated()` only — there is no
    // `.with_permission_on(...)` on this route in `services/Hive/http/src/lib.rs`,
    // so any authenticated user reaches the handler. The test's stale
    // 403 expectation predated that route shape; the test name's
    // "returns_empty_initially" already encoded the right intent. Assert
    // 200 + empty data array.
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let items = body["data"]
        .as_array()
        .or_else(|| body["organizations"].as_array())
        .or_else(|| body.as_array())
        .expect("list response should expose an array of organizations");
    assert!(items.is_empty(), "fresh org list should be empty initially");
}

#[tokio::test]
#[serial]
async fn update_and_delete_organization_with_permissions() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);

    // Seed an org owned by owner_id with membership and owner role
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // PUT and DELETE on `/api/organizations/{org_id}` both require
    // `Permission::Admin, "organization"`. Trailing UUID = org.id.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization admin");

    // Update
    let update_body = serde_json::json!({
        "name": "Org Updated",
        "description": "new description"
    });
    let res = client
        .put(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {token}"))
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
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn search_organizations_is_public_and_returns_results() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();

    // Seed a couple orgs
    let _ = DbFixtures::organization()
        .owner_user_id(owner_id)
        .settings(serde_json::json!({
            "visibility": "Public"
        }))
        .commit(fixture.db())
        .await
        .unwrap();

    let res = client
        .get(format!(
            "{server_url}/api/organizations/search?query=Org&page=0&page_size=10"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(!body["organizations"].as_array().unwrap().is_empty());
}

async fn update_and_delete_require_auth() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    let res = client
        .put(format!("{}/api/organizations/{}", server_url, org.id))
        .json(&serde_json::json!({"name":"X"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = client
        .delete(format!("{}/api/organizations/{}", server_url, org.id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn sync_jobs_requires_auth() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    let body = serde_json::json!({
        "external_link_id": Uuid::new_v4(),
        "job_type": "full_sync",
        "options": null
    });
    let res = client
        .post(format!(
            "{}/api/organizations/{}/sync-jobs",
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
async fn roles_endpoints_require_auth() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let org = DbFixtures::organization()
        .owner_user_id(owner_id)
        .commit(fixture.db())
        .await
        .unwrap();

    let res = client
        .get(format!(
            "{}/api/organizations/{}/roles?page=1&page_size=10",
            server_url, org.id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = client
        .get(format!(
            "{}/api/organizations/{}/roles/{}",
            server_url,
            org.id,
            Uuid::new_v4()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn update_delete_forbidden_for_read_only_member() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let read_user_id = Uuid::new_v4();
    let token = create_jwt_token(read_user_id);

    let org = DbFixtures::create_org(
        fixture.db().as_ref(),
        owner_id,
        HashMap::from([
            (owner_id.to_string(), "owner".to_string()),
            (read_user_id.to_string(), "read".to_string()),
        ]),
    )
    .await
    .unwrap();

    // Default-deny: PUT and DELETE on `/api/organizations/{org_id}` both
    // require `Permission::Admin, "organization"`. Trailing UUID = org.id.
    // Real OpenFGA returns false for
    // `Check(read_user, admin, organization:<org_id>)` because no tuple
    // has been written, so both calls 403.

    let res = client
        .put(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"name":"New"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = client
        .delete(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn sync_jobs_forbidden_for_read_only_member() {
    let (fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let read_user_id = Uuid::new_v4();
    let token = create_jwt_token(read_user_id);

    let org = DbFixtures::create_org(
        fixture.db().as_ref(),
        owner_id,
        HashMap::from([
            (owner_id.to_string(), "owner".to_string()),
            (read_user_id.to_string(), "read".to_string()),
        ]),
    )
    .await
    .unwrap();

    // Default-deny: `POST /api/organizations/{org_id}/sync-jobs` requires
    // `Permission::Write, "organization"`. The path's trailing UUID is
    // `org.id` (`sync-jobs` is a string segment). Real OpenFGA returns
    // false because no tuple has been written, so the call 403s.

    let body = serde_json::json!({
        "external_link_id": Uuid::new_v4(),
        "job_type": "full_sync",
        "options": null
    });
    let res = client
        .post(format!(
            "{}/api/organizations/{}/sync-jobs",
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
async fn get_nonexistent_organization_returns_404() {
    let (_fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let user_id = Uuid::new_v4();
    let token = create_jwt_token(user_id);
    let missing_id = Uuid::new_v4();

    // GET is authenticated + `with_permission_on(Read, organization)`.
    // Grant on the missing id so the request reaches the handler 404.
    openfga
        .allow(
            Subject::new(user_id),
            Permission::Read,
            ResourceRef::new("organization", missing_id),
        )
        .await
        .expect("Failed to grant organization read");

    let res = client
        .get(format!("{server_url}/api/organizations/{missing_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn sync_jobs_nonexistent_external_link_returns_404() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Route guard must pass so this test reaches the handler's
    // nonexistent-external-link branch.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization write");

    let body = serde_json::json!({
        "external_link_id": Uuid::new_v4(),
        "job_type": "full_sync",
        "options": null
    });
    let res = client
        .post(format!(
            "{}/api/organizations/{}/sync-jobs",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn create_validation_errors() {
    let (_fixture, server_url, client, _openfga) = setup_test_server().await.unwrap();
    let user_id = Uuid::new_v4();
    let token = create_jwt_token(user_id);

    // Create with invalid payload (empty name)
    let bad_create = serde_json::json!({
        "name": "",
        "slug": "abc",
        "description": null,
        "avatar_url": null
    });
    let res = client
        .post(format!("{server_url}/api/organizations"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&bad_create)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 422);
}

#[tokio::test]
#[serial]
async fn start_sync_job_happy_path() {
    let (fixture, server_url, client, openfga) = setup_test_server().await.unwrap();
    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    // Route guard: `with_permission_on(Permission::Write, "organization")`
    // on `POST /api/organizations/{org_id}/sync-jobs`.
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("Failed to grant organization write");

    // Seed external provider
    let provider = external_providers::ActiveModel {
        id: Set(Uuid::new_v4()),
        provider_type: Set("github".to_string()),
        name: Set("GitHub".to_string()),
        config_schema: Set(None),
        is_active: Set(true),
        created_at: Set(chrono::Utc::now()),
    }
    .insert(fixture.db().as_ref())
    .await
    .unwrap();

    // Seed external link with sync enabled
    let elink = external_links::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(org.id),
        provider_id: Set(provider.id),
        provider_config: Set(serde_json::json!({"org":"dummy"})),
        sync_enabled: Set(true),
        sync_settings: Set(serde_json::json!({})),
        last_sync_at: Set(None),
        last_sync_status: Set(None),
        sync_error: Set(None),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    }
    .insert(fixture.db().as_ref())
    .await
    .unwrap();

    let body = serde_json::json!({
        "external_link_id": elink.id,
        "job_type": "full_sync",
        "options": null
    });
    let res = client
        .post(format!(
            "{}/api/organizations/{}/sync-jobs",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
