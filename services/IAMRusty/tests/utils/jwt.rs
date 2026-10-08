use anyhow::Result;
use base64;
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use iam_configuration::JwtConfig;
use iam_domain::entity::registration_token::{RegistrationFlow, RegistrationTokenClaims};
use iam_domain::entity::token::TokenClaims;
use iam_domain::port::service::{JwtTokenEncoder, RegistrationTokenService};
use iam_infra::token::{registration_token_service::RegistrationTokenServiceImpl, JwtTokenService};
use std::sync::Arc;
use uuid::Uuid;

/// Explicit synthetic codec for validator tests only, never the real publisher fixture.
/// Platform and organization scope are selected by API, not inferred from issuer text.
pub struct FakeJwtCodec {
    platform_issuer: String,
}

impl FakeJwtCodec {
    pub fn from_config(config: &JwtConfig) -> Self {
        Self {
            platform_issuer: config.platform_issuer(),
        }
    }

    pub fn platform_token(&self, user_id: Uuid) -> String {
        rustycog::testing::http::jwt::create_rs256_jwt_token_with_issuer(
            user_id,
            &self.platform_issuer,
        )
    }

    pub fn platform_jwks(
        &self,
        status: rustycog::testing::http::jwt::TestSigningKeyStatus,
    ) -> String {
        rustycog::testing::http::jwt::CanonicalJwk::platform(self.platform_issuer.as_str())
            .with_status(status)
            .to_jwks_json()
    }

    pub fn organization_token(user_id: Uuid, owner: Uuid, issuer: &str) -> String {
        rustycog::testing::http::jwt::create_organization_rs256_jwt_token(user_id, owner, issuer)
    }

    pub fn organization_jwks(
        owner: Uuid,
        issuer: &str,
        status: rustycog::testing::http::jwt::TestSigningKeyStatus,
    ) -> String {
        rustycog::testing::http::jwt::CanonicalJwk::organization(issuer, owner)
            .with_status(status)
            .to_jwks_json()
    }
}

/// Create a JWT token service from configuration for testing
fn create_jwt_service_from_config(config: &JwtConfig) -> Result<Arc<JwtTokenService>> {
    if config.uses_rsa() {
        // Preserve compatibility without inventing an Active row: reuse the
        // actual owned app's provider AND primary writer emission fence.
        return crate::common::fixture_jwt_codec_from_config(config);
    }
    let iam_configuration::JwtAlgorithm::HS256(secret) = config.create_jwt_algorithm()? else {
        anyhow::bail!("isolated codec requires HMAC configuration");
    };
    Ok(Arc::new(
        JwtTokenService::with_refresh_expiration(
            iam_infra::token::JwtAlgorithm::HS256(secret),
            config.expiration_seconds,
            config.refresh_token_expiration_seconds,
        )
        .with_issuer_audience(config.platform_issuer(), config.audience.clone()),
    ))
}

/// Create a registration token service from configuration for testing.
///
/// Config-only compatibility is explicitly isolated HMAC, never an unbound RSA
/// signer. RSA tests must supply their real writer/provider-bound shared codec.
fn create_registration_token_service_from_config(
    config: &JwtConfig,
) -> Result<RegistrationTokenServiceImpl, anyhow::Error> {
    RegistrationTokenServiceImpl::new(isolated_registration_codec(config)?)
        .map_err(|e| anyhow::anyhow!("Failed to create registration token service: {e}"))
}

fn isolated_registration_codec(config: &JwtConfig) -> Result<Arc<JwtTokenService>> {
    let app_config = iam_configuration::load_config_fresh::<iam_configuration::AppConfig>()?;
    if app_config.security.mode != iam_configuration::security::SecurityMode::IsolatedTest
        || !config.uses_hmac()
    {
        anyhow::bail!(
            "config-only registration helper requires explicit isolated-test HMAC; RSA requires a bound shared codec"
        );
    }
    create_jwt_service_from_config(config)
}

/// Uses the caller's actual shared signer/primary registry; never invents an Active row.
pub async fn create_valid_registration_token_with_codec(
    user_id: Uuid,
    email: String,
    codec: Arc<JwtTokenService>,
) -> Result<String> {
    RegistrationTokenServiceImpl::new(codec)?
        .generate_registration_token(user_id, email)
        .await
        .map_err(anyhow::Error::from)
}

