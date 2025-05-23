use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use domain::entity::provider::{Provider, ProviderTokens};
use domain::port::repository::TokenWriteRepository;
use tracing::debug;

use super::entity::{provider_token, prelude::ProviderToken};

/// SeaORM implementation of TokenWriteRepository
#[derive(Clone)]
pub struct TokenWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl TokenWriteRepositoryImpl {
    /// Create a new TokenWriteRepositoryImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert domain ProviderTokens to a database model
    fn to_model(
        user_id: Uuid,
        provider: Provider,
        tokens: &ProviderTokens,
    ) -> provider_token::ActiveModel {
        provider_token::ActiveModel {
            id: Default::default(), // Auto-generated
            user_id: Set(user_id),
            provider: Set(provider.as_str().to_string()),
            access_token: Set(tokens.access_token.clone()),
            refresh_token: Set(tokens.refresh_token.clone()),
            expires_in: Set(tokens.expires_in.map(|e| e as i32)),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
        }
    }
}

#[async_trait]
impl TokenWriteRepository for TokenWriteRepositoryImpl {
    type Error = DbErr;

    async fn save_provider_tokens(
        &self,
        user_id: Uuid,
        provider: Provider,
        tokens: ProviderTokens,
    ) -> Result<(), Self::Error> {
        debug!(user_id = %user_id, provider = %provider.as_str(), "Saving provider tokens");
        
        // Check if tokens already exist for this user and provider
        let existing = ProviderToken::find()
            .filter(provider_token::Column::UserId.eq(user_id))
            .filter(provider_token::Column::Provider.eq(provider.as_str()))
            .one(self.db.as_ref())
            .await?;

        if let Some(existing) = existing {
            // Update existing tokens
            let mut model = provider_token::ActiveModel::from(existing);
            model.access_token = Set(tokens.access_token.clone());
            model.refresh_token = Set(tokens.refresh_token.clone());
            model.expires_in = Set(tokens.expires_in.map(|e| e as i32));
            model.updated_at = Set(Utc::now().naive_utc());
            
            model.update(self.db.as_ref()).await?;
            
            debug!(user_id = %user_id, provider = %provider.as_str(), "Updated provider tokens");
        } else {
            // Insert new tokens
            let model = Self::to_model(user_id, provider, &tokens);
            model.insert(self.db.as_ref()).await?;
            
            debug!(user_id = %user_id, provider = %provider.as_str(), "Saved new provider tokens");
        }

        Ok(())
    }
} 