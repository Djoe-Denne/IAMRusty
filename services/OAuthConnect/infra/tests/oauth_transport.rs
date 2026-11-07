//! OAuth A-1..A-4 / G-3: real adapter, outgoing HTTP collaborators only.
//!
//! Obligations -> risks -> scenarios -> oracles:
//! - Authorize/complete validation -> redirect or PKCE downgrade -> exact versus
//!   near-match callbacks/context -> decoded URL/form and zero vendor I/O on refusal.
//! - Exchange/profile isolation -> replay or wrong credentials/identity -> Basic,
//!   Post, independent APIs and successive instances -> exact per-phase receipts
//!   and complete tokens/profile, never a partial successful result.
//! - Bounded transport -> unbounded decode or silent downgrade -> identity/gzip
//!   small/1MiB/+1, bad encoding/status/redirect/delay -> exact profile or typed
//!   sanitized error plus phase-specific receipts, no retry/destination request.
//!
//! Every test is KEEP. Constructor acceptance is a business assertion: refusal
//! by T0 is expected RED, not a fixture/compile error. Negative method scenarios
//! only become discriminating after that nominal construction succeeds.
//! B/C own the actual setup builder/TLS/secrets. Error-only bounds are not RSS,
//! streaming-without-length, connection-3s, or raw pre-decoder-header proofs.

#[path = "support/oauth_vendor.rs"]
mod support;

use std::time::{Duration, Instant};

use idp_connect_contract::{FederatedOAuthClient, FederatedOAuthError};
use oauth_connect_configuration::TokenAuthMethod;
use serde_json::{json, Value};
use support::{
    assert_post, assert_redacted, authorize_request, body_response, complete_request, failed,
    fields, gzip, header, json_response, must, nested_body, nested_config, nonce, oauth, padded,
    parsed_url, tokens, trusted_client, Flow, ACCESS, CALLBACK, CHALLENGE, CLIENT_ID, CODE,
    EMAIL_PATH, LIMIT, REFRESH, RELINK, SECRET, STATE, TOKEN_PATH, USER_PATH, VERIFIER,
};
use wiremock::ResponseTemplate;

fn near_callbacks() -> [&'static str; 8] {
    [
        "https://other.example.test/callback?flow=login",
        "https://app.example.test:444/callback?flow=login",
        "http://app.example.test/callback?flow=login",
        "https://app.example.test/callback/extra?flow=login",
        "https://app.example.test/Callback?flow=login",
        "https://app.example.test/callback?flow=LOGIN",
        "https://app.example.test/callback?flow=login&extra=1",
        "https://app.example.test/callback?flow=login#fragment",
    ]
}

async fn assert_get_failure_phase(flow: &Flow, user_failed: bool) {
    let token = flow.token.requests().await;
    let user = flow.user.requests().await;
    let emails = flow.emails.requests().await;
    assert_eq!(
        token.len(),
        1,
        "one exchange, no replay after profile failure"
    );
    assert_post(&token[0], &flow.config, SECRET, Some(VERIFIER));
    assert_eq!(
        if user_failed {
            user.len()
        } else {
            emails.len()
        },
        1,
        "failed GET attempted once"
    );
    assert!(
        user.len() <= 1 && emails.len() <= 1,
        "no profile retry/chaining; no ordering imposed"
    );
    assert!(user
        .iter()
        .all(|r| r.method.as_str() == "GET" && r.url.path() == USER_PATH));
    assert!(emails
        .iter()
        .all(|r| r.method.as_str() == "GET" && r.url.path() == EMAIL_PATH));
}

#[tokio::test]
async fn a1_valid_arbitrary_instance_constructs_redacted_without_vendor_io() {
    let flow = Flow::new().await;
    let client = flow.client();
    assert_redacted(&format!("{client:?}"));
    assert_redacted(&format!("{:?}", flow.config));
    // An accepted instance must also perform the nominal operation, not merely
    // construct a value whose Debug happens to be redacted.
    let auth = must(
        client.authorize_authentication(&authorize_request()).await,
        "nominal authorize",
    );
    assert!(parsed_url(&auth.authorization_url).path() == "/authorize/arbitrary");
    flow.counts(0, 0, 0).await;
}

