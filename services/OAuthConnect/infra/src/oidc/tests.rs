//! A / G-2 UTC and G-4 cache against the real client, signed JWTs and HTTP.
//!
//! Obligation -> risk -> scenario -> oracle:
//! - Independent clocks -> expiry masks cache state -> fixed signed UTC, manual
//!   monotone 29.999/30 and 299.999/300 -> exact accept/refuse and endpoint counts.
//! - Refresh lifecycle -> stale trust/herd/stuck guard -> failures, rotation,
//!   concurrent distinct codes and owner cancellation -> real crypto outcomes,
//!   exactly one owned refresh, no repeated token POST, bounded waiter completion.
//! - Claims time -> permissive verifier -> signed exact exp/iat/nbf/coherence
//!   boundaries -> expected profile or closed error with no `UserInfo`.
//!
//! KEEP throughout. No implementation/cache evaluator substitute; no global
//! `Tokio` time advance, 30/300-second sleeps or public clock/cache configuration.

#[path = "../../tests/support/oidc_fixture.rs"]
mod fixture;

use chrono::{DateTime, Utc};
use fixture::{
    authentication_failed, completion, http, json_response, jwk, jwks, key_a, key_b, must, profile,
    redacted, secret, sign, OidcFixture, CLIENT_ID, KID, SUBJECT,
};
use idp_connect_contract::{FederatedOAuthClient, FederatedOAuthError, ProviderAuthentication};
use serde_json::json;
use std::{
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
use tokio::{task::JoinSet, time::Instant};

use super::OidcClient;

struct Clocks {
    monotone: Arc<Mutex<Instant>>,
    utc: Arc<Mutex<DateTime<Utc>>>,
    start: Instant,
}

impl Clocks {
    fn new() -> Self {
        let start = Instant::now();
        let utc = DateTime::from_timestamp(2_000_000_000, 0)
            .map_or_else(|| panic!("fixed UTC fixture"), std::convert::identity);
        Self {
            monotone: Arc::new(Mutex::new(start)),
            utc: Arc::new(Mutex::new(utc)),
            start,
        }
    }
    fn at_millis(&self, millis: u64) {
        *self.monotone.lock().unwrap_or_else(PoisonError::into_inner) =
            self.start + Duration::from_millis(millis);
    }
    fn utc(&self) -> DateTime<Utc> {
        *self.utc.lock().unwrap_or_else(PoisonError::into_inner)
    }
    async fn client(&self, f: &OidcFixture) -> Arc<OidcClient> {
        let monotone = Arc::clone(&self.monotone);
        let utc = Arc::clone(&self.utc);
        Arc::new(must(
            OidcClient::new_with_clocks(
                &f.provider,
                secret(),
                http(),
                Arc::new(move || *monotone.lock().unwrap_or_else(PoisonError::into_inner)),
                Arc::new(move || *utc.lock().unwrap_or_else(PoisonError::into_inner)),
            )
            .await,
            "valid OIDC startup with independent monotone/UTC hooks",
        ))
    }
}

async fn completed(
    client: &OidcClient,
    code: &str,
) -> Result<ProviderAuthentication, FederatedOAuthError> {
    must(
        tokio::time::timeout(
            Duration::from_secs(3),
            client.complete_authorization(&completion(code)),
        )
        .await,
        "cache operation must complete, not sleep for cooldown/TTL or retain a stuck guard",
    )
}

async fn accepted(f: &OidcFixture, client: &OidcClient, code: &str) {
    let auth = must(
        completed(client, code).await,
        "valid signed cache/clock authentication",
    );
    profile(
        &auth,
        SUBJECT,
        "name",
        Some("one@example.test"),
        true,
        Some(&f.url("/avatar")),
    );
}

#[tokio::test]
async fn g2_exact_utc_exp_iat_nbf_and_coherence_boundaries_use_injected_clock() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    let client = clocks.client(&f).await;
    let now = clocks.utc().timestamp();
    for (index, (field, value, valid)) in [
        ("exp", now + 1, true),
        ("exp", now, false),
        ("exp", now - 1, false),
        ("iat", now + 60, true),
        ("iat", now + 61, false),
        ("nbf", now, true),
        ("nbf", now + 1, false),
    ]
    .iter()
    .enumerate()
    {
        let mut claims = f.claims_at(clocks.utc());
        claims[*field] = json!(value);
        f.set_claims(&claims);
        let result = completed(&client, &format!("oidc-code-utc-{index}")).await;
        if *valid {
            profile(
                &must(result, "inclusive UTC boundary"),
                SUBJECT,
                "name",
                Some("one@example.test"),
                true,
                Some(&f.url("/avatar")),
            );
        } else {
            authentication_failed(&result);
        }
    }
    let mut incoherent = f.claims_at(clocks.utc());
    incoherent["iat"] = json!(now + 2);
    incoherent["exp"] = json!(now + 1);
    f.set_claims(&incoherent);
    authentication_failed(&completed(&client, "oidc-code-incoherent").await);
    let mut once_fresh = f.claims_at(clocks.utc());
    once_fresh["exp"] = json!(now + 1);
    let advanced_utc = clocks.utc() + chrono::Duration::seconds(1);
    *clocks.utc.lock().unwrap_or_else(PoisonError::into_inner) = advanced_utc;
    f.set_claims(&once_fresh);
    authentication_failed(&completed(&client, "oidc-code-utc-resampled").await);
    f.assert_counts(1, 1, 9, 0).await;
}

