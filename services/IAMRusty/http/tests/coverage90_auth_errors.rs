//! QA coverage90 A: public error envelopes, status and credential redaction.
use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
use iam_application::command::CommandError;
use iam_http_server::AuthError;
use serde_json::{json, Value};

const PRIVATE_CAUSE: &str = "sentinel-private-provider-credential";

async fn assert_public(error: AuthError, status: StatusCode, code: &str, message: &str) {
    let diagnostic = format!("{error:?}");
    assert!(
        !diagnostic.contains(PRIVATE_CAUSE),
        "debug leaked cause: {diagnostic}"
    );
    let response = error.into_response();
    assert_eq!(response.status(), status, "error code {code}");
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .expect("public error response must include Content-Type"),
        "application/json"
    );
    let bytes = to_bytes(response.into_body(), 16_384)
        .await
        .expect("bounded error body");
    let body: Value = serde_json::from_slice(&bytes).expect("JSON error response");
    assert_eq!(
        body,
        json!({"error": {"error_code": code, "message": message, "status": status.as_u16()}})
    );
    assert!(!String::from_utf8_lossy(&bytes).contains(PRIVATE_CAUSE));
}

#[tokio::test]
async fn protocol_error_envelopes_are_closed_and_redact_raw_causes() {
    let cases = [
        (
            AuthError::InvalidProvider,
            StatusCode::BAD_REQUEST,
            "invalid_provider",
            "Invalid provider",
        ),
        (
            AuthError::InvalidAuthorizationHeader(PRIVATE_CAUSE.into()),
            StatusCode::BAD_REQUEST,
            "invalid_authorization_header",
            "Invalid Authorization header",
        ),
        (
            AuthError::InvalidToken(PRIVATE_CAUSE.into()),
            StatusCode::UNAUTHORIZED,
            "invalid_token",
            "Invalid or expired token",
        ),
        (
            AuthError::StateEncodingFailed(PRIVATE_CAUSE.into()),
            StatusCode::INTERNAL_SERVER_ERROR,
            "state_encoding_failed",
            "Failed to create OAuth state",
        ),
        (
            AuthError::UrlGenerationFailed(PRIVATE_CAUSE.into()),
            StatusCode::INTERNAL_SERVER_ERROR,
            "url_generation_failed",
            "Failed to generate authorization URL",
        ),
        (
            AuthError::InvalidUrl(PRIVATE_CAUSE.into()),
            StatusCode::BAD_REQUEST,
            "invalid_url",
            "Invalid URL in OAuth callback",
        ),
        (
            AuthError::OAuthError(PRIVATE_CAUSE.into(), PRIVATE_CAUSE.into()),
            StatusCode::BAD_REQUEST,
            "oauth_provider_error",
            "OAuth provider rejected the request",
        ),
        (
            AuthError::MissingCode,
            StatusCode::BAD_REQUEST,
            "missing_code",
            "Missing authorization code",
        ),
        (
            AuthError::InvalidState(PRIVATE_CAUSE.into()),
            StatusCode::BAD_REQUEST,
            "invalid_state",
            "Invalid OAuth state parameter",
        ),
        (
            AuthError::MissingState,
            StatusCode::BAD_REQUEST,
            "missing_state",
            "Missing OAuth state parameter",
        ),
        (
            AuthError::InvalidStateOperation,
            StatusCode::BAD_REQUEST,
            "invalid_state_operation",
            "Invalid OAuth state operation",
        ),
    ];
    for (error, status, code, message) in cases {
        assert_public(error, status, code, message).await;
    }
}

