//! RS256 encode + JWKS + extractor acceptance (no postgres).

use iam_domain::entity::signing_key::{SigningKeyLifecyclePolicy, SigningKeyStatus, TrustScope};
use iam_domain::entity::token::{Jwk, JwkSet, TokenClaims, DEFAULT_JWT_AUDIENCE};
use iam_domain::port::service::JwtTokenEncoder;
use iam_domain::port::SigningProvider;
use iam_infra::repository::signing_key_registry::SeaOrmSigningKeyRegistry;
use iam_infra::signing::PemSigningProvider;
use iam_infra::token::{JwtAlgorithm, JwtTokenService};
use rustycog::http::{UserIdExtractor, ACCESS_TOKEN_TYP};
use sea_orm::{DbBackend, MockDatabase};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

const PLATFORM_ISSUER: &str = "http://127.0.0.1:8080/iam";
const KID: &str = "test-rs256-kid-01";

fn pem_provider() -> PemSigningProvider {
    PemSigningProvider::new(
        include_str!("../../config/keys/test-platform.pem"),
        include_str!("../../config/keys/test-platform.pub"),
    )
    .expect("pem provider")
}

#[tokio::test]
async fn rs256_encode_jwks_and_extractor_accept() {
    let provider = Arc::new(pem_provider()) as Arc<dyn SigningProvider>;
    let mut jwk = Jwk::from_rsa_pem(
        include_str!("../../config/keys/test-platform.pub"),
        KID,
        PLATFORM_ISSUER,
    )
    .expect("jwk");
    // PEM material alone is not trusted: explicitly bootstrap this fixture's
    // canonical platform metadata, as required by the fail-closed verifier.
    jwk.status = Some(SigningKeyStatus::Active);
    jwk.trust_scope = Some(TrustScope::Platform);
    assert_eq!(jwk.kty, "RSA");
    assert_eq!(jwk.use_, "sig");
    assert_eq!(jwk.alg, "RS256");
    assert_eq!(jwk.kid, KID);
    assert_eq!(jwk.iss, PLATFORM_ISSUER);

    let jwks = JwkSet {
        keys: vec![jwk.clone()],
    };
    let jwks_json = serde_json::to_string(&jwks).expect("jwks json");
    assert!(jwks_json.contains("\"iss\""));
    assert!(jwks_json.contains(PLATFORM_ISSUER));

    // The encoder also requires an authoritative Active epoch and a final
    // emission fence. Keep this no-Postgres roundtrip on the existing ORM seam.
    let now = chrono::Utc::now().naive_utc();
    let row = iam_infra::repository::entity::signing_keys::Model {
        id: Uuid::new_v4(),
        kid: KID.into(),
        algorithm: "RS256".into(),
        trust_scope: "platform".into(),
        issuer: PLATFORM_ISSUER.into(),
        provider_type: "pem_file".into(),
        provider_key_ref: "test-platform.pem".into(),
        provider_key_version: None,
        credential_ref: None,
        public_key: include_str!("../../config/keys/test-platform.pub").into(),
        status: "active".into(),
        organization_id: None,
        created_at: now,
        updated_at: now,
    };
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row.clone()], vec![row]])
        .into_connection();
    let registry = SeaOrmSigningKeyRegistry::new(
        Arc::new(db),
        Arc::new(SigningKeyLifecyclePolicy::new().expect("lifecycle policy")),
        900,
    )
    .expect("fixture registry");

    let mut service = JwtTokenService::with_refresh_expiration(
        JwtAlgorithm::RS256(iam_domain::entity::token::JwtKeyPair {
            private_key: include_str!("../../config/keys/test-platform.pem").to_string(),
            public_key: include_str!("../../config/keys/test-platform.pub").to_string(),
            kid: KID.to_string(),
        }),
        900,
        2592000,
    )
    .with_local_pem_allowed(true) // explicit nonprod PEM provider fixture
    .with_issuer_audience(PLATFORM_ISSUER, DEFAULT_JWT_AUDIENCE);
    service = service
        .with_signing_provider(provider, KID.to_string(), PLATFORM_ISSUER.to_string(), jwks)
        .with_signing_registry(Arc::new(registry));

    let user_id = Uuid::new_v4();
    let claims = TokenClaims::new_with_issuer_audience(
        &user_id.to_string(),
        "alice",
        chrono::Duration::hours(1),
        PLATFORM_ISSUER,
        DEFAULT_JWT_AUDIENCE,
    );
    let token = service.encode(&claims).await.expect("encode");

    let header_b64 = token.split('.').next().expect("header");
    let header_json = String::from_utf8(
        base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            header_b64,
        )
        .expect("b64"),
    )
    .expect("utf8");
    let header: serde_json::Value = serde_json::from_str(&header_json).expect("json");
    assert_eq!(header["alg"], "RS256");
    assert_eq!(header["kid"], KID);
    assert_eq!(header["typ"], ACCESS_TOKEN_TYP);

    let payload_b64 = token.split('.').nth(1).expect("payload");
    let payload_json = String::from_utf8(
        base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            payload_b64,
        )
        .expect("b64"),
    )
    .expect("utf8");
    let payload: serde_json::Value = serde_json::from_str(&payload_json).expect("json");
    assert_eq!(payload["iss"], PLATFORM_ISSUER);
    assert_eq!(payload["aud"], DEFAULT_JWT_AUDIENCE);

    let extractor = UserIdExtractor::from_inline_jwks(&jwks_json, Some(DEFAULT_JWT_AUDIENCE))
        .expect("extractor");
    let principal = extractor
        .extract_principal(&token)
        .await
        .expect("accept RS256");
    assert_eq!(principal.sub, user_id);
    assert_eq!(principal.iss, PLATFORM_ISSUER);

    // HS256 token must be rejected on RS256-only extractor.
    let hs_header = json!({"alg":"HS256","typ":"JWT"});
    let hs_payload = json!({
        "sub": user_id.to_string(),
        "iss": PLATFORM_ISSUER,
        "aud": DEFAULT_JWT_AUDIENCE,
        "exp": chrono::Utc::now().timestamp() + 3600,
        "iat": chrono::Utc::now().timestamp(),
        "jti": Uuid::new_v4().to_string(),
    });
    let enc = |v: &serde_json::Value| {
        base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            serde_json::to_vec(v).unwrap(),
        )
    };
    let hs_token = format!("{}.{}.sig", enc(&hs_header), enc(&hs_payload));
    let err = extractor
        .extract_user_id(&hs_token)
        .await
        .expect_err("HS256 rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("not allowed") || msg.contains("invalid") || msg.contains("algorithm"),
        "{msg}"
    );
}

#[test]
fn jwks_from_registry_keys_skips_invalid_pem_keeps_good_keys() {
    use chrono::Utc;
    use iam_domain::entity::signing_key::{
        SigningKey, SigningKeyStatus, SigningProviderType, TrustScope,
    };

    let now = Utc::now();
    let good = SigningKey {
        id: Uuid::new_v4(),
        kid: "good-kid".to_string(),
        algorithm: "RS256".to_string(),
        trust_scope: TrustScope::Platform,
        issuer: PLATFORM_ISSUER.to_string(),
        provider_type: SigningProviderType::PemFile,
        provider_key_ref: "opaque".to_string(),
        provider_key_version: None,
        credential_ref: None,
        public_key: include_str!("../../config/keys/test-platform.pub").to_string(),
        status: SigningKeyStatus::Active,
        organization_id: None,
        created_at: now,
        updated_at: now,
    };
    let bad = SigningKey {
        kid: "bad-kid".to_string(),
        public_key: "not-a-pem".to_string(),
        ..good.clone()
    };
    let jwks = JwkSet::from_registry_keys(&[good, bad]);
    assert_eq!(jwks.keys.len(), 1);
    assert_eq!(jwks.keys[0].kid, "good-kid");
}
