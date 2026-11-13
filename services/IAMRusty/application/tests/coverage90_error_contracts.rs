//! QA coverage90 A: stable command error classification and safe authentication messages.
use iam_application::command::{
    password_login,
    password_reset::PasswordResetErrorMapper,
    provider::{LinkProviderErrorMapper, ProviderErrorMapper},
    registration::RegistrationErrorMapper,
    signup,
    token::TokenErrorMapper,
    user::UserErrorMapper,
    verify_email, CommandError, CommandErrorMapper,
};
use iam_application::usecase::{
    link_provider::LinkProviderError, login::LoginError, password_reset::PasswordResetError,
    provider::ProviderError, registration::RegistrationError, token::TokenError, user::UserError,
};
use iam_domain::error::DomainError;

fn assert_error(error: CommandError, kind: &str, expected_code: &str, expected_message: &str) {
    let (actual_kind, code, message) = match error {
        CommandError::Validation { code, message } => ("validation", code, message),
        CommandError::Authentication { code, message } => ("authentication", code, message),
        CommandError::Business { code, message } => ("business", code, message),
        CommandError::Infrastructure { code, message } => ("infrastructure", code, message),
        other => panic!("unexpected command error: {other:?}"),
    };
    assert_eq!(
        (actual_kind, code.as_str(), message.as_str()),
        (kind, expected_code, expected_message)
    );
}

fn login_cases() -> Vec<(LoginError, &'static str, &'static str, &'static str)> {
    vec![
        (
            LoginError::UserNotFound,
            "business",
            "user_not_found",
            "User not found",
        ),
        (
            LoginError::InvalidCredentials,
            "business",
            "invalid_credentials",
            "Invalid credentials",
        ),
        (
            LoginError::EmailNotVerified,
            "business",
            "email_not_verified",
            "Email not verified",
        ),
        (
            LoginError::UserAlreadyExists,
            "business",
            "user_already_exists",
            "User already exists",
        ),
        (
            LoginError::WeakPassword,
            "validation",
            "weak_password",
            "Password is too weak",
        ),
        (
            LoginError::InvalidEmail,
            "validation",
            "invalid_email",
            "Invalid email format",
        ),
        (
            LoginError::EmailNotFound,
            "business",
            "email_not_found",
            "Invalid verification request",
        ),
        (
            LoginError::EmailAlreadyVerified,
            "business",
            "email_already_verified",
            "Email is already verified",
        ),
        (
            LoginError::InvalidVerificationToken,
            "validation",
            "invalid_verification_token",
            "Invalid or expired verification token",
        ),
        (
            LoginError::VerificationTokenExpired,
            "validation",
            "verification_token_expired",
            "Verification token has expired",
        ),
        (
            LoginError::AuthServiceError("storage unavailable".into()),
            "infrastructure",
            "repository_error",
            "storage unavailable",
        ),
        (
            LoginError::AuthServiceError("invalid signature".into()),
            "validation",
            "authentication_failed",
            "Authentication failed: invalid signature",
        ),
    ]
}

#[test]
fn login_signup_and_verification_preserve_typed_error_contracts() {
    let mappers: [&dyn CommandErrorMapper; 3] = [
        &password_login::AuthErrorMapper,
        &signup::AuthErrorMapper,
        &verify_email::AuthErrorMapper,
    ];
    for mapper in mappers {
        for (error, kind, code, message) in login_cases() {
            assert_error(mapper.map_error(Box::new(error)), kind, code, message);
        }
        assert_error(
            mapper.map_error(Box::new(std::io::Error::other("JWT error"))),
            "validation",
            "authentication_failed",
            "Authentication failed: JWT error",
        );
        assert_error(
            mapper.map_error(Box::new(std::io::Error::other("transport unavailable"))),
            "infrastructure",
            "repository_error",
            "transport unavailable",
        );
    }
}

