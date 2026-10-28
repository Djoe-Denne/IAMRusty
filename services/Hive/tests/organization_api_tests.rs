use reqwest::StatusCode;
use rustycog::testing::http::jwt::create_jwt_token;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serial_test::serial;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use hive_application::dto::organization::{
    CreateOrganizationRequest, OrganizationListResponse, OrganizationResponse,
};
use hive_infra::repository::entity::organizations;
use hive_infra::repository::entity::{external_links, external_providers};
use hive_infra::repository::entity::{
    organization_member_role_permissions, organization_members, permissions, resources,
    role_permissions,
};
use hive_migration::{Migrator, MigratorTrait, SchemaManager};

mod common;
use common::{fixtures::db::DbFixtures, setup_test_server, Permission, ResourceRef, Subject};

async fn with_organization_oracle_cleanup<T>(
    fixture: &common::HiveTestFixture,
    body: impl std::future::Future<Output = T>,
) -> T {
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;

    let result = AssertUnwindSafe(body).catch_unwind().await;
    let cleanup = common::cleanup_test_servers(fixture).await;
    if cleanup.is_err() {
        eprintln!(
            "fixture-owned Hive listener cleanup failed; parent lease reconciliation required"
        );
    }
    match result {
        Ok(value) => {
            assert!(
                cleanup.is_ok(),
                "owned Hive listeners must join after oracle"
            );
            value
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

async fn insert_qualified_oracle_member(
    db: &impl ConnectionTrait,
    organization: Uuid,
    user: Uuid,
    issuer: &str,
    status: &str,
) -> organization_members::Model {
    let now = chrono::Utc::now();
    // The existing member builder always inserts the historical issuer. Insert
    // the actual qualified key atomically, including when that UUID already exists.
    organization_members::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(organization),
        user_id: Set(user),
        issuer: Set(issuer.to_owned()),
        status: Set(status.to_owned()),
        invited_by_user_id: Set(None),
        invited_at: Set(None),
        joined_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("qualified membership fixture must preserve the full unique key")
}

fn recognized_oracle_authority(issuer: &str, user: Uuid) -> (serde_json::Value, String) {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use rsa::{
        pkcs8::{EncodePrivateKey, LineEnding},
        traits::PublicKeyParts,
    };
    use rustycog::testing::http::jwt::{CanonicalJwk, TEST_JWT_AUDIENCE};

    // Same independent-key and canonical issuer binding as the signer IT, not
    // two unrecognized JWTs whose authentication failures could hide a leak.
    let organization = Uuid::new_v4();
    let private = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
    let public = rsa::RsaPublicKey::from(&private);
    let canonical = CanonicalJwk::organization(issuer, organization);
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(canonical.kid().to_owned());
    header.typ = Some(rustycog::http::ACCESS_TOKEN_TYP.to_owned());
    let mut jwk = serde_json::to_value(canonical).unwrap();
    jwk["n"] = URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()).into();
    jwk["e"] = URL_SAFE_NO_PAD.encode(public.e().to_bytes_be()).into();
    let pem = private.to_pkcs8_pem(LineEnding::LF).unwrap();
    let encoding = jsonwebtoken::EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap();
    let now = chrono::Utc::now().timestamp();
    let claims = serde_json::json!({
        "iss": issuer, "sub": user, "org": organization,
        "aud": TEST_JWT_AUDIENCE, "iat": now, "exp": now + 300,
        "jti": Uuid::new_v4(),
    });
    (
        jwk,
        jsonwebtoken::encode(&header, &claims, &encoding).unwrap(),
    )
}

async fn organization_oracle_auth(
    issuers: [&str; 2],
    user: Uuid,
) -> (
    rustycog::testing::wiremock::MockServerFixture,
    rustycog::config::AuthConfig,
    [String; 2],
) {
    use rustycog::{
        config::{AuthConfig, JwtAuthConfig},
        testing::{http::jwt::TEST_JWT_AUDIENCE, wiremock::MockServerFixture},
    };
    use wiremock::{
        matchers::{method, path},
        Mock, ResponseTemplate,
    };

    let [(jwk_a, token_a), (jwk_b, token_b)] =
        issuers.map(|issuer| recognized_oracle_authority(issuer, user));
    let jwks = MockServerFixture::isolated().await;
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"keys": [jwk_a, jwk_b]})),
        )
        .mount(&jwks.server())
        .await;
    let auth = AuthConfig {
        jwt: JwtAuthConfig {
            platform_issuer: Some("https://platform.example/iam".into()),
            allow_historical_platform_issuer: Some(false),
            issuer: Some("https://platform.example/iam".into()),
            audience: Some(TEST_JWT_AUDIENCE.into()),
            jwks_url: Some(format!("{}/jwks", jwks.base_url())),
            allowed_algorithms: vec!["RS256".into()],
            ..Default::default()
        },
        ..Default::default()
    };
    let tokens = [token_a, token_b];
    let verifier = rustycog::http::UserIdExtractor::new(auth.clone()).unwrap();
    for (issuer, token) in issuers.into_iter().zip(&tokens) {
        let principal = verifier
            .extract_principal(token)
            .await
            .expect("each authority must be positively recognized before HTTP isolation checks");
        assert_eq!(principal.sub, user);
        assert_eq!(principal.iss, issuer);
    }
    (jwks, auth, tokens)
}

