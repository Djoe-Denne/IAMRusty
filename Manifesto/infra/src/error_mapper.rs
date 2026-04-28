use manifesto_domain::DomainError;
use rustycog_core::error::ServiceError;
use rustycog_events::ErrorMapper;

/// Error mapper for Manifesto domain errors
pub struct ManifestoErrorMapper;

impl ErrorMapper<DomainError> for ManifestoErrorMapper {
    fn to_service_error(&self, error: DomainError) -> ServiceError {
        match error {
            DomainError::EntityNotFound { entity_type, id } => {
                ServiceError::not_found_resource("", entity_type, id)
            }
            DomainError::InvalidInput { message } => ServiceError::validation(message),
            DomainError::BusinessRuleViolation { rule } => ServiceError::business(rule),
            DomainError::Unauthorized { operation } => ServiceError::authorization(operation),
            DomainError::ResourceAlreadyExists {
                resource_type,
                identifier: _,
            } => ServiceError::conflict(resource_type),
            DomainError::ExternalServiceError {
                service: _,
                message,
            } => ServiceError::infrastructure(message),
            DomainError::PermissionDenied { message } => ServiceError::authorization(message),
            DomainError::Internal { message } => ServiceError::internal(message),
        }
    }

    fn from_service_error(&self, error: ServiceError) -> DomainError {
        match error {
            ServiceError::Validation { message, .. } => DomainError::invalid_input(&message),
            ServiceError::NotFound { message, .. } => {
                DomainError::entity_not_found("Unknown", &message)
            }
            ServiceError::Authentication { message, .. } => DomainError::unauthorized(&message),
            ServiceError::Authorization { message, .. } => DomainError::unauthorized(&message),
            ServiceError::Conflict { message, .. } => {
                DomainError::resource_already_exists(&message, "")
            }
            ServiceError::Business { message, .. } => {
                DomainError::business_rule_violation(&message)
            }
            ServiceError::Infrastructure { message, .. } => DomainError::internal_error(&message),
            ServiceError::RateLimit { message, .. } => DomainError::internal_error(&message),
            ServiceError::ServiceUnavailable { message, .. } => {
                DomainError::internal_error(&message)
            }
            ServiceError::Timeout { message, .. } => DomainError::internal_error(&message),
            ServiceError::Internal { message, .. } => DomainError::internal_error(&message),
        }
    }
}
