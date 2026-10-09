//! S6/C1 real IAM HTTP + PostgreSQL harness, outbound connector only is mocked.
//! Requires integration to expose fixture request inspection and a second app
//! sharing the first fixture's writer DB (not a cached first server).
#[path = "support/browser_flow.rs"]
mod browser_flow;
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;

use common::setup_test_server;
use fixtures::{idp_connect::service::IdpConnectMockService, IdpConnectFixtures};
use reqwest::{Client, Response};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{json, Value};
use serial_test::serial;
use url::Url;

fn browser_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("browser client")
}

async fn start(client: &Client, base: &str, cookie: Option<&str>) -> (String, String) {
    let mut request = client.get(format!("{base}/api/auth/github/login"));
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let response = request.send().await.expect("OAuth BEGIN");
    assert_eq!(response.status(), 303);
    let cookie = response
        .headers()
        .get("set-cookie")
        .expect("browser binding cookie")
        .to_str()
        .expect("cookie header")
        .to_owned();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    let authorization = Url::parse(
        response
            .headers()
            .get("location")
            .expect("authorize location")
            .to_str()
            .expect("location header"),
    )
    .expect("authorize URL");
    let state = authorization
        .query_pairs()
        .find(|(k, _)| k == "state")
        .expect("opaque state")
        .1
        .into_owned();
    assert!(!state.is_empty());
    (
        state,
        cookie.split(';').next().expect("cookie pair").to_owned(),
    )
}

async fn callback(
    client: &Client,
    base: &str,
    route: &str,
    state: &str,
    cookie: Option<&str>,
) -> Response {
    let mut request = client
        .get(format!("{base}/api/auth/{route}"))
        .query(&[("code", "test_auth_code"), ("state", state)]);
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    request
        .send()
        .await
        .unwrap_or_else(|_| panic!("OAuth callback transport failed (URL redacted)"))
}

async fn exchange_count(idp: &IdpConnectMockService) -> usize {
    idp.received_requests()
        .await
        .iter()
        .filter(|r| r.url.path().ends_with("/token") || r.url.path().ends_with("/profile"))
        .count()
}

async fn assert_denied_without_exchange(
    response: Response,
    idp: &IdpConnectMockService,
    before: usize,
) {
    assert_eq!(
        response.status(),
        400,
        "invalid browser transaction must be rejected by IAM"
    );
    let error: Value = response.json().await.expect("IAM error JSON");
    assert_eq!(error["error"]["error_code"], "invalid_state");
    assert!(error.get("access_token").is_none());
    assert!(error.get("refresh_token").is_none());
    assert_eq!(
        exchange_count(idp).await,
        before,
        "guard must reject before any connector exchange/profile call"
    );
}

#[tokio::test]
#[serial]
async fn browser_binding_provider_intention_expiry_and_replay_fail_before_connector() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;
    let client = browser_client();
    let (state, cookie) = start(&client, &base, None).await;
    let (_, other_cookie) = start(&client, &base, None).await;
    assert!(
        cookie != other_cookie,
        "different browsers must have different nonce bindings"
    );
    for (route, supplied_cookie) in [
        ("github/callback", None),
        ("github/callback", Some(other_cookie.as_str())),
        ("gitlab/callback", Some(cookie.as_str())),
        ("github/relink-callback", Some(cookie.as_str())),
    ] {
        let before = exchange_count(&idp).await;
        assert_denied_without_exchange(
            callback(&client, &base, route, &state, supplied_cookie).await,
            &idp,
            before,
        )
        .await;
    }
    let valid = callback(&client, &base, "github/callback", &state, Some(&cookie)).await;
    assert_eq!(
        valid.status(),
        202,
        "invalid attempts must not consume a valid transaction"
    );
    let before = exchange_count(&idp).await;
    assert_denied_without_exchange(
        callback(&client, &base, "github/callback", &state, Some(&cookie)).await,
        &idp,
        before,
    )
    .await;
    let (expired, expired_cookie) = start(&client, &base, Some(&cookie)).await;
    // Expire the real persisted BEGIN row; don't fabricate a state or transaction.
    fixture.db().execute(Statement::from_string(DatabaseBackend::Postgres,
        "UPDATE oauth_transactions SET expires_at = NOW() - INTERVAL '1 second' WHERE consumed_at IS NULL".to_owned()))
        .await.expect("expire transaction");
    let before = exchange_count(&idp).await;
    assert_denied_without_exchange(
        callback(
            &client,
            &base,
            "github/callback",
            &expired,
            Some(&expired_cookie),
        )
        .await,
        &idp,
        before,
    )
    .await;
    }).await;
}

