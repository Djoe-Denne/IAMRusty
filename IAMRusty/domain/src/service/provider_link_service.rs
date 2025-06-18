use std::sync::Arc;
use uuid::Uuid;

use crate::entity::{
    provider::{Provider, ProviderTokens, ProviderUserProfile},
    user::User,
    user_email::UserEmail,
};
use crate::error::DomainError;
use crate::port::repository::{TokenRepository, UserEmailRepository, UserRepository};

/// Domain service for linking OAuth providers to existing users
pub struct ProviderLinkService<UR, UER, TR>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    TR: TokenRepository,
{
    user_repo: Arc<UR>,
    user_email_repo: Arc<UER>,
    token_repo: Arc<TR>,
}

/// Result of provider linking operation
#[derive(Debug)]
pub struct ProviderLinkResult {
    /// The user to whom the provider was linked
    pub user: User,
    /// All emails for the user (including any newly added)
    pub emails: Vec<UserEmail>,
    /// Whether a new email was added during linking
    pub new_email_added: bool,
    /// The new email that was added (if any)
    pub new_email: Option<String>,
}

impl<UR, UER, TR> ProviderLinkService<UR, UER, TR>
where
    UR: UserRepository,
    UER: UserEmailRepository,
    TR: TokenRepository,
{
    /// Create a new ProviderLinkService
    pub fn new(user_repo: Arc<UR>, user_email_repo: Arc<UER>, token_repo: Arc<TR>) -> Self {
        Self {
            user_repo,
            user_email_repo,
            token_repo,
        }
    }

    /// Link a provider to an existing user
    pub async fn link_provider_to_user(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        provider_tokens: ProviderTokens,
        provider_profile: ProviderUserProfile,
    ) -> Result<ProviderLinkResult, DomainError>
    where
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
        <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        // Step 1: Verify the user exists
        let user = self.load_user(user_id).await?;

        // Step 2: Check for provider conflicts
        self.check_provider_conflicts(user_id, provider, &provider_user_id)
            .await?;

        // Step 3: Handle email from provider
        let (new_email_added, new_email) = self
            .handle_provider_email(user_id, provider, provider_profile.email)
            .await?;

        // Step 4: Save provider tokens
        self.save_provider_tokens(user_id, provider, provider_user_id, provider_tokens)
            .await?;

        // Step 5: Get all user emails for response
        let emails = self.get_user_emails(user_id).await?;

        Ok(ProviderLinkResult {
            user,
            emails,
            new_email_added,
            new_email,
        })
    }

    /// Relink a provider for an existing user (replace existing tokens)
    pub async fn relink_provider_for_user(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        provider_tokens: ProviderTokens,
        provider_profile: ProviderUserProfile,
    ) -> Result<ProviderLinkResult, DomainError>
    where
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
        <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        // Step 1: Verify the user exists
        let user = self.load_user(user_id).await?;

        // Step 2: Verify user already has this provider linked
        let existing_tokens = self
            .token_repo
            .get_provider_tokens(user_id, provider)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        if existing_tokens.is_none() {
            return Err(DomainError::BusinessRuleViolation(
                "Cannot relink provider that is not currently linked".to_string(),
            ));
        }

        // Step 3: Handle email from provider (don't enforce uniqueness for relink)
        let (new_email_added, new_email) = self
            .handle_provider_email_for_relink(user_id, provider, provider_profile.email)
            .await?;

        // Step 4: Save provider tokens (this will replace existing ones)
        self.save_provider_tokens(user_id, provider, provider_user_id, provider_tokens)
            .await?;

        // Step 5: Get all user emails for response
        let emails = self.get_user_emails(user_id).await?;

        Ok(ProviderLinkResult {
            user,
            emails,
            new_email_added,
            new_email,
        })
    }

    /// Verify user exists and return the user
    async fn load_user(&self, user_id: Uuid) -> Result<User, DomainError>
    where
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?
            .ok_or(DomainError::UserNotFound)
    }

    /// Check if provider is already linked to another user or the same user
    async fn check_provider_conflicts(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: &str,
    ) -> Result<(), DomainError>
    where
        <UR as UserRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        let existing_user = self
            .user_repo
            .find_by_provider_user_id(provider, provider_user_id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        match existing_user {
            Some(user) if user.id == user_id => Err(DomainError::BusinessRuleViolation(
                "Provider is already linked to your account".to_string(),
            )),
            Some(_) => Err(DomainError::BusinessRuleViolation(
                "Provider account is already linked to another user".to_string(),
            )),
            None => Ok(()),
        }
    }

    /// Handle email from provider - add if new, check conflicts if exists
    async fn handle_provider_email(
        &self,
        user_id: Uuid,
        provider: Provider,
        email: Option<String>,
    ) -> Result<(bool, Option<String>), DomainError>
    where
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        let Some(email) = email else {
            return Ok((false, None));
        };

        let existing_email = self
            .user_email_repo
            .find_by_email(&email)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        match existing_email {
            Some(existing) if existing.user_id != user_id => {
                tracing::error!(
                    "Email {} from provider {} already belongs to user {}, not adding to user {}",
                    email,
                    provider.as_str(),
                    existing.user_id,
                    user_id
                );
                Err(DomainError::BusinessRuleViolation(
                    "Provider email is already associated with another user".to_string(),
                ))
            }
            Some(_) => Ok((false, None)), // Email already exists for this user
            None => {
                // Create new secondary email
                let user_email = UserEmail::new_secondary(user_id, email.clone(), false);
                self.user_email_repo
                    .create(user_email)
                    .await
                    .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
                Ok((true, Some(email)))
            }
        }
    }

    /// Handle email from provider for relink - more lenient validation
    async fn handle_provider_email_for_relink(
        &self,
        user_id: Uuid,
        provider: Provider,
        email: Option<String>,
    ) -> Result<(bool, Option<String>), DomainError>
    where
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        let Some(email) = email else {
            return Ok((false, None));
        };

        let existing_email = self
            .user_email_repo
            .find_by_email(&email)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))?;

        match existing_email {
            Some(existing) if existing.user_id != user_id => {
                tracing::warn!(
                    "Email {} from provider {} already belongs to user {}, skipping for relink to user {}",
                    email,
                    provider.as_str(),
                    existing.user_id,
                    user_id
                );
                // For relink, we don't fail if email belongs to another user
                Ok((false, None))
            }
            Some(_) => Ok((false, None)), // Email already exists for this user
            None => {
                // Create new secondary email
                let user_email = UserEmail::new_secondary(user_id, email.clone(), false);
                self.user_email_repo
                    .create(user_email)
                    .await
                    .map_err(|e| DomainError::RepositoryError(e.to_string()))?;
                Ok((true, Some(email)))
            }
        }
    }

    /// Save provider tokens for the user
    async fn save_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        tokens: ProviderTokens,
    ) -> Result<(), DomainError>
    where
        <TR as TokenRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.token_repo
            .save_provider_tokens(user_id, provider, provider_user_id, tokens)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))
    }

    /// Get all emails for the user
    async fn get_user_emails(&self, user_id: Uuid) -> Result<Vec<UserEmail>, DomainError>
    where
        <UER as UserEmailRepository>::Error: std::error::Error + Send + Sync + 'static,
    {
        self.user_email_repo
            .find_by_user_id(user_id)
            .await
            .map_err(|e| DomainError::RepositoryError(e.to_string()))
    }
}
