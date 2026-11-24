//! STORY-008 backend contract. The outbox is evidence of enqueue, not of mail delivery.

mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
mod fixtures;
#[path = "support/profile_outbox_trace.rs"]
mod profile_outbox_trace;

use fixtures::DbFixtures;
use iam_infra::{auth::PasswordService, repository::entity::user_emails};
use reqwest::{Client, Response, StatusCode};
use rustycog::outbox::entity::{Column as OutboxColumn, OutboxEvents};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter,
    Statement,
};
use serde_json::{json, Value};
use serial_test::serial;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

const ADDRESSES: &str = "/api/me/email-addresses";
const REQUEST: &str = "/api/me/email-verification/request";
const CONFIRM: &str = "/api/me/email-verification/confirm";
const LANDING: &str = "https://profile-verify.example.invalid/confirm-address";
const PASSWORD: &str = "ProfileFixture12!";

struct Api {
    base: String,
    client: Client,
}

impl Api {
    async fn post(&self, path: &str, access: Option<&str>, body: Value) -> Response {
        let mut request = self.client.post(format!("{}{path}", self.base)).json(&body);
        if let Some(access) = access {
            request = request.bearer_auth(access);
        }
        request.send().await.expect("send IAM fixture POST")
    }

    async fn get(&self, path: &str, access: &str) -> Response {
        self.client
            .get(format!("{}{path}", self.base))
            .bearer_auth(access)
            .send()
            .await
            .expect("send IAM fixture GET")
    }

    async fn login(&self, email: &str) -> Response {
        self.post(
            "/api/auth/login",
            None,
            json!({"email": email, "password": PASSWORD}),
        )
        .await
    }

    async fn request(&self, access: Option<&str>, id: Uuid) -> Response {
        self.post(REQUEST, access, json!({"address_id": id})).await
    }

    async fn confirm(&self, access: Option<&str>, token: &str) -> Response {
        self.post(CONFIRM, access, json!({"token": token})).await
    }

    async fn addresses(&self, access: &str) -> Vec<Value> {
        let response = self.get(ADDRESSES, access).await;
        assert_eq!(response.status(), StatusCode::OK, "owned address list");
        response.json().await.expect("address list JSON")
    }

    async fn get_without_session(&self, path: &str) -> Response {
        self.client
            .get(format!("{}{path}", self.base))
            .send()
            .await
            .expect("send signed-out IAM fixture GET")
    }
}

async fn assert_empty(response: Response, status: StatusCode) {
    assert_eq!(response.status(), status);
    assert!(!response.headers().contains_key(reqwest::header::SET_COOKIE));
    assert!(!response.headers().contains_key(reqwest::header::LOCATION));
    assert!(
        response.bytes().await.expect("response body").is_empty(),
        "response must not disclose identity, session or verification data"
    );
}

async fn assert_pending_digest(db: &DatabaseConnection, address_id: Uuid, token: &str) {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT token_digest = $2 AS digest_matches,
                    expires_at - issued_at = interval '24 hours' AS bounded,
                    confirmed_at IS NULL AS pending
             FROM profile_email_verifications WHERE address_id = $1",
            [
                address_id.into(),
                Sha256::digest(token.as_bytes()).to_vec().into(),
            ],
        ))
        .await
        .expect("read verification record")
        .expect("verification record exists");
    for property in ["digest_matches", "bounded", "pending"] {
        assert!(
            row.try_get::<bool>("", property)
                .expect("verification invariant"),
            "verification record did not satisfy {property}"
        );
    }
}

async fn seed_user(db: &Arc<DatabaseConnection>, username: &str, hash: String) -> Uuid {
    DbFixtures::user()
        .username(username)
        .password_hash(hash)
        .commit(db.clone())
        .await
        .expect("seed user")
        .id()
}

async fn seed_address(
    db: &Arc<DatabaseConnection>,
    user_id: Uuid,
    label: &str,
    primary: bool,
    verified: bool,
) -> user_emails::Model {
    DbFixtures::user_email()
        .user_id(user_id)
        .email(format!("{label}-{}@example.com", Uuid::new_v4().simple()))
        .with_primary(primary)
        .with_verified(verified)
        .commit(db.clone())
        .await
        .expect("seed saved address")
        .model()
        .clone()
}

