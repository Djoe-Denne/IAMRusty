//! C-1 configuration/SDK seam only. No provider, database or HTTP fixture.
use iam_configuration::{AppConfig, JwtConfig, SecretStorage};
use iam_domain::entity::signing_key::{SigningKeyStatus, TrustScope};
use iam_domain::entity::token::{Jwk, JwkSet};
use rustycog::http::{LocalJwksSeed, UserIdExtractor};

fn config() -> AppConfig {
    let jwt = JwtConfig {
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
    };
    let mut config = AppConfig {
        jwt,
        ..AppConfig::default()
    };
    config.auth.jwt.platform_issuer = Some(config.jwt.platform_issuer());
    config.auth.jwt.allow_historical_platform_issuer = Some(false);
    config
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
        Some(config.jwt.platform_issuer().as_str())
    );
    assert!(auth.jwt.hs256_secret.is_none());
    auth.mesh.trusted_gateway_san = "spiffe://fixture/mesh".into();
    let extractor = UserIdExtractor::from_config_with_seeded_jwks(
        auth,
        seed(&config.jwt, &config.jwt.platform_issuer()).await,
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
            seed(&config.jwt, "https://foreign.example/iam").await,
        )
        .is_err()
    );
}

#[tokio::test]
async fn distinct_hmac_issuer_is_not_silently_normalized_for_a_platform_seed() {
    let mut config = config();
    config.jwt.secret = SecretStorage::PlainText {
        value: "explicit-legacy-hmac-secret".into(),
    };
    config.jwt.allowed_algorithms.push("HS256".into());
    let auth = config.http_verifier_auth().expect("explicit mixed config");
    assert_eq!(auth.jwt.issuer.as_deref(), Some(config.jwt.issuer.as_str()));
    assert_eq!(
        auth.jwt.hs256_secret.as_deref(),
        Some("explicit-legacy-hmac-secret")
    );
    // SDK has only one issuer field. Distinct-issuer support needs an explicit
    // contract decision; rejection is preferable to changing legacy trust.
    assert!(
        UserIdExtractor::from_config_with_seeded_jwks(
            auth,
            seed(&config.jwt, &config.jwt.platform_issuer()).await,
        )
        .is_err()
    );
}

#[tokio::test]
async fn iam_http_verifier_preserves_configured_authority_and_legacy_window() {
    for historical in [false, true] {
        let mut config = config();
        config.auth.jwt.allow_historical_platform_issuer = Some(historical);
        config.auth.mesh.trusted_gateway_san = "spiffe://fixture/mesh".into();
        let auth = config.http_verifier_auth().expect("explicit IAM policy");
        assert_eq!(auth.jwt.platform_issuer, config.auth.jwt.platform_issuer);
        assert_eq!(auth.jwt.allow_historical_platform_issuer, Some(historical));
        let extractor = UserIdExtractor::from_config_with_seeded_jwks(
            auth,
            seed(&config.jwt, &config.jwt.platform_issuer()).await,
        )
        .expect("seeded verifier");
        let policy = extractor.principal_policy().expect("policy installed");
        assert_eq!(policy.platform_issuer, config.jwt.platform_issuer());
        assert_eq!(policy.allow_historical_platform_issuer, historical);
        assert_eq!(extractor.gateway_san(), Some("spiffe://fixture/mesh"));
    }
}

#[tokio::test]
async fn iam_http_verifier_rejects_foreign_hmac_issuer_at_uuid_only_boundary() {
    use iam_domain::entity::token::TokenClaims;
    use iam_domain::port::service::JwtTokenEncoder;
    use iam_infra::token::JwtTokenService;
    let subject = uuid::Uuid::new_v4();
    for (issuer, historical, allowed) in [
        ("https://platform.example/iam", false, true),
        ("independent-legacy-hmac-issuer", false, false),
        ("https://foreign.example/iam", true, false),
        ("iamrusty", false, false),
        ("iamrusty", true, true),
    ] {
        let mut config = config();
        config.jwt.secret = SecretStorage::PlainText {
            value: "explicit-legacy-hmac-secret".into(),
        };
        config.jwt.issuer = issuer.into();
        config.jwt.allowed_algorithms = vec!["HS256".into()];
        // HS256 verification is independent of the explicit signing authority.
        // No JWKS request or seed is needed for this verification-only path.
        config.auth.jwt.platform_issuer = Some(config.jwt.platform_issuer());
        config.auth.jwt.allow_historical_platform_issuer = Some(historical);
        let now = chrono::Utc::now().timestamp();
        let token = JwtTokenService::with_hmac("explicit-legacy-hmac-secret".into(), 900)
            .encode(&TokenClaims {
                sub: subject.to_string(),
                username: "fixture".into(),
                iss: issuer.into(),
                aud: config.jwt.audience.clone(),
                org: None,
                exp: now + 900,
                iat: now,
                jti: uuid::Uuid::new_v4().to_string(),
            })
            .await
            .expect("real HS256 signature");
        let extractor = UserIdExtractor::new(config.http_verifier_auth().expect("explicit policy"))
            .expect("HS verifier");
        assert_eq!(
            extractor
                .extract_principal(&token)
                .await
                .expect("recognized issuer")
                .iss,
            issuer
        );
        let result = extractor.extract_user_id(&token).await;
        if allowed {
            assert_eq!(result.expect("platform UUID view"), subject);
        } else {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("PRINCIPAL_INCOMPATIBLE")
            );
        }
    }
}