#[tokio::test]
async fn a1_authorize_callback_and_relink_decode_exact_fields_without_nonce_or_verifier() {
    let flow = Flow::new().await;
    let client = flow.client();
    for callback in [CALLBACK, RELINK] {
        let mut request = authorize_request();
        request.redirect_uri = callback.to_owned();
        let response = must(
            client.authorize_authentication(&request).await,
            "allowlisted authorize",
        );
        let url = parsed_url(&response.authorization_url);
        let expected_origin = parsed_url(&flow.token.url("/"));
        assert!(url.origin() == expected_origin.origin() && url.path() == "/authorize/arbitrary");
        let params = fields(url.query().unwrap_or_default().as_bytes());
        let expected = [
            ("prompt", "select account"),
            ("response_type", "code"),
            ("client_id", CLIENT_ID),
            ("state", STATE),
            ("redirect_uri", callback),
            ("scope", "read:user user:email"),
            ("code_challenge", CHALLENGE),
            ("code_challenge_method", "S256"),
        ];
        assert_eq!(
            params.len(),
            expected.len(),
            "no vendor nonce/verifier or implicit scope"
        );
        for (key, value) in expected {
            assert!(
                params.get(key).map(String::as_str) == Some(value),
                "exact decoded authorize field {key}"
            );
        }
        assert!(
            response.scope == "read:user user:email",
            "declared scopes only"
        );
    }
    flow.counts(0, 0, 0).await;
}

#[tokio::test]
async fn a1_authorize_rejects_callback_near_matches_before_vendor_io() {
    let flow = Flow::new().await;
    let client = flow.client();
    for callback in near_callbacks() {
        let mut request = authorize_request();
        request.redirect_uri = callback.to_owned();
        assert!(
            matches!(
                client.authorize_authentication(&request).await,
                Err(FederatedOAuthError::Authorize)
            ),
            "near-match callback must be refused"
        );
    }
    flow.counts(0, 0, 0).await;
}

#[tokio::test]
async fn a1_authorize_rejects_invalid_pair_method_challenge_nonce_and_empty_state() {
    let flow = Flow::new().await;
    let client = flow.client();
    let mut invalid = Vec::new();
    let mut request = authorize_request();
    request.code_challenge = None;
    invalid.push(request);
    let mut request = authorize_request();
    request.code_challenge_method = None;
    invalid.push(request);
    let mut request = authorize_request();
    request.code_challenge_method = Some("plain".to_owned());
    invalid.push(request);
    let mut request = authorize_request();
    request.code_challenge = Some("short".to_owned());
    invalid.push(request);
    let mut request = authorize_request();
    request.code_challenge = None;
    request.code_challenge_method = None;
    invalid.push(request);
    let mut request = authorize_request();
    request.state.clear();
    invalid.push(request);
    for bad_nonce in [
        String::new(),
        "A".repeat(42),
        format!("{}=", nonce()),
        format!("{}B", "A".repeat(42)),
    ] {
        let mut request = authorize_request();
        request.nonce = bad_nonce;
        invalid.push(request);
    }
    for request in invalid {
        assert!(
            matches!(
                client.authorize_authentication(&request).await,
                Err(FederatedOAuthError::Authorize)
            ),
            "invalid v2 authorize context must be refused"
        );
    }
    flow.counts(0, 0, 0).await;
}

#[tokio::test]
async fn a1_pkce_incapable_instance_allows_no_pair_and_refuses_supplied_s256() {
    let mut flow = Flow::new().await;
    flow.config.pkce_supported = false;
    let client = flow.client();
    let mut request = authorize_request();
    request.code_challenge = None;
    request.code_challenge_method = None;
    let response = must(
        client.authorize_authentication(&request).await,
        "optional PKCE authorize",
    );
    let url = parsed_url(&response.authorization_url);
    let params = fields(url.query().unwrap_or_default().as_bytes());
    assert!(
        !params.contains_key("code_challenge") && !params.contains_key("code_challenge_method")
    );
    assert!(params.get("redirect_uri").map(String::as_str) == Some(CALLBACK));
    assert!(
        matches!(
            client.authorize_authentication(&authorize_request()).await,
            Err(FederatedOAuthError::Authorize)
        ),
        "do not advertise an unsupported capability"
    );
    flow.counts(0, 0, 0).await;
}

