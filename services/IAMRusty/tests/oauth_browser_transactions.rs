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
    let [_, complete, authorize_v1, token_v1, profile_v1] = idp.endpoint_counts().await;
    assert_eq!(
        [authorize_v1, token_v1, profile_v1],
        [0, 0, 0],
        "IAM must never fall back to legacy endpoints"
    );
    complete
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
        "guard must reject before any connector complete call"
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
            2,
            "both tabs must complete independently"
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
async fn oauth_registration_completion_then_second_login_uses_configured_authority() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
        let idp = IdpConnectFixtures::service().await;
        idp.mock_github_happy_arthur().await;
        let client = browser_client();
        let before = exchange_count(&idp).await;
        let (registration_state, cookie) = start(&client, &base, None).await;
        let registration = callback(&client, &base, "github/callback", &registration_state, Some(&cookie)).await;
        assert!(registration.status() == 202, "initial OAuth registration not accepted");
        let registration: Value = registration.json().await.expect("registration JSON");
        let complete = client.post(format!("{base}/api/auth/complete-registration"))
            .json(&json!({"registration_token": registration["registration_token"], "username": "secondoauthlogin"}))
            .send().await.expect("complete registration");
        assert!(complete.status() == 200, "registration completion failed");
        assert!(exchange_count(&idp).await == before + 1, "registration complete count mismatch");
        let (login_state, same_cookie) = start(&client, &base, Some(&cookie)).await;
        assert!(login_state != registration_state && same_cookie == cookie,
            "second login must use a new transaction and the same browser binding");
        let login = callback(&client, &base, "github/callback", &login_state, Some(&same_cookie)).await;
        assert!(login.status() == 200, "complete OAuth user must log in with configured authority");
        let session: Value = login.json().await.expect("login JSON");
        assert!(session["operation"] == "login", "unexpected OAuth operation");
        let access = session["access_token"].as_str().expect("access token present");
        let refresh = session["refresh_token"].as_str().expect("refresh token present");
        assert!(!access.is_empty() && !refresh.is_empty(), "session tokens absent");
        let config = iam_configuration::load_config_fresh::<iam_configuration::AppConfig>()
            .expect("test authority configuration");
        let jwks = client.get(format!("{base}/.well-known/jwks.json"))
            .send().await.expect("public JWKS request");
        assert!(jwks.status() == 200, "JWKS unavailable");
        let jwks = jwks.text().await.expect("public JWKS");
        let verifier = rustycog::http::UserIdExtractor::from_inline_jwks(&jwks, Some(&config.jwt.audience))
            .expect("configured verifier");
        let principal = verifier.extract_principal(access).await
            .unwrap_or_else(|_| panic!("login access signature or authority refused"));
        assert!(principal.iss == config.jwt.platform_issuer()
            && session["user"]["id"] == principal.sub.to_string(), "login principal mismatch");
        let hash = iam_domain::entity::token::RefreshToken::hash_token(refresh);
        let row = fixture.db().query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS count FROM refresh_tokens WHERE token = $1 AND user_id = $2 AND is_valid",
            [hash.into(), principal.sub.into()],
        )).await.expect("persisted session query").expect("session count row");
        assert!(row.try_get::<i64>("", "count").expect("count") == 1, "login refresh not persisted");
        assert!(exchange_count(&idp).await == before + 2, "second login must complete once");
        let after = exchange_count(&idp).await;
        for consumed in [&registration_state, &login_state] {
            assert_denied_without_exchange(
                callback(&client, &base, "github/callback", consumed, Some(&same_cookie)).await,
                &idp, after,
            ).await;
        }
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
        1,
        "only the committed consumer may complete"
    );
    let hash = iam_domain::entity::oauth_transaction::hash_oauth_bytes(state.as_bytes()).to_vec();
    let row = fixture.db().query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS count FROM oauth_transactions WHERE state_hash=$1 AND consumed_at IS NOT NULL AND pkce_verifier IS NULL AND authentication_nonce IS NULL", [hash.into()]))
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

