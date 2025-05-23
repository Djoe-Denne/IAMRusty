use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, HeaderMap, StatusCode},
};
use application::service::TokenService;
use domain::entity::token::TokenClaims;
use crate::error::ApiError;

/// JWT auth extractor for Axum
#[derive(Debug, Clone)]
pub struct JwtAuth(pub TokenClaims);

#[async_trait]
impl<S> FromRequestParts<S> for JwtAuth
where
    S: Send + Sync,
    State<TokenService>: FromRequestParts<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let State(token_service) = State::<TokenService>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::InternalServerError("Token service not available".to_string()))?;

        let headers = HeaderMap::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::InternalServerError("Failed to extract headers".to_string()))?;

        let auth_header = headers
            .get("Authorization")
            .ok_or(ApiError::AuthenticationRequired)?
            .to_str()
            .map_err(|_| ApiError::AuthenticationRequired)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::AuthenticationRequired);
        }

        let token = auth_header[7..].trim();
        
        let claims = token_service
            .validate_token(token)
            .map_err(ApiError::Application)?;

        Ok(JwtAuth(claims))
    }
} 