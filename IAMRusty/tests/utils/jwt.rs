use anyhow::Result;
use base64;
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use iam_configuration::{JwtAlgorithm, JwtConfig};
use iam_domain::entity::registration_token::{RegistrationFlow, RegistrationTokenClaims};
use iam_domain::entity::token::{JwtKeyPair, TokenClaims};
use iam_domain::port::service::{JwtTokenEncoder, RegistrationTokenService};
use iam_infra::token::{registration_token_service::RegistrationTokenServiceImpl, JwtTokenService};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use uuid::Uuid;

/// Create a JWT token service from configuration for testing
fn create_jwt_service_from_config(config: &JwtConfig) -> Result<JwtTokenService, anyhow::Error> {
    let jwt_algorithm_config = config.create_jwt_algorithm()?;

    let jwt_algorithm = match jwt_algorithm_config {
        JwtAlgorithm::HS256(secret) => iam_infra::token::JwtAlgorithm::HS256(secret),
        JwtAlgorithm::RS256(key_pair) => iam_infra::token::JwtAlgorithm::RS256(JwtKeyPair {
            private_key: key_pair.private_key,
            public_key: key_pair.public_key,
            kid: key_pair.kid,
        }),
    };

    Ok(JwtTokenService::with_refresh_expiration(
        jwt_algorithm,
        config.expiration_seconds,
        config.refresh_token_expiration_seconds,
    ))
}

/// Create a registration token service from configuration for testing.
///
/// The `test-relaxed-jwt` feature on `iam-infra` (enabled by
/// `IAMRusty/Cargo.toml`'s dev-dep entry) lets `RegistrationTokenServiceImpl`
/// accept either algorithm in this crate's tests, so we hand whatever
/// the test config produced straight through instead of duplicating an
/// RS256-only guard here. Production builds keep the strict check inside
/// the constructor itself.
fn create_registration_token_service_from_config(
    config: &JwtConfig,
) -> Result<RegistrationTokenServiceImpl, anyhow::Error> {
    let jwt_algorithm_config = config.create_jwt_algorithm()?;

    let jwt_algorithm = match jwt_algorithm_config {
        JwtAlgorithm::HS256(secret) => iam_infra::token::JwtAlgorithm::HS256(secret),
        JwtAlgorithm::RS256(key_pair) => iam_infra::token::JwtAlgorithm::RS256(JwtKeyPair {
            private_key: key_pair.private_key,
            public_key: key_pair.public_key,
            kid: key_pair.kid,
        }),
    };

    RegistrationTokenServiceImpl::new(jwt_algorithm)
        .map_err(|e| anyhow::anyhow!("Failed to create registration token service: {}", e))
}

/// JWT Test Utilities for creating and validating tokens in tests
pub struct JwtTestUtils;

