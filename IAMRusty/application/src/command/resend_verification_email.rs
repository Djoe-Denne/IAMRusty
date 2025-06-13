use crate::usecase::login::{LoginUseCase, ResendVerificationEmailRequest, ResendVerificationEmailResponse};
use super::signup::AuthErrorMapper; // Reuse existing mapper implementation
use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandHandler};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rustycog_command::CommandErrorMapper;

/// Resend verification email command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResendVerificationEmailCommand {
    /// Command instance ID
    pub command_id: Uuid,
    /// Email address to which verification email should be resent
    pub email: String,
}

impl ResendVerificationEmailCommand {
    /// Create a new command instance
    pub fn new(email: String) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            email,
        }
    }
}

impl Command for ResendVerificationEmailCommand {
    type Result = ResendVerificationEmailResponse;

    fn command_type(&self) -> &'static str {
        "resend_verification_email"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.email.trim().is_empty() {
            return Err(CommandError::validation(
                "ValidationFailed",
                "Email cannot be empty",
            ));
        }
        Ok(())
    }
}

/// Command handler
pub struct ResendVerificationEmailCommandHandler<A>
where
    A: LoginUseCase + ?Sized,
{
    login_use_case: std::sync::Arc<A>,
}

impl<A> ResendVerificationEmailCommandHandler<A>
where
    A: LoginUseCase + ?Sized,
{
    pub fn new(login_use_case: std::sync::Arc<A>) -> Self {
        Self { login_use_case }
    }
}

#[async_trait]
impl<A> CommandHandler<ResendVerificationEmailCommand> for ResendVerificationEmailCommandHandler<A>
where
    A: LoginUseCase + Send + Sync + ?Sized,
{
    async fn handle(&self, command: ResendVerificationEmailCommand) -> Result<ResendVerificationEmailResponse, CommandError> {
        let request = ResendVerificationEmailRequest {
            email: command.email,
        };

        self
            .login_use_case
            .resend_verification_email(request)
            .await
            .map_err(|e| AuthErrorMapper.map_error(Box::new(e)))
    }
} 