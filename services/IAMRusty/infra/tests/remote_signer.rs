//! Remote HTTP `SigningProvider` proofs (ADR-0309).

#[path = "../../tests/fixtures/remote_signer/mod.rs"]
mod remote_signer_fixtures;

use iam_configuration::{JwtConfig, RemoteSignerConfig, SecurityMode};
use iam_domain::error::DomainError;
use iam_domain::port::WorkloadCredential;
use iam_domain::port::{SigningProvider, WorkloadIdentity};
use iam_infra::signing::{compose_workload_identity, RemoteSigningProvider, StaticCredential};
use rustycog::testing::http::jwt::test_rs256_public_pem;
use serial_test::serial;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Default)]
struct CountingIdentity(AtomicUsize);

fn local_provider(url: &str) -> RemoteSigningProvider {
    RemoteSigningProvider::new_with_security_mode(
        url,
        "kid-1",
        "RS256",
        "remote-signer",
        Arc::new(StaticCredential::from_pair("remote-signer", "synthetic-token").unwrap()),
        SecurityMode::IsolatedTest,
    )
    .unwrap()
}

#[tokio::test]
async fn remote_rejects_premature_eof() {
    use remote_signer_fixtures::service::{Framing, RawResponse, RawSigner};
    for framing in [Framing::Length, Framing::Chunked] {
        for public in [false, true] {
            let mut response = RawResponse::ok(
                serde_json::json!({"signature":"AQ==", "public_key":test_rs256_public_pem()})
                    .to_string()
                    .into_bytes(),
                framing,
            );
            response.truncate_after = Some(10);
            let raw = RawSigner::start(response).await;
            let provider = local_provider(&raw.url);
            let refused = if public {
                provider.public_key().await.is_err()
            } else {
                provider.sign_digest(&[0; 32]).await.is_err()
            };
            raw.shutdown().await;
            assert!(refused);
        }
    }
}

#[tokio::test]
#[serial]
async fn remote_key_id_stays_one_encoded_segment() {
    let mock = remote_signer_fixtures::RemoteSignerFixtures::service().await;
    mock.reset().await;
    mock.mock_response(
        "GET",
        "/base/keys/a%2Fb%3Fq%23f",
        wiremock::ResponseTemplate::new(200).set_body_string(test_rs256_public_pem()),
    )
    .await;
    let provider = RemoteSigningProvider::new_with_security_mode(
        format!("{}/base/", mock.base_url()),
        "a/b?q#f",
        "RS256",
        "ref",
        Arc::new(StaticCredential::from_pair("ref", "synthetic-token").unwrap()),
        SecurityMode::IsolatedTest,
    )
    .unwrap();
    assert!(provider.public_key().await.is_ok());
    let requests = mock.received_requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/base/keys/a%2Fb%3Fq%23f");
    assert!(requests[0].url.query().is_none());
    assert!(requests[0].url.fragment().is_none());
}

#[tokio::test]
#[serial]
async fn remote_accepts_rsa8192_public_material() {
    use rsa::{
        pkcs8::{EncodePublicKey, LineEnding},
        traits::PublicKeyParts,
        BigUint, RsaPublicKey,
    };
    // Public-format/size witness only: not proof that a vendor possesses a
    // matching private key, nor an RSA signature qualification.
    let key = RsaPublicKey::new_with_max_size(
        BigUint::from_bytes_be(&[0xa5; 1024]),
        BigUint::from(65537u32),
        8192,
    )
    .unwrap();
    let public = key.to_public_key_pem(LineEnding::LF).unwrap();
    let mock = remote_signer_fixtures::RemoteSignerFixtures::service().await;
    mock.reset().await;
    mock.mock_get_public_key_ok(&public).await;
    let received = local_provider(&mock.base_url()).public_key().await.unwrap();
    assert_eq!(received.trim(), public.trim());
    assert_eq!(key.n().bits(), 8192);
}