#[tokio::test]
#[serial]
async fn two_tabs_share_browser_nonce_but_have_independent_consumable_transactions() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
        let idp = IdpConnectFixtures::service().await;
        idp.mock_github_happy_arthur().await;
        let client = browser_client();
        let (one, cookie) = start(&client, &base, None).await;
        let (two, same_cookie) = start(&client, &base, Some(&cookie)).await;
        assert!(
            cookie == same_cookie,
            "tabs must share the same browser binding"
        );
        assert_ne!(one, two);
        for state in [&one, &two] {
            assert_eq!(
                callback(&client, &base, "github/callback", state, Some(&cookie))
                    .await
                    .status(),
                202
            );
        }
        assert_eq!(
            exchange_count(&idp).await,
            4,
            "both tabs must exchange and read a profile independently"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn oauth_registration_returns_persisted_refresh_without_fixture_token_insertion() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;
    let client = browser_client();
    let (state, cookie) = start(&client, &base, None).await;
    let response = callback(&client, &base, "github/callback", &state, Some(&cookie)).await;
    assert_eq!(response.status(), 202);
    let registration: Value = response.json().await.expect("OAuth registration JSON");
    let complete = client.post(format!("{base}/api/auth/complete-registration"))
        .json(&json!({"registration_token": registration["registration_token"], "username": "browseroauthuser"}))
        .send().await.expect("complete OAuth registration");
    assert_eq!(complete.status(), 200);
    let tokens: Value = complete.json().await.expect("session JSON");
    let refresh = tokens["refresh_token"].as_str().expect("OAuth refresh");
    let hash = iam_domain::entity::token::RefreshToken::hash_token(refresh);
    let row = fixture
        .db()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS count FROM refresh_tokens WHERE token = $1 AND is_valid",
            [hash.into()],
        ))
        .await
        .expect("read issued session")
        .expect("count row");
    assert_eq!(row.try_get::<i64>("", "count").expect("count"), 1);
    assert_eq!(
        client
            .post(format!("{base}/api/token/refresh"))
            .json(&json!({"refresh_token": refresh}))
            .send()
            .await
            .expect("refresh OAuth-issued session")
            .status(),
        200
    );
    }).await;
}

#[tokio::test]
#[serial]
async fn replica_race_consumes_the_browser_transaction_once() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, Box::pin(async {
    // Integration API must construct a distinct app/listener against this writer DB;
    // returning the harness singleton URL would invalidate the replica proof.
    let (replica, _) = Box::pin(common::setup_test_replica(&fixture))
        .await
        .expect("second IAM replica");
    assert_ne!(base, replica, "replica must have a distinct listener");
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;
    let client = browser_client();
    let (state, cookie) = start(&client, &base, None).await;
    let (one, two) = tokio::join!(
        callback(&client, &base, "github/callback", &state, Some(&cookie)),
        callback(&client, &replica, "github/callback", &state, Some(&cookie))
    );
    let mut statuses = [one.status().as_u16(), two.status().as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [202, 400]);
    assert_eq!(
        exchange_count(&idp).await,
        2,
        "only the committed consumer may exchange/read profile"
    );
    let row = fixture.db().query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS count FROM oauth_transactions WHERE consumed_at IS NOT NULL AND pkce_verifier IS NULL".to_owned()))
        .await.expect("read committed consumption").expect("count row");
    assert_eq!(row.try_get::<i64>("", "count").expect("count"), 1);
    })).await;
}