async fn mock_writer_observed_flow(
    idp: &IdpConnectMockService,
    writer: std::sync::Arc<sea_orm::DatabaseConnection>,
) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    // G1/G2/G8: observe real writer state inside each signed responder,
    // before its HTTP response lets IAM advance to the next phase.
    let context = std::sync::Arc::new(std::sync::Mutex::new(None));
    let begin_context = context.clone();
    let db = writer.clone();
    let runtime = tokio::runtime::Handle::current();
    idp.mock_authorize_observed("github", move |request| {
            let hash = iam_domain::entity::oauth_transaction::hash_oauth_bytes(request.state.as_bytes()).to_vec();
            let row = std::thread::scope(|scope| scope.spawn(|| runtime.block_on(db.query_one(
                Statement::from_sql_and_values(DatabaseBackend::Postgres,
                    "SELECT id,authentication_nonce,pkce_verifier,redirect_uri,consumed_at IS NULL AS pending FROM oauth_transactions WHERE state_hash=$1", [hash.into()])
            ))).join().expect("authorize observer joined"))
                .expect("observe durable BEGIN").expect("persisted before authorize");
            let id: uuid::Uuid = row.try_get("", "id").expect("transaction id");
            let nonce: String = row.try_get("", "authentication_nonce").expect("durable nonce");
            let verifier: String = row.try_get("", "pkce_verifier").expect("durable verifier");
            let redirect: String = row.try_get("", "redirect_uri").expect("durable redirect");
            assert!(row.try_get::<bool>("", "pending").expect("pending"));
            assert!(nonce == request.nonce, "authorize must forward the exact durable nonce");
            assert!(redirect == request.redirect_uri, "authorize redirect binding");
            assert!(request.code_challenge_method.as_deref() == Some("S256"));
            assert!(request.code_challenge.as_deref() == Some(URL_SAFE_NO_PAD.encode(
                iam_domain::entity::oauth_transaction::hash_oauth_bytes(verifier.as_bytes())
            ).as_str()), "authorize must forward the durable S256 challenge");
            *begin_context.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some((id, nonce, verifier, redirect));
        }).await;
    let complete_context = context;
    let db = writer;
    let runtime = tokio::runtime::Handle::current();
    let observed_complete = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let completed = observed_complete.clone();
    idp.mock_complete_with("github", move |request| {
            let context = complete_context.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let (id, nonce, verifier, redirect) = context.as_ref().expect("authorize observed first");
            assert!(request.expected_nonce == *nonce, "complete must use the original writer nonce");
            assert!(request.code_verifier.as_deref() == Some(verifier.as_str()), "complete must use the original verifier");
            assert!(request.redirect_uri == *redirect && request.code == "test_auth_code", "complete context binding");
            let row = std::thread::scope(|scope| scope.spawn(|| runtime.block_on(db.query_one(
                Statement::from_sql_and_values(DatabaseBackend::Postgres,
                    "SELECT consumed_at IS NOT NULL AND authentication_nonce IS NULL AND pkce_verifier IS NULL AS purged FROM oauth_transactions WHERE id=$1", [(*id).into()])
            ))).join().expect("complete observer joined"))
                .expect("observe committed consumption").expect("consumed row");
            assert!(row.try_get::<bool>("", "purged").expect("double purge"), "commit and double purge must be visible before complete responds");
            completed.store(true, std::sync::atomic::Ordering::SeqCst);
            wiremock::ResponseTemplate::new(200).set_body_json(idp_connect_contract::dto::ProviderAuthentication {
                tokens: fixtures::idp_connect::resources::success_tokens(),
                profile: fixtures::idp_connect::resources::github_arthur(),
            })
        }).await;
    observed_complete
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn pkce_s256_survives_connector_authorize_and_complete() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let (fixture, base, _) = Box::pin(common::setup_test_server_with_pkce())
        .await
        .expect("PKCE-enabled IAM harness");
    fixture_cleanup::run(&fixture, async {
        let idp = IdpConnectFixtures::service().await;
        let observed_complete = mock_writer_observed_flow(&idp, fixture.db()).await;
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
            .find(|request| request.url.path().ends_with("/v2/complete"))
            .expect("complete collaborator call");
        let authorize: Value = serde_json::from_slice(&authorize.body).expect("authorize DTO");
        let exchange: Value = serde_json::from_slice(&exchange.body).expect("complete DTO");
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
            authorize.get("code_verifier").is_none(),
            "START must not expose the verifier"
        );
        assert!(authorize["nonce"]
            .as_str()
            .is_some_and(|nonce| nonce.len() == 43));
        assert!(
            authorize["nonce"] == exchange["expected_nonce"],
            "complete must use the persisted START nonce"
        );
        assert_eq!(exchange_count(&idp).await, 1);
        assert!(
            observed_complete.load(std::sync::atomic::Ordering::SeqCst),
            "complete responder must observe committed double purge"
        );
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
    let original = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, authentication_nonce, pkce_verifier FROM oauth_transactions WHERE state_hash=$1",
            [hash.into()],
        ))
        .await
        .expect("actual transaction")
        .expect("BEGIN row");
    let id: uuid::Uuid = original.try_get("", "id").expect("ID");
    let nonce: String = original.try_get("", "authentication_nonce").expect("original durable nonce");
    let verifier: Option<String> = original.try_get("", "pkce_verifier").expect("original verifier");
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
            "SELECT consumed_at IS NULL AS untouched, authentication_nonce=$2 AND pkce_verifier IS NOT DISTINCT FROM $3 AS secrets_retained FROM oauth_transactions WHERE id=$1",
            [id.into(), nonce.into(), verifier.into()],
        ))
        .await
        .expect("rollback consume")
        .expect("BEGIN row retained");
    assert!(row.try_get::<bool>("", "untouched").expect("untouched"));
    assert!(row.try_get::<bool>("", "secrets_retained").expect("original nonce and verifier retained after rollback"));
    assert_eq!(
        callback(&client, &base, "github/callback", &state, Some(&cookie))
            .await
            .status(),
        202,
        "storage failure must rollback and permit a later committed consume"
    );
    }).await;
}

