use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;
use crate::error::ValidationError;

/// Custom JSON extractor that validates using the validator crate
/// and returns errors in our uniform format
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    /// Extract the inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ValidationError;

    fn from_request(req: Request, state: &S) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let Json(value) = Json::<T>::from_request(req, state).await
                .map_err(|rejection| match rejection {
                    JsonRejection::JsonDataError(_) => ValidationError::new(
                        "invalid_json_data",
                        "Invalid JSON data in request body"
                    ),
                    JsonRejection::JsonSyntaxError(_) => ValidationError::new(
                        "invalid_json_syntax", 
                        "Invalid JSON syntax in request body"
                    ),
                    JsonRejection::MissingJsonContentType(_) => ValidationError::new(
                        "missing_content_type",
                        "Missing 'Content-Type: application/json' header"
                    ),
                    JsonRejection::BytesRejection(_) => ValidationError::new(
                        "request_body_error",
                        "Failed to read request body"
                    ),
                    _ => ValidationError::new(
                        "json_extraction_error",
                        "Failed to extract JSON from request"
                    ),
                })?;
            
            value.validate()
                .map_err(ValidationError::from)?;
            
            Ok(ValidatedJson(value))
        }
    }
} 