#[tokio::test]
#[serial]
async fn relink_start_requires_platform_auth_but_bound_callback_needs_no_bearer() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
    let client = browser_client();
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_bob().await;
    let signup = client
        .post(format!("{base}/api/auth/signup"))
        .json(&json!({"email": "browser-relink@example.com", "password": "BrowserRelink1a!"}))
        .send()
        .await
        .expect("signup");
    assert_eq!(signup.status(), 202);
    let signup: Value = signup.json().await.expect("signup JSON");
    // The account contract issues access/refresh only after email verification.
    // This test targets browser binding, not delivery of the verification email.
    let verified = fixture.db().execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE user_emails SET is_verified=true WHERE email=$1",
        ["browser-relink@example.com".into()],
    )).await.expect("verified platform-account fixture");
    assert_eq!(verified.rows_affected(), 1);
    let complete = client.post(format!("{base}/api/auth/complete-registration"))
        .json(&json!({"registration_token": signup["registration_token"], "username": "browserrelink"}))
        .send().await.expect("complete");
    assert_eq!(complete.status(), 200);
    let complete: Value = complete.json().await.expect("account JSON");
    let user: uuid::Uuid = complete["user"]["id"]
        .as_str()
        .expect("user id")
        .parse()
        .expect("UUID");
    let token = complete["access_token"]
        .as_str()
        .expect("platform access token");
    assert!(!token.is_empty(), "verified account must receive an access token");
    let before = idp.received_requests().await.len();
    assert_eq!(
        client
            .get(format!("{base}/api/auth/github/relink-start"))
            .send()
            .await
            .expect("unauthorized START")
            .status(),
        401
    );
    assert_eq!(
        idp.received_requests().await.len(),
        before,
        "START must reject before authorize call"
    );
    let link = client
        .get(format!("{base}/api/auth/github/link"))
        .bearer_auth(token)
        .send()
        .await
        .expect("authorized Link START");
    assert_eq!(link.status(), 303);
    let link_cookie = link
        .headers()
        .get("set-cookie")
        .expect("Link cookie")
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();
    let link_url = Url::parse(
        link.headers()
            .get("location")
            .expect("Link redirect")
            .to_str()
            .expect("redirect header"),
    )
    .expect("Link authorize URL");
    let link_state = link_url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .expect("Link state")
        .1
        .into_owned();
    assert_eq!(
        callback(
            &client,
            &base,
            "github/callback",
            &link_state,
            Some(&link_cookie)
        )
        .await
        .status(),
        200
    );
    let begun = client
        .get(format!("{base}/api/auth/github/relink-start"))
        .bearer_auth(token)
        .send()
        .await
        .expect("authorized START");
    assert_eq!(begun.status(), 200);
    let cookie = begun
        .headers()
        .get("set-cookie")
        .expect("browser cookie")
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();
    let begun: Value = begun.json().await.expect("START JSON");
    let authorization = Url::parse(begun["auth_url"].as_str().expect("auth URL")).expect("URL");
    let state = authorization
        .query_pairs()
        .find(|(k, _)| k == "state")
        .expect("state")
        .1
        .into_owned();
    let before = exchange_count(&idp).await;
    assert_denied_without_exchange(
        callback(&client, &base, "github/callback", &state, Some(&cookie)).await,
        &idp,
        before,
    )
    .await;
    // Deliberately NO Authorization header on the browser return.
    let completed = callback(
        &client,
        &base,
        "github/relink-callback",
        &state,
        Some(&cookie),
    )
    .await;
    assert_eq!(completed.status(), 200);
    let linked = fixture.db().query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS count FROM provider_tokens WHERE user_id = $1 AND provider = 'github'", [user.into()]))
        .await.expect("read canonical target link").expect("count row");
    assert_eq!(linked.try_get::<i64>("", "count").expect("count"), 1);
    let before = exchange_count(&idp).await;
    assert_denied_without_exchange(
        callback(
            &client,
            &base,
            "github/relink-callback",
            &state,
            Some(&cookie),
        )
        .await,
        &idp,
        before,
    )
    .await;
    }).await;
}

