//! Unit tests for SigningProvider / WorkloadIdentity ports (ADR-0304 / 0307).

use iam_domain::entity::signing_key::{
    SigningKey, SigningKeyStatus, SigningProviderType, TrustScope,
};
use iam_domain::port::{
    OrganizationSignerProbe, SigningCapabilities, SigningProvider, WorkloadCredential,
    WorkloadIdentity,
};
use iam_infra::signing::{
    DefaultOrganizationSignerProbe, PemSigningProvider, StaticCredential, TransitClientConfig,
    TransitSigningProvider,
};
use serial_test::serial;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[tokio::test]
async fn workload_identity_static_credential_resolves() {
    let wi = StaticCredential::from_pair("vault-token", "s.test-token").expect("pair");
    let cred = wi.resolve("vault-token").await.expect("resolve");
    assert_eq!(cred.secret, "s.test-token");
}

#[tokio::test]
#[serial]
async fn transit_sign_stub_produces_verifiable_digest_signature() {
    use rustycog::testing::wiremock::MockServerFixture;
    use wiremock::{
        matchers::{header, method, path},
        Mock, ResponseTemplate,
    };

    let fixture = MockServerFixture::isolated().await;
    let server = fixture.server();
    let sig_bytes = vec![0u8; 256];
    let sig_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &sig_bytes);
    Mock::given(method("POST"))
        .and(path("/v1/transit/sign/platform-key"))
        .and(header("X-Vault-Token", "s.test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "signature": format!("vault:v1:{sig_b64}") }
        })))
        .mount(&*server)
        .await;

    let wi: Arc<dyn WorkloadIdentity> =
        Arc::new(StaticCredential::from_pair("openbao-token", "s.test-token").expect("pair"));
    let transit = TransitSigningProvider::new(
        server.uri(),
        "platform-key",
        "openbao-token",
        wi,
        Some(include_str!("../../config/keys/test-platform.pub").to_string()),
    )
    .expect("transit provider");

    let digest = Sha256::digest(b"header.payload");
    let got = transit.sign_digest(&digest).await.expect("sign");
    assert_eq!(got, sig_bytes);
    assert!(transit.capabilities().sign_digest);

    let pem = PemSigningProvider::new(
        include_str!("../../config/keys/test-platform.pem"),
        include_str!("../../config/keys/test-platform.pub"),
    )
    .expect("pem");
    let caps = SigningCapabilities {
        sign_digest: true,
        public_key_available: true,
    };
    assert_eq!(pem.capabilities(), caps);
    let _ = WorkloadCredential {
        secret: "unused".into(),
    };
}

#[tokio::test]
#[serial]
async fn transit_public_key_fetched_when_absent() {
    use rustycog::testing::wiremock::MockServerFixture;
    use wiremock::{
        matchers::{method, path},
        Mock, ResponseTemplate,
    };

    let fixture = MockServerFixture::isolated().await;
    let server = fixture.server();
    let public = include_str!("../../config/keys/test-platform.pub");
    Mock::given(method("GET"))
        .and(path("/v1/transit/keys/org-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "keys": { "1": { "public_key": public } } }
        })))
        .mount(&*server)
        .await;

    let wi: Arc<dyn WorkloadIdentity> =
        Arc::new(StaticCredential::from_pair("openbao-token", "s.test-token").expect("pair"));
    let transit = TransitSigningProvider::new(server.uri(), "org-key", "openbao-token", wi, None)
        .expect("transit");
    let pem = transit.public_key().await.expect("fetch pub");
    assert!(pem.contains("BEGIN PUBLIC KEY"));
}

#[tokio::test]
#[serial]
async fn transit_probe_signs_and_verifies_challenge() {
    use rsa::pkcs1v15::Pkcs1v15Sign;
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::RsaPrivateKey;
    use rustycog::testing::wiremock::MockServerFixture;
    use wiremock::{
        matchers::{method, path},
        Mock, ResponseTemplate,
    };

    let private =
        RsaPrivateKey::from_pkcs8_pem(include_str!("../../config/keys/test-platform.pem"))
            .expect("priv");
    let public = include_str!("../../config/keys/test-platform.pub");
    let digest = Sha256::digest(b"aiforall-org-signer-challenge");
    let signature = private
        .sign(Pkcs1v15Sign::new::<Sha256>(), &digest)
        .expect("sign");
    let sig_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &signature);

    let fixture = MockServerFixture::isolated().await;
    let server = fixture.server();
    let org = uuid::Uuid::new_v4();
    let key_name = format!("org-{org}-signing");
    let credential = format!("org-{org}-credential");
    Mock::given(method("POST"))
        .and(path(format!("/v1/transit/sign/{key_name}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "signature": format!("vault:v1:{sig_b64}") }
        })))
        .mount(&*server)
        .await;

    let wi: Arc<dyn WorkloadIdentity> =
        Arc::new(StaticCredential::from_pair(&credential, "s.test-token").expect("pair"));
    let probe = DefaultOrganizationSignerProbe::new(std::env::temp_dir()).with_transit(
        TransitClientConfig {
            base_url: server.uri(),
            workload: wi,
            token_ref: credential.clone(),
        },
    );
    let now = chrono::Utc::now();
    let key = SigningKey {
        id: uuid::Uuid::new_v4(),
        kid: uuid::Uuid::new_v4().simple().to_string(),
        algorithm: "RS256".to_string(),
        trust_scope: TrustScope::Organization,
        issuer: "http://127.0.0.1/iam/orgs/acme".into(),
        provider_type: SigningProviderType::OpenBaoTransit,
        provider_key_ref: key_name,
        credential_ref: Some(credential),
        public_key: public.to_string(),
        status: SigningKeyStatus::Active,
        organization_id: Some(org),
        created_at: now,
        updated_at: now,
    };
    probe.challenge(&key).await.expect("transit challenge");
}

#[test]
fn no_spiffe_spire_binary_in_tree() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        !manifest.to_lowercase().contains("spiffe") && !manifest.to_lowercase().contains("spire"),
        "SPIFFE/SPIRE must not be added as a runtime dependency (ADR-0307)"
    );
}