#[tokio::test]
async fn session_error_envelopes_are_closed_and_redact_raw_causes() {
    let cases = [
        (
            AuthError::AuthenticationFailed(PRIVATE_CAUSE.into()),
            StatusCode::UNAUTHORIZED,
            "authentication_failed",
            "Authentication failed",
        ),
        (
            AuthError::ValidationFailed(PRIVATE_CAUSE.into()),
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Request validation failed",
        ),
        (
            AuthError::LoginFailed,
            StatusCode::UNAUTHORIZED,
            "login_failed",
            "Login failed",
        ),
        (
            AuthError::ProviderAlreadyLinkedToSameUser("github".into()),
            StatusCode::CONFLICT,
            "provider_already_linked_same_user",
            "Provider github is already linked to this user",
        ),
        (
            AuthError::ProviderAlreadyLinked("github".into()),
            StatusCode::CONFLICT,
            "provider_already_linked",
            "Provider github is already linked to another user",
        ),
        (
            AuthError::LinkFailed,
            StatusCode::INTERNAL_SERVER_ERROR,
            "link_failed",
            "Failed to link provider",
        ),
    ];
    for (error, status, code, message) in cases {
        assert_public(error, status, code, message).await;
    }
}

#[tokio::test]
async fn registration_incomplete_returns_completion_token_but_never_logs_it() {
    let token = "sentinel-registration-token";
    let error = AuthError::RegistrationIncomplete {
        registration_token: token.into(),
        message: "Choose a username".into(),
    };
    assert!(!format!("{error:?}").contains(token));
    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::LOCKED);
    let bytes = to_bytes(response.into_body(), 16_384)
        .await
        .expect("bounded response");
    let body: Value = serde_json::from_slice(&bytes).expect("JSON response");
    assert_eq!(
        body,
        json!({"error":"registration_incomplete", "message":"Choose a username", "registration_token":token})
    );
}

type ErrorFactory = fn(&CommandError) -> AuthError;

#[tokio::test]
async fn unconfigured_connector_is_distinct_from_invalid_input_and_transport_failure() {
    let cause = CommandError::validation("connector_not_configured", PRIVATE_CAUSE);
    for error in [
        AuthError::oauth_login_failed("login", &cause),
        AuthError::oauth_link_failed("link", &cause, "github"),
        AuthError::oauth_start_failed(&cause, "github"),
        AuthError::link_provider_failed(&cause, "github"),
        AuthError::provider_token_failed(&cause, "github"),
    ] {
        assert_public(
            error,
            StatusCode::UNPROCESSABLE_ENTITY,
            "connector_not_configured",
            "IdP connector is not configured for this provider",
        )
        .await;
    }
}

#[tokio::test]
async fn username_checks_and_registration_bad_inputs_do_not_echo_untrusted_values() {
    let cause = CommandError::validation("invalid_username", PRIVATE_CAUSE);
    assert_public(
        AuthError::username_check_failed(&cause),
        StatusCode::BAD_REQUEST,
        "invalid_username",
        "Invalid username format",
    )
    .await;
    let cause = CommandError::validation("validation_failed", PRIVATE_CAUSE);
    assert_public(
        AuthError::username_check_failed(&cause),
        StatusCode::BAD_REQUEST,
        "validation_failed",
        "Request validation failed",
    )
    .await;
    for (cause, code) in [
        (
            CommandError::validation("validation_failed", PRIVATE_CAUSE),
            "validation_failed",
        ),
        (
            CommandError::business("registration_already_complete", PRIVATE_CAUSE),
            "registration_already_complete",
        ),
    ] {
        assert_public(
            AuthError::registration_failed(&cause),
            StatusCode::BAD_REQUEST,
            code,
            "Request validation failed",
        )
        .await;
    }
}