#[tokio::test]
#[serial]
async fn pkce_s256_survives_connector_authorize_and_token_exchange() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    // Integration configures sender AND mock receiver capability explicitly.
    let (fixture, base, _) = Box::pin(common::setup_test_server_with_pkce())
        .await
        .expect("PKCE-enabled IAM harness");
    fixture_cleanup::run(&fixture, async {
        let idp = IdpConnectFixtures::service().await;
        idp.mock_github_happy_arthur().await;
        let client = browser_client();
        let begin = client
            .get(format!("{base}/api/auth/github/login"))
            .send()
            .await
            .expect("PKCE BEGIN");
        assert_eq!(begin.status(), 303);
        let cookie = begin
            .headers()
            .get("set-cookie")
            .expect("browser cookie")
            .to_str()
            .expect("cookie header")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_owned();
        let browser_url = Url::parse(
            begin
                .headers()
                .get("location")
                .expect("browser authorize URL")
                .to_str()
                .expect("location header"),
        )
        .expect("URL");
        let browser_params: std::collections::HashMap<_, _> =
            browser_url.query_pairs().into_owned().collect();
        let state = browser_params.get("state").expect("opaque state");
        assert_eq!(
            browser_params
                .get("code_challenge_method")
                .map(String::as_str),
            Some("S256"),
            "receiver must not drop PKCE from the browser authorization URL"
        );
        assert_eq!(
            callback(&client, &base, "github/callback", state, Some(&cookie))
                .await
                .status(),
            202
        );
        let requests = idp.received_requests().await;
        let authorize = requests
            .iter()
            .find(|request| request.url.path().ends_with("/authorize"))
            .expect("authorize collaborator call");
        let exchange = requests
            .iter()
            .find(|request| request.url.path().ends_with("/token"))
            .expect("token collaborator call");
        let authorize: Value = serde_json::from_slice(&authorize.body).expect("authorize DTO");
        let exchange: Value = serde_json::from_slice(&exchange.body).expect("token DTO");
        let verifier = exchange["code_verifier"]
            .as_str()
            .expect("PKCE verifier must not be silently ignored");
        assert!((43..=128).contains(&verifier.len()));
        assert_eq!(authorize["code_challenge_method"], "S256");
        assert_eq!(
            authorize["code_challenge"],
            URL_SAFE_NO_PAD.encode(iam_domain::entity::oauth_transaction::hash_oauth_bytes(
                verifier.as_bytes()
            ))
        );
        assert_eq!(authorize["redirect_uri"], exchange["redirect_uri"]);
        assert!(
            browser_params
                .get("code_challenge")
                .is_some_and(|value| authorize["code_challenge"].as_str() == Some(value.as_str())),
            "browser and connector must carry the same S256 challenge"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn stored_target_and_redirect_mismatch_deny_but_query_cannot_retarget_consumed_capability() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let owner = fixtures::DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("authorized account");
    let attacker = fixtures::DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("other account");
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;
    let client = browser_client();
    let (state, cookie) = browser_flow::link(&client, &base, "github", owner.id(), &idp).await;
    let state_hash =
        iam_domain::entity::oauth_transaction::hash_oauth_bytes(state.as_bytes()).to_vec();
    let original = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT redirect_uri FROM oauth_transactions WHERE state_hash=$1",
            [state_hash.clone().into()],
        ))
        .await
        .expect("actual BEGIN binding")
        .expect("BEGIN row")
        .try_get::<String>("", "redirect_uri")
        .expect("redirect binding");
    let changed = db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE oauth_transactions SET target_user_id=$2 WHERE state_hash=$1",
            [state_hash.clone().into(), attacker.id().into()],
        ))
        .await
        .expect("target mismatch arrangement");
    assert_eq!(changed.rows_affected(), 1);
    let before = exchange_count(&idp).await;
    assert_denied_without_exchange(
        callback(&client, &base, "github/callback", &state, Some(&cookie)).await,
        &idp,
        before,
    )
    .await;
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE oauth_transactions SET target_user_id=$2,redirect_uri='https://attacker.example/callback' WHERE state_hash=$1", [state_hash.clone().into(), owner.id().into()]))
        .await.expect("redirect mismatch arrangement");
    assert_denied_without_exchange(
        callback(&client, &base, "github/callback", &state, Some(&cookie)).await,
        &idp,
        before,
    )
    .await;
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE oauth_transactions SET redirect_uri=$2 WHERE state_hash=$1",
        [state_hash.into(), original.into()],
    ))
    .await
    .expect("restore actual binding");
    let attacker_id = attacker.id().to_string();
    let result = client
        .get(format!("{base}/api/auth/github/callback"))
        .header("cookie", &cookie)
        .query(&[
            ("code", "test_auth_code"),
            ("state", state.as_str()),
            ("user_id", attacker_id.as_str()),
            ("redirect_uri", "https://attacker.example/callback"),
        ])
        .send()
        .await
        .unwrap_or_else(|_| panic!("callback transport failed (URL redacted)"));
    assert_eq!(
        result.status(),
        200,
        "query cannot override the consumed target/configured redirect"
    );
    let result: Value = result.json().await.expect("link response");
    assert_eq!(result["user"]["id"], owner.id().to_string());
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT COUNT(*) FILTER (WHERE user_id=$1) AS owner, COUNT(*) FILTER (WHERE user_id=$2) AS attacker FROM provider_tokens WHERE provider='github'",
        [owner.id().into(), attacker.id().into()])).await.expect("canonical mutation target").expect("count row");
    assert_eq!(row.try_get::<i64>("", "owner").expect("owner links"), 1);
    assert_eq!(
        row.try_get::<i64>("", "attacker").expect("attacker links"),
        0
    );
    }).await;
}

