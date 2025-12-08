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

impl From<ApplicationError> for rustycog_command::CommandError {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::Domain(e) => {
                rustycog_command::CommandError::business("domain_error", &e.to_string())
            }
            ApplicationError::Validation(msg) => {
                rustycog_command::CommandError::validation("validation_error", &msg)
            }
            ApplicationError::NotFound(msg) => {
                rustycog_command::CommandError::business("not_found", &msg)
            }
            ApplicationError::AlreadyExists(msg) => {
                rustycog_command::CommandError::business("already_exists", &msg)
            }
            ApplicationError::Internal(msg) => {
                rustycog_command::CommandError::infrastructure("internal_error", &msg)
            }
        }
    }
}