#[test]
fn password_reset_hides_account_existence_and_distinguishes_token_failures() {
    let cases = [
        (
            PasswordResetError::UserNotFound,
            "business",
            "anti_enumeration_security",
            "Password reset request processed",
        ),
        (
            PasswordResetError::NoPasswordAuth,
            "business",
            "anti_enumeration_security",
            "Password reset request processed",
        ),
        (
            PasswordResetError::InvalidResetToken,
            "validation",
            "invalid_token",
            "Invalid or malformed reset token",
        ),
        (
            PasswordResetError::ResetTokenExpired,
            "validation",
            "token_expired",
            "Reset token has expired",
        ),
        (
            PasswordResetError::ResetTokenAlreadyUsed,
            "validation",
            "token_already_used",
            "Reset token has already been used",
        ),
        (
            PasswordResetError::IncorrectCurrentPassword,
            "validation",
            "incorrect_current_password",
            "Current password is incorrect",
        ),
        (
            PasswordResetError::WeakPassword,
            "validation",
            "validation_failed",
            "Password does not meet security requirements",
        ),
        (
            PasswordResetError::RepositoryError("offline".into()),
            "infrastructure",
            "repository_error",
            "Repository error: offline",
        ),
        (
            PasswordResetError::EventPublishingError("offline".into()),
            "infrastructure",
            "event_publishing_error",
            "Event publishing error: offline",
        ),
        (
            PasswordResetError::ServiceError("offline".into()),
            "infrastructure",
            "token_service_error",
            "Service error: offline",
        ),
    ];
    for (error, kind, code, message) in cases {
        assert_error(
            PasswordResetErrorMapper.map_error(Box::new(error)),
            kind,
            code,
            message,
        );
    }
}

#[test]
fn registration_maps_domain_rejections_without_collapsing_conflicts_into_infrastructure() {
    let cases = [
        (
            DomainError::InvalidToken,
            "validation",
            "invalid_token",
            "Invalid or expired registration token",
        ),
        (
            DomainError::TokenExpired,
            "validation",
            "token_expired",
            "Registration token has expired",
        ),
        (
            DomainError::UsernameTaken,
            "business",
            "username_taken",
            "Username already taken",
        ),
        (
            DomainError::InvalidUsername,
            "validation",
            "invalid_username",
            "Invalid username format",
        ),
        (
            DomainError::UserNotFound,
            "business",
            "user_not_found",
            "User not found",
        ),
        (
            DomainError::RegistrationAlreadyComplete,
            "validation",
            "registration_already_complete",
            "Registration already completed",
        ),
        (
            DomainError::TokenNotFound,
            "infrastructure",
            "repository_error",
            "Registration failed",
        ),
    ];
    for (error, kind, code, message) in cases {
        assert_error(
            RegistrationErrorMapper.map_error(Box::new(RegistrationError::DomainError(error))),
            kind,
            code,
            message,
        );
    }
}

#[test]
fn token_mapper_classifies_expired_invalid_and_missing_sessions_as_authentication() {
    let cases = [
        (
            TokenError::TokenNotFound,
            "token_not_found",
            "Authentication failed: Invalid refresh token",
        ),
        (
            TokenError::TokenInvalid,
            "token_invalid",
            "Authentication failed: Invalid refresh token",
        ),
        (
            TokenError::TokenExpired,
            "token_expired",
            "Authentication failed: Expired refresh token",
        ),
        (
            TokenError::DomainError(DomainError::TokenNotFound),
            "token_not_found",
            "Authentication failed: Token not found",
        ),
        (
            TokenError::DomainError(DomainError::TokenExpired),
            "token_expired",
            "Authentication failed: Token expired",
        ),
        (
            TokenError::DomainError(DomainError::InvalidToken),
            "token_invalid",
            "Authentication failed: Invalid token",
        ),
        (
            TokenError::DomainError(DomainError::TokenValidationFailed(
                "private-key-location".into(),
            )),
            "token_invalid",
            "Authentication failed: Token validation failed",
        ),
    ];
    for (error, code, message) in cases {
        assert_error(
            TokenErrorMapper.map_error(Box::new(error)),
            "authentication",
            code,
            message,
        );
    }
    assert_error(
        TokenErrorMapper.map_error(Box::new(TokenError::DomainError(
            DomainError::RepositoryError("db credentials".into()),
        ))),
        "infrastructure",
        "repository_error",
        "Repository error during token operation",
    );
    assert_error(
        TokenErrorMapper.map_error(Box::new(TokenError::DomainError(DomainError::UserNotFound))),
        "infrastructure",
        "token_service_error",
        "Token service error",
    );
    assert_error(
        TokenErrorMapper.map_error(Box::new(TokenError::TokenServiceError("expired".into()))),
        "business",
        "authentication_failed",
        "Authentication failed: expired",
    );
    assert_error(
        TokenErrorMapper.map_error(Box::new(TokenError::TokenServiceError("offline".into()))),
        "infrastructure",
        "token_service_error",
        "Token service error: offline",
    );
    assert_error(
        TokenErrorMapper.map_error(Box::new(TokenError::RepositoryError("offline".into()))),
        "infrastructure",
        "repository_error",
        "Repository error: offline",
    );
}