/// Deliberately expired completion claims, still signed through the real shared codec.
pub async fn create_expired_registration_token_with_codec(
    user_id: Uuid,
    email: String,
    platform_issuer: &str,
    codec: Arc<JwtTokenService>,
) -> Result<String> {
    // Enforce the same constructor binding checks as normal completion generation.
    let _service = RegistrationTokenServiceImpl::new(codec.clone())?;
    let claims = RegistrationTokenClaims {
        sub: "registration".to_string(),
        user_id: user_id.to_string(),
        email,
        flow: RegistrationFlow::EmailPassword,
        provider_info: None,
        iss: Some(platform_issuer.to_owned()),
        aud: Some("registration".to_owned()),
        exp: (Utc::now() - Duration::hours(1)).timestamp(),
        iat: (Utc::now() - Duration::hours(2)).timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    codec
        .encode_registration(&claims)
        .await
        .map_err(anyhow::Error::from)
}

/// JWT Test Utilities for creating and validating tokens in tests
pub struct JwtTestUtils;

impl JwtTestUtils {
    /// Create a valid JWT token for testing with JWT encoder
    pub async fn create_valid_token(
        user_id: Uuid,
        config: &JwtConfig,
    ) -> Result<String, anyhow::Error> {
        let jwt_service = create_jwt_service_from_config(config)?;

        let claims = TokenClaims {
            sub: user_id.to_string(),
            username: "test_user".to_string(),
            iss: config.platform_issuer(),
            org: None,
            aud: iam_domain::entity::token::DEFAULT_JWT_AUDIENCE.to_string(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = jwt_service
            .encode(&claims)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {e}"))?;

        Ok(token)
    }

    /// Create an expired JWT token for testing
    pub async fn create_expired_token(
        user_id: Uuid,
        config: &JwtConfig,
    ) -> Result<String, anyhow::Error> {
        let jwt_service = create_jwt_service_from_config(config)?;

        let claims = TokenClaims {
            sub: user_id.to_string(),
            username: "test_user".to_string(),
            iss: config.platform_issuer(),
            org: None,
            aud: iam_domain::entity::token::DEFAULT_JWT_AUDIENCE.to_string(),
            exp: (Utc::now() - Duration::hours(1)).timestamp(),
            iat: (Utc::now() - Duration::hours(2)).timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = jwt_service
            .encode(&claims)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {e}"))?;

        Ok(token)
    }

    /// Create an invalid JWT token for testing
    pub async fn create_invalid_token(
        user_id: Uuid,
        config: &JwtConfig,
    ) -> Result<String, anyhow::Error> {
        let jwt_service = create_jwt_service_from_config(config)?;

        let claims = TokenClaims {
            sub: user_id.to_string(),
            username: "test_user".to_string(),
            iss: config.platform_issuer(),
            org: None,
            aud: iam_domain::entity::token::DEFAULT_JWT_AUDIENCE.to_string(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let mut token = jwt_service
            .encode(&claims)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {e}"))?;

        // Corrupt the token to make it invalid
        token.push_str("invalid");

        Ok(token)
    }

    /// Create a valid registration token for testing
    pub async fn create_valid_registration_token(
        user_id: Uuid,
        email: String,
        config: &JwtConfig,
    ) -> Result<String, anyhow::Error> {
        let service = create_registration_token_service_from_config(config)?;

        service
            .generate_registration_token(user_id, email)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate registration token: {e}"))
    }

    /// Create an expired registration token for testing
    pub async fn create_expired_registration_token(
        user_id: Uuid,
        email: String,
        config: &JwtConfig,
    ) -> Result<String, anyhow::Error> {
        create_expired_registration_token_with_codec(
            user_id,
            email,
            &config.platform_issuer(),
            isolated_registration_codec(config)?,
        )
        .await
    }

    /// Create a JWT token with custom expiration
    pub async fn create_token_with_expiration(
        user_id: Uuid,
        config: &JwtConfig,
        expiration_hours: i64,
    ) -> Result<String, anyhow::Error> {
        if expiration_hours > 0 {
            Self::create_valid_token(user_id, config).await
        } else {
            Self::create_expired_token(user_id, config).await
        }
    }

    /// Verify JWT token structure (basic validation)
    pub fn verify_structure(token: &str) -> bool {
        let parts: Vec<&str> = token.split('.').collect();
        parts.len() == 3 && !parts[0].is_empty() && !parts[1].is_empty() && !parts[2].is_empty()
    }

    /// Alias for `verify_structure` to match existing code
    pub fn verify_jwt_structure(token: &str) -> bool {
        Self::verify_structure(token)
    }

    /// Decode JWT payload for testing (without signature verification)
    pub fn decode_payload(token: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        // Decode the payload (second part)
        general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .ok()
            .and_then(|decoded| String::from_utf8(decoded).ok())
            .and_then(|json_str| serde_json::from_str(&json_str).ok())
    }

    /// Assert JWT has valid structure
    pub fn assert_valid_structure(token: &str, context: &str) {
        assert!(
            Self::verify_structure(token),
            "JWT should have valid structure for {context}"
        );
    }

    /// Assert JWT payload contains expected claims
    pub fn assert_payload_claims(token: &str, expected_claims: &[&str]) {
        let payload = Self::decode_payload(token).expect("Should decode JWT payload");

        for claim in expected_claims {
            assert!(
                payload.get(claim).is_some(),
                "JWT payload should contain '{claim}' claim"
            );
        }
    }

    /// Assert JWT is not expired
    pub fn assert_not_expired(token: &str) {
        let payload = Self::decode_payload(token).expect("Should decode JWT payload");
        let exp = payload["exp"].as_i64().expect("Should have exp claim");
        let now = Utc::now().timestamp();
        assert!(exp > now, "JWT token should not be expired");
    }

    /// Assert JWT has specific subject
    pub fn assert_subject(token: &str, expected_subject: &str) {
        let payload = Self::decode_payload(token).expect("Should decode JWT payload");
        let subject = payload["sub"].as_str().expect("Should have sub claim");
        assert_eq!(
            subject, expected_subject,
            "JWT subject should match expected value"
        );
    }

    /// Create a simple test JWT token (not cryptographically valid, just for structure testing)
    pub fn create_test_token(user_id: Uuid) -> String {
        let payload = format!(r#"{{"sub":"{user_id}"}}"#);
        let encoded_payload = general_purpose::URL_SAFE_NO_PAD.encode(payload.as_bytes());
        format!("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.{encoded_payload}.signature")
    }

    /// Extract payload from JWT token for testing (basic base64 decode)
    pub fn extract_payload(token: &str) -> Option<String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        // This is a basic implementation for testing - in real scenarios you'd want proper JWT decoding
        general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .ok()
            .and_then(|decoded| String::from_utf8(decoded).ok())
    }
}

// Legacy function interfaces for backward compatibility
// These delegate to the new JwtTestUtils struct methods

/// Create a valid JWT token for testing with JWT encoder
pub async fn create_valid_jwt_token_with_encoder(
    user_id: Uuid,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_valid_token(user_id, config).await
}

/// Create an expired JWT token for testing
pub async fn create_expired_jwt_token_with_encoder(
    user_id: Uuid,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_expired_token(user_id, config).await
}

/// Create an invalid JWT token for testing
pub async fn create_invalid_jwt_token_with_encoder(
    user_id: Uuid,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_invalid_token(user_id, config).await
}

/// Create a JWT token with custom expiration
pub async fn create_jwt_token_with_expiration(
    user_id: Uuid,
    config: &JwtConfig,
    expiration_hours: i64,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_token_with_expiration(user_id, config, expiration_hours).await
}

/// Create an invalid JWT token
pub async fn create_invalid_jwt_token(
    user_id: Uuid,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_invalid_token(user_id, config).await
}

/// Create a valid registration token for testing
pub async fn create_valid_registration_token_with_encoder(
    user_id: Uuid,
    email: String,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_valid_registration_token(user_id, email, config).await
}

/// Create an expired registration token for testing
pub async fn create_expired_registration_token_with_encoder(
    user_id: Uuid,
    email: String,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_expired_registration_token(user_id, email, config).await
}
