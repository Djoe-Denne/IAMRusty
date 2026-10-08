//! WIF adapter exchange proofs (ADR-0307).

#[path = "../../tests/fixtures/wif/mod.rs"]
mod wif_fixtures;

use hive_configuration::{
    AwsWorkloadConfig, AzureWorkloadConfig, GcpWorkloadConfig, WorkloadIdentityConfig,
};
use hive_domain::port::service::{
    ConfigureOrganizationSignerRequest, IamOrganizationSignerClient, WorkloadIdentity,
};
use hive_infra::iam::{
    compose_workload_identity, AwsWif, AzureWif, GcpWif, HttpIamOrganizationSignerClient,
};
use serial_test::serial;
use std::io::Write;
use uuid::Uuid;
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
        role_session_name: "hive-test".into(),
        audience: "sts.amazonaws.com".into(),
    })
    .expect("aws wif");

    let cred = adapter
        .resolve("iam-internal-token")
        .await
        .expect("resolve");
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

    let cred = adapter
        .resolve("iam-internal-token")
        .await
        .expect("resolve");
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
        scope: "api://iam/.default".into(),
    })
    .expect("azure wif");

    let cred = adapter
        .resolve("iam-internal-token")
        .await
        .expect("resolve");
    assert_eq!(cred.secret, "azure-access-token-from-wiremock");
}

/// Hive HTTP IAM signer sends WIF-resolved `x-iam-internal-token`, not static `api_key`.
#[tokio::test]
#[serial]
async fn http_iam_signer_uses_wif_resolved_token() {
    let jwt = write_oidc_jwt();
    let wif = wif_fixtures::WifFixtures::service().await;
    let wif_token = "gcp-wif-resolved-iam-token";
    wif.mock_gcp_token_ok(wif_token).await;

    let org_id = Uuid::from_u128(0x1111_2222_3333_4444_5555_6666_7777_8888);
    let signing_profile_id = Uuid::from_u128(0xaaaa_bbbb_cccc_dddd_eeee_ffff_0000_1111);

    Mock::given(method("POST"))
        .and(path(format!(
            "/iam/internal/organizations/{org_id}/signer/configure"
        )))
        .and(header("x-iam-internal-token", wif_token))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "signing_profile_id": signing_profile_id,
            "kid": "kid-wif",
            "status": "active",
            "issuer": "https://iam.test"
        })))
        .mount(&wif.server())
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
        "wrong-static",
        "iam-internal-token",
    )
    .expect("compose gcp wif");

    let client = HttpIamOrganizationSignerClient::with_workload(
        wif.base_url(),
        "iam-internal-token",
        workload,
        10,
    )
    .expect("http iam signer");

    let got = client
        .configure_organization_signer(
            org_id,
            &ConfigureOrganizationSignerRequest {
                provider_type: "openbao".into(),
                provider_key_ref: "org-key".into(),
                provider_key_version: Some(1),
                credential_ref: None,
                public_key: "pk".into(),
                org_slug: "acme".into(),
            },
        )
        .await
        .expect("configure with WIF token");

    assert_eq!(got.kid, "kid-wif");
    assert_eq!(got.signing_profile_id, signing_profile_id);
}
