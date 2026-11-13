//! QA coverage90 C: adapter error categories and reverse mapping remain closed.
use rustycog::{core::error::ServiceError, events::ErrorMapper};
use telegraph_domain::DomainError;
use telegraph_infra::event::TelegraphErrorMapper;

#[test]
fn publish_errors_keep_typed_status_and_message() {
    let detail = "welcome-email: unavailable";
    // DomainError documents which failures should retry; publishing must not
    // turn poison events into retries or suppress retryable template loads.
    // Static discrepancy in the current adapter is deliberately not blessed.
    let cases = [
        (
            DomainError::InvalidMessage(detail.into()),
            "validation",
            400,
        ),
        (
            DomainError::InvalidRecipient(detail.into()),
            "validation",
            400,
        ),
        (DomainError::InvalidEmail(detail.into()), "validation", 400),
        (
            DomainError::InvalidPhoneNumber(detail.into()),
            "validation",
            400,
        ),
        (
            DomainError::TemplateNotFound(detail.into()),
            "not_found",
            404,
        ),
        (
            DomainError::NotificationNotFound(detail.into()),
            "not_found",
            404,
        ),
        (
            DomainError::Unauthorized(detail.into()),
            "authorization",
            403,
        ),
        (
            DomainError::DeliveryFailed(detail.into()),
            "infrastructure",
            500,
        ),
        (
            DomainError::InfrastructureError(detail.into()),
            "infrastructure",
            500,
        ),
        (
            DomainError::EventProcessingError(detail.into()),
            "business",
            422,
        ),
        (
            DomainError::ServiceUnavailable(detail.into()),
            "service_unavailable",
            503,
        ),
        (
            DomainError::RateLimitExceeded(detail.into()),
            "rate_limit",
            429,
        ),
        (
            DomainError::TemplateLoadError(detail.into()),
            "infrastructure",
            500,
        ),
        (
            DomainError::TemplateRenderError(detail.into()),
            "business",
            422,
        ),
        (
            DomainError::OperationNotSupported(detail.into()),
            "business",
            422,
        ),
        (DomainError::UnsupportedMode(detail.into()), "business", 422),
        (
            DomainError::ConfigurationError(detail.into()),
            "business",
            422,
        ),
        (DomainError::InternalError(detail.into()), "internal", 500),
    ];
    for (error, category, status) in cases {
        let mapped = TelegraphErrorMapper.to_service_error(error);
        assert_eq!(mapped.category(), category);
        assert_eq!(mapped.http_status_code(), status);
        assert_eq!(
            mapped.is_retryable(),
            matches!(
                category,
                "infrastructure" | "rate_limit" | "service_unavailable"
            )
        );
        assert!(mapped.to_string().ends_with(detail), "{mapped}");
    }
}

#[test]
fn service_failures_restore_domain_meaning_without_losing_diagnostics() {
    let message = "notification-42";
    let cases = [
        (
            ServiceError::validation(message),
            DomainError::InvalidMessage(message.into()),
        ),
        (
            ServiceError::not_found(message),
            DomainError::NotificationNotFound(message.into()),
        ),
        (
            ServiceError::authorization(message),
            DomainError::Unauthorized(message.into()),
        ),
        (
            ServiceError::authentication(message),
            DomainError::Unauthorized(message.into()),
        ),
        (
            ServiceError::infrastructure(message),
            DomainError::InfrastructureError(message.into()),
        ),
        (
            ServiceError::ServiceUnavailable {
                message: message.into(),
                retry_after: Some(12),
            },
            DomainError::ServiceUnavailable(message.into()),
        ),
        (
            ServiceError::RateLimit {
                message: message.into(),
                retry_after: Some(12),
            },
            DomainError::RateLimitExceeded(message.into()),
        ),
        (
            ServiceError::business(message),
            DomainError::InternalError(message.into()),
        ),
        (
            ServiceError::Conflict {
                message: message.into(),
                resource_type: None,
            },
            DomainError::InternalError(message.into()),
        ),
        (
            ServiceError::Timeout {
                message: message.into(),
                operation: Some("publish".into()),
            },
            DomainError::InternalError(message.into()),
        ),
        (
            ServiceError::internal(message),
            DomainError::InternalError(message.into()),
        ),
    ];
    for (service, expected) in cases {
        assert_eq!(TelegraphErrorMapper.from_service_error(service), expected);
    }
}