#[tokio::test]
async fn a1_complete_rejects_bad_callback_nonce_verifier_or_code_before_token_post() {
    let flow = Flow::new().await;
    let client = flow.client();
    let mut invalid = Vec::new();
    for callback in near_callbacks() {
        let mut request = complete_request();
        request.redirect_uri = callback.to_owned();
        invalid.push(request);
    }
    let mut request = complete_request();
    request.code.clear();
    invalid.push(request);
    let mut request = complete_request();
    request.code_verifier = None;
    invalid.push(request);
    let mut request = complete_request();
    request.code_verifier = Some("bad+verifier".repeat(5));
    invalid.push(request);
    let mut request = complete_request();
    request.code_verifier = Some("x".repeat(42));
    invalid.push(request);
    for bad_nonce in [String::new(), "A".repeat(42), format!("{}=", nonce())] {
        let mut request = complete_request();
        request.expected_nonce = bad_nonce;
        invalid.push(request);
    }
    for request in invalid {
        failed(client.complete_authorization(&request).await);
    }
    flow.counts(0, 0, 0).await;
}

#[tokio::test]
async fn a2_basic_auth_uses_encoded_credentials_exact_code_redirect_and_consumed_verifier() {
    let mut flow = Flow::new().await;
    flow.config.token_auth_method = TokenAuthMethod::ClientSecretBasic;
    let auth = must(
        flow.client()
            .complete_authorization(&complete_request())
            .await,
        "Basic completion",
    );
    flow.assert_profile(&auth);
    assert!(
        auth.tokens.access_token == ACCESS && auth.tokens.refresh_token.as_deref() == Some(REFRESH)
    );
    assert_eq!(auth.tokens.expires_in, Some(3600));
    let requests = flow.token.requests().await;
    assert_eq!(requests.len(), 1, "one token POST");
    assert_post(&requests[0], &flow.config, SECRET, Some(VERIFIER));
    flow.counts(1, 1, 1).await;
}

#[tokio::test]
async fn a2_post_auth_preserves_secret_spaces_and_encodes_form_without_basic_or_query_leaks() {
    let flow = Flow::new().await;
    let auth = must(
        flow.client()
            .complete_authorization(&complete_request())
            .await,
        "Post completion",
    );
    flow.assert_profile(&auth);
    let requests = flow.token.requests().await;
    assert_eq!(requests.len(), 1);
    assert_post(&requests[0], &flow.config, SECRET, Some(VERIFIER));
    flow.counts(1, 1, 1).await;
    flow.defaults().await;
    let mut relink = complete_request();
    relink.redirect_uri = RELINK.to_owned();
    let auth = must(
        flow.client().complete_authorization(&relink).await,
        "exact relink completion",
    );
    flow.assert_profile(&auth);
    let requests = flow.token.requests().await;
    assert_eq!(requests.len(), 1);
    assert!(
        fields(&requests[0].body)
            .get("redirect_uri")
            .map(String::as_str)
            == Some(RELINK),
        "relink callback reaches the POST unchanged"
    );
    flow.counts(1, 1, 1).await;
}

#[tokio::test]
async fn a2_bearer_tokens_allow_missing_optionals_zero_and_exact_u64_expiry() {
    let flow = Flow::new().await;
    let client = flow.client();
    for (body, expiry, refresh) in [
        (
            json!({"access_token": ACCESS, "token_type": "Bearer"}),
            None,
            None,
        ),
        (
            json!({"access_token": ACCESS, "token_type": "bearer", "expires_in": 0}),
            Some(0),
            None,
        ),
        (
            json!({"access_token": ACCESS, "token_type": "Bearer", "expires_in": u64::MAX,
            "refresh_token": REFRESH}),
            Some(u64::MAX),
            Some(REFRESH),
        ),
    ] {
        flow.defaults().await;
        flow.token.reset().await;
        flow.token
            .mount("POST", TOKEN_PATH, json_response(&body))
            .await;
        let auth = must(
            client.complete_authorization(&complete_request()).await,
            "minimal token DTO completion",
        );
        assert!(
            auth.tokens.access_token == ACCESS && auth.tokens.refresh_token.as_deref() == refresh
        );
        assert_eq!(
            auth.tokens.expires_in, expiry,
            "unsigned expiry preserved exactly"
        );
        flow.assert_profile(&auth);
        flow.counts(1, 1, 1).await;
    }
}