async fn platform_session(api: &Api, row: &user_emails::Model) -> String {
    let response = api.login(&row.email).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "baseline login with a saved legacy-verified address"
    );
    let body: Value = response.json().await.expect("baseline login JSON");
    let access = body["access_token"]
        .as_str()
        .expect("baseline login issued an access token");
    assert_eq!(
        api.get("/api/me", access).await.status(),
        StatusCode::OK,
        "baseline authenticated profile path must work"
    );
    access.to_owned()
}

fn password_hash() -> String {
    PasswordService::new()
        .hash_password(PASSWORD)
        .expect("hash fixture password")
}

fn assert_address(list: &[Value], row: &user_emails::Model, verified: bool) {
    assert!(
        list.contains(&json!({
            "id": row.id,
            "email": row.email,
            "is_primary": row.is_primary,
            "is_verified": verified,
        })),
        "saved address projection must match the expected identity and flags"
    );
}

async fn intents(db: &DatabaseConnection, user_id: Uuid) -> Vec<Value> {
    OutboxEvents::find()
        .filter(OutboxColumn::AggregateId.eq(user_id))
        .filter(OutboxColumn::EventType.eq("profile_email_verification_requested"))
        .all(db)
        .await
        .expect("read dedicated outbox delivery intents")
        .into_iter()
        .map(|row| row.payload_json)
        .collect()
}

fn capability(payload: &Value, row: &user_emails::Model) -> String {
    assert!(
        payload["email"].as_str() == Some(row.email.as_str()),
        "delivery must use exactly the saved address"
    );
    let mut link = payload["verification_url"]
        .as_str()
        .and_then(|url| url::Url::parse(url).ok())
        .expect("delivery intent contains a valid verification URL");
    let token = link
        .fragment()
        .expect("capability is in the fragment")
        .to_owned();
    link.set_fragment(None);
    assert_eq!(link.as_str(), LANDING, "only the configured HTTPS landing");
    assert!(
        token.len() >= 32
            && token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')),
        "opaque URL-safe capability, not account data"
    );
    token
}

async fn auth_state(db: &DatabaseConnection, user_id: Uuid) -> Value {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT jsonb_build_object(
            'sessions', (SELECT jsonb_agg(r ORDER BY r.id) FROM refresh_tokens r WHERE r.user_id=$1),
            'providers', (SELECT jsonb_agg(p ORDER BY p.id) FROM provider_tokens p WHERE p.user_id=$1),
            'account', (SELECT to_jsonb(u) FROM users u WHERE u.id=$1)
        ) AS state",
        [user_id.into()],
    ))
    .await
    .expect("read auth state")
    .expect("auth state row")
    .try_get("", "state")
    .expect("auth state JSON")
}

