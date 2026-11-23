use crate::error::ApiError;
use crate::platform_user::PlatformUser;
use axum::{extract::State, http::StatusCode, response::Response, Json};
use iam_application::{
    command::{
        user::{GetUserCommand, PatchProfileCommand},
        CommandContext, CommandError,
    },
    usecase::user::UserProfile,
};
use iam_domain::service::user_service::ProfilePatch;
use rustycog::http::ValidatedJson;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;
use validator::Validate;

use tracing::debug;

/// User response
#[derive(Debug, Serialize)]
pub struct UserResponse {
    /// User ID
    pub id: Uuid,
    /// Username
    pub username: String,
    /// Email address
    pub email: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
}

impl From<UserProfile> for UserResponse {
    fn from(profile: UserProfile) -> Self {
        Self {
            id: profile.user.id,
            username: profile.user.username.unwrap_or_default(),
            email: profile.email,
            avatar_url: profile.user.avatar_url,
        }
    }
}

/// A missing field is distinct from an explicit JSON null.
#[derive(Validate)]
pub struct PatchProfileRequest {
    username: Option<Option<String>>,
    avatar_url: Option<Option<String>>,
}

impl<'de> Deserialize<'de> for PatchProfileRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectOnly;
        impl<'de> serde::de::Visitor<'de> for ObjectOnly {
            type Value = PatchProfileRequest;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a profile patch object")
            }

            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                map: M,
            ) -> Result<Self::Value, M::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Fields {
                    #[serde(default, deserialize_with = "submitted_field")]
                    username: Option<Option<String>>,
                    #[serde(default, deserialize_with = "submitted_field")]
                    avatar_url: Option<Option<String>>,
                }

                let fields =
                    Fields::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                Ok(PatchProfileRequest {
                    username: fields.username,
                    avatar_url: fields.avatar_url,
                })
            }
        }
        deserializer.deserialize_map(ObjectOnly)
    }
}

fn submitted_field<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

/// Get the current user's profile
///
/// # Errors
///
/// Returns [`ApiError`] when the user cannot be loaded.
pub async fn get_user(
    State(state): State<rustycog::http::AppState>,
    auth_user: PlatformUser,
) -> Result<Json<UserResponse>, ApiError> {
    debug!("Getting user profile for ID: {}", auth_user.user_id);

    let context = CommandContext::new()
        .with_user_id(auth_user.user_id)
        .with_metadata("operation".to_string(), "get_user".to_string());

    // Get the user using command service
    let command = GetUserCommand::new(auth_user.user_id);
    let user = state.command_service.execute(command, context).await?;

    Ok(Json(user.into()))
}

fn profile_rejection(error: CommandError) -> Response {
    use axum::response::IntoResponse;

    let (status, code, message) = match error {
        CommandError::Business { ref code, .. } if code == "username_taken" => (
            StatusCode::CONFLICT,
            "username_taken",
            "Username is already taken",
        ),
        CommandError::Validation { ref code, .. } if code == "invalid_username" => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_username",
            "Invalid username format",
        ),
        CommandError::Validation { ref code, .. } if code == "invalid_avatar_url" => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_avatar_url",
            "Avatar must be an absolute HTTPS URL without user information",
        ),
        other => return ApiError::from(other).into_response(),
    };
    crate::error::uniform_error(status, code, message)
}

/// Apply a partial update to the current human account.
///
/// # Errors
///
/// Returns a profile validation, username conflict, or command error response.
pub async fn patch_profile(
    State(state): State<rustycog::http::AppState>,
    auth_user: PlatformUser,
    ValidatedJson(request): ValidatedJson<PatchProfileRequest>,
) -> Result<Json<UserResponse>, Response> {
    if request.username == Some(None) {
        return Err(profile_rejection(CommandError::validation(
            "invalid_username",
            "Invalid username format",
        )));
    }
    let patch = ProfilePatch {
        username: request.username.flatten(),
        avatar_url: request.avatar_url,
    };
    let command = PatchProfileCommand::new(auth_user.user_id, patch);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    let user = state
        .command_service
        .execute_once(command, context)
        .await
        .map_err(profile_rejection)?;
    Ok(Json(user.into()))
}