#[tokio::test]
#[serial]
async fn writer_consume_failure_is_unavailable_and_has_no_connector_call_or_consumption() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let idp = IdpConnectFixtures::service().await;
    idp.mock_github_happy_arthur().await;
    let client = browser_client();
    let (state, cookie) = start(&client, &base, None).await;
    let hash = iam_domain::entity::oauth_transaction::hash_oauth_bytes(state.as_bytes()).to_vec();
    let id: uuid::Uuid = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM oauth_transactions WHERE state_hash=$1",
            [hash.into()],
        ))
        .await
        .expect("actual transaction")
        .expect("BEGIN row")
        .try_get("", "id")
        .expect("ID");
    let name = format!("test_consume_{}", uuid::Uuid::new_v4().simple());
    db.execute_unprepared(&format!("CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF OLD.id='{id}' THEN RAISE EXCEPTION 'SentinelABC-fixed-test-storage-payload'; END IF; RETURN NEW; END $$"))
        .await.expect("fixture-owned failure seam");
    let installed = db.execute_unprepared(&format!("CREATE TRIGGER {name} BEFORE UPDATE ON oauth_transactions FOR EACH ROW EXECUTE FUNCTION {name}()")).await;
    if installed.is_err() {
        db.execute_unprepared(&format!("DROP FUNCTION {name}()"))
            .await
            .expect("cleanup unused seam");
        panic!("could not install fixture-owned consume failure seam");
    }
    let before = exchange_count(&idp).await;
    let response = callback(&client, &base, "github/callback", &state, Some(&cookie)).await;
    db.execute_unprepared(&format!(
        "DROP TRIGGER {name} ON oauth_transactions; DROP FUNCTION {name}()"
    ))
    .await
    .expect("remove fixture seam before assertions");
    assert_eq!(response.status(), 503);
    let body = response.text().await.expect("generic error body");
    assert!(!body.contains("SentinelABC"));
    assert!(!body.contains(&state));
    assert!(!body.contains(&cookie));
    let body: Value = serde_json::from_str(&body).expect("generic error JSON");
    assert_eq!(body["error"]["error_code"], "oauth_transaction_unavailable");
    assert_eq!(exchange_count(&idp).await, before);
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT consumed_at IS NULL AS untouched FROM oauth_transactions WHERE id=$1",
            [id.into()],
        ))
        .await
        .expect("rollback consume")
        .expect("BEGIN row retained");
    assert!(row.try_get::<bool>("", "untouched").expect("untouched"));
    assert_eq!(
        callback(&client, &base, "github/callback", &state, Some(&cookie))
            .await
            .status(),
        202,
        "storage failure must rollback and permit a later committed consume"
    );
    }).await;
}
