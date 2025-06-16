use super::common::{CommittedFixture, DbFixture, TestData};
use chrono::Utc;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr};
use std::sync::Arc;
use uuid::Uuid;

// Import the entity types
use infra::repository::entity::user_email_verification::{
    ActiveModel as EmailVerificationActiveModel, Entity as EmailVerificationEntity,
    Model as EmailVerificationModel,
};

/// Email verification fixture builder with fluent API
#[derive(Debug, Clone)]
pub struct EmailVerificationFixtureBuilder {
    id: Option<Uuid>,
    email: Option<String>,
    verification_token: Option<String>,
    expires_at: Option<DateTimeWithTimeZone>,
    created_at: Option<DateTimeWithTimeZone>,
}

impl EmailVerificationFixtureBuilder {
    /// Create a new email verification fixture builder
    pub fn new() -> Self {
        Self {
            id: None,
            email: None,
            verification_token: None,
            expires_at: None,
            created_at: None,
        }
    }

    /// Set the verification ID
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the email address
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the verification token
    pub fn verification_token(mut self, token: impl Into<String>) -> Self {
        self.verification_token = Some(token.into());
        self
    }

    /// Set the expires_at timestamp
    pub fn expires_at(mut self, expires_at: DateTimeWithTimeZone) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set the created_at timestamp
    pub fn created_at(mut self, created_at: DateTimeWithTimeZone) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Commit the email verification to the database
    pub async fn commit(
        self,
        db: Arc<DatabaseConnection>,
    ) -> Result<EmailVerificationFixture, DbErr> {
        let fixture = DbFixture::commit(self, &*db).await?;
        Ok(EmailVerificationFixture { inner: fixture })
    }

    // Factory methods for common email verification scenarios

    /// Create a valid email verification token
    pub fn valid(self, email: impl Into<String>) -> Self {
        self.email(email)
            .verification_token(Self::generate_verification_token())
            .expires_at(Self::default_expiry())
    }

    /// Create an expired email verification token
    pub fn expired(self, email: impl Into<String>) -> Self {
        let past = (Utc::now() - chrono::Duration::hours(1)).into();
        self.email(email)
            .verification_token(Self::generate_verification_token())
            .expires_at(past)
    }

    /// Create an email verification token that expires soon (in 5 minutes)
    pub fn expires_soon(self, email: impl Into<String>) -> Self {
        let soon = (Utc::now() + chrono::Duration::minutes(5)).into();
        self.email(email)
            .verification_token(Self::generate_verification_token())
            .expires_at(soon)
    }

    /// Create Arthur's email verification
    pub fn arthur_verification(self) -> Self {
        self.valid("arthur@example.com")
            .verification_token("arthur_verification_token_123")
    }

    /// Create Bob's email verification
    pub fn bob_verification(self) -> Self {
        self.valid("bob@example.com")
            .verification_token("bob_verification_token_456")
    }

    /// Create Alice's email verification
    pub fn alice_verification(self) -> Self {
        self.valid("alice@example.com")
            .verification_token("alice_verification_token_789")
    }

    /// Create Charlie's email verification
    pub fn charlie_verification(self) -> Self {
        self.valid("charlie@example.com")
            .verification_token("charlie_verification_token_012")
    }

    /// Create verification for unverified email
    pub fn unverified_email(self, email: impl Into<String>) -> Self {
        self.valid(email)
            .verification_token(Self::generate_verification_token())
    }

    /// Create verification for test email
    pub fn test_email(self) -> Self {
        self.valid(TestData::email())
            .verification_token(Self::generate_verification_token())
    }

    // Helper methods

    /// Generate a verification token
    fn generate_verification_token() -> String {
        format!("verify_{}", TestData::random_string(32))
    }

    /// Default expiry time (24 hours from now)
    fn default_expiry() -> DateTimeWithTimeZone {
        (Utc::now() + chrono::Duration::hours(24)).into()
    }
}

impl Default for EmailVerificationFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DbFixture<EmailVerificationEntity, EmailVerificationModel, EmailVerificationActiveModel>
    for EmailVerificationFixtureBuilder
{
    async fn commit(
        self,
        db: &DatabaseConnection,
    ) -> Result<CommittedFixture<EmailVerificationModel>, DbErr> {
        let active_model = self.model();
        let model = active_model.insert(db).await?;
        Ok(CommittedFixture::new(model))
    }

    fn model(&self) -> EmailVerificationActiveModel {
        let now = TestData::now_with_tz();

        EmailVerificationActiveModel {
            id: ActiveValue::Set(self.id.unwrap_or_else(TestData::uuid)),
            email: ActiveValue::Set(self.email.clone().unwrap_or_else(TestData::email)),
            verification_token: ActiveValue::Set(
                self.verification_token
                    .clone()
                    .unwrap_or_else(Self::generate_verification_token),
            ),
            expires_at: ActiveValue::Set(self.expires_at.unwrap_or_else(Self::default_expiry)),
            created_at: ActiveValue::Set(self.created_at.unwrap_or(now)),
        }
    }
}

/// Committed email verification fixture
#[derive(Debug, Clone)]
pub struct EmailVerificationFixture {
    inner: CommittedFixture<EmailVerificationModel>,
}

impl EmailVerificationFixture {
    /// Check if the email verification exists in the database and matches the fixture
    pub async fn check(&self, db: Arc<DatabaseConnection>) -> Result<bool, DbErr> {
        use sea_orm::EntityTrait;

        // Find the email verification in the database by ID
        let db_verification = EmailVerificationEntity::find_by_id(self.model().id)
            .one(&*db)
            .await?;

        match db_verification {
            Some(verification) => {
                // Compare all fields
                Ok(verification.id == self.model().id
                    && verification.email == self.model().email
                    && verification.verification_token == self.model().verification_token
                    && verification.expires_at == self.model().expires_at
                    && verification.created_at == self.model().created_at)
            }
            None => Ok(false), // Verification doesn't exist in database
        }
    }

    /// Get the email verification model
    pub fn model(&self) -> &EmailVerificationModel {
        self.inner.model()
    }

    /// Get the verification ID
    pub fn id(&self) -> Uuid {
        self.model().id
    }

    /// Get the email address
    pub fn email(&self) -> &str {
        &self.model().email
    }

    /// Get the verification token
    pub fn verification_token(&self) -> &str {
        &self.model().verification_token
    }

    /// Get the expires_at timestamp
    pub fn expires_at(&self) -> DateTimeWithTimeZone {
        self.model().expires_at
    }

    /// Get the created_at timestamp
    pub fn created_at(&self) -> DateTimeWithTimeZone {
        self.model().created_at
    }

    /// Check if the verification token has expired
    pub fn is_expired(&self) -> bool {
        let now: DateTimeWithTimeZone = Utc::now().into();
        self.model().expires_at < now
    }

    /// Check if the verification token is still valid (not expired)
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    /// Check if the verification token expires soon (within 1 hour)
    pub fn expires_soon(&self) -> bool {
        let soon: DateTimeWithTimeZone = (Utc::now() + chrono::Duration::hours(1)).into();
        self.model().expires_at <= soon
    }
}

// Add the TestData extension for verification tokens
impl TestData {
    /// Generate a verification token
    pub fn verification_token() -> String {
        format!("verify_{}", Self::random_string(32))
    }
}