#[tokio::test]
async fn g4_cache_is_fresh_before_300_seconds_and_requires_refresh_at_equality() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    f.set_claims(&f.claims_at(clocks.utc()));
    let client = clocks.client(&f).await;
    clocks.at_millis(299_999);
    accepted(&f, &client, "oidc-code-ttl-before").await;
    f.assert_counts(1, 1, 1, 0).await;
    clocks.at_millis(300_000);
    accepted(&f, &client, "oidc-code-ttl-equal").await;
    f.assert_counts(1, 2, 2, 0).await;
    assert_eq!(
        clocks.utc().timestamp(),
        2_000_000_000,
        "cache movement must not move signed claims time"
    );
}

#[tokio::test]
async fn g4_initial_fetch_consumes_cooldown_and_new_attempt_is_allowed_at_exactly_30_seconds() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    let client = clocks.client(&f).await;
    f.keys.json(&jwks(key_b(), "kid-B"));
    f.set_signed(key_b(), "kid-B", &f.claims_at(clocks.utc()));
    clocks.at_millis(29_999);
    authentication_failed(&completed(&client, "oidc-code-cooldown-before").await);
    f.assert_counts(1, 1, 1, 0).await;
    clocks.at_millis(30_000);
    accepted(&f, &client, "oidc-code-cooldown-equal").await;
    f.assert_counts(1, 2, 2, 0).await;
    f.keys.json(&jwks(key_b(), "kid-C"));
    f.set_signed(key_b(), "kid-C", &f.claims_at(clocks.utc()));
    clocks.at_millis(59_999);
    authentication_failed(&completed(&client, "oidc-code-next-before").await);
    f.assert_counts(1, 2, 3, 0).await;
    clocks.at_millis(60_000);
    accepted(&f, &client, "oidc-code-next-equal").await;
    f.assert_counts(1, 3, 4, 0).await;
}

#[tokio::test]
async fn g4_failed_attempt_also_consumes_cooldown_without_waiting_inside_handler() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    let client = clocks.client(&f).await;
    f.set_signed(key_b(), "kid-B", &f.claims_at(clocks.utc()));
    f.keys.set(wiremock::ResponseTemplate::new(503));
    clocks.at_millis(30_000);
    authentication_failed(&completed(&client, "oidc-code-failed-refresh").await);
    f.keys.json(&jwks(key_b(), "kid-B"));
    clocks.at_millis(59_999);
    authentication_failed(&completed(&client, "oidc-code-failed-cooldown").await);
    f.assert_counts(1, 2, 2, 0).await;
    clocks.at_millis(60_000);
    accepted(&f, &client, "oidc-code-recovery-window").await;
    f.assert_counts(1, 3, 3, 0).await;
}

#[tokio::test]
async fn g4_expired_snapshot_is_not_used_when_refresh_fails_or_is_in_cooldown() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    f.set_claims(&f.claims_at(clocks.utc()));
    let client = clocks.client(&f).await;
    clocks.at_millis(299_999);
    accepted(&f, &client, "oidc-code-last-fresh").await;
    f.keys.set(wiremock::ResponseTemplate::new(503));
    clocks.at_millis(300_000);
    authentication_failed(&completed(&client, "oidc-code-expired-failure").await);
    clocks.at_millis(300_001);
    authentication_failed(&completed(&client, "oidc-code-no-stale-in-cooldown").await);
    f.assert_counts(1, 2, 3, 0).await;
    f.keys.json(&jwks(key_a(), KID));
    clocks.at_millis(330_000);
    accepted(&f, &client, "oidc-code-expired-recovery").await;
    f.assert_counts(1, 3, 4, 0).await;
}

