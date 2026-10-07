//! S9 outbound HTTP protocol tests. No live OpenBao, ACL or HTTPS deployment claim.
use iam_domain::port::SigningProvider;
use iam_infra::signing::{StaticCredential, TransitSigningProvider};
use rustycog::testing::wiremock::MockServerFixture;
use serial_test::serial;
use std::{sync::Arc, time::Duration};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

struct TransitMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl TransitMockService {
    async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        Self {
            server: fixture.server(),
            _fixture: fixture,
        }
    }
    async fn sign_response(&self, key: &str, response: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path(format!("/v1/transit/sign/{key}")))
            .respond_with(response)
            .mount(&self.server)
            .await;
    }
    fn provider(&self, key: &str, credential: &str) -> TransitSigningProvider {
        self.provider_at(key, 1, credential)
    }
    fn provider_at(&self, key: &str, version: u32, credential: &str) -> TransitSigningProvider {
        TransitSigningProvider::new(
            self.server.uri(),
            key,
            version,
            credential,
            Arc::new(
                StaticCredential::from_pair(
                    "org-fixture-credential",
                    "nonsecret-fixed-test-credential",
                )
                .expect("fixture credential"),
            ),
            None,
        )
        .expect("scoped client")
    }
}

#[tokio::test]
#[serial]
async fn missing_organization_credential_never_falls_back_or_calls_transit() {
    let fixture = TransitMockService::new().await;
    let key = format!("org-{}-signer", uuid::Uuid::new_v4());
    assert!(fixture
        .provider(&key, "missing-org-credential")
        .sign_digest(&[1; 32])
        .await
        .is_err());
    assert!(fixture
        .server
        .received_requests()
        .await
        .expect("request recording")
        .is_empty());
}

#[tokio::test]
#[serial]
async fn redirects_cannot_forward_credentials_to_an_alternate_operation() {
    let fixture = TransitMockService::new().await;
    let key = format!("org-{}-signer", uuid::Uuid::new_v4());
    let provider = fixture.provider(&key, "org-fixture-credential");
    fixture
        .sign_response(
            &key,
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"data": {"signature": "vault:v1:AQID"}})),
        )
        .await;
    assert_eq!(
        provider
            .sign_digest(&[1; 32])
            .await
            .expect("positive Sign protocol control"),
        [1, 2, 3]
    );
    fixture._fixture.reset().await;
    fixture
        .sign_response(
            &key,
            ResponseTemplate::new(307).insert_header(
                "location",
                format!("{}/v1/transit/export/foreign", fixture.server.uri()),
            ),
        )
        .await;
    assert!(provider.sign_digest(&[1; 32]).await.is_err());
    assert_eq!(
        fixture
            .server
            .received_requests()
            .await
            .expect("requests")
            .len(),
        1
    );
}

#[tokio::test]
#[serial]
async fn transit_sign_has_bounded_timeout_without_alternate_retry() {
    let fixture = TransitMockService::new().await;
    let key = format!("org-{}-signer", uuid::Uuid::new_v4());
    fixture
        .sign_response(
            &key,
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(30))
                .set_body_json(serde_json::json!({"data": {"signature": "vault:v1:AQID"}})),
        )
        .await;
    let result = tokio::time::timeout(
        Duration::from_secs(15),
        fixture
            .provider(&key, "org-fixture-credential")
            .sign_digest(&[1; 32]),
    )
    .await
    .expect("client's 10-second timeout must precede the 15-second guard");
    assert!(result.is_err());
    assert_eq!(
        fixture
            .server
            .received_requests()
            .await
            .expect("requests")
            .len(),
        1
    );
}

#[tokio::test]
#[serial]
async fn unsafe_transit_paths_are_refused_before_any_http_request() {
    let fixture = TransitMockService::new().await;
    for key in [
        "../platform",
        "parent/foreign",
        "%2Fplatform",
        "apparatus-p4-cosign",
        "",
    ] {
        assert!(TransitSigningProvider::new(
            fixture.server.uri(),
            key,
            1,
            "org-fixture-credential",
            Arc::new(StaticCredential::default()),
            None
        )
        .is_err());
    }
    assert!(fixture
        .server
        .received_requests()
        .await
        .expect("requests")
        .is_empty());
}

