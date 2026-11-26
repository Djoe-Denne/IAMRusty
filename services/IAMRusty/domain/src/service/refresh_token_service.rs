use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use crate::entity::token::JwkSet;
use crate::error::DomainError;
use crate::port::{
    repository::{IdentityRepository, RefreshTokenRepository},
    service::{AuthTokenService, JwtTokenEncoder},
};

/// Response for token refresh
#[derive(Debug)]
pub struct RefreshTokenResponse {
    /// New access token
    pub access_token: String,
    /// Access token expiration time in seconds
    pub expires_in: u64,
    /// New refresh token (replaces the old one)
    pub refresh_token: String,
    /// Refresh token expiration time in seconds
    pub refresh_expires_in: u64,
    /// User that owns the refreshed session
    pub user_id: Uuid,
}

/// Refresh token domain service trait
#[async_trait]
pub trait RefreshTokenService: Send + Sync {
    /// Refresh an access token using a refresh token
    async fn refresh_token(
        &self,
        refresh_token: String,
    ) -> Result<RefreshTokenResponse, DomainError>;

    /// Revoke a refresh token
    async fn revoke_token(&self, refresh_token: String) -> Result<(), DomainError>;

    /// Revoke all refresh tokens for a user
    async fn revoke_all_tokens(&self, user_id: Uuid) -> Result<u64, DomainError>;

    /// Get the JSON Web Key Set (JWKS) for token verification
    fn get_jwks(&self) -> JwkSet;
}

/// Refresh token domain service implementation
pub struct RefreshTokenServiceImpl<R, T>
where
    R: RefreshTokenRepository,
    T: AuthTokenService + JwtTokenEncoder,
{
    refresh_token_repo: Arc<R>,
    token_service: Arc<T>,
    identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
    platform_issuer: String,
}

impl<R, T> RefreshTokenServiceImpl<R, T>
where
    R: RefreshTokenRepository,
    T: AuthTokenService + JwtTokenEncoder,
{
    /// Create a new `RefreshTokenServiceImpl`
    pub fn new(
        refresh_token_repo: Arc<R>,
        token_service: Arc<T>,
        identity_repo: Arc<dyn IdentityRepository<Error = DomainError>>,
        platform_issuer: String,
    ) -> Self {
        Self {
            refresh_token_repo,
            token_service,
            identity_repo,
            platform_issuer,
        }
    }
    fn is_rotation_lost_race(error: &dyn std::error::Error) -> bool {
        let message = error.to_string();
        message.contains("RecordNotFound") || message.contains("Refresh token not found")
    }
}