#[tokio::test]
#[serial]
async fn remote_redirects_never_reach_target() {
    use wiremock::ResponseTemplate;
    let mock = remote_signer_fixtures::RemoteSignerFixtures::service().await;
    let other = remote_signer_fixtures::service::RawSigner::start(
        remote_signer_fixtures::service::RawResponse::ok(
            vec![],
            remote_signer_fixtures::service::Framing::Length,
        ),
    )
    .await;
    for status in [301, 302, 303, 307, 308] {
        for location in [
            "/redirect-target".to_owned(),
            format!("{}/redirect-target", other.url),
        ] {
            for (method, path) in [("POST", "/sign"), ("GET", "/keys/kid-1")] {
                mock.reset().await;
                mock.mock_response(
                    method,
                    path,
                    ResponseTemplate::new(status).insert_header("Location", location.as_str()),
                )
                .await;
                let provider = local_provider(&mock.base_url());
                let refused = if method == "POST" {
                    provider.sign_digest(&[0; 32]).await.is_err()
                } else {
                    provider.public_key().await.is_err()
                };
                assert!(refused);
                let requests = mock.received_requests().await;
                assert_eq!(requests.len(), 1);
                assert_eq!(requests[0].url.path(), path);
                assert_eq!(other.requests.load(Ordering::SeqCst), 0);
            }
        }
    }
    other.shutdown().await;
}

#[tokio::test]
async fn remote_success_bodies_are_bounded_without_content_length() {
    use remote_signer_fixtures::service::{Framing, RawResponse, RawSigner};
    for framing in [Framing::Length, Framing::Chunked, Framing::Close] {
        for (body, cap, public) in [
            ("{\"signature\":\"AQ==\"}".to_owned(), 4096, false),
            (
                serde_json::json!({"public_key":test_rs256_public_pem()}).to_string(),
                16384,
                true,
            ),
            (test_rs256_public_pem().to_owned(), 16384, true),
        ] {
            for extra in [0, 1] {
                let mut bytes = body.as_bytes().to_vec();
                bytes.resize(cap + extra, b' ');
                let raw = RawSigner::start(RawResponse::ok(bytes, framing)).await;
                let provider = local_provider(&raw.url);
                let accepted = if public {
                    provider.public_key().await.is_ok()
                } else {
                    provider.sign_digest(&[0; 32]).await.is_ok()
                };
                let seen = raw.requests.load(Ordering::SeqCst);
                raw.shutdown().await;
                assert_eq!(seen, 1);
                assert_eq!(accepted, extra == 0);
            }
        }
    }
}

#[tokio::test]
async fn remote_rejects_encoded_responses() {
    use remote_signer_fixtures::service::{Framing, RawResponse, RawSigner};
    for encoding in ["gzip", "br", "deflate", "identity, gzip"] {
        for public in [false, true] {
            let mut response = RawResponse::ok(
                serde_json::json!({"signature":"AQ==", "public_key":test_rs256_public_pem()})
                    .to_string()
                    .into_bytes(),
                Framing::Chunked,
            );
            response
                .headers
                .push(("Content-Encoding".into(), encoding.into()));
            let raw = RawSigner::start(response).await;
            let provider = local_provider(&raw.url);
            let refused = if public {
                provider.public_key().await.is_err()
            } else {
                provider.sign_digest(&[0; 32]).await.is_err()
            };
            raw.shutdown().await;
            assert!(refused);
        }
    }
}

#[tokio::test]
async fn remote_http_errors_are_not_read_or_reflected() {
    use remote_signer_fixtures::service::{Framing, RawResponse, RawSigner};
    for status in [302, 400, 503] {
        for public in [false, true] {
            let mut response =
                RawResponse::ok(b"hostile-response-sentinel".repeat(10000), Framing::Chunked);
            response.status = status;
            response.chunk_delay = std::time::Duration::from_secs(20);
            let raw = RawSigner::start(response).await;
            let provider = local_provider(&raw.url);
            let start = std::time::Instant::now();
            let error = if public {
                provider.public_key().await.unwrap_err().to_string()
            } else {
                provider
                    .sign_digest(&[0; 32])
                    .await
                    .unwrap_err()
                    .to_string()
            };
            let elapsed = start.elapsed();
            raw.shutdown().await;
            // Generous scheduling tolerance, still well below the adapter's 10s deadline.
            assert!(elapsed < std::time::Duration::from_secs(5));
            assert!(!error.contains("hostile-response-sentinel"));
            assert!(error.contains(&status.to_string()));
        }
    }
}

