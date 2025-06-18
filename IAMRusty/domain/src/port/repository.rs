use crate::entity::{
    email_verification::EmailVerification,
    password_reset_token::PasswordResetToken,
    provider::{Provider, ProviderTokens},
    provider_link::ProviderLink,
    token::RefreshToken,
    user::User,
    user_email::UserEmail,
};
use uuid::Uuid;

/// Read operations for User entity
#[async_trait::async_trait]
pub trait UserReadRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Find a user by ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error>;

    /// Find a user by any of their email addresses
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, Self::Error>;

    /// Find a user by username
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, Self::Error>;

    /// Find a user by provider and provider user ID
    /// This looks up via the provider_tokens table
    async fn find_by_provider_user_id(
        &self,
        provider: Provider,
        provider_user_id: &str,
    ) -> Result<Option<User>, Self::Error>;
}

/// Write operations for User entity
#[async_trait::async_trait]
pub trait UserWriteRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new user
    async fn create(&self, user: User) -> Result<User, Self::Error>;

    /// Update an existing user
    async fn update(&self, user: User) -> Result<User, Self::Error>;
}

/// Combined read and write operations for User entity
#[async_trait::async_trait]
pub trait UserRepository: UserReadRepository + UserWriteRepository
where
    <Self as UserReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <Self as UserWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    /// Error type for this repository
    type Error: std::error::Error + Send + Sync + 'static;
}

// Blanket implementation for types that implement both read and write repositories
impl<T> UserRepository for T
where
    T: UserReadRepository + UserWriteRepository,
    <T as UserReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <T as UserWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = <T as UserReadRepository>::Error;
}

/// Read operations for UserEmail entity
#[async_trait::async_trait]
pub trait UserEmailReadRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get all emails for a user
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<UserEmail>, Self::Error>;

    /// Get a specific email by its ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserEmail>, Self::Error>;

    /// Find a user email by email address
    async fn find_by_email(&self, email: &str) -> Result<Option<UserEmail>, Self::Error>;

    /// Get the primary email for a user
    async fn find_primary_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserEmail>, Self::Error>;
}

/// Write operations for UserEmail entity
#[async_trait::async_trait]
pub trait UserEmailWriteRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new user email
    async fn create(&self, user_email: UserEmail) -> Result<UserEmail, Self::Error>;

    /// Update a user email
    async fn update(&self, user_email: UserEmail) -> Result<UserEmail, Self::Error>;

    /// Delete a user email
    async fn delete(&self, id: Uuid) -> Result<(), Self::Error>;

    /// Set an email as primary (and unset other primary emails for the user)
    async fn set_as_primary(&self, user_id: Uuid, email_id: Uuid) -> Result<(), Self::Error>;
}

/// Combined read and write operations for UserEmail entity
#[async_trait::async_trait]
pub trait UserEmailRepository: UserEmailReadRepository + UserEmailWriteRepository
where
    <Self as UserEmailReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <Self as UserEmailWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    /// Error type for this repository
    type Error: std::error::Error + Send + Sync + 'static;
}

// Blanket implementation for types that implement both read and write repositories
impl<T> UserEmailRepository for T
where
    T: UserEmailReadRepository + UserEmailWriteRepository,
    <T as UserEmailReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <T as UserEmailWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = <T as UserEmailReadRepository>::Error;
}

/// Read operations for OAuth2 tokens
#[async_trait::async_trait]
pub trait TokenReadRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get tokens for a user and provider
    async fn get_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<Option<ProviderTokens>, Self::Error>;

    /// Get provider link information (user_id, provider, provider_user_id)
    async fn get_provider_link(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<Option<ProviderLink>, Self::Error>;

    /// Get all provider links for a user
    async fn get_user_provider_links(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ProviderLink>, Self::Error>;
}

/// Write operations for OAuth2 tokens
#[async_trait::async_trait]
pub trait TokenWriteRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Save tokens for a user and provider, including the provider-specific user ID
    async fn save_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        tokens: ProviderTokens,
    ) -> Result<(), Self::Error>;

    /// Delete provider tokens for a user and provider
    async fn delete_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<(), Self::Error>;
}

/// Combined read and write operations for OAuth2 tokens
#[async_trait::async_trait]
pub trait TokenRepository: TokenReadRepository + TokenWriteRepository
where
    <Self as TokenReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <Self as TokenWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    /// Error type for this repository
    type Error: std::error::Error + Send + Sync + 'static;
}

// Blanket implementation for types that implement both read and write repositories
impl<T> TokenRepository for T
where
    T: TokenReadRepository + TokenWriteRepository,
    <T as TokenReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <T as TokenWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = <T as TokenReadRepository>::Error;
}

/// Read operations for refresh tokens
#[async_trait::async_trait]
pub trait RefreshTokenReadRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Find a refresh token by its token string
    async fn find_by_token(&self, token: &str) -> Result<Option<RefreshToken>, Self::Error>;

    /// Find refresh tokens for a user
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<RefreshToken>, Self::Error>;
}

