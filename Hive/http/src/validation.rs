use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono;
use hive_application::{ApiErrorResponse, ApiValidationError};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::HttpError;

/// A wrapper around JSON that validates the payload
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) =
            Json::<T>::from_request(req, state)
                .await
                .map_err(|_| HttpError::Validation {
                    message: "Invalid JSON payload".to_string(),
                })?;

        // Validate the payload
        match value.validate() {
            Ok(_) => Ok(ValidatedJson(value)),
            Err(validation_errors) => {
                let errors: Vec<ApiValidationError> = validation_errors
                    .field_errors()
                    .iter()
                    .flat_map(|(field, errors)| {
                        errors.iter().map(|error| ApiValidationError {
                            field: field.to_string(),
                            code: Some(error.code.to_string()),
                            message: error
                                .message
                                .as_ref()
                                .map_or_else(|| error.code.to_string(), |m| m.to_string()),
                            rejected_value: None, // We could extract this from the error if needed
                        })
                    })
                    .collect();

                let error_response = ApiErrorResponse {
                    error_type: "validation_error".to_string(),
                    message: "Validation failed".to_string(),
                    timestamp: chrono::Utc::now(),
                    request_id: None,
                    details: None,
                    validation_errors: Some(errors),
                };

                Err(HttpError::Validation {
                    message: "Validation failed".to_string(),
                })
            }
        }
    }
}

/// Validate query parameters
pub fn validate_query_params<T>(params: &T) -> Result<(), HttpError>
where
    T: Validate,
{
    match params.validate() {
        Ok(_) => Ok(()),
        Err(_) => Err(HttpError::Validation {
            message: "Invalid query parameters".to_string(),
        }),
    }
}

/// Validate pagination parameters
pub fn validate_pagination(
    page: Option<u32>,
    page_size: Option<u32>,
    max_page_size: u32,
) -> Result<(u32, u32), HttpError> {
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);

    if page == 0 {
        return Err(HttpError::Validation {
            message: "Page number must be greater than 0".to_string(),
        });
    }

    if page_size == 0 || page_size > max_page_size {
        return Err(HttpError::Validation {
            message: format!("Page size must be between 1 and {}", max_page_size),
        });
    }

    Ok((page, page_size))
}