#[tokio::test]
#[serial]
async fn explicit_version_is_sent_and_only_the_exact_vault_envelope_is_accepted() {
    let fixture = TransitMockService::new().await;
    let provider = fixture.provider_at("pinned-key", 7, "org-fixture-credential");
    for (envelope, valid) in [
        ("vault:v7:AQID", true),
        ("vault:v8:AQID", false),
        ("vault:v1:AQID", false),
        ("vault:v07:AQID", false),
        ("other:v7:AQID", false),
        ("AQID", false),
        ("vault:v7:AQID:extra", false),
        ("vault:v7:", false),
        ("vault:v7:!!!", false),
    ] {
        fixture._fixture.reset().await;
        fixture
            .sign_response(
                "pinned-key",
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"data":{"signature":envelope}})),
            )
            .await;
        let result = provider.sign_digest(&[5; 32]).await;
        if valid {
            assert_eq!(result.expect("exact version"), [1, 2, 3]);
        } else {
            assert!(result.is_err(), "must reject {envelope}");
        }
        let requests = fixture.server.received_requests().await.expect("recording");
        assert_eq!(requests.len(), 1, "no alternate operation or version retry");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("JSON Sign");
        assert_eq!(body["key_version"], 7);
        assert_eq!(body["prehashed"], true);
        assert_eq!(body["hash_algorithm"], "sha2-256");
        assert_eq!(body["signature_algorithm"], "pkcs1v15");
        assert_eq!(
            body["input"],
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [5; 32])
        );
    }
}

#[tokio::test]
#[serial]
async fn invalid_version_and_non_sha256_digest_fail_before_network() {
    let fixture = TransitMockService::new().await;
    assert!(TransitSigningProvider::new(
        fixture.server.uri(),
        "key",
        0,
        "org-fixture-credential",
        Arc::new(StaticCredential::default()),
        None
    )
    .is_err());
    let provider = fixture.provider_at("key", 7, "org-fixture-credential");
    for digest in [&[][..], &[1; 31][..], &[1; 33][..]] {
        assert!(provider.sign_digest(digest).await.is_err());
    }
    assert!(fixture
        .server
        .received_requests()
        .await
        .expect("recording")
        .is_empty());
}

#[tokio::test]
#[serial]
async fn public_read_is_exact_even_after_external_rotation_and_never_uses_latest_fallback() {
    let fixture = TransitMockService::new().await;
    let provider = fixture.provider_at("key", 7, "org-fixture-credential");
    let public = include_str!("../config/keys/test-platform.pub");
    for latest in [7, 8, 42] {
        fixture._fixture.reset().await;
        Mock::given(method("GET"))
            .and(path("/v1/transit/keys/key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data":{"latest_version":latest,"keys":{
                    "1":{"public_key":"wrong old public"},
                    "7":{"public_key":public},"8":{"public_key":"wrong latest public"}
                },"latest_public_key":"never use this"}
            })))
            .mount(&fixture.server)
            .await;
        assert_eq!(provider.public_key().await.expect("pinned public"), public);
    }
    for data in [
        serde_json::json!({"keys":{"8":{"public_key":public}},"latest_public_key":public}),
        serde_json::json!({"keys":{"7":{}},"latest_public_key":public}),
        serde_json::json!({"keys":{"7":{"public_key":""}},"latest_public_key":public}),
    ] {
        fixture._fixture.reset().await;
        Mock::given(method("GET"))
            .and(path("/v1/transit/keys/key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":data})),
            )
            .mount(&fixture.server)
            .await;
        assert!(
            provider.public_key().await.is_err(),
            "missing pinned version is not latest"
        );
    }
}

