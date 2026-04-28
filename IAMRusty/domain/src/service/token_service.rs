use crate::entity::token::{JwkSet, TokenClaims};
use crate::error::DomainError;
use crate::port::service::JwtTokenEncoder;
use chrono::Duration;
use std::sync::Arc;

/// Service for JWT token operations
pub struct TokenService {
    token_encoder: Arc<dyn JwtTokenEncoder>,
    token_duration: Duration,
}

impl TokenService {
    /// Create a new token service
    pub fn new(token_encoder: Arc<dyn JwtTokenEncoder>, token_duration: Duration) -> Self {
        Self {
            token_encoder,
            token_duration,
        }
    }

    /// Generate a JWT token for a user
    pub fn generate_token(&self, user_id: &str, username: &str) -> Result<String, DomainError> {
        let claims = TokenClaims::new(user_id, username, self.token_duration);

        self.token_encoder
            .encode(&claims)
            .map_err(|e| DomainError::TokenGenerationFailed(e.to_string()))
    }

    /// Validate a JWT token
    pub fn validate_token(&self, token: &str) -> Result<TokenClaims, DomainError> {
        self.token_encoder.decode(token).map_err(|e| match e {
            DomainError::TokenExpired => DomainError::TokenExpired,
            DomainError::InvalidToken => DomainError::InvalidToken,
            _ => DomainError::TokenValidationFailed(e.to_string()),
        })
    }