/// Write operations for refresh tokens
#[async_trait::async_trait]
pub trait RefreshTokenWriteRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new refresh token
    async fn create(&self, token: RefreshToken) -> Result<RefreshToken, Self::Error>;

    /// Update a refresh token's validity
    async fn update_validity(&self, token_id: Uuid, is_valid: bool) -> Result<(), Self::Error>;

    /// Delete a refresh token by its ID
    async fn delete_by_id(&self, token_id: Uuid) -> Result<(), Self::Error>;

    /// Delete all refresh tokens for a user
    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<u64, Self::Error>;
}

/// Combined read and write operations for refresh tokens
#[async_trait::async_trait]
pub trait RefreshTokenRepository: RefreshTokenReadRepository + RefreshTokenWriteRepository
where
    <Self as RefreshTokenReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <Self as RefreshTokenWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    /// Error type for this repository
    type Error: std::error::Error + Send + Sync + 'static;
}

// Blanket implementation for types that implement both read and write repositories
impl<T> RefreshTokenRepository for T
where
    T: RefreshTokenReadRepository + RefreshTokenWriteRepository,
    <T as RefreshTokenReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <T as RefreshTokenWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = <T as RefreshTokenReadRepository>::Error;
}

/// Read operations for EmailVerification entity
#[async_trait::async_trait]
pub trait EmailVerificationReadRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Find email verification by email and token
    async fn find_by_email_and_token(
        &self,
        email: &str,
        token: &str,
    ) -> Result<Option<EmailVerification>, Self::Error>;

    /// Find email verification by email
    async fn find_by_email(&self, email: &str) -> Result<Option<EmailVerification>, Self::Error>;
}

/// Write operations for EmailVerification entity
#[async_trait::async_trait]
pub trait EmailVerificationWriteRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new email verification
    async fn create(&self, verification: &EmailVerification) -> Result<(), Self::Error>;

    /// Delete email verification by email
    async fn delete_by_email(&self, email: &str) -> Result<(), Self::Error>;

    /// Delete email verification by id
    async fn delete_by_id(&self, id: Uuid) -> Result<(), Self::Error>;
}

/// Combined read and write operations for EmailVerification entity
#[async_trait::async_trait]
pub trait EmailVerificationRepository:
    EmailVerificationReadRepository + EmailVerificationWriteRepository
where
    <Self as EmailVerificationReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <Self as EmailVerificationWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    /// Error type for this repository
    type Error: std::error::Error + Send + Sync + 'static;
}

// Blanket implementation for types that implement both read and write repositories
impl<T> EmailVerificationRepository for T
where
    T: EmailVerificationReadRepository + EmailVerificationWriteRepository,
    <T as EmailVerificationReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <T as EmailVerificationWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = <T as EmailVerificationReadRepository>::Error;
}

/// Read operations for password reset tokens
#[async_trait::async_trait]
pub trait PasswordResetTokenReadRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Find a token by user ID and token hash
    async fn find_by_user_and_token_hash(
        &self,
        user_id: Uuid,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Find a token by token hash alone (used when user is unknown)
    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Find the most recent valid token for a user
    async fn find_latest_valid_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Find token by ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PasswordResetToken>, Self::Error>;

    /// Count valid tokens for a user
    async fn count_valid_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error>;
}

/// Write operations for password reset tokens
#[async_trait::async_trait]
pub trait PasswordResetTokenWriteRepository {
    /// Error type returned by this repository
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new password reset token
    async fn create(&self, token: &PasswordResetToken) -> Result<(), Self::Error>;

    /// Update a token (typically to mark as used)
    async fn update(&self, token: &PasswordResetToken) -> Result<(), Self::Error>;

    /// Mark a token as used
    async fn mark_as_used(&self, token_id: Uuid) -> Result<(), Self::Error>;

    /// Delete expired tokens (cleanup operation)
    async fn delete_expired(&self) -> Result<u64, Self::Error>;

    /// Delete all tokens for a user
    async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error>;
}

/// Combined read and write operations for password reset tokens
#[async_trait::async_trait]
pub trait PasswordResetTokenRepository: PasswordResetTokenReadRepository + PasswordResetTokenWriteRepository
where
    <Self as PasswordResetTokenReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <Self as PasswordResetTokenWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    /// Error type for this repository
    type Error: std::error::Error + Send + Sync + 'static;
}

// Blanket implementation for types that implement both read and write repositories
impl<T> PasswordResetTokenRepository for T
where
    T: PasswordResetTokenReadRepository + PasswordResetTokenWriteRepository,
    <T as PasswordResetTokenReadRepository>::Error: std::error::Error + Send + Sync + 'static,
    <T as PasswordResetTokenWriteRepository>::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = <T as PasswordResetTokenReadRepository>::Error;
}
