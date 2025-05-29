use async_trait::async_trait;
use application::usecase::auth::TokenService as AppTokenService;
use super::JwtTokenService;
use domain::port::service::TokenService;
use std::sync::Arc;
use uuid::Uuid;

/// Adapter that bridges the application's TokenService trait with the infrastructure implementation
pub struct TokenServiceAdapter {
    jwt_service: Arc<JwtTokenService>,
}

impl TokenServiceAdapter {
    pub fn new(jwt_service: Arc<JwtTokenService>) -> Self {
        Self {
            jwt_service,
        }
    }
}

#[async_trait]
impl AppTokenService for TokenServiceAdapter {
    async fn generate_jwt_for_user(&self, user_id: Uuid) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let jwt_token = self.jwt_service
            .generate_access_token(user_id)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
        Ok(jwt_token.token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_generate_jwt_for_user() {
        let jwt_service = Arc::new(JwtTokenService::new(
            "test_secret_key_that_is_long_enough_for_HS256".to_string(),
            3600,
        ));
        let adapter = TokenServiceAdapter::new(jwt_service);
        
        let user_id = Uuid::new_v4();
        let token = adapter.generate_jwt_for_user(user_id).await.unwrap();
        
        // Should generate a valid JWT token string
        assert!(!token.is_empty());
        assert!(token.contains('.'));  // JWT tokens have dots as separators
    }

    #[tokio::test]
    async fn test_different_users_get_different_tokens() {
        let jwt_service = Arc::new(JwtTokenService::new(
            "test_secret_key_that_is_long_enough_for_HS256".to_string(),
            3600,
        ));
        let adapter = TokenServiceAdapter::new(jwt_service);
        
        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();
        
        let token1 = adapter.generate_jwt_for_user(user_id1).await.unwrap();
        let token2 = adapter.generate_jwt_for_user(user_id2).await.unwrap();
        
        assert_ne!(token1, token2);
    }
} 