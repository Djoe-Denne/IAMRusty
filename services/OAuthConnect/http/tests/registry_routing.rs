//! B/G-5/G-6: real OAuth adapters behind the service registry and existing v2 receiver.
//!
//! Obligation -> risk -> scenario -> oracle:
//! - Immutable complete registry -> lost/overwritten entries -> two clients, invalid
//!   lists, raw noncanonical targets -> exact DTOs/HTTP statuses and vendor receipts.
//! - `OriginalUri` and isolated keys -> cross-instance replay -> full-path signatures
//!   versus key/path/body/context mutations -> 401 before any outgoing request.
//! - Existing receiver semantics -> new permissive/error layer -> method/JSON/size
//!   boundaries and collaborator failures -> inherited 405/400/422/415/413/502.
//! - Platform endpoints/ownership -> false OAuth readiness or leaked task -> actual
//!   probe changes and one SDK-owned TCP witness -> statuses, no vendor I/O, joined stop.
//!
//! KEEP all tests. The direct OAuth preflight is independent of HTTP's T0 refusal.
//! A refusal by an empty T0 router does not qualify the passing registry negatives.
//! Raw URI tests use oneshot, not a URL client. No query restriction is invented.
//! This is B routing, not C's mixed OIDC Application/binary boot proof.

#[path = "support/r_fixture.rs"]
mod r_fixture;
#[path = "support/r_owned.rs"]
mod r_owned;

use axum::{body::Body, http::StatusCode, Router};
use idp_connect_contract::{
    hmac::{unix_timestamp_secs, SIGNATURE_HEADER, TIMESTAMP_HEADER},
    FederatedOAuthClient,
};
use oauth_connect_http::{create_prefixed_router, create_router, ReadinessProbe, SERVICE_PREFIX};
use r_fixture::{
    authorize, authorize_body, captured, complete, complete_body, invalid_registry, must,
    raw_request, send, sign_headers, signed, state, Backend, Pair, BODY_LIMIT, CHALLENGE,
    ENTERPRISE_AUTHORIZE, ENTERPRISE_COMPLETE, FORGE_AUTHORIZE, FORGE_COMPLETE, KEY_A, KEY_B,
    NONCE, VERIFIER,
};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use wiremock::ResponseTemplate;

fn prefixed(pair: &Pair) -> Router {
    must(
        create_prefixed_router(
            state(),
            pair.entries(),
            Arc::new(ReadinessProbe::new("oauth-connect-routing-test")),
        ),
        "valid prefixed registry construction",
    )
}

#[tokio::test]
async fn b_fixture_actual_oauth_two_vendor_preflight_is_independent_of_registry_t0() {
    let pair = Pair::start().await;
    for backend in [&pair.forge, &pair.enterprise] {
        let auth = must(
            backend.client.authorize_authentication(&authorize()).await,
            "direct actual OAuth authorize",
        );
        backend.assert_authorize(&must(serde_json::to_value(auth), "preflight authorize DTO"));
        backend.counts(0, 0).await;
        let authenticated = must(
            backend.client.complete_authorization(&complete()).await,
            "direct actual OAuth complete",
        );
        backend.assert_authentication(&must(
            serde_json::to_value(authenticated),
            "preflight complete DTO",
        ));
        backend.counts(1, 1).await;
        backend.credentials().await;
    }
}

#[tokio::test]
async fn b5_two_post_operations_select_two_actual_clients_with_identical_request_bodies() {
    let pair = Pair::start().await;
    assert_eq!(SERVICE_PREFIX, "/oauth-connect");
    captured(async {
        let router = prefixed(&pair);
        // The body is intentionally identical for both instances. Only the full route/key selects a client.
        for (backend, authorize_path, complete_path, key) in [
            (&pair.forge, FORGE_AUTHORIZE, FORGE_COMPLETE, KEY_A),
            (
                &pair.enterprise,
                ENTERPRISE_AUTHORIZE,
                ENTERPRISE_COMPLETE,
                KEY_B,
            ),
        ] {
            let auth = send(
                &router,
                signed("POST", authorize_path, &authorize_body(), key),
            )
            .await;
            auth.status(StatusCode::OK);
            backend.assert_authorize(&auth.json());
            backend.counts(0, 0).await;
            let authenticated = send(
                &router,
                signed("POST", complete_path, &complete_body(), key),
            )
            .await;
            authenticated.status(StatusCode::OK);
            backend.assert_authentication(&authenticated.json());
            backend.counts(1, 1).await;
            backend.credentials().await;
        }
    })
    .await;
}

