//! S3 real outbound TLS/HMAC/PKCE protocol IT, final execution only.
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "support/owned_task.rs"]
mod owned_task;
mod utils;
#[path = "support/verified_idp_tls.rs"]
mod verified_idp_tls;

use base64::Engine;
use futures::FutureExt;
use iam_configuration::SecurityMode;
use iam_domain::port::service::FederatedOAuthClient;
use iam_infra::auth::HttpIdpConnector;
use idp_connect_contract::dto::{AuthorizeAuthenticationRequest, CompleteAuthorizationRequest};
use serial_test::serial;
use std::panic::AssertUnwindSafe;
use verified_idp_tls::{VerifiedIdpTls, HMAC_SECRET, NONCE, REDIRECT_URI, STATE, VERIFIER};

fn authentication_request(fixture: &VerifiedIdpTls) -> AuthorizeAuthenticationRequest {
    AuthorizeAuthenticationRequest {
        redirect_uri: REDIRECT_URI.into(),
        state: STATE.into(),
        nonce: NONCE.into(),
        code_challenge: Some(fixture.challenge().into()),
        code_challenge_method: Some("S256".into()),
    }
}

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
        assert_eq!(receipts.hmac, 3);
        assert_eq!(receipts.authorize, 1);
        assert_eq!(receipts.token, 1);
        assert_eq!(receipts.profile, 1);
        assert_eq!(receipts.authorize_authentication, 0);
        assert_eq!(receipts.complete_authorization, 0);
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

// G-9/G-4: the same verified transport must carry the grouped v2 protocol,
// not silently compose the historical token/profile endpoints above.
#[tokio::test]
#[serial]
async fn verified_root_and_ip_san_allow_v2_full_path_hmac_nonce_pkce_and_grouped_identity() {
    let mut fixture = VerifiedIdpTls::start(true).await;
    let result = AssertUnwindSafe(async {
        let nonce_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(NONCE)
            .expect("canonical fixture nonce");
        assert_eq!(nonce_bytes.len(), 32);
        assert!(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&nonce_bytes) == NONCE);
        assert_eq!(fixture.challenge().len(), 43);
        let client = HttpIdpConnector::with_security_mode_and_roots(
            fixture.uri(),
            HMAC_SECRET,
            SecurityMode::Verified,
            vec![fixture.root()],
        )
        .expect("verified instance-specific public root");
        let authorization = client
            .authorize_authentication(&authentication_request(&fixture))
            .await
            .expect("real HTTPS v2 authorize");
        let url = url::Url::parse(&authorization.authorization_url).expect("authorize URL");
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("provider.example"));
        assert_eq!(url.path(), "/authorize");
        assert_eq!(authorization.scope, "profile");
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert!(query.get("nonce").map(String::as_str) == Some(NONCE));
        assert!(query.get("state").map(String::as_str) == Some(STATE));
        assert!(query.get("redirect_uri").map(String::as_str) == Some(REDIRECT_URI));
        assert!(query.get("code_challenge").map(String::as_str) == Some(fixture.challenge()));
        assert_eq!(
            query.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        assert!(!query.contains_key("code_verifier"));
        let authentication = client
            .complete_authorization(&CompleteAuthorizationRequest {
                code: "fixed-test-code".into(),
                redirect_uri: REDIRECT_URI.into(),
                code_verifier: Some(VERIFIER.into()),
                expected_nonce: NONCE.into(),
            })
            .await
            .expect("real HTTPS v2 complete");
        let expected_tokens = fixtures::idp_connect::resources::success_tokens();
        assert!(authentication.tokens.access_token == expected_tokens.access_token);
        assert!(authentication.tokens.refresh_token == expected_tokens.refresh_token);
        assert_eq!(authentication.tokens.expires_in, expected_tokens.expires_in);
        let expected_profile = fixtures::idp_connect::resources::github_arthur();
        assert_eq!(authentication.profile.id, expected_profile.id);
        assert_eq!(authentication.profile.username, expected_profile.username);
        assert_eq!(authentication.profile.email, expected_profile.email);
        assert_eq!(
            authentication.profile.avatar_url,
            expected_profile.avatar_url
        );
        assert_eq!(
            authentication.profile.email_verified,
            expected_profile.email_verified
        );
        let receipts = fixture.receipts();
        assert_eq!(fixture.count(), 2, "one request per v2 operation, no retry");
        assert_eq!(
            receipts.hmac, 2,
            "HMAC verified over raw body and full /connect/v2 PATH"
        );
        assert_eq!(receipts.authorize_authentication, 1);
        assert_eq!(receipts.complete_authorization, 1);
        assert_eq!(
            receipts.authorize + receipts.token + receipts.profile,
            0,
            "no legacy fallback"
        );
    })
    .catch_unwind()
    .await;
    finish(&mut fixture, result).await;
}

// G-9: certificate failure must happen before any v2 body reaches a handler.
#[tokio::test]
#[serial]
async fn wrong_ca_denies_v2_before_any_http_handler_despite_matching_server_ip_san() {
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
            .authorize_authentication(&authentication_request(&fixture))
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

// G-9: trust the exact issuer so only the SAN mismatch can deny v2.
#[tokio::test]
#[serial]
async fn wrong_san_denies_v2_before_http_even_with_the_exact_leaf_signing_ca_trusted() {
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
            .authorize_authentication(&authentication_request(&fixture))
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

// G-9/G-4: both roots are trusted, but redirects must never forward v2 secrets.
#[tokio::test]
#[serial]
async fn verified_https_v2_redirect_does_not_forward_hmac_or_body_to_another_trusted_tls_endpoint()
{
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
            .authorize_authentication(&authentication_request(&original))
            .await
            .is_err());
        assert_eq!(original.count(), 1, "exactly one attempted operation");
        assert_eq!(
            original.receipts().hmac,
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
