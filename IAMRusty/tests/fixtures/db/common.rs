use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, ModelTrait};
use std::fmt::Debug;
use uuid::Uuid;

/// Common trait for all DB fixtures
#[async_trait::async_trait]
pub trait DbFixture<Entity, Model, ActiveModel>
where
    Entity: EntityTrait<Model = Model>,
    Model: ModelTrait + Debug + PartialEq + Clone,
    ActiveModel: ActiveModelTrait<Entity = Entity> + Send,
{
    /// Commit the fixture to the database and return the fixture instance
    async fn commit(self, db: &DatabaseConnection) -> Result<CommittedFixture<Model>, DbErr>;

    /// Get the model that would be inserted (without committing)
    fn model(&self) -> ActiveModel;
}

/// A committed fixture that can be used for verification
#[derive(Debug, Clone)]
pub struct CommittedFixture<Model> {
    pub model: Model,
}

impl<Model> CommittedFixture<Model>
where
    Model: ModelTrait + Debug + PartialEq + Clone,
{
    pub fn new(model: Model) -> Self {
        Self { model }
    }

    /// Check if the current fixture matches what's in the database
    pub async fn check<Entity>(&self, db: &DatabaseConnection) -> Result<bool, DbErr>
    where
        Entity: EntityTrait<Model = Model>,
        Model: ModelTrait,
    {
        // This is a generic placeholder implementation
        // Each specific fixture type should implement its own check method
        // that properly compares the fixture data with the database
        Ok(true)
    }

    /// Get the committed model
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Get the ID of the committed model (assumes UUID primary key)
    pub fn id(&self) -> Uuid
    where
        Model: ModelTrait,
    {
        // This is a simplified version - in practice you'd need to handle different PK types
        // For now, assuming UUID primary keys for most entities
        todo!("Implement based on specific entity type")
    }
}

/// Utility functions for generating test data
pub struct TestData;

impl TestData {
    /// Generate a test UUID
    pub fn uuid() -> Uuid {
        Uuid::new_v4()
    }

    /// Generate a test username
    pub fn username() -> String {
        format!("test_user_{}", Self::random_string(8))
    }

    /// Generate a test email
    pub fn email() -> String {
        format!("test_{}@example.com", Self::random_string(8))
    }

    /// Generate a test provider user ID
    pub fn provider_user_id() -> String {
        Self::random_string(10)
    }

    /// Generate a test access token
    pub fn access_token() -> String {
        format!("gho_{}", Self::random_string(36))
    }

    /// Generate a test refresh token
    pub fn refresh_token() -> String {
        format!("ghr_{}", Self::random_string(36))
    }

    /// Generate a JWT token
    pub fn jwt_token() -> String {
        format!(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.{}.{}",
            Self::random_string(20),
            Self::random_string(20)
        )
    }

    /// Generate current timestamp
    pub fn now() -> DateTime<Utc> {
        Utc::now()
    }

    /// Generate current timestamp as NaiveDateTime
    pub fn now_naive() -> NaiveDateTime {
        Utc::now().naive_utc()
    }

    /// Generate current timestamp with timezone
    pub fn now_with_tz() -> DateTimeWithTimeZone {
        Utc::now().into()
    }

    /// Generate future timestamp (1 hour from now)
    pub fn future() -> DateTimeWithTimeZone {
        (Utc::now() + chrono::Duration::hours(1)).into()
    }

    /// Generate a random string of given length
    pub fn random_string(len: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..len)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}