async fn organization_oracle_ids(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    page: u32,
    size: u32,
) -> Vec<Uuid> {
    let mut request = client
        .get(url)
        .query(&[("page", page), ("page_size", size)]);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "authorized page {page} of {url}"
    );
    response
        .json::<OrganizationListResponse>()
        .await
        .unwrap()
        .organizations
        .into_iter()
        .map(|organization| organization.id)
        .collect()
}

async fn assert_organization_oracle_pages(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    expected: &[Uuid],
) {
    assert_eq!(
        organization_oracle_ids(client, url, token, 0, 100).await,
        expected,
        "complete authorized IDs before checking single-item pages"
    );
    // The existing HTTP search test explicitly uses page=0. Exercise that same
    // indexing, not the unrelated/inconsistent total-items metadata.
    for (page, ids) in expected
        .iter()
        .map(|id| vec![*id])
        .chain(std::iter::once(Vec::new()))
        .enumerate()
    {
        let page = u32::try_from(page).unwrap();
        assert_eq!(
            organization_oracle_ids(client, url, token, page, 1).await,
            ids,
            "filtering must precede pagination, including terminal empty page {page}"
        );
    }
}

// T-1: exact issuer isolation must apply to the writer query before pagination.
#[tokio::test]
#[serial]
async fn organization_lists_and_search_scope_exact_issuer_before_pagination() {
    let issuers = ["https://alpha.example/iam", "https://beta.example/iam"];
    let user = Uuid::new_v4();
    let (jwks, auth, tokens) = organization_oracle_auth(issuers, user).await;
    let iam = hive_configuration::IamServiceConfig {
        base_url: jwks.base_url(),
        api_key: "synthetic-unused-list-oracle-credential".into(),
        ..Default::default()
    };
    let (fixture, base, client, _openfga, _) =
        Box::pin(common::setup_test_server_with_auth_and_iam(iam, Some(auth)))
            .await
            .unwrap();
    with_organization_oracle_cleanup(&fixture, async {
        let marker = format!("issuer-scope-{}", Uuid::new_v4());
        let rows = [
            ("beta-new", Some((issuers[1], "Active"))),
            ("alpha-new", Some((issuers[0], "Active"))),
            ("inactive", Some((issuers[0], "Inactive"))),
            ("public", None),
            ("alpha-old", Some((issuers[0], "Active"))),
            ("beta-old", Some((issuers[1], "Active"))),
        ];
        let mut ids = Vec::new();
        let newest = chrono::Utc::now();
        for ((label, member), age) in rows.into_iter().zip(0_i64..) {
            let organization = DbFixtures::organization()
                .name(format!("{marker}-{label}"))
                .settings(serde_json::json!({"visibility": if label == "public" { "Public" } else { "Private" }}))
                .commit(fixture.db().as_ref()).await.unwrap();
            let mut dated = organization.into_active_model();
            dated.created_at = Set(newest - chrono::Duration::days(age));
            let organization = dated.update(fixture.db().as_ref()).await.unwrap();
            if let Some((issuer, status)) = member {
                insert_qualified_oracle_member(fixture.db().as_ref(), organization.id, user, issuer, status).await;
            }
            ids.push(organization.id);
        }
        let list = format!("{base}/api/organizations");
        let search = format!("{base}/api/organizations/search?query={marker}");
        assert_eq!(client.get(&list).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);
        for (index, private_ids, search_ids) in [
            (0, [ids[1], ids[4]], [ids[1], ids[3], ids[4]]),
            (1, [ids[0], ids[5]], [ids[0], ids[3], ids[5]]),
        ] {
            assert_organization_oracle_pages(&client, &list, Some(&tokens[index]), &private_ids).await;
            assert_organization_oracle_pages(&client, &search, Some(&tokens[index]), &search_ids).await;
        }
        assert_organization_oracle_pages(&client, &search, None, &[ids[3]]).await;
    }).await;
}