#[tokio::test]
async fn b5_unprefixed_router_nested_once_retains_full_original_uri_for_hmac() {
    let pair = Pair::start().await;
    let inner = must(
        create_router(state(), pair.entries()),
        "valid embeddable registry construction",
    );
    let router = Router::new().nest("/oauth-connect", inner);
    let reply = send(
        &router,
        signed("POST", FORGE_COMPLETE, &complete_body(), KEY_A),
    )
    .await;
    reply.status(StatusCode::OK);
    pair.forge.assert_authentication(&reply.json());
    pair.forge.counts(1, 1).await;
    pair.enterprise.counts(0, 0).await;
    for raw_path in [
        "/forge/v2/complete",
        "/oauth-connect/oauth-connect/forge/v2/complete",
    ] {
        let reply = send(&router, signed("POST", raw_path, &complete_body(), KEY_A)).await;
        reply.status(StatusCode::NOT_FOUND);
        reply.redacted();
    }
    pair.forge.counts(1, 1).await;
    pair.enterprise.counts(0, 0).await;
}

#[tokio::test]
async fn b5_canonical_slug_boundaries_one_and_fifty_ascii_letters_accept_real_clients() {
    const FIFTY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const LONG_PATH: &str =
        "/oauth-connect/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/v2/complete";
    assert_eq!(FIFTY.len(), 50, "boundary fixture length must be exact");
    assert!(
        LONG_PATH
            .strip_prefix("/oauth-connect/")
            .and_then(|s| s.strip_suffix("/v2/complete"))
            == Some(FIFTY),
        "boundary fixture path must match its provisioned slug"
    );
    let short = Backend::start(
        "z",
        KEY_A,
        "access-short-sentinel",
        1,
        "shortuser",
        "short@example.test",
    )
    .await;
    let long = Backend::start(
        FIFTY,
        KEY_B,
        "access-long-sentinel",
        2,
        "longuser",
        "long@example.test",
    )
    .await;
    let router = must(
        create_prefixed_router(
            state(),
            vec![
                ("z".into(), short.connect()),
                (FIFTY.into(), long.connect()),
            ],
            Arc::new(ReadinessProbe::new("routing-boundary-test")),
        ),
        "canonical boundary registry construction",
    );
    for (backend, path, key) in [
        (&short, "/oauth-connect/z/v2/complete", KEY_A),
        (&long, LONG_PATH, KEY_B),
    ] {
        let reply = send(&router, signed("POST", path, &complete_body(), key)).await;
        reply.status(StatusCode::OK);
        backend.assert_authentication(&reply.json());
        backend.counts(1, 1).await;
        backend.credentials().await;
    }
}

#[tokio::test]
async fn b5_empty_registry_is_rejected_by_both_builders_without_panic() {
    invalid_registry(&create_router(state(), Vec::new()));
    invalid_registry(&create_prefixed_router(
        state(),
        Vec::new(),
        Arc::new(ReadinessProbe::new("routing-test")),
    ));
}

#[tokio::test]
async fn b5_noncanonical_registry_slugs_are_rejected_before_publishing_routes() {
    let pair = Pair::start().await;
    for id in [
        String::new(),
        "Forge".into(),
        "with-dash".into(),
        "forge1".into(),
        "é".into(),
        "%66orge".into(),
        "a".repeat(51),
    ] {
        invalid_registry(&create_router(
            state(),
            vec![(id.clone(), pair.forge.connect())],
        ));
        invalid_registry(&create_prefixed_router(
            state(),
            vec![(id, pair.forge.connect())],
            Arc::new(ReadinessProbe::new("routing-test")),
        ));
    }
    pair.zero().await;
}

