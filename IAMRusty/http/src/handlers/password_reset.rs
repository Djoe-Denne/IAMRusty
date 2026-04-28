use crate::error::AuthError;
use axum::{extract::State, Json};
use iam_application::command::{
    password_reset::{
        RequestPasswordResetCommand, ResetPasswordAuthenticatedCommand,
        ResetPasswordUnauthenticatedCommand, ValidateResetTokenCommand,
    },
    user::GetUserCommand,
    CommandContext,
};
use rustycog_http::AppState;
use rustycog_http::{AuthUser, ValidatedJson};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use validator::Validate;

/// Request password reset request
#[derive(Debug, Deserialize, Validate)]
pub struct RequestPasswordResetRequest {
    #[validate(custom(
        function = "crate::validation::validate_email_format",
        message = "Invalid email format"
    ))]
    pub email: String,
}

/// Request password reset response - always success for security
#[derive(Debug, Serialize)]
pub struct RequestPasswordResetResponse {
    pub message: String,
}

/// Validate reset token request
#[derive(Debug, Deserialize, Validate)]
pub struct ValidateResetTokenRequest {
    #[validate(custom(
        function = "crate::validation::validate_reset_token_format",
        message = "Invalid token format"
    ))]
    pub token: String,
}

/// Validate reset token response
#[derive(Debug, Serialize)]
pub struct ValidateResetTokenResponse {
    pub valid: bool,
    pub message: String,
    pub email: Option<String>,
}

/// Reset password unauthenticated request
#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordUnauthenticatedRequest {
    #[validate(custom(
        function = "crate::validation::validate_reset_token_format",
        message = "Invalid token format"
    ))]
    pub token: String,
    #[validate(custom(
        function = "crate::validation::validate_strong_password",
        message = "Password must be at least 8 characters and contain both letters and numbers"
    ))]
    pub new_password: String,
}

/// Reset password authenticated request
#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordAuthenticatedRequest {
    #[validate(custom(
        function = "crate::validation::validate_non_empty_string",
        message = "Current password is required"
    ))]
    pub current_password: String,
    #[validate(custom(
        function = "crate::validation::validate_strong_password",
        message = "Password must be at least 8 characters and contain both letters and numbers"
    ))]
    pub new_password: String,
}

/// Reset password response
#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    pub message: String,
}

/// Request password reset - POST /auth/password/reset-request
/// Always returns 200 to prevent user enumeration
pub async fn request_password_reset(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<RequestPasswordResetRequest>,
) -> Result<Json<RequestPasswordResetResponse>, AuthError> {
    debug!(
        "Processing password reset request for email: {}",
        request.email
    );

    let command = RequestPasswordResetCommand::new(request.email);
    let context = CommandContext::default();

    // Execute command - we always return success for anti-enumeration
    match state.command_service.execute(command, context).await {
        Ok(_) => {
            debug!("Password reset request processed successfully");
        }
        Err(e) => {
            // Log the error but don't expose it to prevent enumeration
            error!("Password reset request failed: {:?}", e);
            // Still return success for anti-enumeration
            return Err(AuthError::password_reset_request_failed(&e));
        }
    }

    // Always return success message for security
    Ok(Json(RequestPasswordResetResponse {
        message: "If an account with that email exists, a password reset link has been sent."
            .to_string(),
    }))
}

/// Validate reset token - POST /auth/password/reset-validate
pub async fn validate_reset_token(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<ValidateResetTokenRequest>,
) -> Result<Json<ValidateResetTokenResponse>, AuthError> {
    debug!("Validating reset token: {}", request.token);

    let command = ValidateResetTokenCommand::new(request.token);
    let context = CommandContext::default();

    match state.command_service.execute(command, context).await {
        Ok(result) => {
            let masked_email = result
                .email
                .as_ref()
                .map(|email| crate::validation::mask_email(email));

            Ok(Json(ValidateResetTokenResponse {
                valid: true,
                message: "Reset token is valid".to_string(),
                email: masked_email,
            }))
        }
        Err(e) => {
            error!("Token validation failed: {:?}", e);
            Err(AuthError::password_reset_validate_failed(&e))
        }
    }
}

/// Reset password unauthenticated - POST /auth/password/reset-confirm
pub async fn reset_password_unauthenticated(
    State(state): State<AppState>,
    ValidatedJson(request): ValidatedJson<ResetPasswordUnauthenticatedRequest>,
) -> Result<Json<ResetPasswordResponse>, AuthError> {
    debug!(
        "Processing unauthenticated password reset with token: {}",
        request.token
    );

    let command = ResetPasswordUnauthenticatedCommand::new(request.token, request.new_password);
    let context = CommandContext::default();

    match state.command_service.execute(command, context).await {
        Ok(_) => {
            debug!("Password reset completed successfully");
            Ok(Json(ResetPasswordResponse {
                message: "Password has been successfully reset".to_string(),
            }))
        }
        Err(e) => {
            error!("Password reset failed: {:?}", e);
            Err(AuthError::password_reset_confirm_failed(&e))
        }
    }
}

/// Reset password authenticated - POST /auth/password/reset-confirm (with JWT)
pub async fn reset_password_authenticated(
    State(state): State<AppState>,
    auth_user: AuthUser,
    ValidatedJson(request): ValidatedJson<ResetPasswordAuthenticatedRequest>,
) -> Result<Json<ResetPasswordResponse>, AuthError> {
    debug!(
        "Processing authenticated password reset for user: {}",
        auth_user.user_id
    );

    // check if user exists
    let user_context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata("operation".to_string(), "get_user".to_string());
    let _user = state
        .command_service
        .execute(GetUserCommand::new(auth_user.user_id), user_context)
        .await
        .map_err(|_e| AuthError::InvalidToken("Invalid token".to_string()))?;

    let command = ResetPasswordAuthenticatedCommand::new(
        auth_user.user_id,
        request.current_password,
        request.new_password,
    );
    let context = CommandContext::default();

    match state.command_service.execute(command, context).await {
        Ok(_) => {
            debug!("Authenticated password reset completed successfully");
            Ok(Json(ResetPasswordResponse {
                message: "Password has been successfully changed".to_string(),
            }))
        }
        Err(e) => {
            error!("Authenticated password reset failed: {:?}", e);
            Err(AuthError::password_reset_authenticated_failed(&e))
        }
    }
}
