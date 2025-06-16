use anyhow::Result;
use chrono::{Duration, Utc};
use configuration::AppConfig;
use domain::entity::registration_token::{RegistrationFlow, RegistrationTokenClaims};
use domain::entity::token::TokenClaims;
use domain::port::service::{JwtTokenEncoder, RegistrationTokenService};
use infra::token::{JwtTokenService, registration_token_service::RegistrationTokenServiceImpl};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use uuid::Uuid;

/// Create a JWT token service from configuration for testing
fn create_jwt_service_from_config(config: &AppConfig) -> Result<JwtTokenService, anyhow::Error> {
    let jwt_algorithm_config = config.jwt.create_jwt_algorithm()?;

    let jwt_algorithm = match jwt_algorithm_config {
        configuration::JwtAlgorithm::HS256(secret) => infra::token::JwtAlgorithm::HS256(secret),
        configuration::JwtAlgorithm::RS256(key_pair) => {
            infra::token::JwtAlgorithm::RS256(domain::entity::token::JwtKeyPair {
                private_key: key_pair.private_key,
                public_key: key_pair.public_key,
                kid: key_pair.kid,
            })
        }
    };

    Ok(JwtTokenService::with_refresh_expiration(
        jwt_algorithm,
        config.jwt.expiration_seconds,
        config.jwt.refresh_token_expiration_seconds,
    ))
}

/// Create a valid JWT token for testing with JWT encoder
pub fn create_valid_jwt_token_with_encoder(
    user_id: Uuid,
    config: &AppConfig,
) -> Result<String, anyhow::Error> {
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
pub fn create_expired_jwt_token_with_encoder(
    user_id: Uuid,
    config: &AppConfig,
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
pub fn create_invalid_jwt_token_with_encoder(
    user_id: Uuid,
    config: &AppConfig,
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

/// Create a JWT token with custom expiration
pub fn create_jwt_token_with_expiration(
    user_id: Uuid,
    config: AppConfig,
    expiration_hours: i64,
) -> Result<String, anyhow::Error> {
    if expiration_hours > 0 {
        let token = create_valid_jwt_token_with_encoder(user_id, &config)
            .map_err(|e| anyhow::anyhow!("Failed to create valid JWT token: {}", e))?;
        Ok(token)
    } else {
        let token = create_expired_jwt_token_with_encoder(user_id, &config)
            .map_err(|e| anyhow::anyhow!("Failed to create expired JWT token: {}", e))?;
        Ok(token)
    }
}

/// Create an invalid JWT token
pub fn create_invalid_jwt_token(user_id: Uuid, config: AppConfig) -> Result<String, anyhow::Error> {
    let token = create_invalid_jwt_token_with_encoder(user_id, &config)
        .map_err(|e| anyhow::anyhow!("Failed to create invalid JWT token: {}", e))?;
    Ok(token)
}

/// Create a registration token service from configuration for testing
fn create_registration_token_service_from_config(
    config: &AppConfig,
) -> Result<RegistrationTokenServiceImpl, anyhow::Error> {
    let jwt_algorithm_config = config.jwt.create_jwt_algorithm()?;

    let jwt_algorithm = match jwt_algorithm_config {
        configuration::JwtAlgorithm::HS256(_) => {
            return Err(anyhow::anyhow!(
                "Registration tokens must use RSA256 algorithm"
            ));
        }
        configuration::JwtAlgorithm::RS256(key_pair) => {
            infra::token::JwtAlgorithm::RS256(domain::entity::token::JwtKeyPair {
                private_key: key_pair.private_key,
                public_key: key_pair.public_key,
                kid: key_pair.kid,
            })
        }
    };

    RegistrationTokenServiceImpl::new(jwt_algorithm)
        .map_err(|e| anyhow::anyhow!("Failed to create registration token service: {}", e))
}

/// Create a valid registration token for testing
pub fn create_valid_registration_token_with_encoder(
    user_id: Uuid,
    email: String,
    config: &AppConfig,
) -> Result<String, anyhow::Error> {
    let service = create_registration_token_service_from_config(config)?;

    service
        .generate_registration_token(user_id, email)
        .map_err(|e| anyhow::anyhow!("Failed to generate registration token: {}", e))
}

/// Create an expired registration token for testing
pub fn create_expired_registration_token_with_encoder(
    user_id: Uuid,
    email: String,
    config: &AppConfig,
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

    // Get the algorithm config to encode manually
    let jwt_algorithm_config = config.jwt.create_jwt_algorithm()?;

    let (encoding_key, kid) = match jwt_algorithm_config {
        configuration::JwtAlgorithm::RS256(key_pair) => {
            let encoding_key = EncodingKey::from_rsa_pem(key_pair.private_key.as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to create encoding key: {}", e))?;
            (encoding_key, Some(key_pair.kid))
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Registration tokens must use RSA256 algorithm"
            ));
        }
    };

    // Create header with proper algorithm and key ID
    let mut header = Header::new(Algorithm::RS256);
    header.kid = kid;

    encode(&header, &expired_claims, &encoding_key)
        .map_err(|e| anyhow::anyhow!("Failed to encode expired registration token: {}", e))
}