#[tokio::test]
async fn g4_failed_unknown_kid_refresh_preserves_fresh_old_key_but_never_renews_success_age() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    let client = clocks.client(&f).await;
    f.keys.set(wiremock::ResponseTemplate::new(503));
    clocks.at_millis(30_000);
    f.set_signed(key_b(), "kid-B", &f.claims_at(clocks.utc()));
    authentication_failed(&completed(&client, "oidc-code-unknown-failure").await);
    clocks.at_millis(31_000);
    f.set_claims(&f.claims_at(clocks.utc()));
    accepted(&f, &client, "oidc-code-still-fresh-old-key").await;
    f.assert_counts(1, 2, 2, 0).await;
    clocks.at_millis(300_000);
    authentication_failed(&completed(&client, "oidc-code-success-age-not-renewed").await);
    f.assert_counts(1, 3, 3, 0).await;
}

#[tokio::test]
async fn g4_same_kid_crypto_failure_does_not_refresh_early_but_new_snapshot_is_used_at_ttl() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    let client = clocks.client(&f).await;
    f.keys.json(&jwks(key_b(), KID));
    f.set_signed(key_b(), KID, &f.claims_at(clocks.utc()));
    clocks.at_millis(30_000);
    authentication_failed(&completed(&client, "oidc-code-same-kid-before-ttl").await);
    f.assert_counts(1, 1, 1, 0).await;
    let mut bad_nonce = f.claims_at(clocks.utc());
    bad_nonce["nonce"] = json!("wrong");
    f.set_claims(&bad_nonce);
    clocks.at_millis(31_000);
    authentication_failed(&completed(&client, "oidc-code-invalid-claim-no-refresh").await);
    f.assert_counts(1, 1, 2, 0).await;
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS384);
    header.kid = Some(KID.to_owned());
    let token = must(
        jsonwebtoken::encode(&header, &f.claims_at(clocks.utc()), &key_a().encoding),
        "signed unapproved algorithm fixture",
    );
    f.token.json(&OidcFixture::token_body(&token));
    clocks.at_millis(32_000);
    authentication_failed(&completed(&client, "oidc-code-alg-no-refresh").await);
    f.assert_counts(1, 1, 3, 0).await;
    f.set_signed(key_b(), KID, &f.claims_at(clocks.utc()));
    clocks.at_millis(300_000);
    accepted(&f, &client, "oidc-code-same-kid-at-ttl").await;
    f.assert_counts(1, 2, 4, 0).await;
    f.set_claims(&f.claims_at(clocks.utc()));
    clocks.at_millis(301_000);
    authentication_failed(&completed(&client, "oidc-code-replaced-old-key").await);
    f.assert_counts(1, 2, 5, 0).await;
}

#[tokio::test]
async fn g4_new_kid_refresh_replaces_snapshot_instead_of_retaining_old_keys_indefinitely() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    let client = clocks.client(&f).await;
    f.keys.json(&jwks(key_b(), "kid-B"));
    f.set_signed(key_b(), "kid-B", &f.claims_at(clocks.utc()));
    clocks.at_millis(30_000);
    accepted(&f, &client, "oidc-code-rotate-new-kid").await;
    // Client::from_provider_metadata copies keys: signed UserInfo must see the
    // refreshed snapshot too, not only a separately updated ID-token verifier.
    let mut no_email = f.claims_at(clocks.utc());
    let Some(object) = no_email.as_object_mut() else {
        panic!("claims fixture object");
    };
    object.remove("email");
    f.set_signed(key_b(), "kid-B", &no_email);
    let info = json!({"iss": f.issuer(), "aud": CLIENT_ID, "sub": SUBJECT,
        "email": "one@example.test", "email_verified": true});
    f.userinfo.set(
        wiremock::ResponseTemplate::new(200)
            .set_body_string(sign(key_b(), "kid-B", &info))
            .insert_header("content-type", "application/jwt"),
    );
    clocks.at_millis(31_000);
    accepted(&f, &client, "oidc-code-rotated-signed-userinfo").await;
    f.assert_counts(1, 2, 2, 1).await;
    f.set_claims(&f.claims_at(clocks.utc()));
    clocks.at_millis(31_000);
    authentication_failed(&completed(&client, "oidc-code-no-old-key-grace").await);
    clocks.at_millis(60_000);
    authentication_failed(&completed(&client, "oidc-code-old-key-still-not-published").await);
    f.assert_counts(1, 3, 4, 1).await;
}

