use rustycog::core::error::ServiceError;
use rustycog::events::ErrorMapper;
use telegraph_domain::DomainError;

/// Maps Telegraph domain errors to rustycog `ServiceError` for event publish.
pub struct TelegraphErrorMapper;

impl ErrorMapper<DomainError> for TelegraphErrorMapper {
    fn to_service_error(&self, error: DomainError) -> ServiceError {
        match error {
            DomainError::InvalidMessage(message)
            | DomainError::InvalidRecipient(message)
            | DomainError::InvalidEmail(message)
            | DomainError::InvalidPhoneNumber(message) => ServiceError::validation(message),
            DomainError::TemplateNotFound(message) | DomainError::NotificationNotFound(message) => {
                ServiceError::not_found(message)
            }
            DomainError::Unauthorized(message) => ServiceError::authorization(message),
            DomainError::DeliveryFailed(message)
            | DomainError::InfrastructureError(message)
            | DomainError::EventProcessingError(message) => ServiceError::infrastructure(message),
            DomainError::ServiceUnavailable(message) => ServiceError::ServiceUnavailable {
                message,
                retry_after: None,
            },
            DomainError::RateLimitExceeded(message) => ServiceError::RateLimit {
                message,
                retry_after: None,
            },
            DomainError::TemplateLoadError(message)
            | DomainError::TemplateRenderError(message)
            | DomainError::OperationNotSupported(message)
            | DomainError::UnsupportedMode(message)
            | DomainError::ConfigurationError(message)
            | DomainError::InternalError(message) => ServiceError::internal(message),
        }
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
