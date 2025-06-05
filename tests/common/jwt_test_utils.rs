use uuid::Uuid;
use chrono::{Utc, Duration};
use configuration::AppConfig;
use domain::entity::token::TokenClaims;
use domain::port::service::JwtTokenEncoder;
use infra::token::JwtTokenService;
use anyhow::Result;

/// Create a JWT token service from configuration for testing
fn create_jwt_service_from_config(config: &AppConfig) -> Result<JwtTokenService, anyhow::Error> {
    let jwt_algorithm_config = config.jwt.create_jwt_algorithm()?;
    
    let jwt_algorithm = match jwt_algorithm_config {
        configuration::JwtAlgorithm::HS256(secret) => {
            infra::token::JwtAlgorithm::HS256(secret)
        }
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
    config: &AppConfig
) -> Result<String, anyhow::Error> {
    let jwt_service = create_jwt_service_from_config(config)?;
    
    let claims = TokenClaims {
        sub: user_id.to_string(),
        username: "test_user".to_string(),
        exp: (Utc::now() + Duration::hours(1)).timestamp(),
        iat: Utc::now().timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    
    let token = jwt_service.encode(&claims)
        .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {}", e))?;
    
    Ok(token)
}

/// Create an expired JWT token for testing
pub fn create_expired_jwt_token_with_encoder(
    user_id: Uuid,
    config: &AppConfig
) -> Result<String, anyhow::Error> {
    let jwt_service = create_jwt_service_from_config(config)?;
    
    let claims = TokenClaims {
        sub: user_id.to_string(),
        username: "test_user".to_string(),
        exp: (Utc::now() - Duration::hours(1)).timestamp(),
        iat: (Utc::now() - Duration::hours(2)).timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    
    let token = jwt_service.encode(&claims)
        .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {}", e))?;
    
    Ok(token)
}

/// Create an invalid JWT token for testing
pub fn create_invalid_jwt_token_with_encoder(
    user_id: Uuid,
    config: &AppConfig
) -> Result<String, anyhow::Error> {
    let jwt_service = create_jwt_service_from_config(config)?;
    
    let claims = TokenClaims {
        sub: user_id.to_string(),
        username: "test_user".to_string(),
        exp: (Utc::now() + Duration::hours(1)).timestamp(),
        iat: Utc::now().timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    
    let mut token = jwt_service.encode(&claims)
        .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {}", e))?;
    
    // Corrupt the token to make it invalid
    token.push_str("invalid");
    
    Ok(token)
}

/// Create a JWT token with custom expiration
pub fn create_jwt_token_with_expiration(user_id: Uuid, config: AppConfig, expiration_hours: i64) -> Result<String, anyhow::Error> {
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