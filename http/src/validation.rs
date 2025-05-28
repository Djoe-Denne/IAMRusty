//! Validation utilities and common patterns for HTTP handlers

use lazy_static::lazy_static;
use regex::Regex;
use validator::ValidationError;

lazy_static! {
    /// Regex for validating provider names (only lowercase letters)
    pub static ref PROVIDER_REGEX: Regex = Regex::new(r"^[a-z]+$").unwrap();
    
    /// Regex for validating JWT tokens (base64url format)
    pub static ref JWT_TOKEN_REGEX: Regex = Regex::new(r"^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$").unwrap();
    
    /// Regex for validating UUIDs
    pub static ref UUID_REGEX: Regex = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
}

/// Custom validation function for provider names
pub fn validate_provider_name(provider: &str) -> Result<(), ValidationError> {
    let valid_providers = ["github", "gitlab"];
    
    if !valid_providers.contains(&provider.to_lowercase().as_str()) {
        return Err(ValidationError::new("invalid_provider"));
    }
    
    Ok(())
}

/// Custom validation function for non-empty strings
pub fn validate_non_empty_string(value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::new("empty_string"));
    }
    Ok(())
}

/// Custom validation function for OAuth codes
pub fn validate_oauth_code(code: &str) -> Result<(), ValidationError> {
    if code.trim().is_empty() {
        return Err(ValidationError::new("empty_oauth_code"));
    }
    
    // OAuth codes should be reasonably sized (typically much smaller than 1000 chars)
    if code.len() > 1000 {
        return Err(ValidationError::new("oauth_code_too_long"));
    }
    
    Ok(())
}

/// Custom validation function for refresh tokens
pub fn validate_refresh_token(token: &str) -> Result<(), ValidationError> {
    if token.trim().is_empty() {
        return Err(ValidationError::new("empty_refresh_token"));
    }
    
    // Basic format check - should be a reasonable length and contain valid characters
    if token.len() < 10 || token.len() > 1000 {
        return Err(ValidationError::new("invalid_refresh_token_length"));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::auth::{OAuthCallbackQuery, ProviderPath};
    use crate::handlers::token::RefreshTokenRequest;
    use validator::Validate;

    #[test]
    fn test_provider_validation() {
        // Valid provider
        let valid_provider = ProviderPath {
            provider_name: "github".to_string(),
        };
        assert!(valid_provider.validate().is_ok());
        
        // Invalid provider
        let invalid_provider = ProviderPath {
            provider_name: "invalid".to_string(),
        };
        assert!(invalid_provider.validate().is_err());
        
        // Empty provider
        let empty_provider = ProviderPath {
            provider_name: "".to_string(),
        };
        assert!(empty_provider.validate().is_err());
        
        // Provider with uppercase letters
        let uppercase_provider = ProviderPath {
            provider_name: "GitHub".to_string(),
        };
        assert!(uppercase_provider.validate().is_err());
    }

    #[test]
    fn test_oauth_callback_query_validation() {
        // Valid query with all fields
        let valid_query = OAuthCallbackQuery {
            code: Some("valid_code_123".to_string()),
            state: Some("state123".to_string()),
            error: None,
            error_description: None,
        };
        assert!(valid_query.validate().is_ok());
        
        // Query with code that's too long
        let long_code_query = OAuthCallbackQuery {
            code: Some("a".repeat(1001)),
            state: Some("state123".to_string()),
            error: None,
            error_description: None,
        };
        assert!(long_code_query.validate().is_err());
        
        // Query with state that's too long
        let long_state_query = OAuthCallbackQuery {
            code: Some("valid_code".to_string()),
            state: Some("a".repeat(2001)),
            error: None,
            error_description: None,
        };
        assert!(long_state_query.validate().is_err());
    }

    #[test]
    fn test_refresh_token_validation() {
        // Valid refresh token
        let valid_token = RefreshTokenRequest {
            refresh_token: "valid_refresh_token_123456789".to_string(),
        };
        assert!(valid_token.validate().is_ok());
        
        // Token too short
        let short_token = RefreshTokenRequest {
            refresh_token: "short".to_string(),
        };
        assert!(short_token.validate().is_err());
        
        // Token too long
        let long_token = RefreshTokenRequest {
            refresh_token: "a".repeat(1001),
        };
        assert!(long_token.validate().is_err());
        
        // Empty token
        let empty_token = RefreshTokenRequest {
            refresh_token: "".to_string(),
        };
        assert!(empty_token.validate().is_err());
    }

    #[test]
    fn test_custom_validation_functions() {
        // Test provider validation
        assert!(validate_provider_name("github").is_ok());
        assert!(validate_provider_name("gitlab").is_ok());
        assert!(validate_provider_name("invalid").is_err());
        assert!(validate_provider_name("").is_err());
        
        // Test refresh token validation
        assert!(validate_refresh_token("valid_token_123").is_ok());
        assert!(validate_refresh_token("short").is_err());
        assert!(validate_refresh_token("").is_err());
        assert!(validate_refresh_token(&"a".repeat(1001)).is_err());
    }
} 