#[tokio::test]
async fn a2_invalid_tokens_types_schema_and_lossy_expiries_stop_before_profile() {
    let flow = Flow::new().await;
    let client = flow.client();
    let mut invalid = vec![
        json!({}),
        json!({"token_type": "Bearer"}),
        json!({"access_token": ACCESS}),
        json!({"access_token": "", "token_type": "Bearer"}),
    ];
    for value in [Value::Null, json!(12), json!(true), json!({})] {
        let mut body = tokens();
        body["access_token"] = value;
        invalid.push(body);
    }
    for value in [
        Value::Null,
        json!(false),
        json!(""),
        json!("MAC"),
        json!("Bearer extra"),
    ] {
        let mut body = tokens();
        body["token_type"] = value;
        invalid.push(body);
    }
    for raw in ["-1", "1.5", "1.0", "\"3600\"", "18446744073709551616"] {
        let mut body = tokens();
        body["expires_in"] = must(serde_json::from_str::<Value>(raw), "expiry fixture");
        invalid.push(body);
    }
    let mut body = tokens();
    body["refresh_token"] = json!(12);
    invalid.push(body);
    for body in invalid {
        flow.defaults().await;
        flow.token.reset().await;
        flow.token
            .mount("POST", TOKEN_PATH, json_response(&body))
            .await;
        failed(client.complete_authorization(&complete_request()).await);
        flow.counts(1, 0, 0).await;
    }
}

#[tokio::test]
async fn a2_token_status_failures_never_retry_or_switch_auth_methods() {
    let mut flow = Flow::new().await;
    for auth_method in [
        TokenAuthMethod::ClientSecretBasic,
        TokenAuthMethod::ClientSecretPost,
    ] {
        flow.config.token_auth_method = auth_method;
        let client = flow.client();
        for status in [401, 403, 405, 429, 500, 502] {
            flow.defaults().await;
            flow.token.reset().await;
            flow.token
                .mount(
                    "POST",
                    TOKEN_PATH,
                    ResponseTemplate::new(status).set_body_json(tokens()),
                )
                .await;
            failed(client.complete_authorization(&complete_request()).await);
            flow.counts(1, 0, 0).await;
            let requests = flow.token.requests().await;
            assert_post(&requests[0], &flow.config, SECRET, Some(VERIFIER));
        }
    }
}

#[tokio::test]
async fn a2_token_html_and_invalid_json_are_not_success_or_profile_fallback() {
    let flow = Flow::new().await;
    let client = flow.client();
    for body in ["<html>token error</html>", "{broken", "null", "[]"] {
        flow.defaults().await;
        flow.token.reset().await;
        flow.token
            .mount(
                "POST",
                TOKEN_PATH,
                body_response(body.as_bytes().to_vec(), None),
            )
            .await;
        failed(client.complete_authorization(&complete_request()).await);
        flow.counts(1, 0, 0).await;
    }
}

#[tokio::test]
async fn a3_independent_apis_use_declared_auth_headers_one_get_each_and_no_picture_fetch() {
    let flow = Flow::new().await;
    let auth = must(
        flow.client()
            .complete_authorization(&complete_request())
            .await,
        "independent API completion",
    );
    flow.assert_profile(&auth);
    flow.counts(1, 1, 1).await;
    let user = flow.user.requests().await;
    let email = flow.emails.requests().await;
    assert!(user[0].url.path() == USER_PATH && email[0].url.path() == EMAIL_PATH);
    assert!(
        user[0].url.query().is_none() && email[0].url.query().is_none(),
        "no token in GET URL"
    );
    assert!(header(&user[0], "authorization") == Some(format!("Bearer {ACCESS}").as_str()));
    assert!(header(&email[0], "authorization") == Some(format!("token {ACCESS}").as_str()));
    assert!(header(&user[0], "accept") == Some("application/json"));
    assert!(header(&user[0], "user-agent") == Some("oauth-a-fixture"));
    assert!(header(&email[0], "x-github-api-version") == Some("2022-11-28"));
    assert!(header(&user[0], "cookie").is_none() && header(&email[0], "cookie").is_none());
}