#[tokio::test]
async fn oauth_provider_failures_remain_authentication_failures_after_retry_wrapping() {
    let causes = [
        CommandError::business("authentication_failed", PRIVATE_CAUSE),
        CommandError::infrastructure("provider_error", PRIVATE_CAUSE),
        CommandError::infrastructure("oauth_transport", PRIVATE_CAUSE),
        CommandError::infrastructure("provider_rejected", PRIVATE_CAUSE),
        CommandError::RetryExhausted {
            code: "retry_exhausted".into(),
            message: format!("OAuth {PRIVATE_CAUSE}"),
        },
        CommandError::RetryExhausted {
            code: "retry_exhausted".into(),
            message: format!("provider_error {PRIVATE_CAUSE}"),
        },
    ];
    for cause in causes {
        assert_public(
            AuthError::oauth_login_failed("login", &cause),
            StatusCode::UNAUTHORIZED,
            "authentication_failed",
            "Authentication failed",
        )
        .await;
        assert_public(
            AuthError::oauth_link_failed("link", &cause, "github"),
            StatusCode::UNAUTHORIZED,
            "authentication_failed",
            "Authentication failed",
        )
        .await;
    }
    let cause = CommandError::infrastructure("repository_error", PRIVATE_CAUSE);
    assert_public(
        AuthError::oauth_login_failed("login", &cause),
        StatusCode::INTERNAL_SERVER_ERROR,
        "login_failed",
        "Login failed",
    )
    .await;
    assert_public(
        AuthError::oauth_link_failed("link", &cause, "github"),
        StatusCode::INTERNAL_SERVER_ERROR,
        "link_failed",
        "Link provider failed",
    )
    .await;
}

#[tokio::test]
async fn oauth_link_conflicts_are_actionable_without_echoing_provider_credentials() {
    for (code, status, message) in [
        (
            "provider_already_linked_same_user",
            StatusCode::CONFLICT,
            "github is already linked to your account",
        ),
        (
            "provider_already_linked",
            StatusCode::CONFLICT,
            "This github account is already linked to another user",
        ),
        ("user_not_found", StatusCode::NOT_FOUND, "User not found"),
        (
            "authentication_failed",
            StatusCode::UNAUTHORIZED,
            "Authentication failed",
        ),
    ] {
        let cause = CommandError::business(code, PRIVATE_CAUSE);
        assert_public(
            AuthError::oauth_link_failed("link", &cause, "github"),
            status,
            code,
            message,
        )
        .await;
        assert_public(
            AuthError::link_provider_failed(&cause, "github"),
            status,
            code,
            message,
        )
        .await;
    }
    let cause = CommandError::business("business_rule_violation", PRIVATE_CAUSE);
    assert_public(
        AuthError::link_provider_failed(&cause, "github"),
        StatusCode::UNPROCESSABLE_ENTITY,
        "business_rule_violation",
        "Cannot relink provider that is not currently linked",
    )
    .await;
}

#[tokio::test]
async fn oauth_start_and_callback_validation_remain_client_errors() {
    let cause = CommandError::validation("validation_failed", PRIVATE_CAUSE);
    for error in [
        AuthError::oauth_start_failed(&cause, "github"),
        AuthError::oauth_login_failed("login", &cause),
        AuthError::oauth_link_failed("link", &cause, "github"),
        AuthError::link_provider_failed(&cause, "github"),
    ] {
        assert_public(
            error,
            StatusCode::BAD_REQUEST,
            "validation_failed",
            "Request validation failed",
        )
        .await;
    }
    let cause = CommandError::business("provider_not_supported", PRIVATE_CAUSE);
    assert_public(
        AuthError::oauth_start_failed(&cause, "github"),
        StatusCode::UNPROCESSABLE_ENTITY,
        "provider_not_supported",
        "Provider github not supported",
    )
    .await;
    let cause = CommandError::infrastructure("repository_error", PRIVATE_CAUSE);
    assert_public(
        AuthError::oauth_start_failed(&cause, "github"),
        StatusCode::INTERNAL_SERVER_ERROR,
        "oauth_start_failed",
        "Failed to generate OAuth start URL",
    )
    .await;
    assert_public(
        AuthError::link_provider_failed(&cause, "github"),
        StatusCode::INTERNAL_SERVER_ERROR,
        "link_provider_failed",
        "Failed to link provider",
    )
    .await;
}