// G5/G6/G7: compare durable business rows, not only counts. Never print this
// snapshot on assertion failure: it includes fixture token values.
async fn business_snapshot(db: &sea_orm::DatabaseConnection) -> String {
    db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object(
            'users',(SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY t.id),'[]'::jsonb) FROM users t),
            'emails',(SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY t.id),'[]'::jsonb) FROM user_emails t),
            'providers',(SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY t.id),'[]'::jsonb) FROM provider_tokens t),
            'sessions',(SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY t.id),'[]'::jsonb) FROM refresh_tokens t)
        )::text AS snapshot".to_owned()))
        .await.expect("business snapshot").expect("snapshot row")
        .try_get("", "snapshot").expect("snapshot JSON")
}

async fn assert_consumed_and_purged(db: &sea_orm::DatabaseConnection, state: &str) {
    let hash = iam_domain::entity::oauth_transaction::hash_oauth_bytes(state.as_bytes()).to_vec();
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT consumed_at IS NOT NULL AND authentication_nonce IS NULL AND pkce_verifier IS NULL AS purged FROM oauth_transactions WHERE state_hash=$1", [hash.into()]))
        .await.expect("consumption snapshot").expect("transaction retained");
    assert!(
        row.try_get::<bool>("", "purged").expect("double purge"),
        "refusal must leave the transaction consumed and both secrets purged"
    );
}