#[tokio::test]
async fn b5_duplicate_vector_entries_are_refused_before_a_map_can_overwrite_a_client() {
    let pair = Pair::start().await;
    for reverse in [false, true] {
        let (first, second) = if reverse {
            (&pair.enterprise, &pair.forge)
        } else {
            (&pair.forge, &pair.enterprise)
        };
        invalid_registry(&create_router(
            state(),
            vec![
                ("forge".into(), first.connect()),
                ("forge".into(), second.connect()),
            ],
        ));
        invalid_registry(&create_prefixed_router(
            state(),
            vec![
                ("forge".into(), first.connect()),
                ("forge".into(), second.connect()),
            ],
            Arc::new(ReadinessProbe::new("routing-test")),
        ));
    }
    pair.zero().await;
}

#[tokio::test]
async fn b5_raw_unknown_case_encoded_and_slash_variants_are_404_without_vendor_io() {
    let pair = Pair::start().await;
    let router = prefixed(&pair);
    for uri in [
        "/oauth-connect/absent/v2/complete",
        "/oauth-connect/Forge/v2/complete",
        "/oauth-connect/%66orge/v2/complete",
        "/oauth-connect/%2566orge/v2/complete",
        "/oauth-connect/forge//v2/complete",
        "/oauth-connect//forge/v2/complete",
        "/oauth-connect/forge/v2/complete/",
        "/oauth-connect/forge/../enterprise/v2/complete",
    ] {
        // raw_request asserts the exact URI bytes survived construction, before router dispatch.
        let reply = send(&router, signed("POST", uri, &complete_body(), KEY_A)).await;
        reply.status(StatusCode::NOT_FOUND);
        reply.redacted();
    }
    pair.zero().await;
}

#[tokio::test]
async fn b5_legacy_profile_callback_and_double_prefix_aliases_do_not_exist() {
    let pair = Pair::start().await;
    let router = prefixed(&pair);
    for uri in [
        "/oauth-connect/forge/v1/authorize",
        "/oauth-connect/forge/v1/token",
        "/oauth-connect/forge/v1/profile",
        "/oauth-connect/forge/v2/profile",
        "/oauth-connect/forge/callback",
        "/oauth-connect/oauth-connect/forge/v2/authorize",
        "/forge/v2/authorize",
    ] {
        let reply = send(&router, signed("POST", uri, &authorize_body(), KEY_A)).await;
        reply.status(StatusCode::NOT_FOUND);
        reply.redacted();
    }
    pair.zero().await;
}

#[tokio::test]
async fn b5_signed_get_on_each_business_endpoint_is_405_not_a_legacy_fallback() {
    let pair = Pair::start().await;
    let router = prefixed(&pair);
    for (path, key, body) in [
        (FORGE_AUTHORIZE, KEY_A, authorize_body()),
        (FORGE_COMPLETE, KEY_A, complete_body()),
        (ENTERPRISE_AUTHORIZE, KEY_B, authorize_body()),
        (ENTERPRISE_COMPLETE, KEY_B, complete_body()),
    ] {
        let reply = send(&router, signed("GET", path, &body, key)).await;
        reply.status(StatusCode::METHOD_NOT_ALLOWED);
        reply.redacted();
    }
    pair.zero().await;
}

#[tokio::test]
async fn b6_wrong_key_truncated_path_and_cross_instance_signature_are_401_before_io() {
    let pair = Pair::start().await;
    let router = prefixed(&pair);
    for (target, body, key, other, short, other_path) in [
        (
            FORGE_AUTHORIZE,
            authorize_body(),
            KEY_A,
            KEY_B,
            "/v2/authorize",
            ENTERPRISE_AUTHORIZE,
        ),
        (
            FORGE_COMPLETE,
            complete_body(),
            KEY_A,
            KEY_B,
            "/v2/complete",
            ENTERPRISE_COMPLETE,
        ),
        (
            ENTERPRISE_AUTHORIZE,
            authorize_body(),
            KEY_B,
            KEY_A,
            "/enterprise/v2/authorize",
            FORGE_AUTHORIZE,
        ),
        (
            ENTERPRISE_COMPLETE,
            complete_body(),
            KEY_B,
            KEY_A,
            "/enterprise/v2/complete",
            FORGE_COMPLETE,
        ),
    ] {
        send(&router, signed("POST", target, &body, other))
            .await
            .empty_error(StatusCode::UNAUTHORIZED);
        for signed_path in [short, other_path] {
            let mut request = raw_request("POST", target, &body);
            sign_headers(
                &mut request,
                "POST",
                signed_path,
                &body,
                key,
                must(unix_timestamp_secs(), "test timestamp"),
            );
            send(&router, request)
                .await
                .empty_error(StatusCode::UNAUTHORIZED);
        }
    }
    pair.zero().await;
}

