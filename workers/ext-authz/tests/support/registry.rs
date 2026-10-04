//! Real IAM publisher serialization, not manually invented trust metadata.
use iam_domain::entity::{
    signing_key::{SigningKey, SigningKeyStatus, SigningProviderType, TrustScope},
    token::JwkSet,
};
use rustycog::testing::http::jwt::{TEST_PLATFORM_ISSUER, TEST_RS256_KID, TEST_RS256_PUBLIC_PEM};
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
        credential_ref: None,
        public_key: TEST_RS256_PUBLIC_PEM.into(),
        status,
        organization_id: organization,
        created_at: "2026-10-03T00:00:00Z".parse().expect("fixture date"),
        updated_at: "2026-10-03T00:00:00Z".parse().expect("fixture date"),
    }
}

pub fn serialize(keys: &[SigningKey]) -> String {
    serde_json::to_string(&JwkSet::from_registry_keys(keys)).expect("IAM JWKS serialization")
}

pub fn platform_jwks() -> String {
    serialize(&[registry_key(SigningKeyStatus::Active, None)])
}