#[test]
fn user_mapper_redacts_domain_causes_and_preserves_authentication_classification() {
    let cases = [
        (
            UserError::UserNotFound,
            "user_not_found",
            "Authentication failed",
        ),
        (
            UserError::InvalidToken,
            "invalid_token",
            "Authentication failed",
        ),
        (
            UserError::TokenExpired,
            "token_expired",
            "Authentication failed",
        ),
        (
            UserError::DomainError(DomainError::UserNotFound),
            "user_not_found",
            "Authentication failed: User not found",
        ),
        (
            UserError::DomainError(DomainError::InvalidToken),
            "invalid_token",
            "Authentication failed: Invalid token",
        ),
        (
            UserError::DomainError(DomainError::TokenExpired),
            "token_expired",
            "Authentication failed: Token expired",
        ),
        (
            UserError::DomainError(DomainError::TokenValidationFailed("secret-cause".into())),
            "invalid_token",
            "Authentication failed: Token validation failed",
        ),
    ];
    for (error, code, message) in cases {
        assert_error(
            UserErrorMapper.map_error(Box::new(error)),
            "authentication",
            code,
            message,
        );
    }
    assert_error(
        UserErrorMapper.map_error(Box::new(UserError::DomainError(
            DomainError::RepositoryError("secret-cause".into()),
        ))),
        "infrastructure",
        "repository_error",
        "Repository error during user operation",
    );
    assert_error(
        UserErrorMapper.map_error(Box::new(UserError::DomainError(DomainError::TokenNotFound))),
        "infrastructure",
        "token_service_error",
        "User service error",
    );
    assert_error(
        UserErrorMapper.map_error(Box::new(std::io::Error::other("secret-cause"))),
        "authentication",
        "authentication_failed",
        "Authentication failed",
    );
    assert_error(
        UserErrorMapper.map_error(Box::new(UserError::TokenServiceError("malformed".into()))),
        "validation",
        "authentication_failed",
        "Authentication failed: malformed",
    );
    assert_error(
        UserErrorMapper.map_error(Box::new(UserError::TokenServiceError("offline".into()))),
        "infrastructure",
        "token_service_error",
        "Token service error: offline",
    );
    assert_error(
        UserErrorMapper.map_error(Box::new(UserError::RepositoryError("offline".into()))),
        "infrastructure",
        "repository_error",
        "Repository error: offline",
    );
}

#[test]
fn provider_token_mapper_distinguishes_missing_link_unsupported_provider_and_authentication() {
    let cases = [
        (
            ProviderError::UserNotFound,
            "authentication",
            "user_not_found",
            "Authentication failed",
        ),
        (
            ProviderError::ProviderNotSupported("github".into()),
            "validation",
            "provider_not_supported",
            "Unsupported provider: github",
        ),
        (
            ProviderError::NoTokenForProvider,
            "business",
            "no_token_for_provider",
            "No token available for the user and provider",
        ),
        (
            ProviderError::AuthError("consent required".into()),
            "authentication",
            "authentication_failed",
            "consent required",
        ),
    ];
    for (error, kind, code, message) in cases {
        assert_error(
            ProviderErrorMapper.map_error(Box::new(error)),
            kind,
            code,
            message,
        );
    }
    assert_error(
        ProviderErrorMapper.map_error(Box::new(ProviderError::DbError(Box::new(
            std::io::Error::other("offline"),
        )))),
        "infrastructure",
        "database_error",
        "Database error: offline",
    );
}

