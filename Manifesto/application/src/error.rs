use rustycog_command::CommandError;
use rustycog_core::error::DomainError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<ApplicationError> for CommandError {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::Domain(domain_error) => match domain_error {
                DomainError::EntityNotFound { .. } => {
                    Self::business("not_found", domain_error.to_string())
                }
                DomainError::InvalidInput { .. } => {
                    Self::validation("invalid_input", domain_error.to_string())
                }
                DomainError::BusinessRuleViolation { .. } => {
                    Self::business("business_rule_violation", domain_error.to_string())
                }
                DomainError::Unauthorized { .. } => {
                    Self::authentication("unauthorized", domain_error.to_string())
                }
                DomainError::ResourceAlreadyExists { .. } => {
                    Self::business("already_exists", domain_error.to_string())
                }
                DomainError::ExternalServiceError { .. } => {
                    Self::infrastructure("external_service_error", domain_error.to_string())
                }
                DomainError::PermissionDenied { .. } => {
                    Self::authentication("permission_denied", domain_error.to_string())
                }
                DomainError::Internal { .. } => {
                    Self::infrastructure("internal_error", domain_error.to_string())
                }
            },
            ApplicationError::Validation(msg) => Self::validation("validation_error", msg),
            ApplicationError::NotFound(msg) => Self::business("not_found", msg),
            ApplicationError::AlreadyExists(msg) => Self::business("already_exists", msg),
            ApplicationError::Internal(msg) => Self::infrastructure("internal_error", msg),
        }
    }
}
