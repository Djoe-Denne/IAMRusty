//! Validation utilities and common patterns for HTTP handlers

use lazy_static::lazy_static;
use regex::Regex;
use tracing::log::{debug, warn};
use validator::ValidationError;

lazy_static! {
    /// Regex for validating provider names (letters only, case-insensitive)
    pub static ref PROVIDER_REGEX: Regex = Regex::new(r"^[a-zA-Z]+$").unwrap();

    /// Regex for validating JWT tokens (base64url format)
    pub static ref JWT_TOKEN_REGEX: Regex = Regex::new(r"^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$").unwrap();

    /// Regex for validating UUIDs
    pub static ref UUID_REGEX: Regex = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    /// Regex for validating email addresses (more comprehensive than basic email validation)
    pub static ref EMAIL_REGEX: Regex = Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+$"
    ).unwrap();

    /// Regex for validating usernames (alphanumeric, underscores, hyphens, 3-50 chars)
    pub static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]{3,50}$").unwrap();

    /// Regex for strong password validation (at least 8 chars, contains letter and number)
    pub static ref STRONG_PASSWORD_REGEX: Regex = Regex::new(r"^.{8,128}$").unwrap();

    /// Regex to check if password contains at least one letter
    pub static ref HAS_LETTER_REGEX: Regex = Regex::new(r"[a-zA-Z]").unwrap();

    /// Regex to check if password contains at least one digit
    pub static ref HAS_DIGIT_REGEX: Regex = Regex::new(r"\d").unwrap();

    /// Regex for verification tokens (alphanumeric and common safe characters)
    pub static ref VERIFICATION_TOKEN_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]{10,100}$").unwrap();
}

// Common weak passwords to reject
const COMMON_WEAK_PASSWORDS: &[&str] = &[
    "password",
    "123456789",
    "qwerty",
    "abc123",
    "letmein",
    "welcome",
    "monkey",
    "1234567890",
    "password123",
    "admin",
    "12345678",
    "iloveyou",
    "princess",
    "1234567",
    "rockyou",
    "12345",
    "123123",
    "baseball",
    "abc123",
    "football",
    "monkey",
    "letmein",
    "696969",
    "shadow",
    "master",
    "666666",
    "qwertyuiop",
    "123321",
    "mustang",
    "1234567890",
];

/// Custom validation function for provider names
pub fn validate_provider_name(provider: &str) -> Result<(), ValidationError> {
    debug!("Validating provider name: '{}'", provider);

    let valid_providers = ["github", "gitlab"];
    let lowercase_provider = provider.to_lowercase();

    debug!("Lowercase provider: '{}'", lowercase_provider);
    debug!("Valid providers: {:?}", valid_providers);

    if !valid_providers.contains(&lowercase_provider.as_str()) {
        warn!(
            "Invalid provider name '{}' (lowercase: '{}')",
            provider, lowercase_provider
        );
        return Err(ValidationError::new("invalid_provider"));
    }

    debug!("Provider name '{}' is valid", provider);
    Ok(())
}

/// Custom validation function for non-empty strings
pub fn validate_non_empty_string(value: &str) -> Result<(), ValidationError> {
    debug!("Validating non-empty string: '{}'", value);

    if value.trim().is_empty() {
        warn!("String is empty or whitespace only: '{}'", value);
        return Err(ValidationError::new("empty_string"));
    }

    debug!("String is valid (non-empty): '{}'", value);
    Ok(())
}

/// Custom validation function for usernames
pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    debug!("Validating username: '{}'", username);

    if username.trim().is_empty() {
        warn!("Username is empty: '{}'", username);
        return Err(ValidationError::new("empty_username"));
    }

    if !USERNAME_REGEX.is_match(username) {
        warn!("Username format invalid: '{}'", username);
        return Err(ValidationError::new("invalid_username_format"));
    }

    // Require at least one letter (cannot be only numbers/symbols)
    if !HAS_LETTER_REGEX.is_match(username) {
        warn!("Username must contain at least one letter: '{}'", username);
        return Err(ValidationError::new("username_needs_letter"));
    }

    debug!("Username is valid: '{}'", username);
    Ok(())
}

