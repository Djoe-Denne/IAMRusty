//! HTTP validation for Telegraph

use validator::ValidationError;
use uuid::Uuid;

/// Validate email address format
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    if email.contains('@') && email.len() > 3 {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_email_format"))
    }
}

/// Validate phone number format
pub fn validate_phone(phone: &str) -> Result<(), ValidationError> {
    if phone.len() >= 10 && phone.chars().all(|c| c.is_ascii_digit() || c == '+' || c == '-' || c == ' ') {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_phone_number_format"))
    }
} 

/// Validate UUID v4 format for path parameters
pub fn validate_uuid_v4(uuid: &str) -> Result<(), ValidationError> {
    if Uuid::parse_str(uuid).is_ok() {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_uuid_format"))
    }
}