#[tokio::test]
#[serial]
async fn same_account_confirms_exact_saved_address_without_enabling_login() {
    let (fixture, base, client) = Box::pin(common::setup_test_server_with_profile_landing(LANDING))
        .await
        .expect("start IAM fixture");
    Box::pin(fixture_cleanup::run(&fixture, async {
        let api = Api { base, client };
        let db = fixture.db();
        let hash = password_hash();
        let owner = seed_user(&db, "proofowner", hash.clone()).await;
        let other = seed_user(&db, "proofother", hash).await;
        let primary = seed_address(&db, owner, "Primary", true, false).await;
        let pending = seed_address(&db, owner, "Pending", false, false).await;
        let login_address = seed_address(&db, owner, "Login", false, true).await;
        let foreign = seed_address(&db, other, "Other", true, true).await;
        let owner_access = platform_session(&api, &login_address).await;
        let other_access = platform_session(&api, &foreign).await;
        let before = api.login(&primary.email).await;
        assert_eq!(
            before.status(),
            StatusCode::UNAUTHORIZED,
            "baseline login eligibility"
        );
        let before: Value = before.json().await.expect("baseline denial JSON");
        assert_eq!(before["error"]["error_code"], "email_not_verified");
        let state = auth_state(db.as_ref(), owner).await;

        assert_empty(
            api.request(Some(&owner_access), primary.id).await,
            StatusCode::ACCEPTED,
        )
        .await;
        let rows = intents(db.as_ref(), owner).await;
        assert_eq!(rows.len(), 1, "exactly one primary-address delivery intent");
        let token = capability(&rows[0], &primary);
        assert_pending_digest(db.as_ref(), primary.id, &token).await;

        assert_empty(
            api.get_without_session(ADDRESSES).await,
            StatusCode::UNAUTHORIZED,
        )
        .await;
        let list = api.addresses(&owner_access).await;
        assert_eq!(
            list.len(),
            3,
            "projection must not expose foreign saved rows"
        );
        assert_address(&list, &primary, false);
        assert_address(&list, &pending, false);
        assert_address(&list, &login_address, true);

        assert_eq!(
            api.get(CONFIRM, &owner_access).await.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "GET must not invoke the confirmation action"
        );
        assert_eq!(
            api.get(CONFIRM, &other_access).await.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "GET must not discriminate between accounts"
        );
        assert_empty(api.confirm(None, &token).await, StatusCode::UNAUTHORIZED).await;
        assert_empty(
            api.confirm(Some(&other_access), &token).await,
            StatusCode::NOT_FOUND,
        )
        .await;
        assert_pending_digest(db.as_ref(), primary.id, &token).await;
        assert_address(&api.addresses(&owner_access).await, &primary, false);
        assert_eq!(
            intents(db.as_ref(), owner).await.len(),
            1,
            "no implicit resend"
        );

        let (base, client) = Box::pin(common::setup_test_replica(&fixture))
            .await
            .expect("start second IAM listener on same writer");
        let second = Api { base, client };
        let (first, retry) = tokio::join!(
            api.confirm(Some(&owner_access), &token),
            second.confirm(Some(&owner_access), &token)
        );
        assert!(
            matches!(
                (first.status(), retry.status()),
                (StatusCode::NO_CONTENT, StatusCode::NOT_FOUND)
                    | (StatusCode::NOT_FOUND, StatusCode::NO_CONTENT)
            ),
            "exactly one explicit confirmation must succeed across listeners"
        );
        let first_status = first.status();
        let retry_status = retry.status();
        assert_empty(first, first_status).await;
        assert_empty(retry, retry_status).await;
        assert_empty(
            api.confirm(Some(&owner_access), &token).await,
            StatusCode::NOT_FOUND,
        )
        .await;

        let list = api.addresses(&owner_access).await;
        assert_eq!(list.len(), 3);
        assert_address(&list, &primary, true);
        assert_address(&list, &pending, false);
        assert_address(&list, &login_address, true);
        let legacy = user_emails::Entity::find_by_id(primary.id)
            .one(db.as_ref())
            .await
            .expect("read selected legacy row")
            .expect("selected row persists");
        assert!(
            legacy == primary,
            "profile proof must not modify the legacy row"
        );
        assert!(
            state == auth_state(db.as_ref(), owner).await,
            "no session, provider or account modification"
        );
        let after = api.login(&primary.email).await;
        assert_eq!(
            after.status(),
            StatusCode::UNAUTHORIZED,
            "proof does not enable sign-in"
        );
        let after: Value = after.json().await.expect("post-proof denial JSON");
        assert!(after == before, "the legacy login denial remains unchanged");
        assert_eq!(
            intents(db.as_ref(), owner).await.len(),
            1,
            "confirmation and replay send no mail"
        );
    }))
    .await;
}

async fn change_one(db: &DatabaseConnection, sql: &str, values: Vec<sea_orm::Value>) {
    let result = db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            values,
        ))
        .await
        .expect("arrange a single fixture-only writer change");
    assert_eq!(
        result.rows_affected(),
        1,
        "fixture arrangement affected exactly one row"
    );
}

async fn issued_at(db: &DatabaseConnection, address_id: Uuid) -> chrono::DateTime<chrono::Utc> {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT issued_at FROM profile_email_verifications WHERE address_id = $1",
        [address_id.into()],
    ))
    .await
    .expect("read persisted issuance timestamp")
    .expect("verification row exists")
    .try_get("", "issued_at")
    .expect("issuance timestamp")
}