#[test]
fn linking_distinguishes_same_user_conflict_other_user_conflict_and_configuration_failure() {
    let cases = [
        (
            LinkProviderError::AuthError("provider secret".into()),
            "authentication",
            "authentication_failed",
            "Authentication failed",
        ),
        (
            LinkProviderError::DomainError(DomainError::UserNotFound),
            "authentication",
            "user_not_found",
            "Authentication failed",
        ),
        (
            LinkProviderError::DomainError(DomainError::BusinessRuleViolation(
                "already associated with another user".into(),
            )),
            "business",
            "provider_already_linked",
            "already associated with another user",
        ),
        (
            LinkProviderError::DomainError(DomainError::BusinessRuleViolation(
                "already linked to your account".into(),
            )),
            "business",
            "provider_already_linked_same_user",
            "already linked to your account",
        ),
        (
            LinkProviderError::DomainError(DomainError::BusinessRuleViolation(
                "link limit reached".into(),
            )),
            "business",
            "business_rule_violation",
            "link limit reached",
        ),
        (
            LinkProviderError::DomainError(DomainError::RepositoryError("offline".into())),
            "infrastructure",
            "repository_error",
            "Database error: offline",
        ),
        (
            LinkProviderError::ProviderNotConfigured("github".into()),
            "infrastructure",
            "provider_not_configured",
            "Provider github not configured",
        ),
    ];
    for (error, kind, code, message) in cases {
        assert_error(
            LinkProviderErrorMapper.map_error(Box::new(error)),
            kind,
            code,
            message,
        );
    }
}

#[test]
fn untyped_token_and_reset_errors_retain_authentication_validation_and_transport_categories() {
    // T-1: characterization_keywords mirrors the current untyped-error heuristics,
    // not an independent authentication specification. Typed-error contracts are tested above.
    for cause in [
        "expired",
        "invalid",
        "Token expired",
        "Invalid token",
        "JWT error",
        "malformed",
        "signature",
    ] {
        assert_error(
            TokenErrorMapper.map_error(Box::new(std::io::Error::other(cause))),
            "validation",
            "authentication_failed",
            &format!("Authentication failed: {cause}"),
        );
        assert_error(
            PasswordResetErrorMapper.map_error(Box::new(std::io::Error::other(cause))),
            "authentication",
            "authentication_failed",
            &format!("Authentication failed: {cause}"),
        );
    }
    for cause in ["validation", "format", "required", "missing", "empty"] {
        assert_error(
            PasswordResetErrorMapper.map_error(Box::new(std::io::Error::other(cause))),
            "validation",
            "validation_failed",
            &format!("Validation failed: {cause}"),
        );
    }
    for cause in ["authentication", "unauthorized"] {
        assert_error(
            PasswordResetErrorMapper.map_error(Box::new(std::io::Error::other(cause))),
            "authentication",
            "authentication_failed",
            &format!("Authentication failed: {cause}"),
        );
    }
    let mappers: [(&dyn CommandErrorMapper, &str); 5] = [
        (&TokenErrorMapper, "repository_error"),
        (&PasswordResetErrorMapper, "repository_error"),
        (&RegistrationErrorMapper, "repository_error"),
        (&LinkProviderErrorMapper, "repository_error"),
        (&ProviderErrorMapper, "database_error"),
    ];
    for (mapper, code) in mappers {
        assert_error(
            mapper.map_error(Box::new(std::io::Error::other("transport unavailable"))),
            "infrastructure",
            code,
            "transport unavailable",
        );
    }
}

#[test]
fn registration_infrastructure_keeps_repository_token_and_event_failure_codes() {
    for (cause, code, message) in [
        (
            DomainError::RepositoryError("offline".into()),
            "repository_error",
            "Repository error: offline",
        ),
        (
            DomainError::TokenServiceError("offline".into()),
            "token_service_error",
            "Token service error: offline",
        ),
        (
            DomainError::EventError("offline".into()),
            "event_error",
            "Event publishing error: offline",
        ),
    ] {
        assert_error(
            RegistrationErrorMapper.map_error(Box::new(RegistrationError::DomainError(cause))),
            "infrastructure",
            code,
            message,
        );
    }
}