#[tokio::test]
#[serial]
async fn supplied_public_is_confronted_with_exact_provider_version() {
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    let fixture = TransitMockService::new().await;
    let public = include_str!("../config/keys/test-platform.pub");
    let provider = TransitSigningProvider::new(
        fixture.server.uri(),
        "key",
        7,
        "org-fixture-credential",
        Arc::new(StaticCredential::from_pair("org-fixture-credential", "fixture-token").unwrap()),
        Some(public.into()),
    )
    .unwrap();
    Mock::given(method("GET"))
        .and(path("/v1/transit/keys/key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data":{"keys":{"7":{"public_key":public}}}
        })))
        .mount(&fixture.server)
        .await;
    assert_eq!(provider.public_key().await.unwrap(), public);
    fixture._fixture.reset().await;
    Mock::given(method("GET"))
        .and(path("/v1/transit/keys/key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data":{"keys":{"8":{"public_key":public}}}
        })))
        .mount(&fixture.server)
        .await;
    assert!(
        provider.public_key().await.is_err(),
        "cached public cannot hide missing provider version"
    );
    fixture._fixture.reset().await;
    let different = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048)
        .unwrap()
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    assert_ne!(different, public);
    Mock::given(method("GET"))
        .and(path("/v1/transit/keys/key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data":{"keys":{"7":{"public_key":different}}}
        })))
        .mount(&fixture.server)
        .await;
    assert!(
        provider.public_key().await.is_err(),
        "same version with different public material is a binding violation"
    );
}

#[tokio::test]
#[serial]
async fn rotation_uses_same_key_name_and_rejects_ambiguous_successor_without_retry() {
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    use std::sync::atomic::{AtomicBool, Ordering};
    let next_public = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048)
        .unwrap()
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    for observed_after in [8, 9] {
        let fixture = TransitMockService::new().await;
        let rotated = Arc::new(AtomicBool::new(false));
        let for_get = rotated.clone();
        let public = include_str!("../config/keys/test-platform.pub");
        let successor = next_public.clone();
        Mock::given(method("GET")).and(path("/v1/transit/keys/key"))
            .respond_with(move |_: &wiremock::Request| {
                let latest = if for_get.load(Ordering::SeqCst) { observed_after } else { 7 };
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{
                    "type":"rsa-2048","exportable":false,"supports_signing":true,
                    "latest_version":latest,"keys":{"7":{"public_key":public},"8":{"public_key":successor}}
                }}))
            }).mount(&fixture.server).await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/keys/key/rotate"))
            .respond_with(move |_: &wiremock::Request| {
                rotated.store(true, Ordering::SeqCst);
                ResponseTemplate::new(204)
            })
            .mount(&fixture.server)
            .await;
        let result = fixture
            .provider_at("key", 7, "org-fixture-credential")
            .rotate_rsa2048_key()
            .await;
        if observed_after == 8 {
            assert_eq!(result.unwrap(), 8);
            assert_eq!(
                fixture
                    .provider_at("key", 8, "org-fixture-credential")
                    .public_key()
                    .await
                    .unwrap(),
                next_public
            );
            assert_eq!(
                fixture
                    .provider_at("key", 7, "org-fixture-credential")
                    .public_key()
                    .await
                    .unwrap(),
                public
            );
        } else {
            assert!(
                result.is_err(),
                "latest alone does not identify our successor after competing rotations"
            );
        }
        let requests = fixture.server.received_requests().await.unwrap();
        assert_eq!(requests.len(), if observed_after == 8 { 5 } else { 3 });
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.method.as_str() == "POST")
                .count(),
            1
        );
        assert_eq!(requests[1].url.path(), "/v1/transit/keys/key/rotate");
    }
}

#[tokio::test]
#[serial]
async fn returned_wrong_version_is_rejected_even_if_the_signature_is_mathematically_valid() {
    use rsa::pkcs8::DecodePrivateKey;
    use sha2::{Digest, Sha256};
    let fixture = TransitMockService::new().await;
    let private =
        rsa::RsaPrivateKey::from_pkcs8_pem(include_str!("../config/keys/test-platform.pem"))
            .unwrap();
    let digest = Sha256::digest(b"version-pinned input");
    let signature = private
        .sign(rsa::pkcs1v15::Pkcs1v15Sign::new::<Sha256>(), &digest)
        .unwrap();
    private
        .to_public_key()
        .verify(
            rsa::pkcs1v15::Pkcs1v15Sign::new::<Sha256>(),
            &digest,
            &signature,
        )
        .unwrap();
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, signature);
    fixture
        .sign_response(
            "key",
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data":{"signature":format!("vault:v8:{b64}")}
            })),
        )
        .await;
    assert!(fixture
        .provider_at("key", 7, "org-fixture-credential")
        .sign_digest(&digest)
        .await
        .is_err());
}