#[tokio::test]
async fn infrastructure_and_unexpected_failures_use_operation_specific_safe_fallbacks() {
    let factories: [(ErrorFactory, &str, &str); 7] = [
        (AuthError::signup_failed, "signup_failed", "Signup failed"),
        (AuthError::login_failed, "login_failed", "Login failed"),
        (
            AuthError::verification_failed,
            "verification_failed",
            "Email verification failed",
        ),
        (
            AuthError::registration_failed,
            "registration_failed",
            "Registration completion failed",
        ),
        (
            AuthError::username_check_failed,
            "username_check_failed",
            "Username availability check failed",
        ),
        (
            AuthError::password_reset_validate_failed,
            "validation_failed",
            "Token validation failed",
        ),
        (
            AuthError::password_reset_confirm_failed,
            "reset_failed",
            "Password reset failed",
        ),
    ];
    for (factory, fallback_code, fallback_message) in factories {
        let infrastructure = CommandError::infrastructure("repository_error", PRIVATE_CAUSE);
        assert_public(
            factory(&infrastructure),
            StatusCode::INTERNAL_SERVER_ERROR,
            "repository_error",
            "Internal server error",
        )
        .await;
        let timeout = CommandError::Timeout {
            code: "deadline".into(),
            message: PRIVATE_CAUSE.into(),
        };
        assert_public(
            factory(&timeout),
            StatusCode::INTERNAL_SERVER_ERROR,
            fallback_code,
            fallback_message,
        )
        .await;
    }
}

