use super::CommandError;
use crate::usecase::login::LoginError;
use crate::usecase::link_provider::LinkProviderError;

/// Utility functions for mapping errors to CommandError consistently across command handlers
pub struct ErrorMapping;

impl ErrorMapping {
    /// Map token service errors to appropriate CommandError based on the error content
    /// 
    /// Authentication-related errors (expired, invalid, malformed tokens, JWT errors, signature issues)
    /// are mapped to CommandError::Validation to return 401 status codes.
    /// Other errors are mapped to CommandError::Infrastructure to return 500 status codes.
    pub fn map_token_service_error_to_validation(error: &dyn std::error::Error) -> CommandError {
        let error_msg = error.to_string();
        if Self::is_authentication_related_error(&error_msg) {
            CommandError::Validation(format!("Authentication failed: {}", error_msg))
        } else {
            CommandError::Infrastructure(error.to_string())
        }
    }

    /// Map token service errors to appropriate CommandError with Business error type
    /// 
    /// Similar to map_token_service_error_to_validation but uses Business error type
    /// for contexts where business logic errors are more appropriate.
    pub fn map_token_service_error_to_business(error: &dyn std::error::Error) -> CommandError {
        let error_msg = error.to_string();
        if Self::is_authentication_related_error(&error_msg) {
            CommandError::Business(format!("Authentication failed: {}", error_msg))
        } else {
            CommandError::Infrastructure(error.to_string())
        }
    }

    /// Map LoginError to appropriate CommandError
    /// 
    /// Provides consistent error mapping for all login-related operations.
    pub fn map_login_error(error: LoginError) -> CommandError {
        match error {
            LoginError::AuthError(msg) => CommandError::Business(format!("Authentication failed: {}", msg)),
            LoginError::DbError(e) => CommandError::Infrastructure(format!("Database error: {}", e)),
            LoginError::TokenError(e) => CommandError::Infrastructure(format!("Token service error: {}", e)),
        }
    }

    /// Map LinkProviderError to appropriate CommandError
    /// 
    /// Provides consistent error mapping for all provider linking operations.
    pub fn map_link_provider_error(error: LinkProviderError) -> CommandError {
        match error {
            LinkProviderError::AuthError(msg) => {
                CommandError::Business(format!("Authentication failed: {}", msg))
            }
            LinkProviderError::DbError(e) => {
                CommandError::Infrastructure(format!("Database error: {}", e))
            }
            LinkProviderError::TokenError(e) => {
                CommandError::Infrastructure(format!("Token service error: {}", e))
            }
            LinkProviderError::UserNotFound => {
                CommandError::Business("User not found".to_string())
            }
            LinkProviderError::ProviderAlreadyLinked => {
                CommandError::Business("Provider account is already linked to another user".to_string())
            }
            LinkProviderError::ProviderAlreadyLinkedToSameUser => {
                CommandError::Business("Provider is already linked to your account".to_string())
            }
        }
    }

    /// Map TokenError to appropriate CommandError
    /// 
    /// Token authentication errors (not found, invalid, expired) are mapped to Validation
    /// to return 401 status codes, while infrastructure errors return 500.
    pub fn map_token_error(error: crate::usecase::token::TokenError) -> CommandError {
        use crate::usecase::token::TokenError;
        match error {
            TokenError::RepositoryError(_) => CommandError::Infrastructure(error.to_string()),
            TokenError::TokenServiceError(inner) => {
                Self::map_token_service_error_to_business(inner.as_ref())
            },
            // Authentication-related token errors should return 401
            TokenError::TokenNotFound => CommandError::Validation("Authentication failed: Invalid refresh token".to_string()),
            TokenError::TokenInvalid => CommandError::Validation("Authentication failed: Invalid refresh token".to_string()),
            TokenError::TokenExpired => CommandError::Validation("Authentication failed: Expired refresh token".to_string()),
        }
    }

    /// Check if an error message indicates an authentication-related issue
    fn is_authentication_related_error(error_msg: &str) -> bool {
        error_msg.contains("expired") || 
        error_msg.contains("invalid") || 
        error_msg.contains("Token expired") ||
        error_msg.contains("Invalid token") ||
        error_msg.contains("JWT error") ||
        error_msg.contains("malformed") ||
        error_msg.contains("signature")
    }
} 