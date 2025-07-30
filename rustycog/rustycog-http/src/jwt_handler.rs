use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use rustycog_command::{Command, CommandError, CommandHandler, ValidateTokenCommand};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

/// Simple user ID extractor with basic JWT validation
#[derive(Clone)]
pub struct UserIdExtractor {
    /// Default user ID to use (for testing/development)
    default_user_id: Option<Uuid>,
}

impl UserIdExtractor {
    /// Create a new user ID extractor
    pub fn new() -> Self {
        Self {
            default_user_id: None,
        }
    }

    /// Create a new user ID extractor with a default user ID
    pub fn with_default_user_id(user_id: Uuid) -> Self {
        Self {
            default_user_id: Some(user_id),
        }
    }

    /// Extract user ID from token with basic validation (format and expiration)
    pub fn extract_user_id(&self, token: &str) -> Result<Uuid, CommandError> {
        debug!("Extracting user ID from token with basic validation");

        // Check JWT format (3 parts separated by dots)
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(CommandError::Authentication {
                code: "invalid_token".to_string(),
                message: "Invalid token format".to_string(),
            });
        }

        // Decode the payload (second part)
        let payload = String::from_utf8(
            general_purpose::URL_SAFE_NO_PAD
                .decode(parts[1])
                .map_err(|_| CommandError::Authentication {
                    code: "invalid_token".to_string(),
                    message: "Invalid token encoding".to_string(),
                })?,
        )
        .map_err(|_| CommandError::Authentication {
            code: "invalid_token".to_string(),
            message: "Invalid token encoding".to_string(),
        })?;

        // Parse JSON payload
        let payload_json: serde_json::Value =
            serde_json::from_str(&payload).map_err(|_| CommandError::Authentication {
                code: "invalid_token".to_string(),
                message: "Invalid token JSON".to_string(),
            })?;

        // Check for required claims - 'sub' (subject/user ID)
        let sub = payload_json["sub"]
            .as_str()
            .ok_or_else(|| CommandError::Authentication {
                code: "invalid_token".to_string(),
                message: "Missing user ID in token".to_string(),
            })?;

        // Check for required claims - 'exp' (expiration time)
        let exp = payload_json["exp"]
            .as_i64()
            .ok_or_else(|| CommandError::Authentication {
                code: "invalid_token".to_string(),
                message: "Missing expiration in token".to_string(),
            })?;

        // Check for required claims - 'iat' (issued at time)
        let _iat = payload_json["iat"]
            .as_i64()
            .ok_or_else(|| CommandError::Authentication {
                code: "invalid_token".to_string(),
                message: "Missing issued at time in token".to_string(),
            })?;

        // Check for required claims - 'jti' (JWT ID)
        let _jti = payload_json["jti"]
            .as_str()
            .ok_or_else(|| CommandError::Authentication {
                code: "invalid_token".to_string(),
                message: "Missing JWT ID in token".to_string(),
            })?;

        // Check if token is expired
        let now = Utc::now().timestamp();
        if exp <= now {
            debug!("Token expired: exp={}, now={}", exp, now);
            return Err(CommandError::Authentication {
                code: "token_expired".to_string(),
                message: "Token has expired".to_string(),
            });
        }

        // Parse and return user ID
        Uuid::parse_str(sub).map_err(|_| CommandError::Authentication {
            code: "invalid_token".to_string(),
            message: "Invalid user ID format".to_string(),
        })
    }
}

/// Command handler for simple user ID extraction
pub struct UserIdExtractionHandler {
    extractor: Arc<UserIdExtractor>,
}

impl UserIdExtractionHandler {
    /// Create a new user ID extraction handler
    pub fn new(extractor: UserIdExtractor) -> Self {
        Self {
            extractor: Arc::new(extractor),
        }
    }
}

#[async_trait]
impl CommandHandler<ValidateTokenCommand> for UserIdExtractionHandler {
    async fn handle(&self, command: ValidateTokenCommand) -> Result<Uuid, CommandError> {
        debug!(
            "Handling ValidateTokenCommand with ID: {}",
            command.command_id()
        );

        // Validate the command first
        command.validate()?;

        // Extract user ID from token (no verification)
        self.extractor.extract_user_id(&command.token)
    }
}
