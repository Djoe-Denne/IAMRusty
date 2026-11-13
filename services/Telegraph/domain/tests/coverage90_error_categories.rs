//! QA coverage90 C: poison messages must not become retryable infrastructure failures.
use rustycog::core::error::ServiceError;
use telegraph_domain::DomainError;

fn nonrecoverable_cases(detail: &str) -> [(DomainError, &'static str, u16, bool); 13] {
    [
        (
            DomainError::invalid_message(detail),
            "validation",
            400,
            false,
        ),
        (
            DomainError::invalid_recipient(detail),
            "validation",
            400,
            false,
        ),
        (DomainError::invalid_email(detail), "validation", 400, false),
        (
            DomainError::invalid_phone_number(detail),
            "validation",
            400,
            false,
        ),
        (
            DomainError::template_not_found(detail),
            "not_found",
            404,
            false,
        ),
        (
            DomainError::notification_not_found(detail),
            "not_found",
            404,
            false,
        ),
        (
            DomainError::unauthorized(detail),
            "authorization",
            403,
            false,
        ),
        (
            DomainError::template_render_error(detail),
            "business",
            422,
            false,
        ),
        (
            DomainError::operation_not_supported(detail),
            "business",
            422,
            false,
        ),
        (
            DomainError::unsupported_mode(detail),
            "business",
            422,
            false,
        ),
        (
            DomainError::configuration_error(detail),
            "business",
            422,
            false,
        ),
        (
            DomainError::event_processing_error(detail),
            "business",
            422,
            false,
        ),
        (DomainError::internal_error(detail), "internal", 500, false),
    ]
}

fn recoverable_cases(detail: &str) -> [(DomainError, &'static str, u16, bool); 5] {
    [
        (
            DomainError::template_load_error(detail),
            "infrastructure",
            500,
            true,
        ),
        (
            DomainError::delivery_failed(detail),
            "infrastructure",
            500,
            true,
        ),
        (
            DomainError::infrastructure_error(detail),
            "infrastructure",
            500,
            true,
        ),
        (
            DomainError::rate_limit_exceeded(detail),
            "rate_limit",
            429,
            true,
        ),
        (
            DomainError::service_unavailable(detail),
            "service_unavailable",
            503,
            true,
        ),
    ]
}

#[test]
fn domain_failures_preserve_category_status_and_retry_policy_for_both_conversions() {
    let detail = "notification-42: échec";
    let cases = nonrecoverable_cases(detail)
        .into_iter()
        .chain(recoverable_cases(detail));
    for (domain, category, status, retryable) in cases {
        assert_eq!(domain.is_recoverable(), retryable, "{domain}");
        assert_eq!(
            domain.is_validation_error(),
            category == "validation",
            "{domain}"
        );
        assert_eq!(
            domain.is_configuration_error(),
            matches!(domain, DomainError::ConfigurationError(_)),
            "{domain}"
        );
        for converted in [
            ServiceError::from(&domain),
            ServiceError::from(domain.clone()),
        ] {
            assert_eq!(converted.category(), category, "{domain}");
            assert_eq!(converted.http_status_code(), status, "{domain}");
            assert_eq!(converted.is_retryable(), retryable, "{domain}");
            assert!(
                converted.to_string().ends_with(detail),
                "lost diagnostic: {converted}"
            );
        }
    }
}