#[tokio::test]
async fn a3_two_completions_reuse_definitions_not_tokens_or_json_between_operations() {
    let flow = Flow::new().await;
    let client = flow.client();
    let first = must(
        client.complete_authorization(&complete_request()).await,
        "first completion",
    );
    flow.assert_profile(&first);
    flow.counts(1, 1, 1).await;
    flow.defaults().await;
    flow.token.reset().await;
    flow.user.reset().await;
    flow.emails.reset().await;
    flow.token
        .mount(
            "POST",
            TOKEN_PATH,
            json_response(&json!({"access_token": "access-second", "token_type": "Bearer"})),
        )
        .await;
    flow.user
        .mount(
            "GET",
            USER_PATH,
            json_response(&json!({"id": "second", "login": "second-login"})),
        )
        .await;
    flow.emails
        .mount("GET", EMAIL_PATH, json_response(&json!([])))
        .await;
    let second = must(
        client.complete_authorization(&complete_request()).await,
        "second completion",
    );
    assert!(second.tokens.access_token == "access-second" && second.tokens.refresh_token.is_none());
    assert_eq!(second.profile.id, "second");
    assert_eq!(second.profile.username, "second-login");
    assert!(
        second.profile.email.is_none()
            && !second.profile.email_verified
            && second.profile.avatar_url.is_none()
    );
    flow.counts(1, 1, 1).await;
    for requests in [flow.user.requests().await, flow.emails.requests().await] {
        assert!(
            header(&requests[0], "authorization").is_some_and(|v| v.ends_with("access-second")),
            "fresh token only"
        );
    }
}

#[tokio::test]
async fn a3_two_instances_route_their_own_credentials_tokens_and_identities() {
    let first = Flow::new().await;
    let mut second = Flow::new().await;
    second.config.id = "anotherforge".to_owned();
    second.config.client_id = "second-client".to_owned();
    second.config.client_secret = Some("second-secret".to_owned());
    second.token.reset().await;
    second.user.reset().await;
    second
        .token
        .mount(
            "POST",
            TOKEN_PATH,
            json_response(&json!({"access_token": "instance-B-token", "token_type": "Bearer"})),
        )
        .await;
    second
        .user
        .mount(
            "GET",
            USER_PATH,
            json_response(&json!({"id": "instance-B"})),
        )
        .await;
    let a = first.client();
    let b = trusted_client(&second.config, "second-secret");
    let a_auth = must(
        a.complete_authorization(&complete_request()).await,
        "instance A",
    );
    let b_auth = must(
        b.complete_authorization(&complete_request()).await,
        "instance B",
    );
    first.assert_profile(&a_auth);
    assert_eq!(b_auth.profile.id, "instance-B");
    assert!(b_auth.tokens.access_token == "instance-B-token");
    first.counts(1, 1, 1).await;
    second.counts(1, 1, 1).await;
    let a_requests = first.token.requests().await;
    let b_requests = second.token.requests().await;
    assert_post(&a_requests[0], &first.config, SECRET, Some(VERIFIER));
    assert_post(
        &b_requests[0],
        &second.config,
        "second-secret",
        Some(VERIFIER),
    );
    let b_user = second.user.requests().await;
    assert!(header(&b_user[0], "authorization") == Some("Bearer instance-B-token"));
}

#[tokio::test]
async fn a3_no_primary_returns_no_email_even_with_public_and_verified_secondary() {
    let flow = Flow::new().await;
    let client = flow.client();
    for body in [
        json!([]),
        json!([{"email": "public@example.test", "primary": false, "verified": true}]),
    ] {
        flow.defaults().await;
        flow.emails.reset().await;
        flow.emails
            .mount("GET", EMAIL_PATH, json_response(&body))
            .await;
        let auth = must(
            client.complete_authorization(&complete_request()).await,
            "primary absent completion",
        );
        assert!(auth.profile.email.is_none() && !auth.profile.email_verified);
        assert_eq!(auth.profile.id, "42");
        flow.counts(1, 1, 1).await;
    }
}

#[tokio::test]
async fn a3_two_primary_addresses_are_errors_even_when_equal() {
    let flow = Flow::new().await;
    let client = flow.client();
    flow.emails.reset().await;
    flow.emails
        .mount(
            "GET",
            EMAIL_PATH,
            json_response(&json!([
                {"primary": true, "email": "same@example.test", "verified": true},
                {"primary": true, "email": "same@example.test", "verified": true}
            ])),
        )
        .await;
    failed(client.complete_authorization(&complete_request()).await);
    assert_get_failure_phase(&flow, false).await;
}

#[tokio::test]
async fn a3_get_status_failures_are_not_swallowed_as_empty_emails_or_partial_success() {
    let flow = Flow::new().await;
    let client = flow.client();
    for is_user in [true, false] {
        for status in [401, 403, 429, 500, 503] {
            flow.defaults().await;
            let (vendor, route) = if is_user {
                (&flow.user, USER_PATH)
            } else {
                (&flow.emails, EMAIL_PATH)
            };
            vendor.reset().await;
            vendor
                .mount(
                    "GET",
                    route,
                    ResponseTemplate::new(status).set_body_json(json!([])),
                )
                .await;
            failed(client.complete_authorization(&complete_request()).await);
            assert_get_failure_phase(&flow, is_user).await;
        }
    }
}

