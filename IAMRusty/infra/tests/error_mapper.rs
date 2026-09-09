use iam_domain::error::DomainError;
use iam_infra::event_adapter::IAMErrorMapper;
use rustycog::core::error::ServiceError;
use rustycog::events::adapter::ErrorMapper;

#[test]
fn iam_error_mapper_round_trips_domain_and_service_errors() {
    let mapper = IAMErrorMapper;
    let domain_errors = [
        DomainError::UserNotFound,
        DomainError::ProviderNotSupported("github".into()),
        DomainError::BusinessRuleViolation("rule".into()),
        DomainError::InvalidToken,
        DomainError::TokenExpired,
        DomainError::AuthorizationError("no".into()),
        DomainError::OAuth2Error("oauth".into()),
        DomainError::UserProfileError("profile".into()),
        DomainError::NoTokenForProvider,
        DomainError::TokenGenerationFailed("x".into()),
        DomainError::TokenValidationFailed("x".into()),
        DomainError::RepositoryError("x".into()),
        DomainError::UsernameTaken,
        DomainError::InvalidUsername,
        DomainError::RegistrationAlreadyComplete,
        DomainError::TokenServiceError("x".into()),
        DomainError::EventError("x".into()),
        DomainError::TokenNotFound,
    ];
    for error in domain_errors {
        let _ = mapper.to_service_error(error);
    }

    let service_errors = [
        ServiceError::authentication("x"),
        ServiceError::authorization("x"),
        ServiceError::not_found("missing"),
        ServiceError::infrastructure("down"),
        ServiceError::validation("bad"),
        ServiceError::business("rule"),
        ServiceError::Timeout {
            message: "late".into(),
            operation: None,
        },
        ServiceError::ServiceUnavailable {
            message: "off".into(),
            retry_after: None,
        },
        ServiceError::internal("boom"),
        ServiceError::conflict("dup"),
        ServiceError::RateLimit {
            message: "slow".into(),
            retry_after: None,
        },
    ];
    for error in service_errors {
        let _ = mapper.from_service_error(error);
    }
}
