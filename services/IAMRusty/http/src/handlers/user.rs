use crate::error::ApiError;
use crate::platform_user::PlatformUser;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use iam_application::{
    command::{
        user::{GetUserCommand, PatchProfileCommand},
        CommandContext, CommandError,
    },
    usecase::user::UserProfile,
};
use iam_domain::{
    error::DomainError,
    service::user_service::{OptionalField, ProfilePatch},
};
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
    username: OptionalField<String>,
    avatar_url: OptionalField<String>,
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
                    username: OptionalField<String>,
                    #[serde(default, deserialize_with = "submitted_field")]
                    avatar_url: OptionalField<String>,
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
) -> Result<OptionalField<String>, D::Error> {
    Option::<String>::deserialize(deserializer).map(OptionalField::Set)
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

/// Compact profile failure, converted to a response at the HTTP boundary.
pub struct ProfileRejection(CommandError);

impl IntoResponse for ProfileRejection {
    fn into_response(self) -> Response {
        match self.0 {
            CommandError::Business { ref code, .. } if code == "username_taken" => {
                crate::error::uniform_error(
                    StatusCode::CONFLICT,
                    "username_taken",
                    "Username is already taken",
                )
            }
            CommandError::Validation { ref code, .. } if code == "invalid_username" => {
                ApiError::from(DomainError::InvalidUsername).into_response()
            }
            CommandError::Validation { ref code, .. } if code == "invalid_avatar_url" => {
                ApiError::from(DomainError::InvalidAvatarUrl).into_response()
            }
            other => ApiError::from(other).into_response(),
        }
    }
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
) -> Result<Json<UserResponse>, ProfileRejection> {
    let username = match request.username {
        OptionalField::Unset => None,
        OptionalField::Set(Some(username)) => Some(username),
        OptionalField::Set(None) => {
            return Err(ProfileRejection(CommandError::validation(
                "invalid_username",
                "Invalid username format",
            )));
        }
    };
    let patch = ProfilePatch {
        username,
        avatar_url: request.avatar_url,
    };
    let command = PatchProfileCommand::new(auth_user.user_id, patch);
    let context = CommandContext::new().with_user_id(auth_user.user_id);
    let user = state
        .command_service
        .execute_once(command, context)
        .await
        .map_err(ProfileRejection)?;
    Ok(Json(user.into()))
}

#[cfg(test)]
mod tests {
    use super::{OptionalField, PatchProfileRequest};

    #[test]
    fn profile_patch_parser_distinguishes_absent_null_and_string() {
        let empty: PatchProfileRequest = serde_json::from_str("{}").expect("empty patch");
        assert!(matches!(empty.username, OptionalField::Unset));
        assert!(matches!(empty.avatar_url, OptionalField::Unset));
        let explicit: PatchProfileRequest =
            serde_json::from_str(r#"{"username":null,"avatar_url":null}"#).expect("explicit nulls");
        assert!(matches!(explicit.username, OptionalField::Set(None)));
        assert!(matches!(explicit.avatar_url, OptionalField::Set(None)));
        let named: PatchProfileRequest =
            serde_json::from_str(r#"{"username":"reader","avatar_url":"https://example.test/a"}"#)
                .expect("string fields");
        assert!(matches!(named.username, OptionalField::Set(Some(name)) if name == "reader"));
        assert!(
            matches!(named.avatar_url, OptionalField::Set(Some(url)) if url == "https://example.test/a")
        );
    }

    #[test]
    fn profile_patch_parser_rejects_nonobjects_unknown_duplicate_and_wrong_type_fields() {
        for (input, reason) in [
            ("[null,null]", "a profile patch object"),
            ("null", "a profile patch object"),
            (r#"{"username":"reader","email":null}"#, "unknown field"),
            (
                r#"{"avatar_url":null,"avatar_url":null}"#,
                "duplicate field",
            ),
            (r#"{"username":false}"#, "expected a string"),
        ] {
            let Err(error) = serde_json::from_str::<PatchProfileRequest>(input) else {
                panic!("unexpectedly accepted {input}");
            };
            assert!(error.to_string().contains(reason), "{error}");
        }
    }
}