#[tokio::test]
async fn a3_bad_get_json_html_and_maltyped_primary_are_errors_not_public_fallback() {
    let flow = Flow::new().await;
    let client = flow.client();
    for body in [
        "<html>profile error</html>",
        "[broken",
        "[{\"primary\":true,\"email\":12,\"verified\":true}]",
        "[{\"primary\":true,\"email\":\"primary@example.test\",\"verified\":\"true\"}]",
    ] {
        flow.defaults().await;
        flow.emails.reset().await;
        flow.emails
            .mount(
                "GET",
                EMAIL_PATH,
                body_response(body.as_bytes().to_vec(), None),
            )
            .await;
        failed(client.complete_authorization(&complete_request()).await);
        assert_get_failure_phase(&flow, false).await;
    }
}

#[tokio::test]
async fn a3_http_projection_keeps_nested_absolute_two_and_four_item_profiles_under_budget() {
    let mut flow = Flow::new().await;
    nested_config(&mut flow.config);
    let client = flow.client();
    for count in [2, 4] {
        flow.defaults().await;
        flow.user.reset().await;
        flow.user
            .mount("GET", USER_PATH, json_response(&nested_body(count)))
            .await;
        let auth = must(
            client.complete_authorization(&complete_request()).await,
            "nested HTTP projection under budget",
        );
        assert_eq!(auth.profile.id, "stable");
        assert_eq!(auth.profile.username, "name");
        assert!(
            auth.profile.email.as_deref() == Some("one@example.test")
                && auth.profile.email_verified
        );
        assert!(auth.profile.avatar_url.is_none());
        flow.counts(1, 1, 0).await;
    }
}

#[tokio::test]
async fn a3_http_projection_refuses_nested_32_after_one_successful_get_without_bypassing_budget() {
    let mut flow = Flow::new().await;
    nested_config(&mut flow.config);
    let client = flow.client();
    flow.user.reset().await;
    flow.user
        .mount("GET", USER_PATH, json_response(&nested_body(32)))
        .await;
    failed(client.complete_authorization(&complete_request()).await);
    flow.counts(1, 1, 0).await;
}

#[tokio::test]
async fn a4_token_identity_small_exact_limit_and_plus_one_use_decompressed_byte_bound() {
    let flow = Flow::new().await;
    let client = flow.client();
    for size in [256, LIMIT, LIMIT + 1] {
        flow.defaults().await;
        flow.token.reset().await;
        flow.token
            .mount(
                "POST",
                TOKEN_PATH,
                body_response(padded(&tokens(), size), Some("identity")),
            )
            .await;
        let result = client.complete_authorization(&complete_request()).await;
        if size <= LIMIT {
            flow.assert_profile(&must(result, "identity token at inclusive limit"));
            flow.counts(1, 1, 1).await;
        } else {
            failed(result);
            flow.counts(1, 0, 0).await;
        }
    }
}

#[tokio::test]
async fn a4_get_identity_small_exact_limit_and_plus_one_do_not_confuse_scan_volume() {
    let flow = Flow::new().await;
    let client = flow.client();
    for size in [512, LIMIT, LIMIT + 1] {
        flow.defaults().await;
        flow.user.reset().await;
        flow.user
            .mount(
                "GET",
                USER_PATH,
                body_response(padded(&flow.user_body(), size), Some("identity")),
            )
            .await;
        let result = client.complete_authorization(&complete_request()).await;
        if size <= LIMIT {
            flow.assert_profile(&must(result, "identity GET at inclusive limit"));
            flow.counts(1, 1, 1).await;
        } else {
            failed(result);
            assert_get_failure_phase(&flow, true).await;
        }
    }
}

#[tokio::test]
async fn a4_token_gzip_small_exact_limit_and_plus_one_are_checked_after_decompression() {
    let flow = Flow::new().await;
    let client = flow.client();
    for size in [256, LIMIT, LIMIT + 1] {
        flow.defaults().await;
        flow.token.reset().await;
        let compressed = gzip(&padded(&tokens(), size));
        assert!(
            compressed.len() < size,
            "positive fixture is genuinely compressed"
        );
        flow.token
            .mount("POST", TOKEN_PATH, body_response(compressed, Some("gzip")))
            .await;
        let result = client.complete_authorization(&complete_request()).await;
        if size <= LIMIT {
            flow.assert_profile(&must(result, "gzip token at inclusive decompressed limit"));
            flow.counts(1, 1, 1).await;
        } else {
            failed(result);
            flow.counts(1, 0, 0).await;
        }
    }
}

