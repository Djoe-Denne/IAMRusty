//! Command / `ServiceError` mapping aligned on rustycog-core categories.
//!
//! rustycog-command retries only `CommandError::Infrastructure` and `Timeout`.
//! rustycog-events treats any `Err` from `handle_event` as a nack (retry).
//! Categories must therefore stay intact so `ServiceError::is_retryable()`
//! can ack poison (validation / business / not-found) and retry infra only.

use rustycog::command::CommandError;
use rustycog::core::error::ServiceError;
use telegraph_domain::DomainError;

use crate::usecase::NotificationUseCaseError;

/// Map a domain failure to a typed `CommandError` (preserves retryability).
#[must_use]
pub fn command_error_from_domain(error: &DomainError) -> CommandError {
    command_error_from_service_error(&ServiceError::from(error))
}

/// Map a notification use-case failure to a typed `CommandError`.
#[must_use]
pub fn command_error_from_usecase(error: &NotificationUseCaseError) -> CommandError {
    match error {
        NotificationUseCaseError::ValidationError(message) => {
            CommandError::validation("validation", message.clone())
        }
        NotificationUseCaseError::Domain(domain_error) => command_error_from_domain(domain_error),
    }
}

/// Restore rustycog `ServiceError` categories from a command-layer error.
#[must_use]
pub fn service_error_from_command_error(error: &CommandError) -> ServiceError {
    match error {
        CommandError::Validation { message, .. } => ServiceError::validation(message.clone()),
        CommandError::Authentication { message, .. } => {
            ServiceError::authentication(message.clone())
        }
        CommandError::Business { code, message } => match code.as_str() {
            "unauthorized" => ServiceError::authorization(message.clone()),
            "not_found" | "template_not_found" | "notification_not_found" => {
                ServiceError::not_found(message.clone())
            }
            "internal_error" | "internal" => ServiceError::internal(message.clone()),
            "conflict" => ServiceError::conflict(message.clone()),
            _ => ServiceError::business(message.clone()),
        },
        CommandError::Infrastructure { code, message } => match code.as_str() {
            "rate_limit" => ServiceError::RateLimit {
                message: message.clone(),
                retry_after: None,
            },
            "service_unavailable" => ServiceError::ServiceUnavailable {
                message: message.clone(),
                retry_after: None,
            },
            _ => ServiceError::infrastructure(message.clone()),
        },
        CommandError::Timeout { message, .. } => ServiceError::Timeout {
            message: message.clone(),
            operation: None,
        },
        CommandError::RetryExhausted { message, .. } => ServiceError::internal(message.clone()),
    }
}

/// Map a boxed registry error, preferring `DomainError` when present.
#[must_use]
pub fn command_error_from_boxed(error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
    match error.downcast::<DomainError>() {
        Ok(domain_error) => command_error_from_domain(&domain_error),
        Err(error) => CommandError::infrastructure("unknown_error", error.to_string()),
    }
}

fn command_error_from_service_error(error: &ServiceError) -> CommandError {
    match error {
        ServiceError::Validation {
            message,
            field,
            code,
        } => CommandError::validation(
            code.clone()
                .or_else(|| field.clone())
                .unwrap_or_else(|| "validation".to_string()),
            message.clone(),
        ),
        ServiceError::Authentication { message, code } => CommandError::authentication(
            code.clone().unwrap_or_else(|| "authentication".to_string()),
            message.clone(),
        ),
        ServiceError::Authorization { message, .. } => {
            CommandError::business("unauthorized", message.clone())
        }
        ServiceError::NotFound { message, .. } => {
            CommandError::business("not_found", message.clone())
        }
        ServiceError::Conflict { message, .. } => {
            CommandError::business("conflict", message.clone())
        }
        ServiceError::Business { message, code } => CommandError::business(
            code.clone().unwrap_or_else(|| "business".to_string()),
            message.clone(),
        ),
        ServiceError::RateLimit { message, .. } => {
            CommandError::infrastructure("rate_limit", message.clone())
        }
        ServiceError::ServiceUnavailable { message, .. } => {
            CommandError::infrastructure("service_unavailable", message.clone())
        }
        ServiceError::Timeout { message, .. } => CommandError::timeout("timeout", message.clone()),
        ServiceError::Infrastructure {
            message,
            error_source,
        } => CommandError::infrastructure(
            error_source
                .clone()
                .unwrap_or_else(|| "infrastructure".to_string()),
            message.clone(),
        ),
        ServiceError::Internal { message, .. } => {
            CommandError::business("internal_error", message.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_round_trip(domain: &DomainError, retryable: bool, category: &str, status: u16) {
        let direct = ServiceError::from(domain);
        assert_eq!(direct.is_retryable(), retryable, "{domain}");
        assert_eq!(direct.category(), category, "{domain}");
        assert_eq!(direct.http_status_code(), status, "{domain}");
        assert_eq!(
            domain.is_recoverable(),
            retryable,
            "is_recoverable must match ServiceError::is_retryable for {domain}"
        );

        let via_command = service_error_from_command_error(&command_error_from_domain(domain));
        assert_eq!(via_command.is_retryable(), retryable, "{domain}");
        assert_eq!(via_command.category(), category, "{domain}");
        assert_eq!(via_command.http_status_code(), status, "{domain}");
    }

    #[test]
    fn domain_errors_keep_rustycog_categories_through_command_layer() {
        assert_round_trip(
            &DomainError::invalid_message("bad"),
            false,
            "validation",
            400,
        );
        assert_round_trip(
            &DomainError::notification_not_found("missing"),
            false,
            "not_found",
            404,
        );
        assert_round_trip(
            &DomainError::unauthorized("nope"),
            false,
            "authorization",
            403,
        );
        assert_round_trip(
            &DomainError::event_processing_error("unknown processor"),
            false,
            "business",
            422,
        );
        assert_round_trip(
            &DomainError::infrastructure_error("db down"),
            true,
            "infrastructure",
            500,
        );
        assert_round_trip(
            &DomainError::service_unavailable("smtp"),
            true,
            "service_unavailable",
            503,
        );
        assert_round_trip(
            &DomainError::rate_limit_exceeded("user-1"),
            true,
            "rate_limit",
            429,
        );
        assert_round_trip(&DomainError::internal_error("bug"), false, "internal", 500);
    }

    #[test]
    fn retry_exhausted_is_not_retryable_at_the_queue() {
        let error = CommandError::retry_exhausted("retries", "gave up");
        let service_error = service_error_from_command_error(&error);
        assert!(!service_error.is_retryable());
        assert_eq!(service_error.category(), "internal");
    }
}