#[tokio::test]
async fn b6_signed_exact_body_binds_nonce_pkce_code_and_even_json_whitespace() {
    let pair = Pair::start().await;
    let router = prefixed(&pair);
    let authorize_original = authorize_body();
    let complete_original = complete_body();
    for (path, original, modified) in [
        (
            FORGE_AUTHORIZE,
            &authorize_original,
            authorize_original.replace(NONCE, &NONCE.replacen('A', "B", 1)),
        ),
        (
            FORGE_AUTHORIZE,
            &authorize_original,
            authorize_original.replace(CHALLENGE, &"A".repeat(43)),
        ),
        (
            FORGE_COMPLETE,
            &complete_original,
            complete_original.replace(NONCE, &NONCE.replacen('A', "B", 1)),
        ),
        (
            FORGE_COMPLETE,
            &complete_original,
            complete_original.replace(VERIFIER, &"A".repeat(43)),
        ),
        (
            FORGE_COMPLETE,
            &complete_original,
            complete_original.replace("opaque-code-sentinel", "other-code-sentinel"),
        ),
        (
            FORGE_COMPLETE,
            &complete_original,
            format!("{complete_original} "),
        ),
    ] {
        assert!(
            original != &modified,
            "body-mutation fixture must actually change bytes"
        );
        let mut request = signed("POST", path, original, KEY_A);
        *request.body_mut() = Body::from(modified);
        send(&router, request)
            .await
            .empty_error(StatusCode::UNAUTHORIZED);
    }
    let mut non_utf8 = signed("POST", FORGE_COMPLETE, &complete_original, KEY_A);
    *non_utf8.body_mut() = Body::from(vec![0xff]);
    send(&router, non_utf8)
        .await
        .empty_error(StatusCode::UNAUTHORIZED);
    pair.zero().await;
}

#[tokio::test]
async fn b6_missing_noncontract_bearer_malformed_and_stale_headers_never_authenticate() {
    let pair = Pair::start().await;
    let router = prefixed(&pair);
    for path in [FORGE_AUTHORIZE, FORGE_COMPLETE] {
        let body = if path == FORGE_AUTHORIZE {
            authorize_body()
        } else {
            complete_body()
        };
        send(&router, raw_request("POST", path, &body))
            .await
            .empty_error(StatusCode::UNAUTHORIZED);
        let mut bearer = raw_request("POST", path, &body);
        bearer.headers_mut().insert(
            "authorization",
            must("Bearer user-bearer-sentinel".parse(), "bearer fixture"),
        );
        send(&router, bearer)
            .await
            .empty_error(StatusCode::UNAUTHORIZED);
        let canonical = signed("POST", path, &body, KEY_A);
        let mut noncontract = raw_request("POST", path, &body);
        noncontract.headers_mut().insert(
            "x-connect-timestamp",
            canonical.headers()[TIMESTAMP_HEADER].clone(),
        );
        noncontract.headers_mut().insert(
            "x-connect-signature",
            canonical.headers()[SIGNATURE_HEADER].clone(),
        );
        send(&router, noncontract)
            .await
            .empty_error(StatusCode::UNAUTHORIZED);
        for header in [TIMESTAMP_HEADER, SIGNATURE_HEADER] {
            let mut missing = signed("POST", path, &body, KEY_A);
            missing.headers_mut().remove(header);
            send(&router, missing)
                .await
                .empty_error(StatusCode::UNAUTHORIZED);
        }
        let mut malformed = signed("POST", path, &body, KEY_A);
        let name: axum::http::HeaderName = must(TIMESTAMP_HEADER.parse(), "timestamp name");
        malformed.headers_mut().insert(
            name,
            must("not-an-integer-sentinel".parse(), "invalid timestamp"),
        );
        send(&router, malformed)
            .await
            .empty_error(StatusCode::UNAUTHORIZED);
        let mut malformed_signature = signed("POST", path, &body, KEY_A);
        let name: axum::http::HeaderName = must(SIGNATURE_HEADER.parse(), "signature name");
        malformed_signature.headers_mut().insert(
            name,
            must("not-hex-sentinel".parse(), "invalid signature encoding"),
        );
        send(&router, malformed_signature)
            .await
            .empty_error(StatusCode::UNAUTHORIZED);
        let mut stale = raw_request("POST", path, &body);
        let old = must(unix_timestamp_secs(), "test timestamp") - 120;
        sign_headers(&mut stale, "POST", path, &body, KEY_A, old);
        send(&router, stale)
            .await
            .empty_error(StatusCode::UNAUTHORIZED);
    }
    pair.zero().await;
}

