use super::{Command, CommandError, CommandHandler, error_mappers::AuthErrorMapper};
use super::registry::CommandErrorMapper;
use crate::usecase::auth::{AuthUseCase, LoginRequest, LoginResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Password login command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordLoginCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Email address for authentication
    pub email: String,
    /// Password for authentication
    pub password: String,
}

impl PasswordLoginCommand {
    /// Create a new password login command
    pub fn new(email: String, password: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            email,
            password,
        }
    }
}

impl Command for PasswordLoginCommand {
    type Result = LoginResponse;

    fn command_type(&self) -> &'static str {
        "password_login"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.email.trim().is_empty() {
            return Err(CommandError::Validation("Email cannot be empty".to_string()));
        }
        
        if self.password.trim().is_empty() {
            return Err(CommandError::Validation("Password cannot be empty".to_string()));
        }
        
        // Basic email format validation
        if !self.email.contains('@') || !self.email.contains('.') {
            return Err(CommandError::Validation("Invalid email format".to_string()));
        }
        
        Ok(())
    }
}

/// Password login command handler
pub struct PasswordLoginCommandHandler<A>
where
    A: AuthUseCase + ?Sized,
{
    auth_use_case: Arc<A>,
}

impl<A> PasswordLoginCommandHandler<A>
where
    A: AuthUseCase + ?Sized,
{
    /// Create a new password login command handler
    pub fn new(auth_use_case: Arc<A>) -> Self {
        Self {
            auth_use_case,
        }
    }
}

#[async_trait]
impl<A> CommandHandler<PasswordLoginCommand> for PasswordLoginCommandHandler<A>
where
    A: AuthUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: PasswordLoginCommand) -> Result<LoginResponse, CommandError> {
        let request = LoginRequest {
            email: command.email,
            password: command.password,
        };

        self.auth_use_case
            .login(request)
            .await
            .map_err(|e| AuthErrorMapper.map_error(Box::new(e)))
    }
} 