#[tokio::test]
async fn a4_get_gzip_small_exact_limit_and_plus_one_preserve_the_same_profile_or_fail_closed() {
    let flow = Flow::new().await;
    let client = flow.client();
    for size in [512, LIMIT, LIMIT + 1] {
        flow.defaults().await;
        flow.user.reset().await;
        let compressed = gzip(&padded(&flow.user_body(), size));
        assert!(compressed.len() < size, "positive GET gzip fixture");
        flow.user
            .mount("GET", USER_PATH, body_response(compressed, Some("gzip")))
            .await;
        let result = client.complete_authorization(&complete_request()).await;
        if size <= LIMIT {
            flow.assert_profile(&must(result, "gzip GET at inclusive decompressed limit"));
            flow.counts(1, 1, 1).await;
        } else {
            failed(result);
            assert_get_failure_phase(&flow, true).await;
        }
    }
}

#[tokio::test]
async fn a4_token_invalid_gzip_and_residual_encodings_refuse_valid_json_without_retry() {
    let flow = Flow::new().await;
    let client = flow.client();
    for encoding in ["gzip", "br", "unknown", "", "identity, br"] {
        flow.defaults().await;
        flow.token.reset().await;
        let bytes = must(serde_json::to_vec(&tokens()), "token JSON fixture");
        flow.token
            .mount("POST", TOKEN_PATH, body_response(bytes, Some(encoding)))
            .await;
        failed(client.complete_authorization(&complete_request()).await);
        flow.counts(1, 0, 0).await;
    }
    flow.defaults().await;
    flow.token.reset().await;
    let mut truncated = gzip(&padded(&tokens(), 256));
    truncated.truncate(truncated.len() - 8);
    flow.token
        .mount("POST", TOKEN_PATH, body_response(truncated, Some("gzip")))
        .await;
    failed(client.complete_authorization(&complete_request()).await);
    flow.counts(1, 0, 0).await;
    // No recognized gzip here: both identity and the unsupported residual entry
    // remain visible. This does not claim inspection of headers stripped by gzip.
    flow.defaults().await;
    flow.token.reset().await;
    let bytes = must(serde_json::to_vec(&tokens()), "token JSON fixture");
    flow.token
        .mount(
            "POST",
            TOKEN_PATH,
            body_response(bytes, Some("identity")).append_header("content-encoding", "br"),
        )
        .await;
    failed(client.complete_authorization(&complete_request()).await);
    flow.counts(1, 0, 0).await;
}

#[tokio::test]
async fn a4_get_invalid_gzip_and_residual_encodings_do_not_return_partial_tokens() {
    let flow = Flow::new().await;
    let client = flow.client();
    for encoding in ["gzip", "br", "unknown", "", "identity, br"] {
        flow.defaults().await;
        flow.user.reset().await;
        let bytes = must(serde_json::to_vec(&flow.user_body()), "user JSON fixture");
        flow.user
            .mount("GET", USER_PATH, body_response(bytes, Some(encoding)))
            .await;
        failed(client.complete_authorization(&complete_request()).await);
        assert_get_failure_phase(&flow, true).await;
    }
    flow.defaults().await;
    flow.user.reset().await;
    let mut truncated = gzip(&padded(&flow.user_body(), 512));
    truncated.truncate(truncated.len() - 8);
    flow.user
        .mount("GET", USER_PATH, body_response(truncated, Some("gzip")))
        .await;
    failed(client.complete_authorization(&complete_request()).await);
    assert_get_failure_phase(&flow, true).await;
    flow.defaults().await;
    flow.user.reset().await;
    let bytes = must(serde_json::to_vec(&flow.user_body()), "user JSON fixture");
    flow.user
        .mount(
            "GET",
            USER_PATH,
            body_response(bytes, Some("identity")).append_header("content-encoding", "br"),
        )
        .await;
    failed(client.complete_authorization(&complete_request()).await);
    assert_get_failure_phase(&flow, true).await;
}

