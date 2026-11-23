//! PostgreSQL writer authority for authenticator state and password session issuance.

use super::{
    authentication_session::lock_user,
    entity::{refresh_tokens, totp_credentials as credentials},
};
use crate::auth::totp::{TotpCipher, STEP_SECONDS};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use iam_domain::{
    entity::token::RefreshToken,
    port::totp::{PasswordFactorProof, TotpEnrollment, TotpError, TotpStore},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use std::{path::Path, sync::Arc};
use uuid::Uuid;

const ENROLLMENT_MINUTES: i64 = 10;
const ATTEMPT_WINDOW_SECONDS: i64 = 60;
const MAX_ATTEMPTS: i32 = 5;

/// All factor mutations serialize on the same account lock as password reset.
pub struct SeaOrmTotpStore {
    writer: Arc<DatabaseConnection>,
    cipher: Option<TotpCipher>,
}

impl SeaOrmTotpStore {
    /// Open the configured key once. No key disables enrollment, not enforcement.
    ///
    /// # Errors
    /// Rejects unreadable or invalid configured key files without exposing the path.
    pub fn new(
        writer: Arc<DatabaseConnection>,
        key_file: Option<&Path>,
    ) -> Result<Self, TotpError> {
        Ok(Self {
            writer,
            cipher: key_file.map(TotpCipher::from_mounted_file).transpose()?,
        })
    }

    fn cipher(&self) -> Result<&TotpCipher, TotpError> {
        self.cipher.as_ref().ok_or(TotpError::Unavailable)
    }

    async fn charge_login_attempt(
        &self,
        user_id: Uuid,
        expected_password_hash: &str,
        has_code: bool,
    ) -> Result<credentials::Model, TotpError> {
        let tx = self.writer.begin().await.map_err(storage_error)?;
        let user = lock_user(&tx, user_id).await.map_err(storage_error)?;
        if user.password_hash.as_deref() != Some(expected_password_hash) || user.username.is_none()
        {
            return Err(TotpError::InvalidProof);
        }
        let row = active(user_id)
            .one(&tx)
            .await
            .map_err(storage_error)?
            .ok_or(TotpError::InvalidProof)?;
        self.cipher()?;
        if !has_code {
            return Err(TotpError::Required);
        }
        let now = Utc::now();
        let (start, attempts) = row
            .login_window_started_at
            .filter(|stamp| {
                now.signed_duration_since(*stamp) < Duration::seconds(ATTEMPT_WINDOW_SECONDS)
            })
            .map_or((now, 0), |start| (start, row.login_attempts));
        if attempts >= MAX_ATTEMPTS {
            return Err(TotpError::Throttled);
        }
        credentials::ActiveModel {
            user_id: Set(user_id),
            login_window_started_at: Set(Some(start)),
            login_attempts: Set(attempts + 1),
            ..Default::default()
        }
        .update(&tx)
        .await
        .map_err(storage_error)?;
        // Keep the debit even if the candidate code turns out to be wrong.
        tx.commit().await.map_err(storage_error)?;
        Ok(row)
    }
}

fn storage_error(_: DbErr) -> TotpError {
    TotpError::Unavailable
}

fn active(user_id: Uuid) -> sea_orm::Select<credentials::Entity> {
    credentials::Entity::find_by_id(user_id).filter(credentials::Column::LastUsedStep.is_not_null())
}

fn step(now: DateTime<Utc>) -> i64 {
    now.timestamp().div_euclid(STEP_SECONDS)
}

#[async_trait]
impl TotpStore for SeaOrmTotpStore {
    async fn begin(&self, user_id: Uuid) -> Result<TotpEnrollment, TotpError> {
        let cipher = self.cipher()?;
        let tx = self.writer.begin().await.map_err(storage_error)?;
        let user = lock_user(&tx, user_id).await.map_err(storage_error)?;
        if user.password_hash.is_none() || user.username.is_none() {
            return Err(TotpError::Conflict);
        }
        let pending = credentials::Entity::find_by_id(user_id)
            .one(&tx)
            .await
            .map_err(storage_error)?;
        if pending
            .as_ref()
            .is_some_and(|row| row.last_used_step.is_some())
        {
            return Err(TotpError::Conflict);
        }
        let id = Uuid::new_v4();
        let (otpauth_uri, sealed_seed) = cipher.provision(user_id, id)?;
        let record = credentials::ActiveModel {
            user_id: Set(user_id),
            enrollment_id: Set(id),
            sealed_seed: Set(sealed_seed),
            expires_at: Set(Utc::now() + Duration::minutes(ENROLLMENT_MINUTES)),
            last_used_step: Set(None),
            login_window_started_at: Set(None),
            login_attempts: Set(0),
        };
        if pending.is_some() {
            record.update(&tx).await.map_err(storage_error)?;
        } else {
            record.insert(&tx).await.map_err(storage_error)?;
        }
        tx.commit().await.map_err(storage_error)?;
        Ok(TotpEnrollment { id, otpauth_uri })
    }

    async fn confirm(
        &self,
        user_id: Uuid,
        enrollment_id: Uuid,
        expected_password_hash: &str,
        code: &str,
    ) -> Result<(), TotpError> {
        let cipher = self.cipher()?;
        let tx = self.writer.begin().await.map_err(storage_error)?;
        let user = lock_user(&tx, user_id).await.map_err(storage_error)?;
        if user.password_hash.as_deref() != Some(expected_password_hash) || user.username.is_none()
        {
            return Err(TotpError::InvalidProof);
        }
        let now = Utc::now();
        let pending = credentials::Entity::find_by_id(user_id)
            .filter(credentials::Column::EnrollmentId.eq(enrollment_id))
            .filter(credentials::Column::LastUsedStep.is_null())
            .filter(credentials::Column::ExpiresAt.gt(now))
            .one(&tx)
            .await
            .map_err(storage_error)?
            .ok_or(TotpError::NotFound)?;
        let matched = cipher.verify(
            user_id,
            enrollment_id,
            &pending.sealed_seed,
            code,
            step(now),
            None,
        )?;
        credentials::ActiveModel {
            user_id: Set(user_id),
            last_used_step: Set(Some(matched)),
            ..Default::default()
        }
        .update(&tx)
        .await
        .map_err(storage_error)?;
        tx.commit().await.map_err(storage_error)
    }

    async fn cancel(&self, user_id: Uuid, enrollment_id: Uuid) -> Result<(), TotpError> {
        let tx = self.writer.begin().await.map_err(storage_error)?;
        lock_user(&tx, user_id).await.map_err(storage_error)?;
        let result = credentials::Entity::delete_by_id(user_id)
            .filter(credentials::Column::EnrollmentId.eq(enrollment_id))
            .filter(credentials::Column::LastUsedStep.is_null())
            .exec(&tx)
            .await
            .map_err(storage_error)?;
        if result.rows_affected != 1 {
            return Err(TotpError::NotFound);
        }
        tx.commit().await.map_err(storage_error)
    }

    async fn verify_login(
        &self,
        user_id: Uuid,
        expected_password_hash: &str,
        code: Option<&str>,
    ) -> Result<PasswordFactorProof, TotpError> {
        if active(user_id)
            .one(&*self.writer)
            .await
            .map_err(storage_error)?
            .is_none()
        {
            return Ok(PasswordFactorProof::NotEnrolled);
        }
        let row = self
            .charge_login_attempt(user_id, expected_password_hash, code.is_some())
            .await?;
        let step = self.cipher()?.verify(
            user_id,
            row.enrollment_id,
            &row.sealed_seed,
            code.ok_or(TotpError::Required)?,
            step(Utc::now()),
            row.last_used_step,
        )?;
        Ok(PasswordFactorProof::Verified {
            enrollment_id: row.enrollment_id,
            step,
        })
    }

    async fn issue_password_session(
        &self,
        token: &RefreshToken,
        expected_password_hash: &str,
        proof: PasswordFactorProof,
    ) -> Result<(), TotpError> {
        let tx = self.writer.begin().await.map_err(storage_error)?;
        let user = lock_user(&tx, token.user_id).await.map_err(storage_error)?;
        let now = Utc::now();
        if user.password_hash.as_deref() != Some(expected_password_hash)
            || user.username.is_none()
            || !token.is_valid
            || token.expires_at <= now
        {
            return Err(TotpError::InvalidProof);
        }
        match (
            active(token.user_id)
                .one(&tx)
                .await
                .map_err(storage_error)?,
            proof,
        ) {
            (None, PasswordFactorProof::NotEnrolled) => {}
            (Some(_), PasswordFactorProof::NotEnrolled) => {
                self.cipher()?;
                return Err(TotpError::Required);
            }
            (
                Some(row),
                PasswordFactorProof::Verified {
                    enrollment_id,
                    step: matched,
                },
            ) if row.enrollment_id == enrollment_id
                && row.last_used_step.is_some_and(|used| used < matched)
                && (step(now) - 1..=step(now)).contains(&matched) =>
            {
                credentials::ActiveModel {
                    user_id: Set(token.user_id),
                    last_used_step: Set(Some(matched)),
                    ..Default::default()
                }
                .update(&tx)
                .await
                .map_err(storage_error)?;
            }
            _ => return Err(TotpError::InvalidProof),
        }
        refresh_tokens::ActiveModel {
            id: Set(token.id),
            user_id: Set(token.user_id),
            token: Set(RefreshToken::hash_token(&token.token)),
            is_valid: Set(true),
            created_at: Set(token.created_at.into()),
            expires_at: Set(token.expires_at.into()),
        }
        .insert(&tx)
        .await
        .map_err(storage_error)?;
        tx.commit().await.map_err(storage_error)
    }
}
