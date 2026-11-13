//! QA coverage90 A: rejected inputs, accepted boundaries, secret-safe diagnostics.
use iam_application::command::{
    password_login::PasswordLoginCommand,
    password_reset::{
        RequestPasswordResetCommand, ResetPasswordAuthenticatedCommand,
        ResetPasswordUnauthenticatedCommand, ValidateResetTokenCommand,
    },
    registration::{CheckUsernameCommand, CompleteRegistrationCommand},
    resend_verification_email::ResendVerificationEmailCommand,
    signup::SignupCommand,
    token::{RefreshTokenCommand, RevokeTokenCommand},
    verify_email::VerifyEmailCommand,
    Command, CommandError,
};
use uuid::Uuid;

fn rejected(result: Result<(), CommandError>, expected_code: &str, expected_message: &str) {
    match result {
        Err(CommandError::Validation { code, message }) => {
            assert_eq!(
                (code.as_str(), message.as_str()),
                (expected_code, expected_message)
            );
        }
        other => panic!("expected validation {expected_code}: {expected_message}, got {other:?}"),
    }
}

#[test]
fn login_rejects_empty_credentials_and_malformed_email() {
    for email in ["", " ", "\t\r\n"] {
        rejected(
            PasswordLoginCommand::new(email.into(), "not-empty".into()).validate(),
            "validation_failed",
            "Email cannot be empty",
        );
    }
    for password in ["", " ", "\t\r\n"] {
        rejected(
            PasswordLoginCommand::new("reader@aiforall.test".into(), password.into()).validate(),
            "validation_failed",
            "Password cannot be empty",
        );
    }
    for email in ["reader.aiforall.test", "reader@localhost"] {
        rejected(
            PasswordLoginCommand::new(email.into(), "not-empty".into()).validate(),
            "invalid_email",
            "Invalid email format",
        );
    }
    assert!(
        PasswordLoginCommand::new("reader@aiforall.test".into(), "not-empty".into())
            .validate()
            .is_ok()
    );
}

#[test]
fn signup_requires_credentials_but_leaves_password_strength_to_domain() {
    rejected(
        SignupCommand::new(" \t".into(), "short".into()).validate(),
        "validation_failed",
        "Email cannot be empty",
    );
    rejected(
        SignupCommand::new("reader@aiforall.test".into(), String::new()).validate(),
        "validation_failed",
        "Password cannot be empty",
    );
    // Command sanity validation is not the HTTP format/domain strength validator.
    assert!(
        SignupCommand::new("reader@aiforall.test".into(), "short".into())
            .validate()
            .is_ok()
    );
}

#[test]
fn verification_requires_email_and_token_and_rejects_bad_email_format() {
    for (email, token, code, message) in [
        (
            " \t",
            "opaque",
            "validation_failed",
            "Email cannot be empty",
        ),
        (
            "reader@aiforall.test",
            "\n ",
            "validation_failed",
            "Verification token cannot be empty",
        ),
        (
            "reader.aiforall.test",
            "opaque",
            "invalid_email",
            "Invalid email format",
        ),
        (
            "reader@localhost",
            "opaque",
            "invalid_email",
            "Invalid email format",
        ),
    ] {
        rejected(
            VerifyEmailCommand::new(email.into(), token.into()).validate(),
            code,
            message,
        );
    }
    assert!(
        VerifyEmailCommand::new("reader@aiforall.test".into(), "opaque".into())
            .validate()
            .is_ok()
    );
}

#[test]
fn resend_and_reset_request_reject_empty_and_malformed_addresses() {
    for (email, message) in [
        (" \n", "Email cannot be empty"),
        ("reader.aiforall.test", "Invalid email format"),
        ("reader@localhost", "Invalid email format"),
    ] {
        rejected(
            RequestPasswordResetCommand::new(email.into()).validate(),
            "invalid_email",
            message,
        );
    }
    // Characterization: resend currently publishes PascalCase, unlike neighboring commands.
    // Preserve the exact public code here; harmonizing it requires a separate contract decision.
    rejected(
        ResendVerificationEmailCommand::new(" \n".into()).validate(),
        "ValidationFailed",
        "Email cannot be empty",
    );
    assert!(
        RequestPasswordResetCommand::new("reader@aiforall.test".into())
            .validate()
            .is_ok()
    );
    assert!(
        ResendVerificationEmailCommand::new("reader@aiforall.test".into())
            .validate()
            .is_ok()
    );
}