impl JwtTestUtils {
    /// Create a valid JWT token for testing with JWT encoder
    pub fn create_valid_token(user_id: Uuid, config: &JwtConfig) -> Result<String, anyhow::Error> {
        let jwt_service = create_jwt_service_from_config(config)?;

        let claims = TokenClaims {
            sub: user_id.to_string(),
            username: "test_user".to_string(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = jwt_service
            .encode(&claims)
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {}", e))?;

        Ok(token)
    }

    /// Create an expired JWT token for testing
    pub fn create_expired_token(
        user_id: Uuid,
        config: &JwtConfig,
    ) -> Result<String, anyhow::Error> {
        let jwt_service = create_jwt_service_from_config(config)?;

        let claims = TokenClaims {
            sub: user_id.to_string(),
            username: "test_user".to_string(),
            exp: (Utc::now() - Duration::hours(1)).timestamp(),
            iat: (Utc::now() - Duration::hours(2)).timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = jwt_service
            .encode(&claims)
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {}", e))?;

        Ok(token)
    }

    /// Create an invalid JWT token for testing
    pub fn create_invalid_token(
        user_id: Uuid,
        config: &JwtConfig,
    ) -> Result<String, anyhow::Error> {
        let jwt_service = create_jwt_service_from_config(config)?;

        let claims = TokenClaims {
            sub: user_id.to_string(),
            username: "test_user".to_string(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let mut token = jwt_service
            .encode(&claims)
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {}", e))?;

        // Corrupt the token to make it invalid
        token.push_str("invalid");

        Ok(token)
    }

    /// Create a valid registration token for testing
    pub fn create_valid_registration_token(
        user_id: Uuid,
        email: String,
        config: &JwtConfig,
    ) -> Result<String, anyhow::Error> {
        let service = create_registration_token_service_from_config(config)?;

        service
            .generate_registration_token(user_id, email)
            .map_err(|e| anyhow::anyhow!("Failed to generate registration token: {}", e))
    }

    /// Create an expired registration token for testing
    pub fn create_expired_registration_token(
        user_id: Uuid,
        email: String,
        config: &JwtConfig,
    ) -> Result<String, anyhow::Error> {
        // Create claims that are already expired
        let expired_claims = RegistrationTokenClaims {
            sub: "registration".to_string(),
            user_id: user_id.to_string(),
            email,
            flow: RegistrationFlow::EmailPassword, // Default to email/password flow for tests
            provider_info: None,                   // No provider info for email/password flow
            exp: (Utc::now() - Duration::hours(1)).timestamp(), // Expired 1 hour ago
            iat: (Utc::now() - Duration::hours(2)).timestamp(), // Issued 2 hours ago
            jti: Uuid::new_v4().to_string(),
        };

        // Get the algorithm config to encode manually. The
        // `test-relaxed-jwt` feature on `iam-infra` lets the
        // registration service accept either algorithm in tests, so
        // build the encoding key from whatever the test config
        // produced. The header's `alg` field then has to match the key
        // material — RS256 from PEM, HS256 from a shared secret —
        // otherwise `jsonwebtoken::encode` rejects the combination.
        let jwt_algorithm_config = config.create_jwt_algorithm()?;

        let (encoding_key, kid, algorithm) = match jwt_algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                let encoding_key = EncodingKey::from_rsa_pem(key_pair.private_key.as_bytes())
                    .map_err(|e| anyhow::anyhow!("Failed to create encoding key: {}", e))?;
                (encoding_key, Some(key_pair.kid), Algorithm::RS256)
            }
            JwtAlgorithm::HS256(secret) => {
                let encoding_key = EncodingKey::from_secret(secret.as_bytes());
                (encoding_key, None, Algorithm::HS256)
            }
        };

        // Create header with proper algorithm and (optional) key ID
        let mut header = Header::new(algorithm);
        header.kid = kid;

        encode(&header, &expired_claims, &encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to encode expired registration token: {}", e))
    }

    /// Create a JWT token with custom expiration
    pub fn create_token_with_expiration(
        user_id: Uuid,
        config: &JwtConfig,
        expiration_hours: i64,
    ) -> Result<String, anyhow::Error> {
        if expiration_hours > 0 {
            Self::create_valid_token(user_id, config)
        } else {
            Self::create_expired_token(user_id, config)
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
        match general_purpose::URL_SAFE_NO_PAD.decode(parts[1]) {
            Ok(decoded) => match String::from_utf8(decoded) {
                Ok(json_str) => serde_json::from_str(&json_str).ok(),
                Err(_) => None,
            },
            Err(_) => None,
        }
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
        match general_purpose::URL_SAFE_NO_PAD.decode(parts[1]) {
            Ok(decoded) => String::from_utf8(decoded).ok(),
            Err(_) => None,
        }
    }
}

// Legacy function interfaces for backward compatibility
// These delegate to the new JwtTestUtils struct methods

/// Create a valid JWT token for testing with JWT encoder
pub fn create_valid_jwt_token_with_encoder(
    user_id: Uuid,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_valid_token(user_id, config)
}

/// Create an expired JWT token for testing
pub fn create_expired_jwt_token_with_encoder(
    user_id: Uuid,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_expired_token(user_id, config)
}

/// Create an invalid JWT token for testing
pub fn create_invalid_jwt_token_with_encoder(
    user_id: Uuid,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_invalid_token(user_id, config)
}

/// Create a JWT token with custom expiration
pub fn create_jwt_token_with_expiration(
    user_id: Uuid,
    config: JwtConfig,
    expiration_hours: i64,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_token_with_expiration(user_id, &config, expiration_hours)
}

/// Create an invalid JWT token
pub fn create_invalid_jwt_token(user_id: Uuid, config: JwtConfig) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_invalid_token(user_id, &config)
}

/// Create a valid registration token for testing
pub fn create_valid_registration_token_with_encoder(
    user_id: Uuid,
    email: String,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_valid_registration_token(user_id, email, config)
}

/// Create an expired registration token for testing
pub fn create_expired_registration_token_with_encoder(
    user_id: Uuid,
    email: String,
    config: &JwtConfig,
) -> Result<String, anyhow::Error> {
    JwtTestUtils::create_expired_registration_token(user_id, email, config)
}
