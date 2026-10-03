//! WIF adapter exchange proofs (ADR-0307).

#[path = "../../tests/fixtures/wif/mod.rs"]
mod wif_fixtures;

use iam_configuration::{
    AwsWorkloadConfig, AzureWorkloadConfig, GcpWorkloadConfig, WorkloadIdentityConfig,
};
use iam_domain::port::{SigningProvider, WorkloadIdentity};
use iam_infra::signing::{
    compose_workload_identity, AwsWif, AzureWif, GcpWif, TransitSigningProvider,
};
use serial_test::serial;
use sha2::{Digest, Sha256};
use std::io::Write;
use wiremock::{
    matchers::{header, method, path},
    Mock, ResponseTemplate,
};

fn write_oidc_jwt() -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().expect("temp jwt");
    f.write_all(b"eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0In0.sig")
        .expect("write jwt");
    f.flush().expect("flush");
    f
}

#[tokio::test]
#[serial]
async fn aws_wif_exchanges_session_token() {
    let jwt = write_oidc_jwt();
    let wif = wif_fixtures::WifFixtures::service().await;
    wif.mock_aws_assume_role_ok("aws-session-token-from-wiremock")
        .await;

    let adapter = AwsWif::new(&AwsWorkloadConfig {
        token_url: format!("{}/", wif.base_url()),
        subject_token_file: jwt.path().to_string_lossy().into(),
        role_arn: "arn:aws:iam::123456789012:role/test".into(),
        role_session_name: "iam-test".into(),
        audience: "sts.amazonaws.com".into(),
    })
    .expect("aws wif");

    let cred = adapter.resolve("openbao-token").await.expect("resolve");
    assert_eq!(cred.secret, "aws-session-token-from-wiremock");
}

#[tokio::test]
#[serial]
async fn gcp_wif_exchanges_access_token() {
    let jwt = write_oidc_jwt();
    let wif = wif_fixtures::WifFixtures::service().await;
    wif.mock_gcp_token_ok("gcp-access-token-from-wiremock")
        .await;

    let adapter = GcpWif::new(&GcpWorkloadConfig {
        token_url: format!("{}/v1/token", wif.base_url()),
        subject_token_file: jwt.path().to_string_lossy().into(),
        audience: "//iam.googleapis.com/projects/test".into(),
    })
    .expect("gcp wif");

    let cred = adapter.resolve("openbao-token").await.expect("resolve");
    assert_eq!(cred.secret, "gcp-access-token-from-wiremock");
}

#[tokio::test]
#[serial]
async fn azure_wif_exchanges_access_token() {
    let jwt = write_oidc_jwt();
    let tenant = "test-tenant";
    let wif = wif_fixtures::WifFixtures::service().await;
    wif.mock_azure_token_ok(tenant, "azure-access-token-from-wiremock")
        .await;

    let adapter = AzureWif::new(&AzureWorkloadConfig {
        token_url: wif.base_url(),
        subject_token_file: jwt.path().to_string_lossy().into(),
        tenant_id: tenant.into(),
        client_id: "test-client".into(),
        scope: "https://vault.azure.net/.default".into(),
    })
    .expect("azure wif");

    let cred = adapter.resolve("openbao-token").await.expect("resolve");
    assert_eq!(cred.secret, "azure-access-token-from-wiremock");
}

/// Transit Sign uses the WIF-resolved secret as `X-Vault-Token` (OpenBao fixture shape).
#[tokio::test]
#[serial]
async fn transit_sign_uses_wif_resolved_token() {
    let jwt = write_oidc_jwt();
    let wif = wif_fixtures::WifFixtures::service().await;
    wif.mock_gcp_token_ok("wif-exchanged-vault-token").await;

    let sig_bytes = vec![0u8; 256];
    let sig_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &sig_bytes);
    // Same response envelope as `services/IAMRusty/tests/fixtures/openbao_transit`.
    Mock::given(method("POST"))
        .and(path("/v1/transit/sign/platform-key"))
        .and(header("X-Vault-Token", "wif-exchanged-vault-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "signature": format!("vault:v1:{sig_b64}") }
        })))
        .mount(&*wif.server())
        .await;

    let workload = compose_workload_identity(
        Some(&WorkloadIdentityConfig {
            provider: Some("gcp".into()),
            aws: None,
            gcp: Some(GcpWorkloadConfig {
                token_url: format!("{}/v1/token", wif.base_url()),
                subject_token_file: jwt.path().to_string_lossy().into(),
                audience: "//iam.googleapis.com/projects/test".into(),
            }),
            azure: None,
        }),
        "",
        "openbao-token",
    )
    .expect("compose gcp wif");

    let transit = TransitSigningProvider::new(
        wif.base_url(),
        "platform-key",
        "openbao-token",
        workload,
        None,
    )
    .expect("transit");

    let digest = Sha256::digest(b"hello-wif");
    let got = transit
        .sign_digest(&digest)
        .await
        .expect("sign with WIF token");
    assert_eq!(got, sig_bytes);
}