async fn concurrent_refresh(success: bool) {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    let client = clocks.client(&f).await;
    f.set_signed(key_b(), "kid-B", &f.claims_at(clocks.utc()));
    clocks.at_millis(30_000);
    let response = if success {
        json_response(&jwks(key_b(), "kid-B"))
    } else {
        wiremock::ResponseTemplate::new(503)
    };
    f.keys.set(response.set_delay(Duration::from_millis(500)));
    let mut tasks = JoinSet::new(); // Drop aborts all owned children on failure.
    for index in 0..4 {
        let client = Arc::clone(&client);
        tasks.spawn(async move { completed(&client, &format!("oidc-code-flight-{index}")).await });
    }
    f.token.wait_count(4).await;
    f.keys.wait_count(2).await;
    assert!(
        tasks.try_join_next().is_none(),
        "fixture phase: four POSTs are in flight before the delayed refresh completes"
    );
    let finished = tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(joined) = tasks.join_next().await {
            let result = must(joined, "owned concurrent completion task");
            if success {
                profile(
                    &must(result, "shared successful JWKS refresh"),
                    SUBJECT,
                    "name",
                    Some("one@example.test"),
                    true,
                    Some(&f.url("/avatar")),
                );
            } else {
                authentication_failed(&result);
            }
        }
    })
    .await;
    assert!(
        finished.is_ok(),
        "single-flight waiters must finish without a stuck guard"
    );
    f.assert_counts(1, 2, 4, 0).await;
    let codes: Vec<_> = f
        .receipts()
        .await
        .iter()
        .filter(|r| r.url.path() == fixture::TOKEN)
        .map(|r| {
            fixture::form(r)
                .get("code")
                .cloned()
                .map_or_else(|| panic!("code form field"), std::convert::identity)
        })
        .collect();
    let unique: std::collections::BTreeSet<_> = codes.iter().collect();
    assert_eq!(
        (codes.len(), unique.len()),
        (4, 4),
        "one POST per distinct operation; never single-flight codes"
    );
    if !success {
        clocks.at_millis(60_000);
        f.keys.json(&jwks(key_b(), "kid-B"));
        accepted(&f, &client, "oidc-code-flight-recovery").await;
        f.assert_counts(1, 3, 5, 0).await;
    }
}

#[tokio::test]
async fn g4_successful_refresh_is_single_flight_for_concurrent_distinct_codes() {
    concurrent_refresh(true).await;
}

#[tokio::test]
async fn g4_failed_refresh_releases_all_waiters_without_herd_and_next_window_recovers() {
    concurrent_refresh(false).await;
}

#[tokio::test]
async fn g4_cancelling_the_refresh_owner_releases_the_guard_for_a_future_window() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    let client = clocks.client(&f).await;
    f.set_signed(key_b(), "kid-B", &f.claims_at(clocks.utc()));
    clocks.at_millis(30_000);
    f.keys
        .set(json_response(&jwks(key_b(), "kid-B")).set_delay(Duration::from_secs(11)));
    let mut tasks = JoinSet::new();
    let owner = Arc::clone(&client);
    tasks.spawn(async move { completed(&owner, "oidc-code-cancel-owner").await });
    f.keys.wait_count(2).await;
    tasks.abort_all();
    assert!(
        matches!(tasks.join_next().await, Some(Err(error)) if error.is_cancelled()),
        "owned request was cancelled during refresh"
    );
    f.keys.json(&jwks(key_b(), "kid-B"));
    clocks.at_millis(59_999);
    authentication_failed(&completed(&client, "oidc-code-cancel-still-consumes-cooldown").await);
    f.assert_counts(1, 2, 2, 0).await;
    clocks.at_millis(60_000);
    let resumed = must(
        tokio::time::timeout(
            Duration::from_secs(3),
            client.complete_authorization(&completion("oidc-code-after-cancel")),
        )
        .await,
        "cancelled owner must not leave a locked/inflight cache",
    );
    profile(
        &must(resumed, "fresh-window recovery after cancellation"),
        SUBJECT,
        "name",
        Some("one@example.test"),
        true,
        Some(&f.url("/avatar")),
    );
    f.assert_counts(1, 3, 3, 0).await;
}

#[tokio::test]
async fn g4_ambiguous_snapshot_does_not_trigger_unknown_key_refresh_even_outside_cooldown() {
    let f = OidcFixture::new().await;
    let clocks = Clocks::new();
    f.keys
        .json(&json!({"keys": [jwk(key_a(), KID), jwk(key_b(), KID)]}));
    f.set_claims(&f.claims_at(clocks.utc()));
    let monotone = Arc::clone(&clocks.monotone);
    let utc = Arc::clone(&clocks.utc);
    let result = OidcClient::new_with_clocks(
        &f.provider,
        secret(),
        http(),
        Arc::new(move || *monotone.lock().unwrap_or_else(PoisonError::into_inner)),
        Arc::new(move || *utc.lock().unwrap_or_else(PoisonError::into_inner)),
    )
    .await;
    match result {
        Ok(client) => {
            clocks.at_millis(30_000);
            authentication_failed(&completed(&client, "oidc-code-ambiguous-not-unknown").await);
            f.assert_counts(1, 1, 1, 0).await;
        }
        Err(error) => {
            redacted(&error.to_string());
            f.assert_counts(1, 1, 0, 0).await;
        }
    }
}
