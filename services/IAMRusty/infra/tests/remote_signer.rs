//! Remote HTTP `SigningProvider` proofs (ADR-0309).

#[path = "../../tests/fixtures/remote_signer/mod.rs"]
mod remote_signer_fixtures;

use iam_configuration::{JwtConfig, RemoteSignerConfig};
use iam_domain::port::{SigningProvider, WorkloadIdentity};
use iam_infra::signing::{compose_workload_identity, RemoteSigningProvider, StaticCredential};
use rustycog::testing::http::jwt::test_rs256_public_pem;
use serial_test::serial;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[tokio::test]
#[serial]
async fn remote_sign_sends_digest_not_claims() {
    let mock = remote_signer_fixtures::RemoteSignerFixtures::service().await;
    let sig = vec![9u8; 32];
    let sig_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &sig);
    mock.mock_sign_ok(&sig_b64).await;

    let wi: Arc<dyn WorkloadIdentity> =
        Arc::new(StaticCredential::from_pair("remote-signer", "s.remote").expect("pair"));
    let provider =
        RemoteSigningProvider::new(mock.base_url(), "kid-1", "RS256", "remote-signer", wi)
            .expect("remote");

    let digest = Sha256::digest(b"header.payload");
    let got = provider.sign_digest(&digest).await.expect("sign");
    assert_eq!(got, sig);

    let posts: Vec<_> = mock
        .received_requests()
        .await
        .into_iter()
        .filter(|r| r.url.path() == "/sign")
        .collect();
    let body = String::from_utf8_lossy(&posts.last().expect("sign request").body);
    assert!(body.contains("digest"), "expected digest field: {body}");
    assert!(body.contains("key_id"), "expected key_id: {body}");
    assert!(body.contains("algorithm"), "expected algorithm: {body}");
    assert!(
        !body.contains("claims") && !body.contains("\"sub\"") && !body.contains("\"iss\""),
        "must not send claims: {body}"
    );
}

#[tokio::test]
#[serial]
async fn remote_get_public_key() {
    let mock = remote_signer_fixtures::RemoteSignerFixtures::service().await;
    mock.mock_get_public_key_ok(test_rs256_public_pem()).await;

    let wi: Arc<dyn WorkloadIdentity> =
        Arc::new(StaticCredential::from_pair("remote-signer", "s.remote").expect("pair"));
    let provider =
        RemoteSigningProvider::new(mock.base_url(), "kid-1", "RS256", "remote-signer", wi)
            .expect("remote");

    let pem = provider.public_key().await.expect("pem");
    assert!(pem.contains("BEGIN PUBLIC KEY"));
    assert!(!pem.contains("PRIVATE KEY"));
}

#[tokio::test]
#[serial]
async fn remote_uses_workload_identity_header() {
    let mock = remote_signer_fixtures::RemoteSignerFixtures::service().await;
    let sig_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [1u8; 8]);
    mock.mock_sign_ok(&sig_b64).await;

    let workload = compose_workload_identity(None, "wif-remote-token", "remote-signer")
        .expect("static workload");
    let provider =
        RemoteSigningProvider::new(mock.base_url(), "kid-1", "RS256", "remote-signer", workload)
            .expect("remote");

    let digest = Sha256::digest(b"digest-input");
    let _ = provider.sign_digest(&digest).await.expect("sign");

    let req = mock
        .received_requests()
        .await
        .into_iter()
        .find(|r| r.url.path() == "/sign")
        .expect("sign request");
    let auth = req
        .headers
        .get("authorization")
        .or_else(|| req.headers.get("Authorization"))
        .expect("Authorization");
    assert_eq!(auth.to_str().expect("header"), "Bearer wif-remote-token");
}

#[test]
fn remote_url_absent_is_fail_closed() {
    let jwt = JwtConfig {
        backend: Some("remote".into()),
        remote: None,
        ..JwtConfig::default()
    };
    let err = jwt
        .remote_http_endpoint()
        .expect_err("empty url must fail-closed");
    assert!(
        err.to_string().contains("fail-closed") || err.to_string().contains("absent"),
        "{err}"
    );

    let jwt_provider = JwtConfig {
        provider: Some("remote".into()),
        remote: Some(RemoteSignerConfig {
            url: String::new(),
            key_id: "kid".into(),
            token: None,
        }),
        ..JwtConfig::default()
    };
    assert!(jwt_provider.remote_http_endpoint().is_err());

    let url_without_backend = JwtConfig {
        remote: Some(RemoteSignerConfig {
            url: "http://remote.example".into(),
            key_id: "kid".into(),
            token: None,
        }),
        ..JwtConfig::default()
    };
    assert!(url_without_backend
        .remote_http_endpoint()
        .expect("url alone must not activate remote")
        .is_none());
}

#[tokio::test]
#[serial]
async fn remote_never_returns_private_key() {
    let mock = remote_signer_fixtures::RemoteSignerFixtures::service().await;
    mock.mock_get_public_key_ok(test_rs256_public_pem()).await;
    mock.mock_sign_ok(&base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        [2u8; 4],
    ))
    .await;

    let wi: Arc<dyn WorkloadIdentity> =
        Arc::new(StaticCredential::from_pair("remote-signer", "s.remote").expect("pair"));
    let provider =
        RemoteSigningProvider::new(mock.base_url(), "kid-1", "RS256", "remote-signer", wi)
            .expect("remote");

    let pem = provider.public_key().await.expect("pem");
    assert!(!pem.contains("PRIVATE KEY"));
    assert!(pem.contains("PUBLIC KEY"));
    assert!(provider.capabilities().public_key_available);
    assert!(provider.capabilities().sign_digest);
}

#[tokio::test]
#[serial]
async fn remote_rejects_private_key_from_remote() {
    let mock = remote_signer_fixtures::RemoteSignerFixtures::service().await;
    mock.mock_get_public_key_ok(
        "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIJ8y\n-----END PRIVATE KEY-----",
    )
    .await;

    let wi: Arc<dyn WorkloadIdentity> =
        Arc::new(StaticCredential::from_pair("remote-signer", "s.remote").expect("pair"));
    let provider =
        RemoteSigningProvider::new(mock.base_url(), "kid-1", "RS256", "remote-signer", wi)
            .expect("remote");

    let err = provider
        .public_key()
        .await
        .expect_err("PRIVATE KEY from remote must be rejected");
    assert!(
        err.to_string().contains("private key")
            || err.to_string().to_ascii_lowercase().contains("private"),
        "{err}"
    );
}