#[test]
fn refresh_revoke_and_reset_validation_reject_blank_tokens_without_parsing_opaque_values() {
    for token in ["", " ", "\n\t"] {
        rejected(
            RefreshTokenCommand::new(token.into()).validate(),
            "validation_failed",
            "Refresh token cannot be empty",
        );
        rejected(
            RevokeTokenCommand::new(token.into()).validate(),
            "validation_failed",
            "Refresh token cannot be empty",
        );
        rejected(
            ValidateResetTokenCommand::new(token.into()).validate(),
            "invalid_token",
            "Reset token cannot be empty",
        );
    }
    assert!(RefreshTokenCommand::new("opaque-not-a-jwt".into())
        .validate()
        .is_ok());
    assert!(RevokeTokenCommand::new("opaque-not-a-jwt".into())
        .validate()
        .is_ok());
    assert!(ValidateResetTokenCommand::new("opaque-not-a-jwt".into())
        .validate()
        .is_ok());
}

#[test]
fn password_reset_validates_token_and_password_before_domain_execution() {
    for (token, password, code, message) in [
        (
            " \t",
            "new-pass",
            "invalid_token",
            "Reset token cannot be empty",
        ),
        (
            "opaque",
            " \t",
            "invalid_password",
            "New password cannot be empty",
        ),
        (
            "opaque",
            "1234567",
            "password_too_short",
            "Password must be at least 8 characters",
        ),
    ] {
        rejected(
            ResetPasswordUnauthenticatedCommand::new(token.into(), password.into()).validate(),
            code,
            message,
        );
    }
    assert!(
        ResetPasswordUnauthenticatedCommand::new("opaque".into(), "12345678".into())
            .validate()
            .is_ok()
    );
    let user_id = Uuid::from_u128(42);
    rejected(
        ResetPasswordAuthenticatedCommand::new(user_id, "old-pass".into(), "1234567".into())
            .validate(),
        "weak_password",
        "Password must be at least 8 characters",
    );
    assert!(
        ResetPasswordAuthenticatedCommand::new(user_id, "old-pass".into(), "12345678".into())
            .validate()
            .is_ok()
    );
}

#[test]
fn complete_registration_enforces_length_charset_and_at_least_one_letter() {
    for (username, message) in [
        (" \t".to_string(), "Username is required"),
        ("ab".into(), "Username must be between 3 and 50 characters"),
        (
            "a".repeat(51),
            "Username must be between 3 and 50 characters",
        ),
        (
            "reader.name".into(),
            "Username can only contain letters, numbers, underscores, and hyphens",
        ),
        (
            "reader name".into(),
            "Username can only contain letters, numbers, underscores, and hyphens",
        ),
        ("123_-".into(), "Username must contain at least one letter"),
    ] {
        rejected(
            CompleteRegistrationCommand::new("registration-ticket".into(), username).validate(),
            "validation_failed",
            message,
        );
    }
    rejected(
        CompleteRegistrationCommand::new(" \t".into(), "reader".into()).validate(),
        "validation_failed",
        "Registration token is required",
    );
    for username in ["abc".to_string(), "a".repeat(50), "reader_01-team".into()] {
        assert!(
            CompleteRegistrationCommand::new("registration-ticket".into(), username.clone())
                .validate()
                .is_ok(),
            "username {username}"
        );
    }
}

#[test]
fn username_availability_allows_partial_input_but_rejects_empty_and_overlong_queries() {
    rejected(
        CheckUsernameCommand::new(" \t".into()).validate(),
        "validation_failed",
        "Username is required",
    );
    rejected(
        CheckUsernameCommand::new("a".repeat(51)).validate(),
        "validation_failed",
        "Username cannot be longer than 50 characters",
    );
    for username in ["a".to_string(), "ab".into(), "a".repeat(50)] {
        assert!(
            CheckUsernameCommand::new(username.clone())
                .validate()
                .is_ok(),
            "query {username}"
        );
    }
}

#[test]
fn secret_bearing_command_debug_never_exposes_credentials_or_personal_data() {
    let email = "private-reader@aiforall.test";
    let secret = "sentinel-credential-do-not-log";
    let current = "sentinel-current-password";
    let debug = [
        format!("{:?}", SignupCommand::new(email.into(), secret.into())),
        format!(
            "{:?}",
            PasswordLoginCommand::new(email.into(), secret.into())
        ),
        format!("{:?}", VerifyEmailCommand::new(email.into(), secret.into())),
        format!("{:?}", RefreshTokenCommand::new(secret.into())),
        format!("{:?}", RevokeTokenCommand::new(secret.into())),
        format!(
            "{:?}",
            CompleteRegistrationCommand::new(secret.into(), "private-username".into())
        ),
        format!(
            "{:?}",
            ResetPasswordUnauthenticatedCommand::new(secret.into(), current.into())
        ),
        format!(
            "{:?}",
            ResetPasswordAuthenticatedCommand::new(
                Uuid::from_u128(42),
                current.into(),
                secret.into()
            )
        ),
    ];
    for diagnostic in debug {
        assert!(
            diagnostic.contains("command_id"),
            "diagnostic must retain correlation: {diagnostic}"
        );
        for sensitive in [email, secret, current, "private-username"] {
            assert!(
                !diagnostic.contains(sensitive),
                "sensitive data leaked in {diagnostic}"
            );
        }
    }
}