async fn mock_refusal_after_committed_consume(
    idp: &IdpConnectMockService,
    writer: std::sync::Arc<sea_orm::DatabaseConnection>,
    state: &str,
    incomplete: bool,
) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    let hash = iam_domain::entity::oauth_transaction::hash_oauth_bytes(state.as_bytes()).to_vec();
    let original = writer.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT id,authentication_nonce,pkce_verifier,redirect_uri FROM oauth_transactions WHERE state_hash=$1 AND consumed_at IS NULL", [hash.into()]))
        .await.expect("original BEGIN context").expect("pending transaction");
    let id: uuid::Uuid = original.try_get("", "id").expect("transaction id");
    let nonce: String = original
        .try_get("", "authentication_nonce")
        .expect("original nonce");
    let verifier: Option<String> = original
        .try_get("", "pkce_verifier")
        .expect("original verifier");
    let redirect: String = original
        .try_get("", "redirect_uri")
        .expect("original redirect");
    let observed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let receipt = observed.clone();
    let runtime = tokio::runtime::Handle::current();
    idp.mock_complete_with("github", move |request| {
        assert!(request.expected_nonce == nonce && request.code_verifier.as_deref() == verifier.as_deref(), "complete must use the original consumed secrets");
        assert!(request.redirect_uri == redirect && request.code == "test_auth_code", "complete context must stay bound");
        let row = std::thread::scope(|scope| scope.spawn(|| runtime.block_on(writer.query_one(
            Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT consumed_at IS NOT NULL AND authentication_nonce IS NULL AND pkce_verifier IS NULL AS purged FROM oauth_transactions WHERE id=$1", [id.into()])
        ))).join().expect("refusal observer joined"))
            .expect("committed consumption before refusal").expect("transaction retained");
        assert!(row.try_get::<bool>("", "purged").expect("double purge"), "commit and double purge must precede every refusal response");
        receipt.store(true, std::sync::atomic::Ordering::SeqCst);
        if incomplete {
            wiremock::ResponseTemplate::new(200).set_body_json(json!({"tokens": fixtures::idp_connect::resources::success_tokens()}))
        } else {
            wiremock::ResponseTemplate::new(401)
        }
    }).await;
    observed
}

/// G5: a provider refusal or a tokens-only DTO cannot create a partial identity,
/// overwrite existing tokens, mint a session, or make a callback replayable.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn complete_refusal_and_tokens_without_profile_have_no_effect_for_login_link_relink() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        let owner = fixtures::DbFixtures::user().arthur().commit(db.clone()).await.expect("existing owner");
        fixtures::DbFixtures::user_email().arthur_primary(owner.id()).commit(db.clone()).await.expect("owner email");
        fixtures::DbFixtures::provider_token().arthur_github(owner.id()).access_token("old_github_token_123")
            .commit(db.clone()).await.expect("tokens to preserve");
        let link_target = fixtures::DbFixtures::user().bob().commit(db.clone()).await.expect("link target");
        fixtures::DbFixtures::user_email().bob_primary(link_target.id()).commit(db.clone()).await.expect("target email");
        let idp = IdpConnectFixtures::service().await;
        let client = browser_client();
        for incomplete in [false, true] {
            for operation in ["login", "link", "relink"] {
                idp.reset().await;
                let (state, cookie) = match operation {
                    "login" => browser_flow::login(&client, &base, "github", &idp).await,
                    "link" => browser_flow::link(&client, &base, "github", link_target.id(), &idp).await,
                    _ => browser_flow::relink(&client, &base, "github", owner.id(), &idp).await,
                };
                let refused = mock_refusal_after_committed_consume(&idp, db.clone(), &state, incomplete).await;
                let before = business_snapshot(&db).await;
                let route = if operation == "relink" { "github/relink-callback" } else { "github/callback" };
                let response = callback(&client, &base, route, &state, Some(&cookie)).await;
                // G5 fixes refusal + no effects, not a new status/phase mapping.
                // Historical exact-status tests remain unchanged separately.
                assert!(response.status().is_client_error() || response.status().is_server_error(), "complete failure must deny {operation}, not mint credentials");
                let body: Value = response.json().await.expect("generic refusal JSON");
                for credential in ["access_token", "refresh_token", "registration_token", "user"] {
                    assert!(body.get(credential).is_none(), "refusal must not expose {credential}");
                }
                assert!(business_snapshot(&db).await == before, "complete refusal mutated business rows for {operation}");
                assert!(refused.load(std::sync::atomic::Ordering::SeqCst), "the intended refusal must be reached after HMAC/DTO validation, not a fixture prerequisite failure");
                assert_consumed_and_purged(&db, &state).await;
                assert_eq!(idp.endpoint_counts().await, [1, 1, 0, 0, 0]);
                assert_denied_without_exchange(callback(&client, &base, route, &state, Some(&cookie)).await, &idp, 1).await;
                assert!(business_snapshot(&db).await == before, "replay mutated business rows for {operation}");
            }
        }
    }).await;
}