#[async_trait]
impl<R, T> RefreshTokenService for RefreshTokenServiceImpl<R, T>
where
    R: RefreshTokenRepository + Send + Sync,
    T: AuthTokenService + JwtTokenEncoder + Send + Sync,
    <R as RefreshTokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    async fn refresh_token(
        &self,
        refresh_token: String,
    ) -> Result<RefreshTokenResponse, DomainError> {
        // Find the refresh token
        let old_token = self
            .refresh_token_repo
            .find_by_token(&refresh_token)
            .await
            .map_err(|_| {
                debug!("Error finding refresh session");
                DomainError::RepositoryError("refresh session lookup failed".into())
            })?
            .ok_or(DomainError::TokenNotFound)?;

        // Check if the token is valid
        if !old_token.is_valid {
            debug!(token_id = %old_token.id, "Invalid refresh session");
            return Err(DomainError::InvalidToken);
        }

        // Check if the token is expired
        let now = Utc::now();
        if old_token.expires_at < now {
            // Invalidate the expired token
            self.refresh_token_repo
                .update_validity(old_token.id, false)
                .await
                .map_err(|_| {
                    debug!("Error updating refresh session validity");
                    DomainError::RepositoryError("refresh session invalidation failed".into())
                })?;

            return Err(DomainError::TokenExpired);
        }

        // Generate a new access token
        let new_access_token = self
            .token_service
            .generate_access_token(old_token.user_id, old_token.auth_epoch)
            .await
            .map_err(|_| {
                debug!("Error generating access token");
                DomainError::TokenServiceError("access token generation failed".into())
            })?;

        // Generate a new refresh token
        let new_refresh_token = self
            .token_service
            .generate_refresh_token(old_token.user_id, old_token.auth_epoch)
            .await
            .map_err(|_| {
                debug!("Error generating refresh token");
                DomainError::TokenServiceError("refresh token generation failed".into())
            })?;

        // Finish fallible principal assurance before the session writer locks the user.
        self.identity_repo
            .ensure_platform_identity(old_token.user_id, &self.platform_issuer)
            .await?;

        // Store the new refresh token and delete the old token atomically.
        self.refresh_token_repo
            .rotate(old_token.id, new_refresh_token.clone())
            .await
            .map_err(|e| {
                debug!("Error rotating refresh session");
                if Self::is_rotation_lost_race(&e) {
                    DomainError::InvalidToken
                } else {
                    DomainError::RepositoryError("refresh session rotation failed".into())
                }
            })?;

        // Calculate expiration times in seconds
        let access_expires_in =
            u64::try_from((new_access_token.expires_at - now).num_seconds()).unwrap_or(0);

        let refresh_expires_in =
            u64::try_from((new_refresh_token.expires_at - now).num_seconds()).unwrap_or(0);

        Ok(RefreshTokenResponse {
            access_token: new_access_token.token,
            expires_in: access_expires_in,
            refresh_token: new_refresh_token.token,
            refresh_expires_in,
            user_id: old_token.user_id,
        })
    }

    async fn revoke_token(&self, refresh_token: String) -> Result<(), DomainError> {
        // Find the refresh token
        let token = self
            .refresh_token_repo
            .find_by_token(&refresh_token)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::TokenNotFound)?;

        // Invalidate the token
        self.refresh_token_repo
            .update_validity(token.id, false)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        Ok(())
    }

    async fn revoke_all_tokens(&self, user_id: Uuid) -> Result<u64, DomainError> {
        // Delete all refresh tokens for the user
        let count = self
            .refresh_token_repo
            .delete_by_user_id(user_id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        Ok(count)
    }

    fn get_jwks(&self) -> JwkSet {
        self.token_service.jwks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity::{
            identity::Identity,
            token::{JwtToken, RefreshToken, TokenClaims},
        },
        port::repository::{RefreshTokenReadRepository, RefreshTokenWriteRepository},
    };
    use mockall::{mock, Sequence};

    mock! {
        Sessions {}
        #[async_trait::async_trait]
        impl RefreshTokenReadRepository for Sessions {
            type Error = DomainError;
            async fn find_by_token(&self, token: &str) -> Result<Option<RefreshToken>, DomainError>;
            async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<RefreshToken>, DomainError>;
        }
        #[async_trait::async_trait]
        impl RefreshTokenWriteRepository for Sessions {
            type Error = DomainError;
            async fn create(&self, token: RefreshToken) -> Result<RefreshToken, DomainError>;
            async fn update_validity(&self, token_id: Uuid, is_valid: bool) -> Result<(), DomainError>;
            async fn delete_by_id(&self, token_id: Uuid) -> Result<(), DomainError>;
            async fn rotate(&self, old_token_id: Uuid, new_token: RefreshToken) -> Result<RefreshToken, DomainError>;
            async fn delete_by_user_id(&self, user_id: Uuid) -> Result<u64, DomainError>;
        }
    }

    mock! {
        Identities {}
        #[async_trait::async_trait]
        impl IdentityRepository for Identities {
            type Error = DomainError;
            async fn find_by_issuer_subject(&self, issuer: &str, subject: &str) -> Result<Option<Identity>, DomainError>;
            async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Identity>, DomainError>;
            async fn create(&self, identity: &Identity) -> Result<Identity, DomainError>;
            async fn ensure_platform_identity(&self, user_id: Uuid, issuer: &str) -> Result<Identity, DomainError>;
            async fn ensure_organization_managed_identity(&self, user_id: Uuid, issuer: &str) -> Result<Identity, DomainError>;
        }
    }

    #[async_trait::async_trait]
    trait TestTokenCodec: Send + Sync {
        async fn issue_access(
            &self,
            user_id: Uuid,
            auth_epoch: u64,
        ) -> Result<JwtToken, DomainError>;
        async fn issue_refresh(
            &self,
            user_id: Uuid,
            auth_epoch: u64,
        ) -> Result<RefreshToken, DomainError>;
        async fn check_access(&self, token: &str) -> Result<Uuid, DomainError>;
        async fn check_refresh(&self, token: &str) -> Result<RefreshToken, DomainError>;
        async fn sign(&self, claims: &TokenClaims) -> Result<String, DomainError>;
        fn verify(&self, token: &str) -> Result<TokenClaims, DomainError>;
        fn keys(&self) -> JwkSet;
    }

    mock! {
        Codec {}
        #[async_trait::async_trait]
        impl TestTokenCodec for Codec {
            async fn issue_access(&self, user_id: Uuid, auth_epoch: u64) -> Result<JwtToken, DomainError>;
            async fn issue_refresh(&self, user_id: Uuid, auth_epoch: u64) -> Result<RefreshToken, DomainError>;
            async fn check_access(&self, token: &str) -> Result<Uuid, DomainError>;
            async fn check_refresh(&self, token: &str) -> Result<RefreshToken, DomainError>;
            async fn sign(&self, claims: &TokenClaims) -> Result<String, DomainError>;
            fn verify(&self, token: &str) -> Result<TokenClaims, DomainError>;
            fn keys(&self) -> JwkSet;
        }
    }

    #[async_trait::async_trait]
    impl AuthTokenService for MockCodec {
        type Error = DomainError;

        async fn generate_access_token(
            &self,
            user_id: Uuid,
            auth_epoch: u64,
        ) -> Result<JwtToken, DomainError> {
            self.issue_access(user_id, auth_epoch).await
        }

        async fn generate_refresh_token(
            &self,
            user_id: Uuid,
            auth_epoch: u64,
        ) -> Result<RefreshToken, DomainError> {
            self.issue_refresh(user_id, auth_epoch).await
        }

        async fn validate_access_token(&self, token: &str) -> Result<Uuid, DomainError> {
            self.check_access(token).await
        }

        async fn validate_refresh_token(&self, token: &str) -> Result<RefreshToken, DomainError> {
            self.check_refresh(token).await
        }
    }

    #[async_trait::async_trait]
    impl JwtTokenEncoder for MockCodec {
        async fn encode(&self, claims: &TokenClaims) -> Result<String, DomainError> {
            self.sign(claims).await
        }

        fn decode(&self, token: &str) -> Result<TokenClaims, DomainError> {
            self.verify(token)
        }

        fn jwks(&self) -> JwkSet {
            self.keys()
        }
    }

    #[tokio::test]
    async fn refresh_rotation_requires_successful_platform_identity_assurance_first() {
        const ISSUER: &str = "https://iam.example";
        for identity_succeeds in [false, true] {
            let user_id = Uuid::new_v4();
            let old_id = Uuid::new_v4();
            let old = RefreshToken {
                id: old_id,
                user_id,
                token: "old".into(),
                is_valid: true,
                created_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
                auth_epoch: 3,
            };
            let mut sessions = MockSessions::new();
            sessions
                .expect_find_by_token()
                .withf(|token| token == "old")
                .once()
                .return_once(move |_| Ok(Some(old)));
            let mut codec = MockCodec::new();
            codec.expect_issue_access().once().returning(|user_id, _| {
                Ok(JwtToken {
                    user_id,
                    token: "access".into(),
                    expires_at: Utc::now() + chrono::Duration::minutes(15),
                })
            });
            codec
                .expect_issue_refresh()
                .once()
                .returning(|user_id, auth_epoch| {
                    Ok(RefreshToken {
                        id: Uuid::new_v4(),
                        user_id,
                        token: "replacement".into(),
                        is_valid: true,
                        created_at: Utc::now(),
                        expires_at: Utc::now() + chrono::Duration::hours(1),
                        auth_epoch,
                    })
                });
            let mut sequence = Sequence::new();
            let mut identities = MockIdentities::new();
            identities
                .expect_ensure_platform_identity()
                .withf(move |id, issuer| *id == user_id && issuer == ISSUER)
                .once()
                .in_sequence(&mut sequence)
                .returning(move |id, issuer| {
                    if identity_succeeds {
                        Ok(Identity::platform(id, issuer))
                    } else {
                        Err(DomainError::RepositoryError("identity unavailable".into()))
                    }
                });
            if identity_succeeds {
                sessions
                    .expect_rotate()
                    .withf(move |id, token| {
                        *id == old_id && token.user_id == user_id && token.auth_epoch == 3
                    })
                    .once()
                    .in_sequence(&mut sequence)
                    .return_once(|_, token| Ok(token));
            } else {
                sessions.expect_rotate().never();
            }
            let service = RefreshTokenServiceImpl::new(
                Arc::new(sessions),
                Arc::new(codec),
                Arc::new(identities),
                ISSUER.into(),
            );
            let result = service.refresh_token("old".into()).await;
            if identity_succeeds {
                let pair = result.expect("rotation succeeds after identity assurance");
                assert_eq!(pair.user_id, user_id);
                assert_eq!(pair.refresh_token, "replacement");
            } else {
                assert!(matches!(
                    result,
                    Err(DomainError::RepositoryError(message)) if message == "identity unavailable"
                ));
            }
        }
    }
}
