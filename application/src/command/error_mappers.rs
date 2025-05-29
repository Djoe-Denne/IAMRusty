use super::{CommandError, registry::CommandErrorMapper};
use crate::usecase::{
    login::LoginError,
    link_provider::LinkProviderError,
    auth::AuthError,
    user::UserError,
    token::TokenError,
};

/// Error mapper for login-related commands
pub struct LoginErrorMapper;

impl CommandErrorMapper for LoginErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        // Try to downcast to known error types
        if let Some(login_error) = error.downcast_ref::<LoginError>() {
            match login_error {
                LoginError::AuthError(msg) => CommandError::Business(format!("Authentication failed: {}", msg)),
                LoginError::DbError(e) => CommandError::Infrastructure(format!("Database error: {}", e)),
                LoginError::TokenError(e) => CommandError::Infrastructure(format!("Token service error: {}", e)),
            }
        } else {
            // Check if it's an authentication-related error by message
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::Business(format!("Authentication failed: {}", error_msg))
            } else {
                CommandError::Infrastructure(error.to_string())
            }
        }
    }
}

impl LoginErrorMapper {
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

/// Error mapper for link provider commands
pub struct LinkProviderErrorMapper;

impl CommandErrorMapper for LinkProviderErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(link_error) = error.downcast_ref::<LinkProviderError>() {
            match link_error {
                LinkProviderError::AuthError(_msg) => {
                    CommandError::Authentication("Authentication failed".to_string())
                }
                LinkProviderError::DbError(e) => {
                    CommandError::Infrastructure(format!("Database error: {}", e))
                }
                LinkProviderError::TokenError(e) => {
                    CommandError::Infrastructure(format!("Token service error: {}", e))
                }
                LinkProviderError::UserNotFound => {
                    CommandError::Authentication("Authentication failed".to_string())
                }
                LinkProviderError::ProviderAlreadyLinked => {
                    CommandError::Business("Provider account is already linked to another user".to_string())
                }
                LinkProviderError::ProviderAlreadyLinkedToSameUser => {
                    CommandError::Business("Provider is already linked to your account".to_string())
                }
            }
        } else {
            CommandError::Infrastructure(error.to_string())
        }
    }
}

/// Error mapper for token-related commands
pub struct TokenErrorMapper;

impl CommandErrorMapper for TokenErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(token_error) = error.downcast_ref::<TokenError>() {
            match token_error {
                TokenError::RepositoryError(_) => CommandError::Infrastructure(error.to_string()),
                TokenError::TokenServiceError(inner) => {
                    let error_msg = inner.to_string();
                    if Self::is_authentication_related_error(&error_msg) {
                        CommandError::Business(format!("Authentication failed: {}", error_msg))
                    } else {
                        CommandError::Infrastructure(error.to_string())
                    }
                },
                // Authentication-related token errors should return 401
                TokenError::TokenNotFound => CommandError::Authentication("Authentication failed: Invalid refresh token".to_string()),
                TokenError::TokenInvalid => CommandError::Authentication("Authentication failed: Invalid refresh token".to_string()),
                TokenError::TokenExpired => CommandError::Authentication("Authentication failed: Expired refresh token".to_string()),
            }
        } else {
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::Validation(format!("Authentication failed: {}", error_msg))
            } else {
                CommandError::Infrastructure(error.to_string())
            }
        }
    }
}

impl TokenErrorMapper {
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

/// Error mapper for user-related commands
pub struct UserErrorMapper;

impl CommandErrorMapper for UserErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(user_error) = error.downcast_ref::<UserError>() {
            match user_error {
                UserError::RepositoryError(_) => CommandError::Infrastructure(error.to_string()),
                UserError::TokenServiceError(inner) => {
                    let error_msg = inner.to_string();
                    if Self::is_authentication_related_error(&error_msg) {
                        CommandError::Validation(format!("Authentication failed: {}", error_msg))
                    } else {
                        CommandError::Infrastructure(error.to_string())
                    }
                },
                _ => CommandError::Authentication("Authentication failed".to_string()),
            }
        } else {
            CommandError::Authentication("Authentication failed".to_string())
        }
    }
}