async fn organization_deleted_oracle_events(
    db: &impl ConnectionTrait,
    organization: Uuid,
) -> Vec<hive_events::HiveDomainEvent> {
    db.query_all(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT payload_json FROM rustycog_outbox_events WHERE aggregate_id=$1 AND event_type=$2",
        [organization.into(), "organization_deleted".into()],
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|row| {
        let payload: serde_json::Value = row.try_get("", "payload_json").unwrap();
        serde_json::from_value(payload).expect("typed event from the real writer outbox")
    })
    .collect()
}

async fn assert_deleted_organization_oracle_rows(
    db: &impl ConnectionTrait,
    organization: Uuid,
    member_ids: &[Uuid],
) {
    assert!(
        organizations::Entity::find_by_id(organization)
            .one(db)
            .await
            .unwrap()
            .is_none(),
        "DELETE must remove the organization writer row"
    );
    assert!(
        organization_members::Entity::find()
            .filter(organization_members::Column::OrganizationId.eq(organization))
            .all(db)
            .await
            .unwrap()
            .is_empty(),
        "DELETE must remove every issuer-qualified membership writer row"
    );
    assert!(
        role_permissions::Entity::find()
            .filter(role_permissions::Column::OrganizationId.eq(organization))
            .all(db)
            .await
            .unwrap()
            .is_empty(),
        "DELETE must remove organization role rows"
    );
    assert!(
        organization_member_role_permissions::Entity::find()
            .filter(
                organization_member_role_permissions::Column::MemberId
                    .is_in(member_ids.iter().copied())
            )
            .all(db)
            .await
            .unwrap()
            .is_empty(),
        "DELETE must remove all member-role assignments"
    );
}