/// Custom validation function for email addresses
pub fn validate_email_format(email: &str) -> Result<(), ValidationError> {
    debug!("Validating email: '{}'", email);

    let trimmed_email = email.trim();

    if trimmed_email.is_empty() {
        warn!("Email is empty: '{}'", email);
        return Err(ValidationError::new("empty_email"));
    }

    if !EMAIL_REGEX.is_match(trimmed_email) {
        warn!("Email format invalid: '{}'", email);
        return Err(ValidationError::new("invalid_email_format"));
    }

    // Additional checks
    if trimmed_email.len() > 254 {
        warn!("Email too long: {} characters", trimmed_email.len());
        return Err(ValidationError::new("email_too_long"));
    }

    debug!("Email is valid: '{}'", email);
    Ok(())
}

/// Custom validation function for strong passwords
pub fn validate_strong_password(password: &str) -> Result<(), ValidationError> {
    debug!("Validating password strength (length: {})", password.len());

    if password.is_empty() {
        warn!("Password is empty");
        return Err(ValidationError::new("empty_password"));
    }

    if password.len() < 8 {
        warn!("Password too short: {} characters", password.len());
        return Err(ValidationError::new("password_too_short"));
    }

    if password.len() > 128 {
        warn!("Password too long: {} characters", password.len());
        return Err(ValidationError::new("password_too_long"));
    }

    // Check basic length requirement
    if !STRONG_PASSWORD_REGEX.is_match(password) {
        warn!("Password doesn't meet length requirements");
        return Err(ValidationError::new("password_invalid_length"));
    }

    // Check for at least one letter
    if !HAS_LETTER_REGEX.is_match(password) {
        warn!("Password must contain at least one letter");
        return Err(ValidationError::new("password_needs_letter"));
    }

    // Check for at least one digit
    if !HAS_DIGIT_REGEX.is_match(password) {
        warn!("Password must contain at least one digit");
        return Err(ValidationError::new("password_needs_digit"));
    }

    // Check against common weak passwords
    let password_lower = password.to_lowercase();
    if COMMON_WEAK_PASSWORDS.contains(&password_lower.as_str()) {
        warn!("Password is in common weak passwords list");
        return Err(ValidationError::new("password_too_common"));
    }

    debug!("Password meets strength requirements");
    Ok(())
}

/// Custom validation function for verification tokens
pub fn validate_verification_token(token: &str) -> Result<(), ValidationError> {
    debug!("Validating verification token: '{}'", token);

    if token.trim().is_empty() {
        warn!("Verification token is empty: '{}'", token);
        return Err(ValidationError::new("empty_verification_token"));
    }

    if !VERIFICATION_TOKEN_REGEX.is_match(token) {
        warn!("Verification token format invalid: '{}'", token);
        return Err(ValidationError::new("invalid_verification_token_format"));
    }

    debug!("Verification token is valid: '{}'", token);
    Ok(())
}

/// Custom validation function for OAuth codes
pub fn validate_oauth_code(code: &str) -> Result<(), ValidationError> {
    debug!("Validating OAuth code: '{}' (length: {})", code, code.len());

    if code.trim().is_empty() {
        warn!("OAuth code is empty or whitespace only: '{}'", code);
        return Err(ValidationError::new("empty_oauth_code"));
    }

    // OAuth codes should be reasonably sized (typically much smaller than 1000 chars)
    if code.len() > 1000 {
        warn!("OAuth code too long: {} characters", code.len());
        return Err(ValidationError::new("oauth_code_too_long"));
    }

    debug!("OAuth code is valid: '{}'", code);
    Ok(())
}

/// Custom validation function for refresh tokens
pub fn validate_refresh_token(token: &str) -> Result<(), ValidationError> {
    debug!(
        "Validating refresh token: '{}' (length: {})",
        token,
        token.len()
    );

    if token.trim().is_empty() {
        warn!("Refresh token is empty or whitespace only: '{}'", token);
        return Err(ValidationError::new("empty_refresh_token"));
    }

    // Basic format check - should be a reasonable length and contain valid characters
    if token.len() < 10 || token.len() > 1000 {
        warn!(
            "Refresh token invalid length: {} characters (should be 10-1000)",
            token.len()
        );
        return Err(ValidationError::new("invalid_refresh_token_length"));
    }

    debug!("Refresh token is valid: '{}'", token);
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
        assert!(uppercase_provider.validate().is_ok());
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
