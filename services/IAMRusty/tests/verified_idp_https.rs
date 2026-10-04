//! S3 real outbound TLS/HMAC/PKCE protocol IT, final execution only.
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "support/owned_task.rs"]
mod owned_task;
mod utils;
#[path = "support/verified_idp_tls.rs"]
mod verified_idp_tls;

use futures::FutureExt;
use iam_configuration::SecurityMode;
use iam_domain::port::service::FederatedOAuthClient;
use iam_infra::auth::HttpIdpConnector;
use serial_test::serial;
use std::panic::AssertUnwindSafe;
use verified_idp_tls::{VerifiedIdpTls, HMAC_SECRET, REDIRECT_URI, STATE, VERIFIER};

async fn finish<T>(fixture: &mut VerifiedIdpTls, result: std::thread::Result<T>) -> T {
    let cleaned = fixture.shutdown().await;
    if cleaned.is_err() {
        eprintln!("owned TLS fixture cleanup failed; parent lease reconciliation required");
    }
    match result {
        Ok(value) => {
            assert!(cleaned.is_ok(), "owned TLS listener must gracefully join");
            value
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[tokio::test]
#[serial]
async fn verified_root_and_ip_san_allow_real_hmac_authorize_s256_exchange_and_profile() {
    let mut fixture = VerifiedIdpTls::start(true).await;
    let result = AssertUnwindSafe(async {
        let client = HttpIdpConnector::with_security_mode_and_roots(
            fixture.uri(),
            HMAC_SECRET,
            SecurityMode::Verified,
            vec![fixture.root()],
        )
        .expect("verified instance-specific public root");
        let authorization = client
            .authorize_with_pkce(REDIRECT_URI, STATE, Some(fixture.challenge()))
            .await
            .expect("real HTTPS authorize");
        let url = url::Url::parse(&authorization.authorization_url).expect("authorize URL");
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query.get("state").map(String::as_str), Some(STATE));
        assert_eq!(
            query.get("redirect_uri").map(String::as_str),
            Some(REDIRECT_URI)
        );
        assert_eq!(
            query.get("code_challenge").map(String::as_str),
            Some(fixture.challenge())
        );
        assert_eq!(
            query.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        let tokens = client
            .exchange_code_with_pkce("fixed-test-code", REDIRECT_URI, Some(VERIFIER))
            .await
            .expect("real HTTPS token exchange");
        let profile = client
            .user_profile(&tokens.access_token)
            .await
            .expect("real HTTPS profile");
        let expected = fixtures::idp_connect::resources::github_arthur();
        assert_eq!(profile.id, expected.id);
        assert_eq!(profile.username, expected.username);
        assert_eq!(profile.email, expected.email);
        assert_eq!(profile.email_verified, expected.email_verified);
        let receipts = fixture.receipts();
        assert_eq!(
            fixture.count(),
            3,
            "one HTTP request per operation, no retry"
        );
        assert_eq!(receipts.hmac_valid, 3);
        assert_eq!(receipts.authorize_valid, 1);
        assert_eq!(receipts.token_valid, 1);
        assert_eq!(receipts.profile_valid, 1);
        assert!(
            HttpIdpConnector::with_security_mode_and_roots(
                "http://127.0.0.1/connect",
                HMAC_SECRET,
                SecurityMode::Verified,
                vec![fixture.root()]
            )
            .is_err(),
            "additional roots must not admit plaintext Verified transport"
        );
    })
    .catch_unwind()
    .await;
    finish(&mut fixture, result).await;
}

#[tokio::test]
#[serial]
async fn wrong_ca_denies_before_any_http_handler_despite_matching_server_ip_san() {
    let mut fixture = VerifiedIdpTls::start(true).await;
    let result = AssertUnwindSafe(async {
        let client = HttpIdpConnector::with_security_mode_and_roots(
            fixture.uri(),
            HMAC_SECRET,
            SecurityMode::Verified,
            vec![verified_idp_tls::unrelated_root()],
        )
        .expect("separate untrusted-root client");
        assert!(client
            .authorize_with_pkce(REDIRECT_URI, STATE, Some(fixture.challenge()))
            .await
            .is_err());
        assert_eq!(
            fixture.count(),
            0,
            "certificate rejection must precede HTTP/body/HMAC handling"
        );
    })
    .catch_unwind()
    .await;
    finish(&mut fixture, result).await;
}

#[tokio::test]
#[serial]
async fn wrong_san_denies_before_http_even_with_the_exact_leaf_signing_ca_trusted() {
    let mut fixture = VerifiedIdpTls::start(false).await;
    let result = AssertUnwindSafe(async {
        // Same certificate authority as the real server: a CA failure cannot mask
        // this name mismatch. URL uses 127.0.0.1; leaf has only wrong-san.test.
        let client = HttpIdpConnector::with_security_mode_and_roots(
            fixture.uri(),
            HMAC_SECRET,
            SecurityMode::Verified,
            vec![fixture.root()],
        )
        .expect("correct root, wrong leaf SAN");
        assert!(client
            .authorize_with_pkce(REDIRECT_URI, STATE, Some(fixture.challenge()))
            .await
            .is_err());
        assert_eq!(
            fixture.count(),
            0,
            "hostname rejection must precede HTTP/body/HMAC handling"
        );
    })
    .catch_unwind()
    .await;
    finish(&mut fixture, result).await;
}

#[tokio::test]
#[serial]
async fn verified_https_redirect_does_not_forward_hmac_or_body_to_another_trusted_tls_endpoint() {
    let mut original = VerifiedIdpTls::start(true).await;
    // Include both roots so a hypothetical redirect follow cannot fail merely
    // because the alternate's certificate is untrusted.
    let setup = AssertUnwindSafe(VerifiedIdpTls::start(true))
        .catch_unwind()
        .await;
    let mut alternate = match setup {
        Ok(fixture) => fixture,
        Err(panic) => {
            let _ = original.shutdown().await;
            std::panic::resume_unwind(panic)
        }
    };
    original.redirect_to(&alternate);
    let result = AssertUnwindSafe(async {
        let client = HttpIdpConnector::with_security_mode_and_roots(
            original.uri(),
            HMAC_SECRET,
            SecurityMode::Verified,
            vec![original.root(), alternate.root()],
        )
        .expect("two trusted instance-specific test roots");
        assert!(client
            .authorize_with_pkce(REDIRECT_URI, STATE, Some(original.challenge()))
            .await
            .is_err());
        assert_eq!(original.count(), 1, "exactly one attempted operation");
        assert_eq!(
            original.receipts().hmac_valid,
            1,
            "redirect response must be reached by a real authenticated request"
        );
        assert_eq!(
            alternate.count(),
            0,
            "no HMAC/credentials/body may reach the Location endpoint"
        );
    })
    .catch_unwind()
    .await;
    let alternate_cleanup = alternate.shutdown().await;
    let original_cleanup = original.shutdown().await;
    match result {
        Ok(()) => assert!(
            alternate_cleanup.is_ok() && original_cleanup.is_ok(),
            "both owned TLS fixtures must join"
        ),
        Err(panic) => {
            if alternate_cleanup.is_err() || original_cleanup.is_err() {
                eprintln!("TLS redirect fixtures cleanup failed; parent reconciliation required");
            }
            std::panic::resume_unwind(panic)
        }
    }
}