#[tokio::test]
#[serial]
async fn rotation_refuses_incompatible_or_already_advanced_key_before_side_effect() {
    let fixture = TransitMockService::new().await;
    let public = include_str!("../config/keys/test-platform.pub");
    for (key_type, exportable, signing, latest) in [
        ("rsa-2048", false, true, 8),
        ("rsa-4096", false, true, 7),
        ("rsa-2048", true, true, 7),
        ("rsa-2048", false, false, 7),
    ] {
        fixture._fixture.reset().await;
        Mock::given(method("GET"))
            .and(path("/v1/transit/keys/key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{
                    "type":key_type,"exportable":exportable,"supports_signing":signing,
                    "latest_version":latest,"keys":{"7":{"public_key":public}}
                }})),
            )
            .mount(&fixture.server)
            .await;
        assert!(fixture
            .provider_at("key", 7, "org-fixture-credential")
            .rotate_rsa2048_key()
            .await
            .is_err());
        let requests = fixture.server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "GET");
    }
}

#[tokio::test]
#[serial]
async fn creation_checks_absence_properties_and_exact_initial_version() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let fixture = TransitMockService::new().await;
    let created = Arc::new(AtomicBool::new(false));
    let for_read = created.clone();
    let public = include_str!("../config/keys/test-platform.pub");
    Mock::given(method("GET"))
        .and(path("/v1/transit/keys/new-key"))
        .respond_with(move |_: &wiremock::Request| {
            if !for_read.load(Ordering::SeqCst) {
                return ResponseTemplate::new(404);
            }
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{
                "type":"rsa-2048","exportable":false,"supports_signing":true,
                "latest_version":1,"keys":{"1":{"public_key":public}}
            }}))
        })
        .mount(&fixture.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/transit/keys/new-key"))
        .respond_with(move |_: &wiremock::Request| {
            created.store(true, Ordering::SeqCst);
            ResponseTemplate::new(204)
        })
        .mount(&fixture.server)
        .await;
    let provider = fixture.provider_at("new-key", 1, "org-fixture-credential");
    provider
        .create_rsa2048_key()
        .await
        .expect("explicit initial version checked after Create");
    assert_eq!(provider.public_key().await.unwrap(), public);
    let requests = fixture.server.received_requests().await.unwrap();
    let posts: Vec<_> = requests
        .iter()
        .filter(|r| r.method.as_str() == "POST")
        .collect();
    assert_eq!(posts.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&posts[0].body).unwrap();
    assert_eq!(body["type"], "rsa-2048");
    assert_eq!(body["exportable"], false);
    // An already-existing name is not silently treated as a fresh Create.
    assert!(provider.create_rsa2048_key().await.is_err());
    assert_eq!(
        fixture
            .server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.method.as_str() == "POST")
            .count(),
        1
    );
}

#[tokio::test]
#[serial]
async fn failed_or_indeterminate_rotation_is_not_retried_or_replaced_by_create() {
    let fixture = TransitMockService::new().await;
    let public = include_str!("../config/keys/test-platform.pub");
    for response in [
        ResponseTemplate::new(500),
        ResponseTemplate::new(204).set_delay(Duration::from_secs(30)),
    ] {
        fixture._fixture.reset().await;
        Mock::given(method("GET"))
            .and(path("/v1/transit/keys/key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{
                    "type":"rsa-2048","exportable":false,"supports_signing":true,
                    "latest_version":7,"keys":{"7":{"public_key":public}}
                }})),
            )
            .mount(&fixture.server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/keys/key/rotate"))
            .respond_with(response)
            .mount(&fixture.server)
            .await;
        let provider = fixture.provider_at("key", 7, "org-fixture-credential");
        let result = tokio::time::timeout(Duration::from_secs(15), provider.rotate_rsa2048_key())
            .await
            .expect("provider timeout precedes outer guard");
        assert!(result.is_err());
        let requests = fixture.server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method.as_str(), "GET");
        assert_eq!(requests[1].url.path(), "/v1/transit/keys/key/rotate");
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.method.as_str() == "POST")
                .count(),
            1
        );
    }
}