/// G6/G7: no-email is allowed for LINK/RELINK, but even an already linked
/// identity cannot LOGIN without an email. RELINK may replace the opaque ID.
#[tokio::test]
#[serial]
async fn no_email_allows_link_and_relink_but_denies_login_for_the_linked_identity() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        let owner = fixtures::DbFixtures::user().arthur().commit(db.clone()).await.expect("owner");
        let email = fixtures::DbFixtures::user_email().arthur_primary(owner.id()).commit(db.clone()).await.expect("primary email");
        let idp = IdpConnectFixtures::service().await;
        let client = browser_client();
        let mut profile = fixtures::idp_connect::resources::github_arthur();
        profile.email = None;
        for operation in ["link", "relink"] {
            idp.reset().await;
            let mut tokens = fixtures::idp_connect::resources::success_tokens();
            if operation == "relink" {
                // §5: do not add equality-of-old/new-sub policy to RELINK.
                profile.id = "00012345".to_owned();
                tokens.access_token = "new_github_relink_token_123".to_owned();
            }
            idp.mock_complete_success("github", tokens.clone(), profile.clone()).await;
            let (state, cookie) = if operation == "link" {
                browser_flow::link(&client, &base, "github", owner.id(), &idp).await
            } else {
                browser_flow::relink(&client, &base, "github", owner.id(), &idp).await
            };
            let route = if operation == "link" { "github/callback" } else { "github/relink-callback" };
            let result = callback(&client, &base, route, &state, Some(&cookie)).await;
            assert_eq!(result.status(), 200, "{operation} without provider email remains permitted");
            let result: Value = result.json().await.expect("link result");
            assert_eq!(result["user"]["id"], owner.id().to_string());
            assert_eq!(result["new_email_added"], false);
            assert!(owner.check(db.clone()).await.expect("owner unchanged"));
            assert!(email.check(db.clone()).await.expect("primary email preserved"));
            let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT provider_user_id=$2 AND access_token=$3 AS expected FROM provider_tokens WHERE user_id=$1 AND provider='github'",
                [owner.id().into(), profile.id.clone().into(), tokens.access_token.into()]))
                .await.expect("link contents").expect("persisted link");
            assert!(row.try_get::<bool>("", "expected").expect("exact link"), "opaque ID and tokens must be preserved exactly");
            assert_consumed_and_purged(&db, &state).await;
            assert_eq!(idp.endpoint_counts().await, [1, 1, 0, 0, 0]);
        }
        idp.reset().await;
        let authentication = idp_connect_contract::dto::ProviderAuthentication {
            tokens: fixtures::idp_connect::resources::success_tokens(), profile,
        };
        let delivered = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let receipt = delivered.clone();
        idp.mock_complete_with("github", move |_| {
            receipt.store(true, std::sync::atomic::Ordering::SeqCst);
            wiremock::ResponseTemplate::new(200).set_body_json(&authentication)
        }).await;
        let (state, cookie) = browser_flow::login(&client, &base, "github", &idp).await;
        let before = business_snapshot(&db).await;
        let result = callback(&client, &base, "github/callback", &state, Some(&cookie)).await;
        assert!(delivered.load(std::sync::atomic::Ordering::SeqCst), "valid complete with email=None must actually reach LOGIN");
        // G6 requires refusal; it does not assign a new browser error category.
        assert!(result.status().is_client_error() || result.status().is_server_error(), "LOGIN must require email even for an already linked identity");
        let body: Value = result.json().await.expect("LOGIN denial");
        for credential in ["access_token", "refresh_token", "registration_token"] {
            assert!(body.get(credential).is_none(), "no-email LOGIN must not mint credentials");
        }
        assert!(business_snapshot(&db).await == before, "no-email LOGIN must preserve tokens, emails, users and sessions");
        assert_consumed_and_purged(&db, &state).await;
        assert_denied_without_exchange(callback(&client, &base, "github/callback", &state, Some(&cookie)).await, &idp, 1).await;
    }).await;
}

