//! G-7/annexe §10: the real `AppState` support grants no principal or user identity.
//! Obligation -> risk -> scenario -> oracle:
//! - Empty inline trust must stay refusing, without a default user or HS fallback.
//! - Supply empty/malformed tokens and genuinely signed, live HS256/RS256 fixtures
//!   with UUID subjects, rather than relying on bad signatures or non-UUID claims.
//! - First verify both signatures/claims independently with jsonwebtoken and the
//!   fixture's own keys; then both SDK extraction APIs must refuse every token.
//!
//! KEEP: catches a default identity, known HS secret or installed fixture RSA trust.

use super::hmac_only_user_id_extractor;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde_json::{json, Value};

const SUBJECT: &str = "11111111-1111-4111-8111-111111111111";
const ISSUER: &str = "https://platform.example.test";
const AUDIENCE: &str = "oauth-connect";
const HS_SECRET: &[u8] = b"fixture-only-platform-hs256-sentinel";

fn require<T, E>(result: Result<T, E>, phase: &str) -> T {
    result.unwrap_or_else(|_| panic!("inline-trust fixture prerequisite failed: {phase}"))
}

fn signed_fixture(algorithm: Algorithm, claims: &Value) -> String {
    let (encoding, decoding) = match algorithm {
        Algorithm::HS256 => (
            EncodingKey::from_secret(HS_SECRET),
            DecodingKey::from_secret(HS_SECRET),
        ),
        Algorithm::RS256 => (
            require(
                EncodingKey::from_rsa_pem(include_bytes!(
                    "../../../../IAMRusty/config/keys/test-platform.pem"
                )),
                "existing RSA private fixture",
            ),
            require(
                DecodingKey::from_rsa_pem(include_bytes!(
                    "../../../../IAMRusty/config/keys/test-platform.pub"
                )),
                "existing RSA public fixture",
            ),
        ),
        _ => panic!("only HS256 and RS256 fixtures are specified"),
    };
    let mut header = Header::new(algorithm);
    header.typ = Some("aiforall-access+jwt".into());
    header.kid = Some("service-key".into());
    let token = require(encode(&header, claims, &encoding), "JWT signing");
    let mut validation = Validation::new(algorithm);
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[AUDIENCE]);
    validation.validate_nbf = true;
    validation.required_spec_claims = ["exp", "iat", "sub", "iss", "aud"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let verified = require(
        decode::<Value>(&token, &decoding, &validation),
        "independent signature and claims verification",
    );
    assert!(
        verified.claims == *claims,
        "independent verification must preserve the complete UUID-bearing fixture claims"
    );
    assert!(
        verified.header.kid.as_deref() == Some("service-key"),
        "fixture must carry a usable key id"
    );
    assert!(
        verified.header.typ.as_deref() == Some("aiforall-access+jwt"),
        "fixture must carry the SDK access-token type"
    );
    token
}

#[tokio::test]
async fn hmac_only_support_refuses_empty_malformed_and_verified_hs256_rs256_tokens() {
    let now = chrono::Utc::now().timestamp();
    let claims = json!({"sub": SUBJECT, "iss": ISSUER, "aud": AUDIENCE,
        "iat": now - 5, "nbf": now - 5, "exp": now + 3600});
    // Positive crypto prerequisites run before any negative SDK observation.
    let hs = signed_fixture(Algorithm::HS256, &claims);
    let rs = signed_fixture(Algorithm::RS256, &claims);
    let extractor = require(hmac_only_user_id_extractor(), "actual composition helper");
    for (scenario, token) in [
        ("empty", ""),
        ("malformed", "not-a-jwt"),
        ("verified-hs256", hs.as_str()),
        ("verified-rs256", rs.as_str()),
    ] {
        assert!(
            extractor.extract_principal(token).await.is_err(),
            "§10: refusing AppState support granted a principal for {scenario}"
        );
        assert!(
            extractor.extract_user_id(token).await.is_err(),
            "§10: refusing AppState support granted a user id for {scenario}"
        );
    }
}
