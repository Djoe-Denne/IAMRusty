use iam_domain::entity::{
    email_verification::EmailVerification, user::User, user_email::UserEmail,
};
use iam_domain::error::DomainError;
use iam_domain::service::auth_service::SignupTransaction;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, Set, TransactionTrait};
use std::sync::Arc;

use super::entity::{user_email_verification, user_emails, users};

#[derive(Clone)]
pub struct SignupTransactionImpl {
    db: Arc<DatabaseConnection>,
}

impl SignupTransactionImpl {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl SignupTransaction for SignupTransactionImpl {
    async fn create_incomplete_user_with_verification(
        &self,
        user: User,
        user_email: UserEmail,
        email_verification: EmailVerification,
    ) -> Result<User, DomainError> {
        let txn = self.db.begin().await.map_err(to_domain_error)?;

        let result = async {
            let user_model = users::ActiveModel {
                id: Set(user.id),
                username: Set(user.username.clone()),
                password_hash: Set(user.password_hash.clone()),
                avatar_url: Set(user.avatar_url.clone()),
                created_at: Set(user.created_at.naive_utc()),
                updated_at: Set(user.updated_at.naive_utc()),
            };

            user_model.insert(&txn).await.map_err(to_domain_error)?;

            let email_model = user_emails::ActiveModel {
                id: Set(user_email.id),
                user_id: Set(user_email.user_id),
                email: Set(user_email.email.clone()),
                is_primary: Set(user_email.is_primary),
                is_verified: Set(user_email.is_verified),
                created_at: Set(user_email.created_at.naive_utc()),
                updated_at: Set(user_email.updated_at.naive_utc()),
            };

            email_model.insert(&txn).await.map_err(to_domain_error)?;

            let verification_model = user_email_verification::ActiveModel {
                id: ActiveValue::Set(email_verification.id),
                email: ActiveValue::Set(email_verification.email.clone()),
                verification_token: ActiveValue::Set(email_verification.verification_token.clone()),
                expires_at: ActiveValue::Set(email_verification.expires_at.into()),
                created_at: ActiveValue::Set(email_verification.created_at.into()),
            };

            verification_model
                .insert(&txn)
                .await
                .map_err(to_domain_error)?;

            Ok::<_, DomainError>(user)
        }
        .await;

        match result {
            Ok(user) => {
                txn.commit().await.map_err(to_domain_error)?;
                Ok(user)
            }
            Err(error) => {
                if let Err(rollback_error) = txn.rollback().await {
                    tracing::error!(
                        "failed to rollback IAM signup transaction: {}",
                        rollback_error
                    );
                }
                Err(error)
            }
        }
    }
}

fn to_domain_error(error: sea_orm::DbErr) -> DomainError {
    DomainError::RepositoryError(error.to_string())
}