fn assert_organization_deleted_oracle_snapshot(
    events: &[hive_events::HiveDomainEvent],
    writer: &organizations::Model,
    members: &[organization_members::Model],
    actor: Uuid,
) {
    assert_eq!(
        events.len(),
        1,
        "exactly one deletion event must be committed"
    );
    let hive_events::HiveDomainEvent::OrganizationDeleted(event) = &events[0] else {
        panic!("organization_deleted outbox row must decode to OrganizationDeleted");
    };
    assert_eq!(event.organization_id, writer.id);
    assert_eq!(event.organization_name, writer.name);
    assert_eq!(event.owner_user_id, writer.owner_user_id);
    assert_eq!(event.owner_issuer, writer.owner_issuer);
    assert_eq!(event.deleted_by_user_id, actor);
    assert_eq!(
        event
            .base
            .metadata
            .get("deleted_by_issuer")
            .map(String::as_str),
        Some(rustycog::testing::http::jwt::TEST_JWT_ISSUER),
        "actor provenance must remain separate from owner and member authorities"
    );
    let expected: HashSet<_> = members
        .iter()
        .map(|member| (member.issuer.clone(), member.user_id))
        .collect();
    assert_eq!(
        expected.len(),
        members.len(),
        "fixture principals must be distinct qualified pairs"
    );
    let principals = event
        .member_principals
        .as_ref()
        .expect("new writer event must qualify every member");
    let actual: HashSet<_> = principals
        .iter()
        .map(|member| (member.issuer.clone(), member.user_id))
        .collect();
    assert_eq!(
        actual, expected,
        "outbox must preserve exact writer principal pairs, not actor authority"
    );
    assert_eq!(
        principals.len(),
        members.len(),
        "set equality must not hide duplicates or UUID-only merging"
    );
    let mut legacy_ids = event.member_user_ids.clone();
    let mut writer_ids: Vec<_> = members.iter().map(|member| member.user_id).collect();
    legacy_ids.sort_unstable();
    writer_ids.sort_unstable();
    assert_eq!(
        legacy_ids, writer_ids,
        "legacy UUID view must retain both same-UUID member entries"
    );
}