    /// Get the JSON Web Key Set
    #[must_use]
    pub fn jwks(&self) -> JwkSet {
        self.token_encoder.jwks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::token::{JwkSet, TokenClaims};
    use crate::error::DomainError;
    use crate::port::service::JwtTokenEncoder;
    use chrono::{Duration, Utc};
    use claims::*;
    use mockall::{mock, predicate::*};
    use rstest::*;

    // Mock implementation for JwtTokenEncoder
    mock! {
        pub TokenEnc {}

        impl JwtTokenEncoder for TokenEnc {
            fn encode(&self, claims: &TokenClaims) -> Result<String, DomainError>;
            fn decode(&self, token: &str) -> Result<TokenClaims, DomainError>;
            fn jwks(&self) -> JwkSet;
        }
    }

    // Test fixtures
    #[fixture]
    fn token_duration() -> Duration {
        Duration::hours(1)
    }

    #[fixture]
    fn sample_user_id() -> String {
        "550e8400-e29b-41d4-a716-446655440000".to_string()
    }

    #[fixture]
    fn sample_username() -> String {
        "testuser".to_string()
    }

    #[fixture]
    fn sample_jwt_token() -> String {
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyIiwibmFtZSI6InRlc3QiLCJpYXQiOjE1MTYyMzkwMjJ9.signature".to_string()
    }

    #[fixture]
    fn sample_token_claims(
        sample_user_id: String,
        sample_username: String,
        token_duration: Duration,
    ) -> TokenClaims {
        TokenClaims::new(&sample_user_id, &sample_username, token_duration)
    }

    #[fixture]
    fn sample_jwks() -> JwkSet {
        JwkSet { keys: vec![] }
    }

    #[fixture]
    fn token_service(token_duration: Duration) -> TokenService {
        let mock_encoder = MockTokenEnc::new();
        TokenService::new(Arc::new(mock_encoder), token_duration)
    }

    mod token_service_creation {
        use super::*;

        #[rstest]
        #[test]
        fn new_creates_token_service_with_correct_duration(token_duration: Duration) {
            let mock_encoder = MockTokenEnc::new();

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            assert_eq!(service.token_duration, token_duration);
        }

        #[rstest]
        #[test]
        fn new_stores_token_encoder(token_duration: Duration) {
            let mock_encoder = MockTokenEnc::new();

            let _service = TokenService::new(Arc::new(mock_encoder), token_duration);

            // Test passes if no panic occurs during creation
        }
    }

    mod generate_token {
        use super::*;

        #[rstest]
        #[test]
        fn success_with_valid_inputs(
            sample_user_id: String,
            sample_username: String,
            token_duration: Duration,
        ) {
            let mut mock_encoder = MockTokenEnc::new();
            let expected_token = "expected_jwt_token".to_string();
            let expected_token_clone = expected_token.clone();

            mock_encoder
                .expect_encode()
                .times(1)
                .returning(move |_| Ok(expected_token_clone.clone()));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.generate_token(&sample_user_id, &sample_username);

            assert_ok!(&result);
            assert_eq!(result.unwrap(), expected_token);
        }

        #[rstest]
        #[test]
        fn creates_claims_with_correct_duration(
            sample_user_id: String,
            sample_username: String,
            token_duration: Duration,
        ) {
            let mut mock_encoder = MockTokenEnc::new();
            let user_id_clone = sample_user_id.clone();
            let username_clone = sample_username.clone();
            mock_encoder
                .expect_encode()
                .times(1)
                .withf(move |claims: &TokenClaims| {
                    claims.sub == user_id_clone
                        && claims.username == username_clone
                        && claims.exp > Utc::now().timestamp() as i64 // Token should expire in the future
                })
                .returning(|_| Ok("token".to_string()));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.generate_token(&sample_user_id, &sample_username);

            assert_ok!(&result);
        }

        #[rstest]
        #[test]
        fn error_when_encoder_fails(
            sample_user_id: String,
            sample_username: String,
            token_duration: Duration,
        ) {
            let mut mock_encoder = MockTokenEnc::new();
            mock_encoder
                .expect_encode()
                .times(1)
                .returning(|_| Err(DomainError::InvalidToken));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.generate_token(&sample_user_id, &sample_username);

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::TokenGenerationFailed(msg) => {
                    assert!(msg.contains("Invalid token"));
                }
                _ => panic!("Expected TokenGenerationFailed error"),
            }
        }

        #[rstest]
        #[case("", "username", "Empty user ID should work")]
        #[case("user_id", "", "Empty username should work")]
        #[case(
            "very_long_user_id_that_exceeds_normal_length_but_should_still_work",
            "very_long_username_that_exceeds_normal_length",
            "Long strings should work"
        )]
        #[test]
        fn handles_edge_case_inputs(
            #[case] user_id: &str,
            #[case] username: &str,
            #[case] _description: &str,
            token_duration: Duration,
        ) {
            let mut mock_encoder = MockTokenEnc::new();
            mock_encoder
                .expect_encode()
                .times(1)
                .returning(|_| Ok("token".to_string()));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.generate_token(user_id, username);

            assert_ok!(&result);
        }
    }

    mod validate_token {
        use super::*;

        #[rstest]
        #[test]
        fn success_with_valid_token(
            sample_jwt_token: String,
            sample_user_id: String,
            sample_username: String,
            token_duration: Duration,
        ) {
            let mut mock_encoder = MockTokenEnc::new();
            let expected_claims =
                TokenClaims::new(&sample_user_id, &sample_username, token_duration);
            let expected_claims_clone = expected_claims.clone();

            mock_encoder
                .expect_decode()
                .with(eq(sample_jwt_token.clone()))
                .times(1)
                .returning(move |_| Ok(expected_claims_clone.clone()));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.validate_token(&sample_jwt_token);

            assert_ok!(&result);
            let claims = result.unwrap();
            assert_eq!(claims.sub, expected_claims.sub);
            assert_eq!(claims.username, expected_claims.username);
        }

        #[rstest]
        #[test]
        fn preserves_token_expired_error(sample_jwt_token: String, token_duration: Duration) {
            let mut mock_encoder = MockTokenEnc::new();
            mock_encoder
                .expect_decode()
                .times(1)
                .returning(|_| Err(DomainError::TokenExpired));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.validate_token(&sample_jwt_token);

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::TokenExpired => {} // Expected
                _ => panic!("Expected TokenExpired error"),
            }
        }

        #[rstest]
        #[test]
        fn preserves_invalid_token_error(sample_jwt_token: String, token_duration: Duration) {
            let mut mock_encoder = MockTokenEnc::new();
            mock_encoder
                .expect_decode()
                .times(1)
                .returning(|_| Err(DomainError::InvalidToken));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.validate_token(&sample_jwt_token);

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::InvalidToken => {} // Expected
                _ => panic!("Expected InvalidToken error"),
            }
        }

        #[rstest]
        #[test]
        fn converts_other_errors_to_validation_failed(
            sample_jwt_token: String,
            token_duration: Duration,
        ) {
            let mut mock_encoder = MockTokenEnc::new();
            mock_encoder
                .expect_decode()
                .times(1)
                .returning(|_| Err(DomainError::RepositoryError("Database error".to_string())));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.validate_token(&sample_jwt_token);

            assert_err!(&result);
            match result.unwrap_err() {
                DomainError::TokenValidationFailed(msg) => {
                    assert!(msg.contains("Database error"));
                }
                _ => panic!("Expected TokenValidationFailed error"),
            }
        }

        #[rstest]
        #[case("", "Empty token")]
        #[case("invalid", "Invalid format token")]
        #[case("Bearer token", "Token with prefix")]
        #[case("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9", "Incomplete JWT")]
        #[test]
        fn handles_various_invalid_token_formats(
            #[case] invalid_token: &str,
            #[case] _description: &str,
            token_duration: Duration,
        ) {
            let mut mock_encoder = MockTokenEnc::new();
            mock_encoder
                .expect_decode()
                .times(1)
                .returning(|_| Err(DomainError::InvalidToken));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.validate_token(invalid_token);

            assert_err!(&result);
        }
    }

    mod jwks {
        use super::*;

        #[rstest]
        #[test]
        fn returns_jwks_from_encoder(token_duration: Duration) {
            let mut mock_encoder = MockTokenEnc::new();
            let expected_jwks = JwkSet { keys: vec![] };
            let expected_jwks_clone = expected_jwks.clone();

            mock_encoder
                .expect_jwks()
                .times(1)
                .returning(move || expected_jwks_clone.clone());

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let result = service.jwks();

            assert_eq!(result.keys.len(), expected_jwks.keys.len());
        }

        #[rstest]
        #[test]
        fn calls_encoder_jwks_method(token_duration: Duration) {
            let mut mock_encoder = MockTokenEnc::new();
            mock_encoder
                .expect_jwks()
                .times(1)
                .returning(|| JwkSet { keys: vec![] });

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            let _result = service.jwks();

            // Test passes if mock expectations are met
        }
    }

    mod integration_tests {
        use super::*;

        #[rstest]
        #[test]
        fn generate_and_validate_token_workflow(
            sample_user_id: String,
            sample_username: String,
            token_duration: Duration,
        ) {
            let mut mock_encoder = MockTokenEnc::new();
            let expected_token = "test_jwt_token".to_string();
            let expected_token_clone = expected_token.clone();
            let expected_claims =
                TokenClaims::new(&sample_user_id, &sample_username, token_duration);
            let expected_claims_clone = expected_claims.clone();

            // Setup for generate_token call
            mock_encoder
                .expect_encode()
                .times(1)
                .returning(move |_| Ok(expected_token_clone.clone()));

            // Setup for validate_token call
            mock_encoder
                .expect_decode()
                .times(1)
                .returning(move |_| Ok(expected_claims_clone.clone()));

            let service = TokenService::new(Arc::new(mock_encoder), token_duration);

            // Generate token
            let generate_result = service.generate_token(&sample_user_id, &sample_username);
            assert_ok!(&generate_result);
            let token = generate_result.unwrap();

            // Validate token
            let validate_result = service.validate_token(&token);
            assert_ok!(&validate_result);
            let claims = validate_result.unwrap();

            assert_eq!(claims.sub, expected_claims.sub);
            assert_eq!(claims.username, expected_claims.username);
        }
    }
}