#[tokio::test]
async fn signup_and_login_distinguish_conflict_bad_input_and_unverified_account() {
    let cases: [(ErrorFactory, CommandError, StatusCode, &str, &str); 7] = [
        (
            AuthError::signup_failed,
            CommandError::business("user_already_exists", PRIVATE_CAUSE),
            StatusCode::CONFLICT,
            "user_already_exists",
            "User with this email already exists",
        ),
        (
            AuthError::signup_failed,
            CommandError::validation("invalid_email", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "invalid_email",
            "Request validation failed",
        ),
        (
            AuthError::signup_failed,
            CommandError::business("weak_password", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "weak_password",
            "Request validation failed",
        ),
        (
            AuthError::login_failed,
            CommandError::validation("invalid_credentials", PRIVATE_CAUSE),
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "Invalid email or password",
        ),
        (
            AuthError::login_failed,
            CommandError::business("invalid_credentials", PRIVATE_CAUSE),
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "Invalid email or password",
        ),
        (
            AuthError::login_failed,
            CommandError::business("email_not_verified", PRIVATE_CAUSE),
            StatusCode::UNAUTHORIZED,
            "email_not_verified",
            "Please verify your email address before logging in",
        ),
        (
            AuthError::login_failed,
            CommandError::validation("invalid_email", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "invalid_email",
            "Request validation failed",
        ),
    ];
    for (factory, cause, status, code, message) in cases {
        assert_public(factory(&cause), status, code, message).await;
    }
}

#[tokio::test]
async fn verification_and_registration_keep_distinct_recovery_errors() {
    let cases: [(ErrorFactory, CommandError, StatusCode, &str, &str); 9] = [
        (
            AuthError::verification_failed,
            CommandError::validation("invalid_verification_token", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "invalid_verification_token",
            "Invalid or expired verification token",
        ),
        (
            AuthError::verification_failed,
            CommandError::business("email_not_found", PRIVATE_CAUSE),
            StatusCode::NOT_FOUND,
            "email_not_found",
            "Verification request not found",
        ),
        (
            AuthError::verification_failed,
            CommandError::business("email_already_verified", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "email_already_verified",
            "Email is already verified",
        ),
        (
            AuthError::verification_failed,
            CommandError::validation("validation_failed", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "validation_failed",
            "Request validation failed",
        ),
        (
            AuthError::registration_failed,
            CommandError::validation("invalid_token", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "invalid_token",
            "Invalid registration token signature",
        ),
        (
            AuthError::registration_failed,
            CommandError::validation("token_expired", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "token_expired",
            "Registration token has expired",
        ),
        (
            AuthError::registration_failed,
            CommandError::business("username_taken", PRIVATE_CAUSE),
            StatusCode::CONFLICT,
            "username_taken",
            "Username is already taken",
        ),
        (
            AuthError::registration_failed,
            CommandError::business("user_not_found", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "user_not_found",
            "Registration session not found",
        ),
        (
            AuthError::registration_failed,
            CommandError::validation("invalid_username", PRIVATE_CAUSE),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_username",
            "Invalid username format",
        ),
    ];
    for (factory, cause, status, code, message) in cases {
        assert_public(factory(&cause), status, code, message).await;
    }
}

#[tokio::test]
async fn reset_requests_are_indistinguishable_for_missing_accounts_and_system_failures() {
    for cause in [
        CommandError::business("anti_enumeration_security", PRIVATE_CAUSE),
        CommandError::validation("invalid_email", PRIVATE_CAUSE),
        CommandError::infrastructure("repository_error", PRIVATE_CAUSE),
        CommandError::authentication("authentication_failed", PRIVATE_CAUSE),
    ] {
        assert_public(
            AuthError::password_reset_request_failed(&cause),
            StatusCode::OK,
            "success",
            "If a matching account was found, a password reset email has been sent",
        )
        .await;
    }
}

#[tokio::test]
async fn reset_validation_and_confirmation_do_not_disclose_token_failure_details() {
    let factories: [ErrorFactory; 2] = [
        AuthError::password_reset_validate_failed,
        AuthError::password_reset_confirm_failed,
    ];
    for factory in factories {
        for code in ["invalid_token", "token_expired", "token_already_used"] {
            let cause = CommandError::validation(code, PRIVATE_CAUSE);
            assert_public(
                factory(&cause),
                StatusCode::BAD_REQUEST,
                code,
                "Invalid or expired reset token",
            )
            .await;
        }
    }
    let cause = CommandError::validation("validation_failed", PRIVATE_CAUSE);
    assert_public(
        AuthError::password_reset_confirm_failed(&cause),
        StatusCode::BAD_REQUEST,
        "validation_failed",
        "Password does not meet security requirements",
    )
    .await;
    assert_public(
        AuthError::password_reset_authenticated_failed(&cause),
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_failed",
        "Password validation failed",
    )
    .await;
    let cause = CommandError::validation("incorrect_current_password", PRIVATE_CAUSE);
    assert_public(
        AuthError::password_reset_authenticated_failed(&cause),
        StatusCode::BAD_REQUEST,
        "incorrect_current_password",
        "Current password is incorrect",
    )
    .await;
}

#[tokio::test]
async fn provider_token_error_mapping_preserves_authentication_and_missing_link_status() {
    let cases = [
        (
            CommandError::authentication("authentication_failed", PRIVATE_CAUSE),
            StatusCode::UNAUTHORIZED,
            "authentication_failed",
            "Authentication failed",
        ),
        (
            CommandError::authentication("user_not_found", PRIVATE_CAUSE),
            StatusCode::UNAUTHORIZED,
            "user_not_found",
            "Authentication failed",
        ),
        (
            CommandError::validation("provider_not_supported", PRIVATE_CAUSE),
            StatusCode::UNPROCESSABLE_ENTITY,
            "provider_not_supported",
            "Unsupported provider: github",
        ),
        (
            CommandError::business("no_token_for_provider", PRIVATE_CAUSE),
            StatusCode::NOT_FOUND,
            "no_token_for_provider",
            "No token available for the user and provider github",
        ),
        (
            CommandError::validation("validation_failed", PRIVATE_CAUSE),
            StatusCode::BAD_REQUEST,
            "validation_failed",
            "Request validation failed",
        ),
        (
            CommandError::infrastructure("database_error", PRIVATE_CAUSE),
            StatusCode::INTERNAL_SERVER_ERROR,
            "database_error",
            "Internal server error",
        ),
        (
            CommandError::business("unexpected", PRIVATE_CAUSE),
            StatusCode::INTERNAL_SERVER_ERROR,
            "token_retrieval_failed",
            "Failed to retrieve provider token",
        ),
    ];
    for (cause, status, code, message) in cases {
        assert_public(
            AuthError::provider_token_failed(&cause, "github"),
            status,
            code,
            message,
        )
        .await;
    }
}