// T-2: the outbox snapshot comes from PG, never the actor or a UUID-deduplicated set.
#[tokio::test]
#[serial]
async fn organization_delete_outbox_preserves_all_qualified_writer_principals() {
    let (fixture, base, client, openfga) = Box::pin(setup_test_server()).await.unwrap();
    with_organization_oracle_cleanup(&fixture, async {
        let db = fixture.db();
        let owner = Uuid::new_v4();
        let actor = Uuid::new_v4();
        let same_member = Uuid::new_v4();
        let organization = DbFixtures::organization()
            .owner_user_id(owner)
            .owner_issuer(Some("https://owner.example/iam".into()))
            .commit(db.as_ref())
            .await
            .unwrap();
        for (user, issuer) in [
            (owner, "https://owner.example/iam"),
            (same_member, "https://alpha.example/iam"),
            (same_member, "https://beta.example/iam"),
        ] {
            insert_qualified_oracle_member(db.as_ref(), organization.id, user, issuer, "Active")
                .await;
        }
        let members = organization_members::Entity::find()
            .filter(organization_members::Column::OrganizationId.eq(organization.id))
            .order_by_asc(organization_members::Column::Id)
            .all(db.as_ref())
            .await
            .unwrap();
        let role = DbFixtures::role_permission()
            .organization_id(organization.id)
            .permission_id("read")
            .resource_id("organization")
            .name("qualified-oracle-reader")
            .commit(db.as_ref())
            .await
            .unwrap();
        for member in &members {
            DbFixtures::member_role_permission_link()
                .member_id(member.id)
                .role_permission_id(role.id)
                .commit(db.as_ref())
                .await
                .unwrap();
        }
        let writer = organizations::Entity::find_by_id(organization.id)
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            members.len(),
            3,
            "fixture must include owner and both same-UUID authorities"
        );
        assert!(
            organization_deleted_oracle_events(db.as_ref(), organization.id)
                .await
                .is_empty()
        );
        openfga
            .allow(
                Subject::new(actor),
                Permission::Admin,
                ResourceRef::new("organization", organization.id),
            )
            .await
            .unwrap();
        let response = client
            .delete(format!("{base}/api/organizations/{}", organization.id))
            .bearer_auth(create_jwt_token(actor))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "distinct authorized actor must reach the writer DELETE"
        );
        let events = organization_deleted_oracle_events(db.as_ref(), organization.id).await;
        assert_organization_deleted_oracle_snapshot(&events, &writer, &members, actor);
        let member_ids: Vec<_> = members.iter().map(|member| member.id).collect();
        assert_deleted_organization_oracle_rows(db.as_ref(), organization.id, &member_ids).await;
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unresolved_legacy_owner_cannot_be_deleted_or_projected() {
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;
    let (fixture, base, client, openfga) = Box::pin(setup_test_server()).await.unwrap();
    let result = AssertUnwindSafe(async {
        let user = Uuid::new_v4();
        let token = create_jwt_token(user);
        // Deliberately unresolved history, not the fixture's explicit default.
        let organization = DbFixtures::organization().owner_user_id(user).owner_issuer(None)
            .commit(fixture.db().as_ref()).await.unwrap();
        openfga.allow(Subject::new(user), Permission::Admin, ResourceRef::new("organization", organization.id)).await.unwrap();
        let count_statement = || Statement::from_string(DbBackend::Postgres, "SELECT COUNT(*) AS count FROM rustycog_outbox_events".to_owned());
        let before: i64 = fixture.db().query_one(count_statement()).await.unwrap().unwrap().try_get("", "count").unwrap();
        let response = client.delete(format!("{base}/api/organizations/{}", organization.id))
            .bearer_auth(token).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "authorized request must reach unresolved writer provenance, not an authentication denial");
        assert!(response.text().await.unwrap().contains("EVENT_PRINCIPAL_UNRESOLVED"));
        let remaining = organizations::Entity::find_by_id(organization.id).one(fixture.db().as_ref()).await.unwrap().unwrap();
        assert!(remaining == organization, "unresolved owner must leave the whole writer row unchanged");
        let after: i64 = fixture.db().query_one(count_statement()).await.unwrap().unwrap().try_get("", "count").unwrap();
        assert_eq!(before, after, "unresolved owner must not emit a projection event");
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

    // T-3: exercise the actual initial migration, not the newer provenance
    // migration whose downgrade intentionally refuses to erase authority.
    let initial = Migrator::migrations()
        .into_iter()
        .find(|migration| migration.name() == "m20240101_000001_initial_schema")
        .expect("registered initial Hive migration");
    let manager = SchemaManager::new(&txn);
    initial.down(&manager).await.unwrap();
    assert_role_permission_schema_dropped(&txn).await;

    initial.up(&manager).await.unwrap();
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
    // T-3: keep the historical dropped-table oracle, targeting the initial
    // schema explicitly instead of the latest owner_issuer migration.
    let initial = Migrator::migrations()
        .into_iter()
        .find(|migration| migration.name() == "m20240101_000001_initial_schema")
        .expect("registered initial Hive migration");
    initial.down(&SchemaManager::new(&txn)).await.unwrap();
    assert_role_permission_schema_dropped(&txn).await;
    txn.rollback().await.unwrap();
    assert_eq!(role_permission_scope_state(db.as_ref()).await, before);
    cleanup_scoped_role_test_organizations(db.as_ref(), vec![first.id, second.id]).await;
}

async fn organization_migration_oracle_snapshot(
    db: &impl ConnectionTrait,
) -> HashMap<String, serde_json::Value> {
    let mut snapshot = HashMap::new();
    // Fixed Hive schema allowlist, including outbox and the migration ledger.
    // Row JSON is ordered independently of physical layout and never printed.
    for table in [
        "organizations",
        "organization_members",
        "organization_member_role_permissions",
        "role_permissions",
        "permissions",
        "resources",
        "organization_invitations",
        "external_links",
        "external_providers",
        "sync_jobs",
        "outbox_events",
        "seaql_migrations",
    ] {
        // Retain the snapshot's logical key while querying the actual migrated
        // SDK table; there is no physical table named outbox_events.
        let physical_table = if table == "outbox_events" {
            "rustycog_outbox_events"
        } else {
            table
        };
        let sql = format!(
            "SELECT COALESCE(jsonb_agg(row_data ORDER BY row_data::text), '[]'::jsonb) AS snapshot \
             FROM (SELECT to_jsonb(source) AS row_data FROM {physical_table} source) AS source_rows"
        );
        let row = db
            .query_one(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .unwrap()
            .unwrap();
        snapshot.insert(table.to_owned(), row.try_get("", "snapshot").unwrap());
    }
    for (key, sql) in [
        ("columns", "SELECT COALESCE(jsonb_agg(to_jsonb(c) ORDER BY c.table_name, c.ordinal_position), '[]'::jsonb) AS snapshot FROM information_schema.columns c WHERE c.table_schema = current_schema()"),
        ("indexes", "SELECT COALESCE(jsonb_agg(to_jsonb(i) ORDER BY i.tablename, i.indexname), '[]'::jsonb) AS snapshot FROM pg_indexes i WHERE i.schemaname = current_schema()"),
    ] {
        let row = db.query_one(Statement::from_string(DbBackend::Postgres, sql))
            .await.unwrap().unwrap();
        snapshot.insert(key.to_owned(), row.try_get("", "snapshot").unwrap());
    }
    snapshot
}

// Descriptor compatibility: each serial fixture must start clean even though
// the production owner_issuer migration deliberately forbids downgrade.
#[tokio::test]
#[serial]
async fn fixture_schema_reset_clears_rows_and_reapplies_owner_provenance() {
    use rustycog::testing::ServiceTestDescriptor;

    let descriptor = std::sync::Arc::new(common::HiveTestDescriptor);
    let fixture = Box::pin(common::HiveTestFixture::new(descriptor.clone()))
        .await
        .unwrap();
    with_organization_oracle_cleanup(&fixture, async {
        let db = fixture.db();
        let owner = Uuid::new_v4();
        let organization = DbFixtures::create_org_with_owner(db.as_ref(), owner).await.unwrap();
        insert_qualified_oracle_member(db.as_ref(), organization.id, owner,
            "https://independent-reset.example/iam", "Active").await;
        db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO rustycog_outbox_events \
             (id,event_id,event_type,aggregate_id,version,occurred_at,payload_json,metadata_json,status) \
             VALUES ($1,$2,'fixture_reset_probe',$3,1,now(),'{}'::jsonb,'{}'::jsonb,'pending')",
            [Uuid::new_v4().into(), Uuid::new_v4().into(), organization.id.into()],
        )).await.unwrap();
        let populated = organization_migration_oracle_snapshot(db.as_ref()).await;
        let cleared_tables = ["organizations", "organization_members", "role_permissions",
            "organization_member_role_permissions", "outbox_events"];
        for table in cleared_tables {
            assert!(!populated[table].as_array().unwrap().is_empty(), "reset witness must populate {table}");
        }
        // No listener/worker was started: this acts only on this fixture's PG.
        descriptor.run_migrations_down(db.as_ref()).await.expect("real private descriptor reset");
        let reset = organization_migration_oracle_snapshot(db.as_ref()).await;
        for table in cleared_tables {
            assert!(reset[table].as_array().unwrap().is_empty(), "private fixture reset must clear {table}");
        }
        let owner_columns: Vec<_> = reset["columns"].as_array().unwrap().iter()
            .filter(|column| column["table_name"] == "organizations" && column["column_name"] == "owner_issuer")
            .collect();
        assert_eq!(owner_columns.len(), 1, "reset must reapply additive owner provenance");
        assert_eq!(owner_columns[0]["is_nullable"], "YES");
        let mut versions: Vec<_> = reset["seaql_migrations"].as_array().unwrap().iter()
            .map(|row| row["version"].as_str().unwrap()).collect();
        versions.sort_unstable();
        assert_eq!(versions, vec!["m20240101_000001_initial_schema", "m20261028_000002_owner_issuer"],
            "both actual Hive migrations must be recorded after reset");
        assert_eq!(role_permission_scope_indexes(db.as_ref()).await,
            vec![String::from("idx_role_permissions_org_unique_combo")]);
        descriptor.run_migrations_up(db.as_ref()).await.expect("generic fixture up after fresh");
        let after_up = organization_migration_oracle_snapshot(db.as_ref()).await;
        assert!(after_up == reset, "the generic up step after fresh must be a data/schema/ledger no-op");
        let inserted = DbFixtures::organization().id(organization.id).slug(organization.slug.clone())
            .owner_user_id(owner).commit(db.as_ref()).await.expect("same primary/unique keys must be reusable");
        let member = insert_qualified_oracle_member(db.as_ref(), inserted.id, owner,
            rustycog::testing::http::jwt::TEST_JWT_ISSUER, "Active").await;
        let stored = organizations::Entity::find_by_id(inserted.id).one(db.as_ref()).await.unwrap().unwrap();
        assert!(stored == inserted, "new legitimate organization must round-trip after reset");
        assert_eq!(stored.owner_issuer.as_deref(), Some(rustycog::testing::http::jwt::TEST_JWT_ISSUER));
        let members = organization_members::Entity::find().all(db.as_ref()).await.unwrap();
        assert_eq!(members, vec![member], "only the new qualified membership may remain");
    }).await;
}

fn assert_legacy_owner_upgrade_snapshot(
    mut before: HashMap<String, serde_json::Value>,
    mut after: HashMap<String, serde_json::Value>,
) {
    let is_owner_column = |column: &serde_json::Value| {
        column["table_name"] == "organizations" && column["column_name"] == "owner_issuer"
    };
    assert!(
        before["columns"]
            .as_array()
            .unwrap()
            .iter()
            .all(|column| !is_owner_column(column)),
        "legacy rows must really predate the owner_issuer column"
    );
    let columns = after.get_mut("columns").unwrap().as_array_mut().unwrap();
    let added: Vec<_> = columns
        .iter()
        .filter(|column| is_owner_column(column))
        .collect();
    assert_eq!(
        added.len(),
        1,
        "additive upgrade must introduce the provenance column"
    );
    assert_eq!(
        added[0]["is_nullable"], "YES",
        "unresolved historical owners must remain representable"
    );
    columns.retain(|column| !is_owner_column(column));
    for row in before["organizations"].as_array().unwrap() {
        assert!(
            row.get("owner_issuer").is_none(),
            "old writer rows must not already contain provenance"
        );
    }
    for row in after
        .get_mut("organizations")
        .unwrap()
        .as_array_mut()
        .unwrap()
    {
        assert_eq!(
            row.as_object_mut().unwrap().remove("owner_issuer"),
            Some(serde_json::Value::Null),
            "neither ambiguous nor apparently-platform ownership may be automatically stamped"
        );
    }
    // Compare every original value, independent of JSON aggregation order after
    // adding a column. Only that new, explicitly NULL field is excluded.
    for snapshot in [&mut before, &mut after] {
        snapshot
            .get_mut("organizations")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .sort_by_key(|row| row["id"].as_str().unwrap().to_owned());
    }
    assert!(after == before,
        "upgrade must preserve all legacy values, UUIDs, owners, memberships, indexes, other columns and migration ledger");
}

// T-3 upgrade gap: existing rows must cross the real additive migration, not be
// inserted with NULL provenance only after the new schema has already run.
#[tokio::test]
#[serial]
async fn owner_issuer_upgrade_preserves_legacy_rows_without_backfill() {
    // Existing PG/FGA fixture only: no application listener or HTTP/outbox call
    // runs against the transient old schema. All DDL/data are rolled back.
    let fixture = Box::pin(common::HiveTestFixture::new(std::sync::Arc::new(
        common::HiveTestDescriptor,
    )))
    .await
    .unwrap();
    with_organization_oracle_cleanup(&fixture, async {
        let db = fixture.db();
        let original = organization_migration_oracle_snapshot(db.as_ref()).await;
        let txn = db.begin().await.unwrap();
        let initial = Migrator::migrations().into_iter()
            .find(|migration| migration.name() == "m20240101_000001_initial_schema")
            .expect("registered initial Hive migration");
        initial.down(&SchemaManager::new(&txn)).await.unwrap();
        initial.up(&SchemaManager::new(&txn)).await.unwrap();
        let legacy = [
            (Uuid::new_v4(), Uuid::new_v4(), "ambiguous-owner"),
            (Uuid::new_v4(), Uuid::new_v4(), "apparently-platform-owner"),
        ];
        for (id, owner, label) in legacy {
            txn.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                r#"INSERT INTO organizations
                   (id, name, slug, description, avatar_url, owner_user_id, settings,
                    signing_status, created_at, updated_at)
                   VALUES ($1, $2, $3, 'historical description', 'https://legacy.example/avatar.png',
                           $4, '{"visibility":"Private","legacy":"preserve"}'::jsonb, 'active',
                           '2024-01-02T03:04:05Z'::timestamptz, '2024-06-07T08:09:10Z'::timestamptz)"#,
                [id.into(), label.into(), format!("legacy-upgrade-{id}").into(), owner.into()],
            )).await.unwrap();
            insert_qualified_oracle_member(&txn, id, owner,
                rustycog::testing::http::jwt::TEST_JWT_ISSUER, "Active").await;
        }
        // A same-UUID second authority makes one owner's provenance ambiguous;
        // the seemingly platform-only owner must not be guessed/backfilled either.
        insert_qualified_oracle_member(&txn, legacy[0].0, legacy[0].1,
            "https://independent-legacy.example/iam", "Active").await;
        let before = organization_migration_oracle_snapshot(&txn).await;
        let provenance = Migrator::migrations().into_iter()
            .find(|migration| migration.name() == "m20261028_000002_owner_issuer")
            .expect("registered additive owner provenance migration");
        provenance.up(&SchemaManager::new(&txn)).await.unwrap();
        let migrated = organizations::Entity::find().all(&txn).await.unwrap();
        let after = organization_migration_oracle_snapshot(&txn).await;
        txn.rollback().await.unwrap();
        let restored = organization_migration_oracle_snapshot(db.as_ref()).await;
        assert!(restored == original, "outer rollback must restore the whole original fixture database");
        assert_eq!(migrated.len(), legacy.len(), "upgrade must retain every historical organization");
        for (id, owner, _) in legacy {
            let row = migrated.iter().find(|row| row.id == id).expect("historical organization UUID must survive");
            assert_eq!(row.owner_user_id, owner);
            assert!(row.owner_issuer.is_none(), "upgrade must not infer an owner authority from memberships or platform configuration");
        }
        assert_legacy_owner_upgrade_snapshot(before, after);
    }).await;
}

