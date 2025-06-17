use async_trait::async_trait;
use domain::entity::password_reset_token::PasswordResetToken;
use domain::port::repository::{
    PasswordResetTokenReadRepository, PasswordResetTokenWriteRepository,
};
use std::sync::Arc;
use uuid::Uuid;

/// Combined repository that implements both read and write operations for password reset tokens
pub struct CombinedPasswordResetTokenRepository<R, W>
where
    R: PasswordResetTokenReadRepository,
    W: PasswordResetTokenWriteRepository,
{
    read_repo: Arc<R>,
    write_repo: Arc<W>,
}

impl<R, W> CombinedPasswordResetTokenRepository<R, W>
where
    R: PasswordResetTokenReadRepository,
    W: PasswordResetTokenWriteRepository,
{
    /// Create a new combined repository
    pub fn new(read_repo: Arc<R>, write_repo: Arc<W>) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl<R, W> PasswordResetTokenReadRepository for CombinedPasswordResetTokenRepository<R, W>
where
    R: PasswordResetTokenReadRepository + Send + Sync,
    W: PasswordResetTokenWriteRepository + Send + Sync,
    R::Error: std::error::Error + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    type Error = R::Error;

    async fn find_by_user_and_token_hash(
        &self,
        user_id: Uuid,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, Self::Error> {
        self.read_repo
            .find_by_user_and_token_hash(user_id, token_hash)
            .await
    }

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, Self::Error> {
        self.read_repo.find_by_token_hash(token_hash).await
    }

    async fn find_latest_valid_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<PasswordResetToken>, Self::Error> {
        self.read_repo.find_latest_valid_for_user(user_id).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PasswordResetToken>, Self::Error> {
        self.read_repo.find_by_id(id).await
    }

    async fn count_valid_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error> {
        self.read_repo.count_valid_for_user(user_id).await
    }
}

#[async_trait]
impl<R, W> PasswordResetTokenWriteRepository for CombinedPasswordResetTokenRepository<R, W>
where
    R: PasswordResetTokenReadRepository + Send + Sync,
    W: PasswordResetTokenWriteRepository + Send + Sync,
    R::Error: std::error::Error + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static + Into<R::Error>,
{
    type Error = R::Error;

    async fn create(&self, token: &PasswordResetToken) -> Result<(), Self::Error> {
        self.write_repo.create(token).await.map_err(Into::into)
    }

    async fn update(&self, token: &PasswordResetToken) -> Result<(), Self::Error> {
        self.write_repo.update(token).await.map_err(Into::into)
    }

    async fn mark_as_used(&self, token_id: Uuid) -> Result<(), Self::Error> {
        self.write_repo.mark_as_used(token_id).await.map_err(Into::into)
    }

    async fn delete_expired(&self) -> Result<u64, Self::Error> {
        self.write_repo.delete_expired().await.map_err(Into::into)
    }

    async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error> {
        self.write_repo.delete_all_for_user(user_id).await.map_err(Into::into)
    }
}

// Note: PasswordResetTokenRepository is automatically implemented by the blanket impl in domain
// for any type that implements both PasswordResetTokenReadRepository + PasswordResetTokenWriteRepository

#[cfg(test)]
mod tests {
    use super::*;
    use domain::port::repository::{PasswordResetTokenReadRepository, PasswordResetTokenWriteRepository};
    use mockall::mock;

    mock! {
        ReadRepo {}
        
        #[async_trait]
        impl PasswordResetTokenReadRepository for ReadRepo {
            type Error = String;
            
            async fn find_by_user_and_token_hash(
                &self,
                user_id: Uuid,
                token_hash: &str,
            ) -> Result<Option<PasswordResetToken>, Self::Error>;
            
            async fn find_latest_valid_for_user(
                &self,
                user_id: Uuid,
            ) -> Result<Option<PasswordResetToken>, Self::Error>;
            
            async fn find_by_id(&self, id: Uuid) -> Result<Option<PasswordResetToken>, Self::Error>;
            
            async fn count_valid_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error>;
        }
    }

    mock! {
        WriteRepo {}
        
        #[async_trait]
        impl PasswordResetTokenWriteRepository for WriteRepo {
            type Error = String;
            
            async fn create(&self, token: &PasswordResetToken) -> Result<(), Self::Error>;
            
            async fn update(&self, token: &PasswordResetToken) -> Result<(), Self::Error>;
            
            async fn mark_as_used(&self, token_id: Uuid) -> Result<(), Self::Error>;
            
            async fn delete_expired(&self) -> Result<u64, Self::Error>;
            
            async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, Self::Error>;
        }
    }

    #[tokio::test]
    async fn test_find_by_user_and_token_hash() {
        let mut read_repo = MockReadRepo::new();
        let write_repo = MockWriteRepo::new();
        
        let user_id = Uuid::new_v4();
        let token_hash = "test_hash";
        
        read_repo
            .expect_find_by_user_and_token_hash()
            .with(mockall::predicate::eq(user_id), mockall::predicate::eq(token_hash))
            .times(1)
            .returning(|_, _| Ok(None));
        
        let combined = CombinedPasswordResetTokenRepository::new(
            Arc::new(read_repo),
            Arc::new(write_repo),
        );
        
        let result = combined.find_by_user_and_token_hash(user_id, token_hash).await;
        assert!(result.is_ok());
    }
} 