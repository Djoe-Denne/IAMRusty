use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ApplicationError;
use rustycog_core::error::DomainError;

/// Standard API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error_type: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub request_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub validation_errors: Option<Vec<ApiValidationError>>,
}

/// Validation error for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiValidationError {
    pub field: String,
    pub code: Option<String>,
    pub message: String,
    pub rejected_value: Option<serde_json::Value>,
}

/// DTO for pagination response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationResponse {
    pub current_page: u32,
    pub page_size: u32,
    pub total_items: Option<i64>,
    pub total_pages: Option<u32>,
    pub has_next: bool,
    pub has_previous: bool,
    pub next_cursor: Option<String>,
    pub previous_cursor: Option<String>,
}

/// DTO for pagination request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationRequest {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub cursor: Option<String>,
}

impl PaginationResponse {
    /// Create a new pagination response
    #[must_use]
    pub fn new(current_page: u32, page_size: u32, total_items: Option<i64>) -> Self {
        let total_pages =
            total_items.map(|total| ((total as f64) / f64::from(page_size)).ceil() as u32);

        let has_next = total_pages.is_some_and(|total| current_page < total);
        let has_previous = current_page > 1;

        Self {
            current_page,
            page_size,
            total_items,
            total_pages,
            has_next,
            has_previous,
            next_cursor: None,
            previous_cursor: None,
        }
    }

    /// Create pagination response with cursor support
    #[must_use]
    pub fn with_cursors(
        current_page: u32,
        page_size: u32,
        total_items: Option<i64>,
        next_cursor: Option<String>,
        previous_cursor: Option<String>,
    ) -> Self {
        let mut pagination = Self::new(current_page, page_size, total_items);
        pagination.next_cursor = next_cursor;
        pagination.previous_cursor = previous_cursor;
        pagination
    }
}

impl Default for PaginationRequest {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
            cursor: None,
        }
    }
}

impl PaginationRequest {
    /// Get page number, defaulting to 1
    #[must_use]
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1)
    }

    /// Get page size, defaulting to 20
    #[must_use]
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100) // Cap at 100
    }
}

impl From<ApplicationError> for ApiErrorResponse {
    fn from(error: ApplicationError) -> Self {
        let timestamp = Utc::now();

        match error {
            ApplicationError::Domain(domain_error) => {
                let (error_type, message) = match domain_error {
                    DomainError::EntityNotFound { entity_type, id } => (
                        "entity_not_found".to_string(),
                        format!("{entity_type} not found: {id}"),
                    ),
                    DomainError::InvalidInput { message } => ("invalid_input".to_string(), message),
                    DomainError::BusinessRuleViolation { rule } => {
                        ("business_rule_violation".to_string(), rule)
                    }
                    DomainError::Unauthorized { operation } => (
                        "unauthorized".to_string(),
                        format!("Unauthorized: {operation}"),
                    ),
                    DomainError::ResourceAlreadyExists {
                        resource_type,
                        identifier,
                    } => (
                        "resource_already_exists".to_string(),
                        format!("{resource_type} already exists: {identifier}"),
                    ),
                    DomainError::ExternalServiceError { service, message } => (
                        "external_service_error".to_string(),
                        format!("External service error ({service}): {message}"),
                    ),
                    DomainError::PermissionDenied { message } => {
                        ("permission_denied".to_string(), message)
                    }
                    DomainError::Internal { message: _ } => (
                        "internal_error".to_string(),
                        "An internal error occurred".to_string(),
                    ),
                };

                Self {
                    error_type,
                    message,
                    timestamp,
                    request_id: None,
                    details: None,
                    validation_errors: None,
                }
            }
            ApplicationError::ValidationError(validation_errors) => {
                let api_validation_errors: Vec<ApiValidationError> = validation_errors
                    .into_iter()
                    .map(|e| ApiValidationError {
                        field: e.field,
                        message: e.message,
                        code: e.code,
                        rejected_value: None,
                    })
                    .collect();

                Self {
                    error_type: "validation_error".to_string(),
                    message: "Validation failed".to_string(),
                    timestamp,
                    request_id: None,
                    details: None,
                    validation_errors: Some(api_validation_errors),
                }
            }
            ApplicationError::ExternalService { service, message } => Self {
                error_type: "external_service_error".to_string(),
                message: format!("External service error ({service}): {message}"),
                timestamp,
                request_id: None,
                details: None,
                validation_errors: None,
            },
            ApplicationError::RateLimit { message } => Self {
                error_type: "rate_limit".to_string(),
                message,
                timestamp,
                request_id: None,
                details: None,
                validation_errors: None,
            },
            ApplicationError::Internal { message: _ } => Self {
                error_type: "internal_error".to_string(),
                message: "An internal error occurred".to_string(),
                timestamp,
                request_id: None,
                details: None,
                validation_errors: None,
            },
        }
    }
}