/// G2/G3, contract §4 Q2: valid browser bindings do not rescue legacy/corrupt
/// nonce rows. Consume and purge first, then deny without calling Connect.
#[tokio::test]
#[serial]
async fn legacy_or_corrupt_nonce_is_consumed_and_double_purged_without_complete_or_replay() {
    let (fixture, base, _) = Box::pin(common::setup_test_server_with_pkce())
        .await
        .expect("PKCE IAM harness");
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        let idp = IdpConnectFixtures::service().await;
        let client = browser_client();
        for nonce in [None, Some(""), Some("not-a-canonical-authentication-nonce")] {
            idp.reset().await;
            idp.mock_github_happy_arthur().await;
            let (state, cookie) = start(&client, &base, None).await;
            let hash = iam_domain::entity::oauth_transaction::hash_oauth_bytes(state.as_bytes()).to_vec();
            let changed = db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "UPDATE oauth_transactions SET authentication_nonce=$2 WHERE state_hash=$1 AND consumed_at IS NULL AND pkce_verifier IS NOT NULL",
                [hash.into(), nonce.map(str::to_owned).into()])).await.expect("legacy/corrupt row arrangement");
            assert_eq!(changed.rows_affected(), 1, "arrange exactly the persisted PKCE transaction");
            let before = business_snapshot(&db).await;
            assert_denied_without_exchange(callback(&client, &base, "github/callback", &state, Some(&cookie)).await, &idp, 0).await;
            assert_consumed_and_purged(&db, &state).await;
            assert_denied_without_exchange(callback(&client, &base, "github/callback", &state, Some(&cookie)).await, &idp, 0).await;
            assert_eq!(idp.endpoint_counts().await, [1, 0, 0, 0, 0]);
            assert!(business_snapshot(&db).await == before, "legacy refusal must not mutate accounts, tokens, emails or sessions");
        }
    }).await;
}

/// G6: the provider+opaque-ID link has priority over another user's email,
/// even when the provider now marks that email unverified.
#[tokio::test]
#[serial]
async fn linked_identity_keeps_its_uuid_when_provider_email_changes_to_another_accounts_email() {
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        let owner = fixtures::DbFixtures::user().arthur().commit(db.clone()).await.expect("linked owner");
        let owner_email = fixtures::DbFixtures::user_email().arthur_primary(owner.id()).commit(db.clone()).await.expect("owner email");
        let other = fixtures::DbFixtures::user().bob().commit(db.clone()).await.expect("other account");
        let other_email = fixtures::DbFixtures::user_email().bob_primary(other.id()).commit(db.clone()).await.expect("other email");
        fixtures::DbFixtures::provider_token().arthur_github(owner.id()).provider_user_id("12345")
            .commit(db.clone()).await.expect("exact linked external ID");
        let idp = IdpConnectFixtures::service().await;
        let client = browser_client();
        for verified in [true, false] {
            idp.reset().await;
            let mut profile = fixtures::idp_connect::resources::github_arthur();
            profile.email = Some("bob@example.com".to_owned());
            profile.email_verified = verified;
            idp.mock_complete_success("github", fixtures::idp_connect::resources::success_tokens(), profile).await;
            let (state, cookie) = browser_flow::login(&client, &base, "github", &idp).await;
            let result = callback(&client, &base, "github/callback", &state, Some(&cookie)).await;
            assert_eq!(result.status(), 200, "an existing identity must not be rematched by email");
            let result: Value = result.json().await.expect("LOGIN result");
            assert_eq!(result["user"]["id"], owner.id().to_string());
            assert!(owner_email.check(db.clone()).await.expect("owner email unchanged"));
            assert!(other_email.check(db.clone()).await.expect("other email unchanged"));
            assert!(other.check(db.clone()).await.expect("other account unchanged"));
            let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT COUNT(*) AS count FROM provider_tokens WHERE provider='github' AND provider_user_id='12345' AND user_id=$1",
                [owner.id().into()])).await.expect("stable identity link").expect("count row");
            assert_eq!(row.try_get::<i64>("", "count").expect("owner link"), 1);
            assert_eq!(idp.endpoint_counts().await, [1, 1, 0, 0, 0]);
        }
    }).await;
}

