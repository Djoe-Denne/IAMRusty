//! Real IAM signing-key fixtures shared by service and worker test binaries.
use iam_domain::entity::signing_key::{
    SigningKey, SigningKeyStatus, SigningProviderType, TrustScope,
};
use rustycog::testing::http::jwt::{test_rs256_public_pem, TEST_PLATFORM_ISSUER, TEST_RS256_KID};
use uuid::Uuid;

pub fn registry_key(status: SigningKeyStatus, organization: Option<Uuid>) -> SigningKey {
    SigningKey {
        id: Uuid::new_v4(),
        kid: TEST_RS256_KID.into(),
        algorithm: "RS256".into(),
        trust_scope: if organization.is_some() {
            TrustScope::Organization
        } else {
            TrustScope::Platform
        },
        issuer: organization.map_or_else(
            || TEST_PLATFORM_ISSUER.into(),
            |id| format!("https://issuer.example/org/{id}"),
        ),
        provider_type: SigningProviderType::PemFile,
        provider_key_ref: if organization.is_none() {
            "pem:config/jwt.secret"
        } else {
            "fixed-test-organization-pem"
        }
        .into(),
        provider_key_version: None,
        credential_ref: None,
        public_key: test_rs256_public_pem().into(),
        status,
        organization_id: organization,
        created_at: "2026-10-03T00:00:00Z".parse().expect("fixture date"),
        updated_at: "2026-10-03T00:00:00Z".parse().expect("fixture date"),
    }
}
