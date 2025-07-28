use axum::{
    async_trait,
    extract::{FromRequest, Request},
    http::StatusCode,
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::HttpError;
use {{SERVICE_NAME}}_application::ValidationError;

/// Validated JSON extractor that automatically validates using the `validator` crate
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|_| HttpError::bad_request("Invalid JSON format"))?;

        // Validate the deserialized value
        value.validate().map_err(|validation_errors| {
            let errors: Vec<ValidationError> = validation_errors
                .field_errors()
                .iter()
                .flat_map(|(field, field_errors)| {
                    field_errors.iter().map(|error| ValidationError {
                        field: field.to_string(),
                        message: error
                            .message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| "Validation error".to_string()),
                    })
                })
                .collect();

            HttpError::Application({{SERVICE_NAME}}_application::ApplicationError::ValidationError(errors))
        })?;

        Ok(ValidatedJson(value))
    }
}

/// Validation utilities
pub mod validators {
    use uuid::Uuid;
    use validator::ValidationError;

    /// Custom validator for UUID strings
    pub fn validate_uuid(uuid_str: &str) -> Result<(), ValidationError> {
        Uuid::parse_str(uuid_str)
            .map(|_| ())
            .map_err(|_| ValidationError::new("invalid_uuid"))
    }

    /// Custom validator for non-empty strings after trimming
    pub fn validate_non_empty_trimmed(value: &str) -> Result<(), ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError::new("cannot_be_empty"))
        } else {
            Ok(())
        }
    }

    /// Custom validator for entity names (alphanumeric, spaces, hyphens, underscores)
    pub fn validate_entity_name(name: &str) -> Result<(), ValidationError> {
        let trimmed = name.trim();
        
        if trimmed.is_empty() {
            return Err(ValidationError::new("cannot_be_empty"));
        }

        if trimmed.len() > 255 {
            return Err(ValidationError::new("too_long"));
        }

        // Allow alphanumeric characters, spaces, hyphens, and underscores
        if !trimmed.chars().all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_') {
            return Err(ValidationError::new("invalid_characters"));
        }

        Ok(())
    }

    /// Custom validator for descriptions
    pub fn validate_description(description: &str) -> Result<(), ValidationError> {
        if description.len() > 1000 {
            Err(ValidationError::new("too_long"))
        } else {
            Ok(())
        }
    }
}

/// Query parameter validation helpers
pub mod query_params {
    use serde::{Deserialize, Deserializer};
    use uuid::Uuid;

    /// Deserialize UUID from string with validation
    pub fn deserialize_uuid<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Uuid::parse_str(&s).map_err(serde::de::Error::custom)
    }

    /// Deserialize optional UUID from string with validation
    pub fn deserialize_optional_uuid<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt_s: Option<String> = Option::deserialize(deserializer)?;
        match opt_s {
            Some(s) => Uuid::parse_str(&s).map(Some).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }

    /// Deserialize page number with minimum value validation
    pub fn deserialize_page<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let page = u32::deserialize(deserializer)?;
        if page == 0 {
            Err(serde::de::Error::custom("Page must be at least 1"))
        } else {
            Ok(page)
        }
    }

    /// Deserialize page size with range validation
    pub fn deserialize_page_size<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let page_size = u32::deserialize(deserializer)?;
        if page_size == 0 || page_size > 100 {
            Err(serde::de::Error::custom("Page size must be between 1 and 100"))
        } else {
            Ok(page_size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::validators::*;
    use serde::{Deserialize, Serialize};
    use validator::Validate;

    #[derive(Debug, Serialize, Deserialize, Validate)]
    struct TestRequest {
        #[validate(custom = "validate_entity_name")]
        name: String,
        
        #[validate(email)]
        email: Option<String>,
        
        #[validate(range(min = 1, max = 100))]
        age: u32,
    }

    #[test]
    fn test_validate_uuid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_uuid("invalid-uuid").is_err());
        assert!(validate_uuid("").is_err());
    }

    #[test]
    fn test_validate_non_empty_trimmed() {
        assert!(validate_non_empty_trimmed("hello").is_ok());
        assert!(validate_non_empty_trimmed("  hello  ").is_ok());
        assert!(validate_non_empty_trimmed("").is_err());
        assert!(validate_non_empty_trimmed("   ").is_err());
    }

    #[test]
    fn test_validate_entity_name() {
        assert!(validate_entity_name("Valid Name").is_ok());
        assert!(validate_entity_name("valid-name_123").is_ok());
        assert!(validate_entity_name("").is_err());
        assert!(validate_entity_name("name@invalid").is_err());
        assert!(validate_entity_name(&"x".repeat(256)).is_err());
    }

    #[test]
    fn test_validate_description() {
        assert!(validate_description("Short description").is_ok());
        assert!(validate_description("").is_ok());
        assert!(validate_description(&"x".repeat(1001)).is_err());
    }

    #[tokio::test]
    async fn test_validation_request() {
        let valid_request = TestRequest {
            name: "Valid Name".to_string(),
            email: Some("test@example.com".to_string()),
            age: 25,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = TestRequest {
            name: "".to_string(), // Invalid: empty name
            email: Some("invalid-email".to_string()), // Invalid: bad email format
            age: 0, // Invalid: age too low
        };
        assert!(invalid_request.validate().is_err());
    }
} 