//! C-1 configuration/SDK seam only. No provider, database or HTTP fixture.
use iam_configuration::{JwtConfig, SecretStorage};
use iam_domain::entity::signing_key::{SigningKeyStatus, TrustScope};
use iam_domain::entity::token::{Jwk, JwkSet};
use rustycog::http::{LocalJwksSeed, UserIdExtractor};

fn config() -> JwtConfig {
    JwtConfig {
        // Deliberately unreadable: constructing a verifier must not load PEM.
        secret: SecretStorage::PemFile {
            private_key_path: "unused-private.pem".into(),
            public_key_path: "unused-public.pem".into(),
            key_id: None,
        },
        issuer: "independent-legacy-hmac-issuer".into(),
        public_base_url: "https://platform.example".into(),
        allowed_algorithms: vec!["RS256".into()],
        ..JwtConfig::default()
    }
}

async fn seed(config: &JwtConfig, issuer: &str) -> LocalJwksSeed {
    let mut key = Jwk::from_rsa_pem(
        include_str!("../../config/keys/test-platform.pub"),
        "1234567890ab4def9234567890abcdef",
        issuer,
    )
    .expect("fixture public material");
    key.trust_scope = Some(TrustScope::Platform);
    key.status = Some(SigningKeyStatus::Active);
    let json = serde_json::to_string(&JwkSet { keys: vec![key] }).expect("publisher DTO");
    LocalJwksSeed::capture(
        &config.effective_jwks_url().expect("trusted URL"),
        || async { Ok(json) },
    )
    .await
    .expect("local seed")
}

#[tokio::test]
async fn normal_rs256_configuration_installs_seed_and_preserves_mesh() {
    let config = config();
    let mut auth = config.http_verifier_auth().expect("IAM helper");
    assert_eq!(
        auth.jwt.issuer.as_deref(),
        Some(config.platform_issuer().as_str())
    );
    assert!(auth.jwt.hs256_secret.is_none());
    auth.mesh.trusted_gateway_san = "spiffe://fixture/mesh".into();
    let extractor = UserIdExtractor::from_config_with_seeded_jwks(
        auth,
        seed(&config, &config.platform_issuer()).await,
    )
    .expect("normal RS256 startup seam");
    assert_eq!(extractor.gateway_san(), Some("spiffe://fixture/mesh"));
}

#[tokio::test]
async fn platform_seed_with_inconsistent_issuer_is_rejected_without_fallback() {
    let config = config();
    let auth = config.http_verifier_auth().expect("IAM helper");
    assert!(
        UserIdExtractor::from_config_with_seeded_jwks(
            auth,
            seed(&config, "https://foreign.example/iam").await,
        )
        .is_err()
    );
}

#[tokio::test]
async fn distinct_hmac_issuer_is_not_silently_normalized_for_a_platform_seed() {
    let mut config = config();
    config.secret = SecretStorage::PlainText {
        value: "explicit-legacy-hmac-secret".into(),
    };
    config.allowed_algorithms.push("HS256".into());
    let auth = config.http_verifier_auth().expect("explicit mixed config");
    assert_eq!(auth.jwt.issuer.as_deref(), Some(config.issuer.as_str()));
    assert_eq!(
        auth.jwt.hs256_secret.as_deref(),
        Some("explicit-legacy-hmac-secret")
    );
    // SDK has only one issuer field. Distinct-issuer support needs an explicit
    // contract decision; rejection is preferable to changing legacy trust.
    assert!(
        UserIdExtractor::from_config_with_seeded_jwks(
            auth,
            seed(&config, &config.platform_issuer()).await,
        )
        .is_err()
    );
}
