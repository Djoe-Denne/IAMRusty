use axum::http::StatusCode;
use axum::response::IntoResponse;
use iam_application::command::CommandError;
use iam_application::usecase::{token::TokenError, user::UserError};
use iam_domain::error::DomainError;
use iam_http_server::{ApiError, AuthError};

fn status_of_auth(error: AuthError) -> StatusCode {
    error.into_response().status()
}

fn status_of_api(error: ApiError) -> StatusCode {
    error.into_response().status()
}

#[test]
fn oauth_protocol_errors_map_to_http() {
    assert_eq!(
        status_of_auth(AuthError::oauth_invalid_provider("login")),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_invalid_authorization_header("login")),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_invalid_token("login")),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_state_encoding_failed("login")),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_url_generation_failed("login")),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_invalid_url("callback")),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_missing_code("callback")),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_invalid_state("callback")),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_missing_state("callback")),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_invalid_state_operation("callback")),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::oauth_provider_error(
            "callback",
            "access_denied".into(),
            "user cancelled".into(),
        )),
        StatusCode::BAD_REQUEST
    );
}

#[test]
fn session_and_account_errors_map_to_http() {
    assert_eq!(
        status_of_auth(AuthError::InvalidProvider),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::InvalidAuthorizationHeader("x".into())),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::InvalidToken("x".into())),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status_of_auth(AuthError::StateEncodingFailed("x".into())),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        status_of_auth(AuthError::UrlGenerationFailed("x".into())),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        status_of_auth(AuthError::InvalidUrl("x".into())),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::OAuthError("e".into(), "d".into())),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::MissingCode),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::InvalidState("x".into())),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::MissingState),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::InvalidStateOperation),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_auth(AuthError::AuthenticationFailed("x".into())),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status_of_auth(AuthError::ValidationFailed("x".into())),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        status_of_auth(AuthError::LoginFailed),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status_of_auth(AuthError::ProviderAlreadyLinkedToSameUser("github".into())),
        StatusCode::CONFLICT
    );
    assert_eq!(
        status_of_auth(AuthError::ProviderAlreadyLinked("github".into())),
        StatusCode::CONFLICT
    );
    assert_eq!(
        status_of_auth(AuthError::UserNotFound("u".into())),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status_of_auth(AuthError::LinkFailed),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        status_of_auth(AuthError::RegistrationIncomplete {
            registration_token: "tok".into(),
            message: "complete signup".into(),
        }),
        StatusCode::LOCKED
    );
    assert_eq!(
        status_of_auth(AuthError::Api(ApiError::AuthenticationRequired)),
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn command_failure_helpers_cover_oauth_and_auth_branches() {
    let auth_failed = CommandError::business("authentication_failed", "nope");
    let validation = CommandError::validation("invalid_email", "bad email");
    let provider_error = CommandError::infrastructure("provider_error", "oauth down");
    let retry = CommandError::RetryExhausted {
        code: "retry".into(),
        message: "OAuth provider_error".into(),
    };
    let oauth_infra = CommandError::infrastructure("oauth_timeout", "timeout");
    let other_business = CommandError::business("other", "nope");
    let timeout = CommandError::Timeout {
        code: "timeout".into(),
        message: "late".into(),
    };
    let already_same = CommandError::business("provider_already_linked_same_user", "dup");
    let already_other = CommandError::business("provider_already_linked", "dup");

    let _ = AuthError::oauth_login_failed("login", &auth_failed);
    let _ = AuthError::oauth_login_failed("login", &validation);
    let _ = AuthError::oauth_login_failed("login", &provider_error);
    let _ = AuthError::oauth_login_failed("login", &retry);
    let _ = AuthError::oauth_login_failed("login", &oauth_infra);
    let _ = AuthError::oauth_login_failed("login", &other_business);
    let _ = AuthError::oauth_login_failed("login", &timeout);

    let _ = AuthError::oauth_link_failed("link", &already_same, "github");
    let _ = AuthError::oauth_link_failed("link", &already_other, "github");
    let _ = AuthError::oauth_link_failed("link", &auth_failed, "github");
    let _ = AuthError::oauth_link_failed("link", &validation, "github");
    let _ = AuthError::oauth_link_failed("link", &timeout, "github");

    let _ = AuthError::oauth_start_failed(&timeout, "github");
    let _ = AuthError::link_provider_failed(&already_same, "github");
    let _ = AuthError::link_provider_failed(&already_other, "github");
    let _ = AuthError::link_provider_failed(&timeout, "github");
    let _ = AuthError::signup_failed(&validation);
    let _ = AuthError::signup_failed(&timeout);
    let _ = AuthError::login_failed(&auth_failed);
    let _ = AuthError::login_failed(&timeout);
    let _ = AuthError::verification_failed(&validation);
    let _ = AuthError::verification_failed(&timeout);
    let _ = AuthError::provider_token_failed(&timeout, "github");
    let _ = AuthError::registration_failed(&validation);
    let _ = AuthError::registration_failed(&timeout);
    let _ = AuthError::username_check_failed(&validation);
    let _ = AuthError::username_check_failed(&timeout);
    let _ = AuthError::password_reset_request_failed(&timeout);
    let _ = AuthError::password_reset_validate_failed(&validation);
    let _ = AuthError::password_reset_validate_failed(&timeout);
    let _ = AuthError::password_reset_confirm_failed(&validation);
    let _ = AuthError::password_reset_confirm_failed(&timeout);
    let _ = AuthError::password_reset_authenticated_failed(&validation);
    let _ = AuthError::password_reset_authenticated_failed(&timeout);
}

#[test]
fn api_error_maps_domain_command_user_and_token_errors() {
    let domain_errors = [
        DomainError::UserNotFound,
        DomainError::ProviderNotSupported("x".into()),
        DomainError::BusinessRuleViolation("x".into()),
        DomainError::InvalidToken,
        DomainError::TokenExpired,
        DomainError::AuthorizationError("x".into()),
        DomainError::OAuth2Error("x".into()),
        DomainError::UserProfileError("x".into()),
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
        let _ = status_of_api(ApiError::Domain(error));
    }

    let _ = status_of_api(ApiError::Command(CommandError::validation("c", "m")));
    let _ = status_of_api(ApiError::Command(CommandError::Authentication {
        code: "c".into(),
        message: "m".into(),
    }));
    let _ = status_of_api(ApiError::Command(CommandError::business("c", "m")));
    let _ = status_of_api(ApiError::Command(CommandError::Timeout {
        code: "c".into(),
        message: "m".into(),
    }));
    let _ = status_of_api(ApiError::Command(CommandError::infrastructure("c", "m")));
    let _ = status_of_api(ApiError::Command(CommandError::RetryExhausted {
        code: "c".into(),
        message: "m".into(),
    }));

    let _ = status_of_api(ApiError::User(UserError::RepositoryError("x".into())));
    let _ = status_of_api(ApiError::User(UserError::TokenServiceError("x".into())));
    let _ = status_of_api(ApiError::User(UserError::UserNotFound));
    let _ = status_of_api(ApiError::User(UserError::InvalidToken));
    let _ = status_of_api(ApiError::User(UserError::TokenExpired));
    let _ = status_of_api(ApiError::User(UserError::DomainError(
        DomainError::UserNotFound,
    )));

    let _ = status_of_api(ApiError::Token(TokenError::RepositoryError("x".into())));
    let _ = status_of_api(ApiError::Token(TokenError::TokenServiceError("x".into())));
    let _ = status_of_api(ApiError::Token(TokenError::TokenNotFound));
    let _ = status_of_api(ApiError::Token(TokenError::TokenInvalid));
    let _ = status_of_api(ApiError::Token(TokenError::TokenExpired));
    let _ = status_of_api(ApiError::Token(TokenError::DomainError(
        DomainError::TokenNotFound,
    )));

    assert_eq!(
        status_of_api(ApiError::AuthenticationRequired),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status_of_api(ApiError::InvalidRequest("x".into())),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of_api(ApiError::InternalServerError("x".into())),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}