#[tokio::test]
#[serial]
async fn resend_is_explicit_throttled_and_scoped_and_stale_links_cannot_verify() {
    let (fixture, base, client) = Box::pin(common::setup_test_server_with_profile_landing(LANDING))
        .await
        .expect("start IAM fixture");
    Box::pin(fixture_cleanup::run(&fixture, async {
        let api = Api { base, client };
        let db = fixture.db();
        let hash = password_hash();
        let owner = seed_user(&db, "resendowner", hash.clone()).await;
        let other = seed_user(&db, "resendother", hash).await;
        let login_address = seed_address(&db, owner, "Login", true, true).await;
        let target = seed_address(&db, owner, "Target", false, false).await;
        let sibling = seed_address(&db, owner, "Sibling", false, false).await;
        let foreign = seed_address(&db, other, "Foreign", true, false).await;
        let access = platform_session(&api, &login_address).await;

        assert_empty(api.request(Some(&access), target.id).await, StatusCode::ACCEPTED).await;
        let rows = intents(db.as_ref(), owner).await;
        assert_eq!(rows.len(), 1, "initial secondary-address intent");
        let old = capability(&rows[0], &target);

        assert_empty(api.request(None, target.id).await, StatusCode::UNAUTHORIZED).await;
        for id in [Uuid::new_v4(), foreign.id, login_address.id] {
            assert_empty(api.request(Some(&access), id).await, StatusCode::NOT_FOUND).await;
        }
        let issue_time = issued_at(db.as_ref(), target.id).await;
        assert_empty(api.request(Some(&access), target.id).await, StatusCode::TOO_MANY_REQUESTS)
            .await;
        assert_eq!(
            issued_at(db.as_ref(), target.id).await,
            issue_time,
            "a throttled request must not extend the cooldown"
        );
        assert_eq!(intents(db.as_ref(), owner).await.len(), 1);
        assert!(intents(db.as_ref(), other).await.is_empty(), "no intent for foreign account");

        assert_empty(api.request(Some(&access), sibling.id).await, StatusCode::ACCEPTED).await;
        change_one(
            db.as_ref(),
            "UPDATE profile_email_verifications
             SET issued_at = issued_at - interval '61 seconds', expires_at = expires_at - interval '61 seconds'
             WHERE address_id = $1",
            vec![target.id.into()],
        )
        .await;
        assert_empty(api.request(Some(&access), target.id).await, StatusCode::ACCEPTED).await;
        let rows = intents(db.as_ref(), owner).await;
        assert_eq!(rows.len(), 3, "only three explicit eligible requests enqueue");
        let mut sibling_links = rows.iter().filter(|row| row["email"].as_str() == Some(sibling.email.as_str()));
        let sibling_token = capability(sibling_links.next().expect("sibling intent"), &sibling);
        assert!(sibling_links.next().is_none(), "resend did not replace sibling intent");
        let mut target_links = rows
            .iter()
            .filter(|row| row["email"].as_str() == Some(target.email.as_str()))
            .map(|row| capability(row, &target))
            .filter(|token| token != &old);
        let fresh = target_links.next().expect("replacement has a different capability");
        assert!(target_links.next().is_none(), "one replacement intent only");
        assert_empty(api.confirm(Some(&access), &old).await, StatusCode::NOT_FOUND).await;

        change_one(
            db.as_ref(),
            "UPDATE profile_email_verifications
             SET issued_at = clock_timestamp() - interval '25 hours', expires_at = clock_timestamp() - interval '1 hour'
             WHERE address_id = $1",
            vec![target.id.into()],
        )
        .await;
        assert_empty(api.confirm(Some(&access), &fresh).await, StatusCode::NOT_FOUND).await;
        assert_address(&api.addresses(&access).await, &target, false);
        assert_eq!(intents(db.as_ref(), owner).await.len(), 3, "expiry cannot issue a replacement");

        assert_empty(api.request(Some(&access), target.id).await, StatusCode::ACCEPTED).await;
        let rows = intents(db.as_ref(), owner).await;
        assert_eq!(rows.len(), 4);
        let newest = rows
            .iter()
            .filter(|row| row["email"].as_str() == Some(target.email.as_str()))
            .map(|row| capability(row, &target))
            .find(|token| token != &old && token != &fresh)
            .expect("a new capability after an explicit request");
        let mut changed = target.clone();
        changed.email = changed.email.to_ascii_lowercase();
        assert!(changed.email != target.email, "test needs an exact-address change");
        change_one(
            db.as_ref(),
            "UPDATE user_emails SET email=$2 WHERE id=$1",
            vec![target.id.into(), changed.email.as_str().into()],
        )
        .await;
        assert_empty(api.confirm(Some(&access), &newest).await, StatusCode::NOT_FOUND).await;
        assert_empty(api.confirm(Some(&access), &sibling_token).await, StatusCode::NO_CONTENT).await;
        let list = api.addresses(&access).await;
        assert_address(&list, &changed, false);
        assert_address(&list, &sibling, true);
        assert_address(&list, &login_address, true);
        assert_empty(api.request(Some(&access), sibling.id).await, StatusCode::NOT_FOUND).await;

        assert_empty(api.request(Some(&access), changed.id).await, StatusCode::ACCEPTED).await;
        let rows = intents(db.as_ref(), owner).await;
        assert_eq!(rows.len(), 5);
        let changed_payload = rows.iter()
            .find(|row| row["email"].as_str() == Some(changed.email.as_str()))
            .expect("explicit intent for changed address");
        let deleted_token = capability(changed_payload, &changed);
        change_one(
            db.as_ref(),
            "DELETE FROM user_emails WHERE id=$1",
            vec![changed.id.into()],
        )
        .await;
        let replacement = DbFixtures::user_email()
            .user_id(owner)
            .email(changed.email.as_str())
            .with_primary(false)
            .with_verified(false)
            .commit(db.clone())
            .await
            .expect("recreate the same address on a new saved row");
        assert_empty(api.confirm(Some(&access), &deleted_token).await, StatusCode::NOT_FOUND).await;
        assert_address(&api.addresses(&access).await, replacement.model(), false);
        assert_eq!(intents(db.as_ref(), owner).await.len(), 5, "no implicit mail");
    }))
    .await;
}