#[tokio::test]
async fn a4_token_redirect_to_configured_api_origin_is_not_followed_or_retried() {
    let flow = Flow::new().await;
    let client = flow.client();
    for status in [302, 307] {
        flow.defaults().await;
        flow.token.reset().await;
        flow.token
            .mount(
                "POST",
                TOKEN_PATH,
                ResponseTemplate::new(status)
                    .insert_header("location", flow.user.url("/capture").as_str()),
            )
            .await;
        flow.user
            .mount("POST", "/capture", json_response(&tokens()))
            .await;
        flow.user
            .mount("GET", "/capture", json_response(&tokens()))
            .await;
        failed(client.complete_authorization(&complete_request()).await);
        flow.counts(1, 0, 0).await; // No destination request can carry any secret.
    }
}

#[tokio::test]
async fn a4_get_redirect_to_other_configured_api_origin_never_forwards_bearer() {
    let flow = Flow::new().await;
    let client = flow.client();
    for status in [302, 307] {
        flow.defaults().await;
        flow.user.reset().await;
        flow.user
            .mount(
                "GET",
                USER_PATH,
                ResponseTemplate::new(status)
                    .insert_header("location", flow.emails.url("/capture").as_str()),
            )
            .await;
        flow.emails
            .mount("GET", "/capture", json_response(&flow.user_body()))
            .await;
        failed(client.complete_authorization(&complete_request()).await);
        assert_get_failure_phase(&flow, true).await; // Only EMAIL_PATH may reach destination server.
    }
}

#[tokio::test]
async fn a4_one_delayed_token_response_times_out_internally_before_watchdog_without_replay() {
    let flow = Flow::new().await;
    let client = flow.client();
    let control = must(
        client.complete_authorization(&complete_request()).await,
        "timeout fixture nominal control",
    );
    flow.assert_profile(&control);
    flow.counts(1, 1, 1).await;
    flow.defaults().await;
    flow.token.reset().await;
    flow.token
        .mount(
            "POST",
            TOKEN_PATH,
            json_response(&tokens()).set_delay(Duration::from_secs(11)),
        )
        .await;
    let started = Instant::now();
    let completed = must(
        tokio::time::timeout(
            Duration::from_secs(15),
            client.complete_authorization(&complete_request()),
        )
        .await,
        "15s watchdog must NOT expire; internal 10s timeout must complete the operation",
    );
    assert!(
        matches!(completed, Err(FederatedOAuthError::ExchangeCode)),
        "delayed valid token response must fail in exchange phase"
    );
    assert!(
        started.elapsed() >= Duration::from_secs(9),
        "a fast validation/connection refusal is not the controlled 10s timeout evidence"
    );
    flow.counts(1, 0, 0).await;
}

#[tokio::test]
async fn a4_vendor_error_bodies_cannot_echo_sensitive_values_in_errors_or_debug() {
    let flow = Flow::new().await;
    let client = flow.client();
    for token_phase in [true, false] {
        flow.defaults().await;
        let (vendor, verb, route) = if token_phase {
            (&flow.token, "POST", TOKEN_PATH)
        } else {
            (&flow.emails, "GET", EMAIL_PATH)
        };
        vendor.reset().await;
        vendor
            .mount(
                verb,
                route,
                ResponseTemplate::new(403).set_body_json(json!({
                    "code": CODE, "secret": SECRET, "access": ACCESS, "refresh": REFRESH,
                    "verifier": VERIFIER, "nonce": nonce(), "state": STATE
                })),
            )
            .await;
        failed(client.complete_authorization(&complete_request()).await);
        assert_redacted(&format!("{client:?}"));
        if token_phase {
            flow.counts(1, 0, 0).await;
        } else {
            assert_get_failure_phase(&flow, false).await;
        }
    }
}

#[tokio::test]
async fn a2_optional_pkce_complete_omits_verifier_and_does_not_send_oauth_nonce() {
    let mut flow = Flow::new().await;
    flow.config.pkce_supported = false;
    let client = flow.client();
    let mut request = complete_request();
    request.code_verifier = None;
    let auth = must(
        client.complete_authorization(&request).await,
        "optional PKCE completion",
    );
    flow.assert_profile(&auth);
    let requests = flow.token.requests().await;
    assert_eq!(requests.len(), 1);
    assert_post(&requests[0], &flow.config, SECRET, None);
    flow.counts(1, 1, 1).await;
    flow.defaults().await;
    failed(client.complete_authorization(&complete_request()).await);
    flow.counts(0, 0, 0).await;
}
