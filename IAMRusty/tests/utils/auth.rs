use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Authentication test utilities for user management and database operations
pub struct AuthTestUtils;

impl AuthTestUtils {
    /// Count entities in database table
    pub async fn count_entities(
        db: Arc<DatabaseConnection>,
        table: &str,
    ) -> Result<i64, sea_orm::DbErr> {
        let count: i64 = db
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("SELECT COUNT(*) as count FROM {}", table),
            ))
            .await?
            .unwrap()
            .try_get("", "count")?;
        Ok(count)
    }

    /// Create a test user with email and password (placeholder implementation)
    pub async fn create_user_with_email_password(
        _db: Arc<DatabaseConnection>,
        _email: &str,
        _password: &str,
    ) -> Result<Uuid, sea_orm::DbErr> {
        // This would typically use the fixtures or direct database operations
        // For now, returning a dummy UUID
        Ok(Uuid::new_v4())
    }

    /// Create a test user via OAuth provider (placeholder implementation)
    pub async fn create_user_with_oauth(
        _db: Arc<DatabaseConnection>,
        _provider: &str,
        _provider_user_id: &str,
        _email: &str,
    ) -> Result<Uuid, sea_orm::DbErr> {
        // This would typically use the fixtures or direct database operations
        // For now, returning a dummy UUID
        Ok(Uuid::new_v4())
    }

    /// Helper to create login request payload
    pub fn create_login_payload(email: &str, password: &str) -> Value {
        serde_json::json!({
            "email": email,
            "password": password
        })
    }

    /// Helper to create signup request payload
    pub fn create_signup_payload(email: &str, password: &str) -> Value {
        serde_json::json!({
            "email": email,
            "password": password
        })
    }

    /// Assert that response has expected authentication success structure
    pub fn assert_auth_success_response(response: &Value) {
        assert!(
            response["access_token"].is_string(),
            "Should contain access_token"
        );
        assert!(
            response["refresh_token"].is_string(),
            "Should contain refresh_token"
        );
        assert!(
            response["expires_in"].is_number(),
            "Should contain expires_in"
        );
        assert!(
            response["refresh_expires_in"].is_number(),
            "Should contain refresh_expires_in"
        );
    }

    /// Assert that response has expected error structure
    pub fn assert_error_response_structure(response: &Value, _expected_status: u16) {
        assert!(
            response.get("error").is_some() || response.get("message").is_some(),
            "Response should contain error information"
        );
    }

    /// Verify that JWT token structure is valid
    pub fn verify_jwt_structure(token: &str) -> bool {
        let parts: Vec<&str> = token.split('.').collect();
        parts.len() == 3 && !parts[0].is_empty() && !parts[1].is_empty() && !parts[2].is_empty()
    }
}