#[tokio::test]
#[serial]
async fn absent_landing_fails_closed_without_proof_or_outbox_event() {
    let (fixture, base, client) = Box::pin(common::setup_test_server_without_profile_landing())
        .await
        .expect("start IAM fixture without the profile landing");
    Box::pin(fixture_cleanup::run(&fixture, async {
        let api = Api { base, client };
        let db = fixture.db();
        let owner = seed_user(&db, "nolandingowner", password_hash()).await;
        let login_address = seed_address(&db, owner, "Login", true, true).await;
        let target = seed_address(&db, owner, "Target", false, false).await;
        let access = platform_session(&api, &login_address).await;
        assert_address(&api.addresses(&access).await, &target, false);

        let outbox_before = outbox_count(db.as_ref()).await;
        assert_empty(
            api.request(Some(&access), target.id).await,
            StatusCode::SERVICE_UNAVAILABLE,
        )
        .await;
        let row = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT EXISTS (
                    SELECT 1 FROM profile_email_verifications
                    WHERE user_id = $1 OR address_id = $2
                 ) AS proof_exists",
                [owner.into(), target.id.into()],
            ))
            .await
            .expect("read persisted proof state")
            .expect("proof lookup result");
        assert!(
            !row.try_get::<bool>("", "proof_exists")
                .expect("proof exists flag"),
            "a missing landing must not persist a pending or confirmed proof"
        );
        assert_eq!(
            outbox_count(db.as_ref()).await,
            outbox_before,
            "a missing landing must not enqueue an outbox event"
        );
        let no_private_intents = intents(db.as_ref(), owner).await.is_empty();
        assert!(
            no_private_intents,
            "no verification delivery intent may be stored"
        );
    }))
    .await;
}

async fn outbox_count(db: &DatabaseConnection) -> i64 {
    db.query_one(Statement::from_string(
        DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS count FROM rustycog_outbox_events".to_owned(),
    ))
    .await
    .expect("read outbox count")
    .expect("outbox count result")
    .try_get("", "count")
    .expect("outbox count")
}
