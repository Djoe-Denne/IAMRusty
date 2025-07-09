//! HTTP validation for Telegraph

use crate::error::HttpError;

/// Validate email address format
pub fn validate_email(email: &str) -> Result<(), HttpError> {
    if email.contains('@') && email.len() > 3 {
        Ok(())
    } else {
        Err(HttpError::Validation {
            message: "Invalid email format".to_string(),
        })
    }
}

/// Validate phone number format
pub fn validate_phone(phone: &str) -> Result<(), HttpError> {
    if phone.len() >= 10 && phone.chars().all(|c| c.is_ascii_digit() || c == '+' || c == '-' || c == ' ') {
        Ok(())
    } else {
        Err(HttpError::Validation {
            message: "Invalid phone number format".to_string(),
        })
    }
} 