impl UserErrorMapper {
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

/// Error mapper for authentication-related commands (signup, password login, verify email)
pub struct AuthErrorMapper;

impl CommandErrorMapper for AuthErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(auth_error) = error.downcast_ref::<AuthError>() {
            match auth_error {
                AuthError::InvalidCredentials => CommandError::Validation("Invalid credentials".to_string()),
                AuthError::UserNotFound => CommandError::Business("Invalid credentials".to_string()), // Don't leak user existence
                AuthError::EmailNotVerified => CommandError::Business("Email not verified".to_string()),
                AuthError::UserAlreadyExists => CommandError::Business("User already exists".to_string()),
                AuthError::WeakPassword => CommandError::Validation("Password is too weak".to_string()),
                AuthError::InvalidEmail => CommandError::Validation("Invalid email format".to_string()),
                AuthError::EmailNotFound => CommandError::Business("Invalid verification request".to_string()), // Don't leak email existence
                AuthError::EmailAlreadyVerified => CommandError::Business("Email is already verified".to_string()),
                AuthError::InvalidVerificationToken => CommandError::Validation("Invalid or expired verification token".to_string()),
                AuthError::VerificationTokenExpired => CommandError::Validation("Verification token has expired".to_string()),
                AuthError::RepositoryError(_) => CommandError::Infrastructure(error.to_string()),
                AuthError::EventPublishingError(_) => CommandError::Infrastructure(error.to_string()),
                AuthError::TokenServiceError(inner) => {
                    let error_msg = inner.to_string();
                    if Self::is_authentication_related_error(&error_msg) {
                        CommandError::Validation(format!("Authentication failed: {}", error_msg))
                    } else {
                        CommandError::Infrastructure(error.to_string())
                    }
                },
                AuthError::PasswordHashingError(_) => CommandError::Infrastructure(error.to_string()),
                AuthError::VerificationTokenGenerationError(_) => CommandError::Infrastructure(error.to_string()),
            }
        } else {
            let error_msg = error.to_string();
            if Self::is_authentication_related_error(&error_msg) {
                CommandError::Validation(format!("Authentication failed: {}", error_msg))
            } else {
                CommandError::Infrastructure(error.to_string())
            }
        }
    }
}

impl AuthErrorMapper {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt;

    // Test error type for testing the mappers
    #[derive(Debug)]
    struct TestError {
        message: String,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl Error for TestError {}

    #[test]
    fn test_auth_error_mapper_with_auth_error() {
        let mapper = AuthErrorMapper;
        let auth_error = AuthError::InvalidCredentials;
        let boxed_error: Box<dyn Error + Send + Sync> = Box::new(auth_error);
        
        let result = mapper.map_error(boxed_error);
        match result {
            CommandError::Validation(msg) => assert_eq!(msg, "Invalid credentials"),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_auth_error_mapper_with_generic_error() {
        let mapper = AuthErrorMapper;
        let test_error = TestError { message: "Some error".to_string() };
        let boxed_error: Box<dyn Error + Send + Sync> = Box::new(test_error);
        
        let result = mapper.map_error(boxed_error);
        match result {
            CommandError::Infrastructure(msg) => assert_eq!(msg, "Some error"),
            _ => panic!("Expected Infrastructure error"),
        }
    }

    #[test]
    fn test_token_error_mapper_with_token_error() {
        let mapper = TokenErrorMapper;
        let token_error = TokenError::TokenNotFound;
        let boxed_error: Box<dyn Error + Send + Sync> = Box::new(token_error);
        
        let result = mapper.map_error(boxed_error);
        match result {
            CommandError::Authentication(msg) => assert!(msg.contains("Invalid refresh token")),
            _ => panic!("Expected Authentication error"),
        }
    }

    #[test]
    fn test_authentication_related_error_detection() {
        assert!(AuthErrorMapper::is_authentication_related_error("Token expired"));
        assert!(AuthErrorMapper::is_authentication_related_error("Invalid token"));
        assert!(AuthErrorMapper::is_authentication_related_error("JWT error"));
        assert!(!AuthErrorMapper::is_authentication_related_error("Database connection failed"));
    }
} 