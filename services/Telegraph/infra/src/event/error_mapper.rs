use rustycog::core::error::ServiceError;
use rustycog::events::ErrorMapper;
use telegraph_domain::DomainError;

/// Maps Telegraph domain errors to rustycog `ServiceError` for event publish.
pub struct TelegraphErrorMapper;

impl ErrorMapper<DomainError> for TelegraphErrorMapper {
    fn to_service_error(&self, error: DomainError) -> ServiceError {
        ServiceError::from(&error)
    }

    fn from_service_error(&self, error: ServiceError) -> DomainError {
        match error {
            ServiceError::Validation { message, .. } => DomainError::invalid_message(message),
            ServiceError::NotFound { message, .. } => DomainError::notification_not_found(message),
            ServiceError::Authorization { message, .. }
            | ServiceError::Authentication { message, .. } => DomainError::unauthorized(message),
            ServiceError::Infrastructure { message, .. } => {
                DomainError::infrastructure_error(message)
            }
            ServiceError::ServiceUnavailable { message, .. } => {
                DomainError::service_unavailable(message)
            }
            ServiceError::RateLimit { message, .. } => DomainError::rate_limit_exceeded(message),
            ServiceError::Business { message, .. }
            | ServiceError::Conflict { message, .. }
            | ServiceError::Timeout { message, .. }
            | ServiceError::Internal { message, .. } => DomainError::internal_error(message),
        }
    }
}