struct HostileIdentity(&'static str);

struct SuspendedIdentity;
#[async_trait::async_trait]
impl WorkloadIdentity for SuspendedIdentity {
    async fn resolve(&self, _: &str) -> Result<WorkloadCredential, DomainError> {
        std::future::pending().await
    }
}

struct DelayedIdentity {
    delay: std::time::Duration,
    resolutions: AtomicUsize,
    credential: StaticCredential,
}

#[async_trait::async_trait]
impl WorkloadIdentity for DelayedIdentity {
    async fn resolve(&self, reference: &str) -> Result<WorkloadCredential, DomainError> {
        self.resolutions.fetch_add(1, Ordering::SeqCst);
        // Deliberate collaborator latency, not a sleep used to synchronize a test.
        tokio::time::sleep(self.delay).await;
        self.credential.resolve(reference).await
    }
}

struct RemoteDeadlineObservation {
    result: Result<Vec<u8>, DomainError>,
    elapsed: std::time::Duration,
    requests: usize,
    resolutions: usize,
}

async fn observe_cumulative_remote_deadline(
    public: bool,
    delay: std::time::Duration,
) -> RemoteDeadlineObservation {
    use remote_signer_fixtures::service::{Framing, RawResponse, RawSigner};
    use std::time::Instant;

    let mut response = RawResponse::ok(
        serde_json::json!({"signature": "AQ==", "public_key": test_rs256_public_pem()})
            .to_string()
            .into_bytes(),
        Framing::Length,
    );
    response.header_delay = delay;
    // RawSigner accepts one connection: each Sign/GET operation gets its own
    // existing fixture, including when the two observations run concurrently.
    let raw = RawSigner::start(response).await;
    let identity = Arc::new(DelayedIdentity {
        delay,
        resolutions: AtomicUsize::new(0),
        credential: StaticCredential::from_pair("ref", "synthetic-deadline-credential").unwrap(),
    });
    let provider = RemoteSigningProvider::new_with_security_mode(
        &raw.url,
        "key",
        "RS256",
        "ref",
        identity.clone(),
        SecurityMode::IsolatedTest,
    )
    .unwrap();
    let start = Instant::now();
    let result = if public {
        provider.public_key().await.map(String::into_bytes)
    } else {
        provider.sign_digest(&[0; 32]).await
    };
    let observation = RemoteDeadlineObservation {
        result,
        elapsed: start.elapsed(),
        requests: raw.requests.load(Ordering::SeqCst),
        resolutions: identity.resolutions.load(Ordering::SeqCst),
    };
    raw.shutdown().await;
    observation
}

// T-4: resetting the 10s budget after WIF would incorrectly succeed at 14s.
#[tokio::test]
async fn remote_deadline_is_shared_between_identity_and_http() {
    use std::time::Duration;

    for (delay, succeeds) in [
        (Duration::from_secs(1), true),
        (Duration::from_secs(7), false),
    ] {
        let (sign, public) = tokio::join!(
            observe_cumulative_remote_deadline(false, delay),
            observe_cumulative_remote_deadline(true, delay),
        );
        for (operation, observation, expected) in [
            ("Sign", sign, vec![1]),
            (
                "GetPublicKey",
                public,
                test_rs256_public_pem().trim().as_bytes().to_vec(),
            ),
        ] {
            assert_eq!(
                observation.resolutions, 1,
                "{operation} must resolve credentials once"
            );
            assert_eq!(
                observation.requests, 1,
                "{operation} must reach HTTP after successful WIF, without retries"
            );
            if succeeds {
                assert_eq!(
                    observation
                        .result
                        .expect("cumulative 1s WIF + 1s HTTP must succeed"),
                    expected,
                    "{operation} positive witness must return the exact signer material"
                );
            } else {
                assert!(
                    observation.result.is_err(),
                    "{operation} must reject cumulative 7s WIF + 7s HTTP"
                );
                // Existing Windows scheduling tolerance, not a new service SLA.
                assert!(
                    (Duration::from_secs(9)..Duration::from_secs(13))
                        .contains(&observation.elapsed),
                    "{operation} must share a single 10s deadline; elapsed {:?}",
                    observation.elapsed
                );
            }
        }
    }
}

#[tokio::test]
async fn remote_operation_deadline_covers_identity_and_body() {
    use remote_signer_fixtures::service::{Framing, RawResponse, RawSigner};
    use std::time::{Duration, Instant};
    // Sign and GET share the same budget, exercised concurrently to keep this
    // real-time regression bounded. No paused clock can hide socket behavior.
    let identity_target = RawSigner::start(RawResponse::ok(Vec::new(), Framing::Length)).await;
    let provider = RemoteSigningProvider::new_with_security_mode(
        &identity_target.url,
        "key",
        "RS256",
        "ref",
        Arc::new(SuspendedIdentity),
        SecurityMode::IsolatedTest,
    )
    .unwrap();
    let start = Instant::now();
    let (sign, public) = tokio::join!(provider.sign_digest(&[0; 32]), provider.public_key());
    let requests = identity_target.requests.load(Ordering::SeqCst);
    identity_target.shutdown().await;
    assert!(sign.is_err() && public.is_err());
    assert_eq!(requests, 0);
    // 3s upper tolerance is Windows scheduling slack, not a service SLA.
    assert!((Duration::from_secs(9)..Duration::from_secs(13)).contains(&start.elapsed()));
    for delay_headers in [true, false] {
        for public in [false, true] {
            let mut response = RawResponse::ok(
                serde_json::json!({"signature":"AQ==", "public_key":test_rs256_public_pem()})
                    .to_string()
                    .into_bytes(),
                Framing::Chunked,
            );
            if delay_headers {
                response.header_delay = Duration::from_secs(20);
            } else {
                response.body.resize(4096, b' ');
                response.chunk_delay = Duration::from_secs(1);
            }
            let raw = RawSigner::start(response).await;
            let provider = local_provider(&raw.url);
            let start = Instant::now();
            let refused = if public {
                provider.public_key().await.is_err()
            } else {
                provider.sign_digest(&[0; 32]).await.is_err()
            };
            let elapsed = start.elapsed();
            let requests = raw.requests.load(Ordering::SeqCst);
            raw.shutdown().await;
            assert!(refused);
            assert_eq!(requests, 1);
            assert!((Duration::from_secs(9)..Duration::from_secs(13)).contains(&elapsed));
        }
    }
}
#[async_trait::async_trait]
impl WorkloadIdentity for HostileIdentity {
    async fn resolve(&self, _: &str) -> Result<WorkloadCredential, DomainError> {
        Ok(WorkloadCredential {
            secret: self.0.into(),
        })
    }
}

#[tokio::test]
async fn remote_authorization_rejects_invalid_credentials_without_leak() {
    for secret in [
        "",
        " ",
        "Bearer ",
        "hostile-credential-sentinel\r\nInjected: yes",
    ] {
        let provider = RemoteSigningProvider::new(
            "https://signer.example",
            "key",
            "RS256",
            "ref",
            Arc::new(HostileIdentity(secret)),
        )
        .unwrap();
        for error in [
            provider
                .sign_digest(&[0; 32])
                .await
                .unwrap_err()
                .to_string(),
            provider.public_key().await.unwrap_err().to_string(),
        ] {
            assert!(!error.contains("hostile-credential-sentinel"));
            assert!(error.contains("invalid credential"));
        }
    }
}

#[tokio::test]
async fn remote_errors_redact_hostile_inputs() {
    let wi = Arc::new(CountingIdentity::default());
    let provider = RemoteSigningProvider::new(
        "https://hostile-url-sentinel.example",
        "hostile-key-sentinel",
        "RS256",
        "hostile-ref-sentinel",
        wi,
    )
    .unwrap();
    for error in [
        provider
            .sign_digest(&[0; 32])
            .await
            .unwrap_err()
            .to_string(),
        provider.public_key().await.unwrap_err().to_string(),
    ] {
        for sentinel in [
            "hostile-url-sentinel",
            "hostile-key-sentinel",
            "hostile-ref-sentinel",
            "hostile-credential-sentinel",
        ] {
            assert!(!error.contains(sentinel));
        }
    }
}

#[async_trait::async_trait]
impl WorkloadIdentity for CountingIdentity {
    async fn resolve(&self, _: &str) -> Result<WorkloadCredential, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(DomainError::external_service_error(
            "fixture",
            "hostile-credential-sentinel",
        ))
    }
}

