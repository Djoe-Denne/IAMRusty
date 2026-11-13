//! QA coverage90 C: queue-facing categories must not retry poison commands.
use rustycog::command::CommandError;
use telegraph_application::{
    command_error_from_boxed, command_error_from_usecase, service_error_from_command_error,
    DomainError, NotificationUseCaseError,
};

#[test]
fn business_error_aliases_restore_status_without_becoming_retryable() {
    for (code, category, status) in [
        ("unauthorized", "authorization", 403),
        ("not_found", "not_found", 404),
        ("template_not_found", "not_found", 404),
        ("notification_not_found", "not_found", 404),
        ("internal_error", "internal", 500),
        ("internal", "internal", 500),
        ("conflict", "conflict", 409),
        ("other_business_rule", "business", 422),
    ] {
        let error =
            service_error_from_command_error(&CommandError::business(code, "notification-42"));
        assert_eq!(error.category(), category, "{code}");
        assert_eq!(error.http_status_code(), status, "{code}");
        assert!(
            !error.is_retryable(),
            "poison command must not retry: {code}"
        );
        assert!(error.to_string().ends_with("notification-42"));
    }
}

#[test]
fn authentication_timeout_and_unclassified_infrastructure_keep_retry_policy() {
    for (command, category, status, retryable) in [
        (
            CommandError::authentication("expired", "notification-42"),
            "authentication",
            401,
            false,
        ),
        (
            CommandError::timeout("publish", "notification-42"),
            "timeout",
            504,
            true,
        ),
        (
            CommandError::infrastructure("smtp", "notification-42"),
            "infrastructure",
            500,
            true,
        ),
    ] {
        let error = service_error_from_command_error(&command);
        assert_eq!(error.category(), category);
        assert_eq!(error.http_status_code(), status);
        assert_eq!(error.is_retryable(), retryable);
        assert!(error.to_string().ends_with("notification-42"));
    }
}

#[test]
fn boxed_domain_error_is_not_misclassified_as_unknown_infrastructure() {
    let error = command_error_from_boxed(Box::new(DomainError::InvalidEmail(
        "invalid address".into(),
    )));
    assert!(
        matches!(error, CommandError::Validation { ref code, ref message } if code == "validation" && message == "invalid address")
    );
    let unknown =
        command_error_from_boxed(Box::new(std::io::Error::other("smtp transport closed")));
    assert!(
        matches!(unknown, CommandError::Infrastructure { ref code, ref message } if code == "unknown_error" && message == "smtp transport closed")
    );
}

#[test]
fn usecase_validation_and_domain_denial_keep_distinct_nonretryable_categories() {
    for (usecase, category, status) in [
        (
            NotificationUseCaseError::ValidationError("empty title".into()),
            "validation",
            400,
        ),
        (
            NotificationUseCaseError::Domain(DomainError::Unauthorized("owner mismatch".into())),
            "authorization",
            403,
        ),
    ] {
        let error = service_error_from_command_error(&command_error_from_usecase(&usecase));
        assert_eq!(error.category(), category);
        assert_eq!(error.http_status_code(), status);
        assert!(!error.is_retryable());
    }
}
