use async_trait::async_trait;
use iam_domain::entity::user_email::UserEmail;
use iam_domain::port::repository::{UserEmailReadRepository, UserEmailWriteRepository};
use sea_orm::DbErr;
use uuid::Uuid;

/// Combined UserEmail Repository that delegates to separate read/write implementations
#[derive(Clone)]
pub struct CombinedUserEmailRepository<R, W>
where
    R: UserEmailReadRepository<Error = DbErr>,
    W: UserEmailWriteRepository<Error = DbErr>,
{
    read_repo: R,
    write_repo: W,
}

impl<R, W> CombinedUserEmailRepository<R, W>
where
    R: UserEmailReadRepository<Error = DbErr>,
    W: UserEmailWriteRepository<Error = DbErr>,
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
impl<R, W> UserEmailReadRepository for CombinedUserEmailRepository<R, W>
where
    R: UserEmailReadRepository<Error = DbErr> + Send + Sync,
    W: UserEmailWriteRepository<Error = DbErr> + Send + Sync,
{
    type Error = DbErr;

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<UserEmail>, Self::Error> {
        self.read_repo.find_by_user_id(user_id).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserEmail>, Self::Error> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserEmail>, Self::Error> {
        self.read_repo.find_by_email(email).await
    }

    async fn find_primary_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserEmail>, Self::Error> {
        self.read_repo.find_primary_by_user_id(user_id).await
    }
}

#[async_trait]
impl<R, W> UserEmailWriteRepository for CombinedUserEmailRepository<R, W>
where
    R: UserEmailReadRepository<Error = DbErr> + Send + Sync,
    W: UserEmailWriteRepository<Error = DbErr> + Send + Sync,
{
    type Error = DbErr;

    async fn create(&self, user_email: UserEmail) -> Result<UserEmail, Self::Error> {
        self.write_repo.create(user_email).await
    }

    async fn update(&self, user_email: UserEmail) -> Result<UserEmail, Self::Error> {
        self.write_repo.update(user_email).await
    }

    async fn delete(&self, id: Uuid) -> Result<(), Self::Error> {
        self.write_repo.delete(id).await
    }

    async fn set_as_primary(&self, user_id: Uuid, email_id: Uuid) -> Result<(), Self::Error> {
        self.write_repo.set_as_primary(user_id, email_id).await
    }
}