// T-3: refusing authority loss must not replace the two initial-schema oracles.
#[tokio::test]
#[serial]
async fn owner_issuer_down_refuses_without_modifying_data() {
    let (fixture, base, client, _openfga) = Box::pin(setup_test_server()).await.unwrap();
    with_organization_oracle_cleanup(&fixture, async {
        let owner = Uuid::new_v4();
        let token = create_jwt_token(owner);
        let organization = create_scoped_role_test_organization(&client, &base, &token, "provenance-down").await;
        insert_qualified_oracle_member(fixture.db().as_ref(), organization.id, owner,
            "https://independent-owner.example/iam", "Active").await;
        let before = organization_migration_oracle_snapshot(fixture.db().as_ref()).await;
        assert!(!before["organizations"].as_array().unwrap().is_empty());
        assert!(!before["outbox_events"].as_array().unwrap().is_empty(), "exercise nonempty transactional outbox");
        assert_eq!(role_permission_scope_indexes(fixture.db().as_ref()).await,
            vec![String::from("idx_role_permissions_org_unique_combo")]);
        let txn = fixture.db().begin().await.unwrap();
        let provenance = Migrator::migrations().into_iter()
            .find(|migration| migration.name() == "m20261028_000002_owner_issuer")
            .expect("registered additive owner provenance migration");
        let result = provenance.down(&SchemaManager::new(&txn)).await;
        let during = organization_migration_oracle_snapshot(&txn).await;
        txn.rollback().await.unwrap();
        let after = organization_migration_oracle_snapshot(fixture.db().as_ref()).await;
        assert!(result.is_err(), "provenance downgrade must refuse without a coordinated authority rollback");
        assert!(during == before, "refusal must not mutate any writer data, column, index or migration ledger inside the transaction");
        assert!(after == before, "refusal and outer rollback must preserve the original persisted database snapshot");
    }).await;
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