/// G7/G8: opaque subject IDs belong to the configured provider namespace;
/// replicas share the writer, not an in-memory identity mapping.
#[tokio::test]
#[serial]
async fn identical_opaque_ids_in_two_provider_instances_stay_distinct_and_replica_login_is_stable()
{
    let (fixture, base, _) = Box::pin(setup_test_server()).await.expect("IAM harness");
    fixture_cleanup::run(
        &fixture,
        Box::pin(async {
            let (replica, _) = Box::pin(common::setup_test_replica(&fixture))
                .await
                .expect("second IAM root");
            assert_ne!(
                base, replica,
                "identity stability must use distinct listeners"
            );
            let db = fixture.db();
            let arthur = fixtures::DbFixtures::user()
                .arthur()
                .commit(db.clone())
                .await
                .expect("Arthur");
            fixtures::DbFixtures::user_email()
                .arthur_primary(arthur.id())
                .commit(db.clone())
                .await
                .expect("Arthur email");
            let bob = fixtures::DbFixtures::user()
                .bob()
                .commit(db.clone())
                .await
                .expect("Bob");
            fixtures::DbFixtures::user_email()
                .bob_primary(bob.id())
                .commit(db.clone())
                .await
                .expect("Bob email");
            let idp = IdpConnectFixtures::service().await;
            let mut github = fixtures::idp_connect::resources::github_arthur();
            github.id = "00012345".to_owned();
            let mut gitlab = fixtures::idp_connect::resources::gitlab_alice();
            gitlab.id = "00012345".to_owned();
            gitlab.email = Some("bob@example.com".to_owned());
            idp.mock_complete_success(
                "github",
                fixtures::idp_connect::resources::success_tokens(),
                github,
            )
            .await;
            idp.mock_complete_success(
                "gitlab",
                fixtures::idp_connect::resources::success_tokens(),
                gitlab,
            )
            .await;
            let client = browser_client();
            for (listener, provider, expected_user) in [
                (&base, "github", arthur.id()),
                (&base, "gitlab", bob.id()),
                (&replica, "github", arthur.id()),
            ] {
                let (state, cookie) = browser_flow::login(&client, listener, provider, &idp).await;
                let response = callback(
                    &client,
                    listener,
                    &format!("{provider}/callback"),
                    &state,
                    Some(&cookie),
                )
                .await;
                assert_eq!(response.status(), 200);
                let response: Value = response.json().await.expect("instance LOGIN result");
                assert_eq!(
                    response["user"]["id"],
                    expected_user.to_string(),
                    "provider namespace must retain its IAM UUID"
                );
                assert_consumed_and_purged(&db, &state).await;
            }
            let row = db
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT COUNT(*) FILTER (WHERE provider='github' AND user_id=$1) AS github,
                    COUNT(*) FILTER (WHERE provider='gitlab' AND user_id=$2) AS gitlab,
                    COUNT(*) AS total FROM provider_tokens WHERE provider_user_id='00012345'",
                    [arthur.id().into(), bob.id().into()],
                ))
                .await
                .expect("opaque instance links")
                .expect("count row");
            assert_eq!(row.try_get::<i64>("", "github").expect("GitHub link"), 1);
            assert_eq!(row.try_get::<i64>("", "gitlab").expect("GitLab link"), 1);
            assert_eq!(row.try_get::<i64>("", "total").expect("distinct links"), 2);
            assert_eq!(idp.endpoint_counts().await, [3, 3, 0, 0, 0]);
        }),
    )
    .await;
}