#[test]
fn remote_constructor_is_secure_by_default() {
    let wi = Arc::new(CountingIdentity::default());
    for endpoint in [
        "http://127.0.0.1:1234",
        "http://[::1]:1234",
        "https://user@signer.example",
        "https://signer.example?",
        "https://signer.example#",
    ] {
        assert!(RemoteSigningProvider::new(endpoint, "key", "RS256", "ref", wi.clone()).is_err());
    }
    assert!(RemoteSigningProvider::new(
        "https://signer.example/prefix",
        "key",
        "RS256",
        "ref",
        wi.clone()
    )
    .is_ok());
    assert_eq!(wi.0.load(Ordering::SeqCst), 0);
}

#[test]
fn remote_rejects_unsupported_algorithm_before_io() {
    let wi = Arc::new(CountingIdentity::default());
    for algorithm in ["HS256", "PS256", "none", ""] {
        assert!(RemoteSigningProvider::new(
            "https://signer.example",
            "key",
            algorithm,
            "ref",
            wi.clone()
        )
        .is_err());
    }
    assert_eq!(wi.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn remote_rejects_invalid_digest_before_credentials_or_io() {
    let wi = Arc::new(CountingIdentity::default());
    let provider =
        RemoteSigningProvider::new("https://signer.example", "key", "RS256", "ref", wi.clone())
            .unwrap();
    for length in [0, 31, 33, 64] {
        assert!(provider.sign_digest(&vec![0; length]).await.is_err());
    }
    assert_eq!(wi.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
#[serial]
async fn remote_sign_sends_digest_not_claims() {
    let mock = remote_signer_fixtures::RemoteSignerFixtures::service().await;
    let sig = vec![9u8; 32];
    let sig_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &sig);
    mock.mock_sign_ok(&sig_b64).await;

    let wi: Arc<dyn WorkloadIdentity> =
        Arc::new(StaticCredential::from_pair("remote-signer", "s.remote").expect("pair"));
    let provider = RemoteSigningProvider::new_with_security_mode(
        mock.base_url(),
        "kid-1",
        "RS256",
        "remote-signer",
        wi,
        SecurityMode::IsolatedTest,
    )
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
    let provider = RemoteSigningProvider::new_with_security_mode(
        mock.base_url(),
        "kid-1",
        "RS256",
        "remote-signer",
        wi,
        SecurityMode::IsolatedTest,
    )
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
    let provider = RemoteSigningProvider::new_with_security_mode(
        mock.base_url(),
        "kid-1",
        "RS256",
        "remote-signer",
        workload,
        SecurityMode::IsolatedTest,
    )
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
    let provider = RemoteSigningProvider::new_with_security_mode(
        mock.base_url(),
        "kid-1",
        "RS256",
        "remote-signer",
        wi,
        SecurityMode::IsolatedTest,
    )
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
    let provider = RemoteSigningProvider::new_with_security_mode(
        mock.base_url(),
        "kid-1",
        "RS256",
        "remote-signer",
        wi,
        SecurityMode::IsolatedTest,
    )
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