#[tokio::test]
async fn b6_incoming_64kib_is_inclusive_and_plus_one_is_413_before_client() {
    let pair = Pair::start().await;
    let router = prefixed(&pair);
    let mut body = authorize_body();
    assert!(
        body.len() < BODY_LIMIT,
        "fixture base body must fit the boundary"
    );
    body.push_str(&" ".repeat(BODY_LIMIT - body.len()));
    let accepted = send(&router, signed("POST", FORGE_AUTHORIZE, &body, KEY_A)).await;
    accepted.status(StatusCode::OK);
    pair.forge.assert_authorize(&accepted.json());
    body.push(' ');
    send(&router, signed("POST", FORGE_AUTHORIZE, &body, KEY_A))
        .await
        .empty_error(StatusCode::PAYLOAD_TOO_LARGE);
    pair.zero().await;
}

#[tokio::test]
async fn b6_authenticated_json_and_v2_context_fail_with_existing_4xx_not_401_or_vendor_io() {
    let pair = Pair::start().await;
    let router = prefixed(&pair);
    let mut bad_nonce = must(serde_json::to_value(complete()), "fixture complete object");
    bad_nonce["expected_nonce"] = json!("bad-nonce-sentinel");
    let mut bad_pkce = must(
        serde_json::to_value(authorize()),
        "fixture authorize object",
    );
    bad_pkce["code_challenge_method"] = json!("plain");
    let mut wrong_type = must(serde_json::to_value(complete()), "fixture complete object");
    wrong_type["code"] = json!(42);
    for (path, body, expected) in [
        (FORGE_AUTHORIZE, "{".to_owned(), StatusCode::BAD_REQUEST),
        (
            FORGE_AUTHORIZE,
            "{}".to_owned(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            FORGE_COMPLETE,
            "{}".to_owned(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            FORGE_COMPLETE,
            bad_nonce.to_string(),
            StatusCode::BAD_REQUEST,
        ),
        (
            FORGE_AUTHORIZE,
            bad_pkce.to_string(),
            StatusCode::BAD_REQUEST,
        ),
        (
            FORGE_COMPLETE,
            wrong_type.to_string(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let reply = send(&router, signed("POST", path, &body, KEY_A)).await;
        reply.status(expected);
        reply.redacted();
    }
    let mut no_json_type = signed("POST", FORGE_COMPLETE, &complete_body(), KEY_A);
    no_json_type.headers_mut().remove("content-type");
    let reply = send(&router, no_json_type).await;
    reply.status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    reply.redacted();
    pair.zero().await;
}

#[tokio::test]
async fn b6_real_vendor_failures_return_empty_502_and_do_not_leak_trace_context() {
    for token_failed in [true, false] {
        let pair = Pair::start().await;
        pair.forge.server.reset().await;
        let bad = ResponseTemplate::new(403)
            .set_body_string("vendor-code-token-secret-nonce-verifier-sentinel");
        let (token, profile) = if token_failed {
            (bad, pair.forge.profile_response())
        } else {
            (pair.forge.token_response(), bad)
        };
        pair.forge.responses(token, profile).await;
        captured(async {
            let router = prefixed(&pair);
            let reply = send(
                &router,
                signed("POST", FORGE_COMPLETE, &complete_body(), KEY_A),
            )
            .await;
            reply.empty_error(StatusCode::BAD_GATEWAY);
            pair.forge.counts(1, usize::from(!token_failed)).await;
            pair.enterprise.counts(0, 0).await;
        })
        .await;
    }
}

#[tokio::test]
async fn b5_health_and_ready_follow_only_the_supplied_platform_probe_without_oauth_io() {
    let pair = Pair::start().await;
    let live = Arc::new(AtomicBool::new(false));
    let probe = Arc::new(
        ReadinessProbe::new("oauth-connect-routing-test")
            .with_liveness("initialized", live.clone()),
    );
    let router = must(
        create_prefixed_router(state(), pair.entries(), probe),
        "registry with platform probe",
    );
    let health = send(&router, raw_request("GET", "/oauth-connect/health", "")).await;
    health.status(StatusCode::OK);
    assert!(
        health.body.as_slice() == b"OK".as_slice(),
        "SDK liveness response unchanged"
    );
    for (flag, expected, label) in [
        (false, StatusCode::SERVICE_UNAVAILABLE, "not_ready"),
        (true, StatusCode::OK, "ready"),
    ] {
        live.store(flag, Ordering::SeqCst);
        let ready = send(&router, raw_request("GET", "/oauth-connect/ready", "")).await;
        ready.status(expected);
        ready.redacted();
        let report = ready.json();
        assert!(report["status"].as_str() == Some(label));
        assert!(report["service"].as_str() == Some("oauth-connect-routing-test"));
        let text = String::from_utf8_lossy(&ready.body);
        for field in [
            "forge",
            "enterprise",
            "providers",
            "client_id",
            "access_token",
            "authorization_url",
        ] {
            assert!(
                !text.contains(field),
                "G-5: platform probe exposed registry/authentication data"
            );
        }
    }
    for path in ["/health", "/ready"] {
        send(&router, raw_request("GET", path, ""))
            .await
            .status(StatusCode::NOT_FOUND);
    }
    pair.zero().await;
}

#[tokio::test]
async fn b6_sdk_owned_tcp_listener_serves_signed_v2_and_uses_joined_shutdown() {
    let pair = Pair::start().await;
    let router = prefixed(&pair);
    r_owned::run(router, move |origin| async move {
        let http = must(
            reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(3))
                .build(),
            "incoming test client, not product transport proof",
        );
        let body = complete_body();
        let signed_request = signed("POST", ENTERPRISE_COMPLETE, &body, KEY_B);
        let timestamp = signed_request.headers()[TIMESTAMP_HEADER].clone();
        let signature = signed_request.headers()[SIGNATURE_HEADER].clone();
        let response = must(
            http.post(format!("{origin}{ENTERPRISE_COMPLETE}"))
                .header("content-type", "application/json")
                .header(TIMESTAMP_HEADER, timestamp)
                .header(SIGNATURE_HEADER, signature)
                .body(body)
                .send()
                .await,
            "real TCP v2 request",
        );
        let reply = r_owned::reply(response).await;
        reply.status(StatusCode::OK);
        pair.enterprise.assert_authentication(&reply.json());
        pair.enterprise.counts(1, 1).await;
        pair.forge.counts(0, 0).await;
        pair.enterprise.credentials().await;
        let wrong = signed("POST", FORGE_COMPLETE, &complete_body(), KEY_B);
        let response = must(
            http.post(format!("{origin}{FORGE_COMPLETE}"))
                .header("content-type", "application/json")
                .header(TIMESTAMP_HEADER, wrong.headers()[TIMESTAMP_HEADER].clone())
                .header(SIGNATURE_HEADER, wrong.headers()[SIGNATURE_HEADER].clone())
                .body(complete_body())
                .send()
                .await,
            "TCP wrong-key request",
        );
        r_owned::reply(response)
            .await
            .empty_error(StatusCode::UNAUTHORIZED);
        pair.forge.counts(0, 0).await;
        pair.enterprise.counts(1, 1).await;
    })
    .await;
}
