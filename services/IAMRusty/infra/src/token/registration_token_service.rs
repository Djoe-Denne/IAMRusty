//! Registration is a business adapter over the one shared JWT crypto authority.

use async_trait::async_trait;
use iam_domain::{
    entity::registration_token::{ProviderInfo, RegistrationTokenClaims},
    error::DomainError,
    port::service::RegistrationTokenService,
};
use std::sync::Arc;
use uuid::Uuid;

use super::JwtTokenService;

#[derive(Clone)]
pub struct RegistrationTokenServiceImpl {
    jwt_codec: Arc<JwtTokenService>,
}

impl RegistrationTokenServiceImpl {
    /// Share the already-bound codec from `setup_jwt`; never create another `PEM` or
    /// signing registry. This concrete dependency stays wholly inside infra.
    ///
    /// # Errors
    /// Rejects an unbound/inconsistent RS256 codec; no synchronous DB/vendor call.
    pub fn new(jwt_codec: Arc<JwtTokenService>) -> Result<Self, DomainError> {
        jwt_codec.require_registration_configuration()?;
        Ok(Self { jwt_codec })
    }
}

#[async_trait]
impl RegistrationTokenService for RegistrationTokenServiceImpl {
    async fn generate_registration_token(
        &self,
        user_id: Uuid,
        email: String,
    ) -> Result<String, DomainError> {
        self.jwt_codec
            .encode_registration(&RegistrationTokenClaims::new(user_id, email))
            .await
    }

    async fn generate_oauth_registration_token(
        &self,
        user_id: Uuid,
        email: String,
        provider_info: ProviderInfo,
    ) -> Result<String, DomainError> {
        let mut claims = RegistrationTokenClaims::new_oauth(user_id, email);
        claims.provider_info = Some(provider_info);
        self.jwt_codec.encode_registration(&claims).await
    }

    async fn validate_registration_token(
        &self,
        token: &str,
    ) -> Result<RegistrationTokenClaims, DomainError> {
        self.jwt_codec.decode_registration(token).await
    }

    async fn is_registration_token_valid(&self, token: &str) -> bool {
        self.validate_registration_token(token).await.is_ok()
    }
}
