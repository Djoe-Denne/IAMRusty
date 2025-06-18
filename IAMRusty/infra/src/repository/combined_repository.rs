use async_trait::async_trait;
use domain::entity::{
    provider::{Provider, ProviderTokens},
    provider_link::ProviderLink,
    token::RefreshToken,
    user::User,
};
use domain::port::repository::{
    RefreshTokenReadRepository, RefreshTokenWriteRepository, TokenReadRepository,
    TokenWriteRepository, UserReadRepository, UserWriteRepository,
};
use sea_orm::DbErr;
use uuid::Uuid;

/// Combined User Repository that delegates to separate read/write implementations
#[derive(Clone)]
pub struct CombinedUserRepository<R, W>
where
    R: UserReadRepository<Error = DbErr>,
    W: UserWriteRepository<Error = DbErr>,
{
    read_repo: R,
    write_repo: W,
}

impl<R, W> CombinedUserRepository<R, W>
where
    R: UserReadRepository<Error = DbErr>,
    W: UserWriteRepository<Error = DbErr>,
{
    /// Create a new combined repository
    pub fn new(read_repo: R, write_repo: W) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl<R, W> UserReadRepository for CombinedUserRepository<R, W>
where
    R: UserReadRepository<Error = DbErr> + Send + Sync,
    W: UserWriteRepository<Error = DbErr> + Send + Sync,
{
    type Error = DbErr;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, Self::Error> {
        self.read_repo.find_by_username(username).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, Self::Error> {
        self.read_repo.find_by_email(email).await
    }

    async fn find_by_provider_user_id(
        &self,
        provider: Provider,
        provider_user_id: &str,
    ) -> Result<Option<User>, Self::Error> {
        self.read_repo
            .find_by_provider_user_id(provider, provider_user_id)
            .await
    }
}

#[async_trait]
impl<R, W> UserWriteRepository for CombinedUserRepository<R, W>
where
    R: UserReadRepository<Error = DbErr> + Send + Sync,
    W: UserWriteRepository<Error = DbErr> + Send + Sync,
{
    type Error = DbErr;

    async fn create(&self, user: User) -> Result<User, Self::Error> {
        self.write_repo.create(user).await
    }

    async fn update(&self, user: User) -> Result<User, Self::Error> {
        self.write_repo.update(user).await
    }
}

/// Combined Token Repository that delegates to separate read/write implementations
pub struct CombinedTokenRepository<R, W>
where
    R: TokenReadRepository<Error = DbErr>,
    W: TokenWriteRepository<Error = DbErr>,
{
    read_repo: R,
    write_repo: W,
}

impl<R, W> CombinedTokenRepository<R, W>
where
    R: TokenReadRepository<Error = DbErr>,
    W: TokenWriteRepository<Error = DbErr>,
{
    /// Create a new combined repository
    pub fn new(read_repo: R, write_repo: W) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl<R, W> TokenReadRepository for CombinedTokenRepository<R, W>
where
    R: TokenReadRepository<Error = DbErr> + Send + Sync,
    W: TokenWriteRepository<Error = DbErr> + Send + Sync,
{
    type Error = DbErr;

    async fn get_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<Option<ProviderTokens>, Self::Error> {
        self.read_repo.get_provider_tokens(user_id, provider).await
    }

    async fn get_provider_link(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<Option<ProviderLink>, Self::Error> {
        self.read_repo.get_provider_link(user_id, provider).await
    }

    async fn get_user_provider_links(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ProviderLink>, Self::Error> {
        self.read_repo.get_user_provider_links(user_id).await
    }
}

#[async_trait]
impl<R, W> TokenWriteRepository for CombinedTokenRepository<R, W>
where
    R: TokenReadRepository<Error = DbErr> + Send + Sync,
    W: TokenWriteRepository<Error = DbErr> + Send + Sync,
{
    type Error = DbErr;

    async fn save_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
        provider_user_id: String,
        tokens: ProviderTokens,
    ) -> Result<(), Self::Error> {
        self.write_repo
            .save_provider_tokens(user_id, provider, provider_user_id, tokens)
            .await
    }

    async fn delete_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
    ) -> Result<(), Self::Error> {
        self.write_repo
            .delete_provider_tokens(user_id, provider)
            .await
    }
}

/// Combined RefreshToken Repository that delegates to separate read/write implementations
#[derive(Clone)]
pub struct CombinedRefreshTokenRepository<R, W>
where
    R: RefreshTokenReadRepository<Error = DbErr>,
    W: RefreshTokenWriteRepository<Error = DbErr>,
{
    read_repo: R,
    write_repo: W,
}

impl<R, W> CombinedRefreshTokenRepository<R, W>
where
    R: RefreshTokenReadRepository<Error = DbErr>,
    W: RefreshTokenWriteRepository<Error = DbErr>,
{
    /// Create a new combined repository
    pub fn new(read_repo: R, write_repo: W) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl<R, W> RefreshTokenReadRepository for CombinedRefreshTokenRepository<R, W>
where
    R: RefreshTokenReadRepository<Error = DbErr> + Send + Sync,
    W: RefreshTokenWriteRepository<Error = DbErr> + Send + Sync,
{
    type Error = DbErr;

    async fn find_by_token(&self, token: &str) -> Result<Option<RefreshToken>, Self::Error> {
        self.read_repo.find_by_token(token).await
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<RefreshToken>, Self::Error> {
        self.read_repo.find_by_user_id(user_id).await
    }
}

#[async_trait]
impl<R, W> RefreshTokenWriteRepository for CombinedRefreshTokenRepository<R, W>
where
    R: RefreshTokenReadRepository<Error = DbErr> + Send + Sync,
    W: RefreshTokenWriteRepository<Error = DbErr> + Send + Sync,
{
    type Error = DbErr;

    async fn create(&self, token: RefreshToken) -> Result<RefreshToken, Self::Error> {
        self.write_repo.create(token).await
    }

    async fn update_validity(&self, token_id: Uuid, is_valid: bool) -> Result<(), Self::Error> {
        self.write_repo.update_validity(token_id, is_valid).await
    }

    async fn delete_by_id(&self, token_id: Uuid) -> Result<(), Self::Error> {
        self.write_repo.delete_by_id(token_id).await
    }

    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<u64, Self::Error> {
        self.write_repo.delete_by_user_id(user_id).await
    }
}
