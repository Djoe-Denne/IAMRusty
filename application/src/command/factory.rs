use super::{
    registry::{CommandRegistry, CommandRegistryBuilder},
    error_mappers::*,
    login::{LoginCommand, GenerateLoginStartUrlCommand, LoginCommandHandler, GenerateLoginStartUrlCommandHandler},
    link_provider::{LinkProviderCommand, GenerateLinkProviderStartUrlCommand, LinkProviderCommandHandler, GenerateLinkProviderStartUrlCommandHandler},
    token::{RefreshTokenCommand, RevokeTokenCommand, RevokeAllTokensCommand, RefreshTokenCommandHandler, RevokeTokenCommandHandler, RevokeAllTokensCommandHandler},
    user::{GetUserCommand, ValidateTokenCommand, GetUserCommandHandler, ValidateTokenCommandHandler},
    signup::{SignupCommand, SignupCommandHandler},
    password_login::{PasswordLoginCommand, PasswordLoginCommandHandler},
    verify_email::{VerifyEmailCommand, VerifyEmailCommandHandler},
};
use crate::usecase::{
    login::LoginUseCase,
    link_provider::LinkProviderUseCase,
    token::TokenUseCase,
    user::UserUseCase,
    auth::AuthUseCase,
};
use std::sync::Arc;

/// Factory for creating a command registry with all standard commands registered
pub struct CommandRegistryFactory;

impl CommandRegistryFactory {
    /// Create a command registry with all standard IAM commands registered
    pub fn create_iam_registry(
        login_usecase: Arc<dyn LoginUseCase>,
        link_provider_usecase: Arc<dyn LinkProviderUseCase>,
        token_usecase: Arc<dyn TokenUseCase>,
        user_usecase: Arc<dyn UserUseCase>,
        auth_usecase: Arc<dyn AuthUseCase>,
    ) -> CommandRegistry {
        let mut builder = CommandRegistryBuilder::new();

        // Register login commands
        let login_handler = Arc::new(LoginCommandHandler::new(login_usecase.clone()));
        let login_start_url_handler = Arc::new(GenerateLoginStartUrlCommandHandler::new(login_usecase));
        let login_error_mapper = Arc::new(LoginErrorMapper);

        builder = builder
            .register::<LoginCommand, _>("login".to_string(), login_handler, login_error_mapper.clone())
            .register::<GenerateLoginStartUrlCommand, _>("generate_login_start_url".to_string(), login_start_url_handler, login_error_mapper);

        // Register link provider commands
        let link_provider_handler = Arc::new(LinkProviderCommandHandler::new(link_provider_usecase.clone()));
        let link_provider_start_url_handler = Arc::new(GenerateLinkProviderStartUrlCommandHandler::new(link_provider_usecase));
        let link_provider_error_mapper = Arc::new(LinkProviderErrorMapper);

        builder = builder
            .register::<LinkProviderCommand, _>("link_provider".to_string(), link_provider_handler, link_provider_error_mapper.clone())
            .register::<GenerateLinkProviderStartUrlCommand, _>("generate_link_provider_start_url".to_string(), link_provider_start_url_handler, link_provider_error_mapper);

        // Register token commands
        let refresh_token_handler = Arc::new(RefreshTokenCommandHandler::new(token_usecase.clone()));
        let revoke_token_handler = Arc::new(RevokeTokenCommandHandler::new(token_usecase.clone()));
        let revoke_all_tokens_handler = Arc::new(RevokeAllTokensCommandHandler::new(token_usecase));
        let token_error_mapper = Arc::new(TokenErrorMapper);

        builder = builder
            .register::<RefreshTokenCommand, _>("refresh_token".to_string(), refresh_token_handler, token_error_mapper.clone())
            .register::<RevokeTokenCommand, _>("revoke_token".to_string(), revoke_token_handler, token_error_mapper.clone())
            .register::<RevokeAllTokensCommand, _>("revoke_all_tokens".to_string(), revoke_all_tokens_handler, token_error_mapper);

        // Register user commands
        let get_user_handler = Arc::new(GetUserCommandHandler::new(user_usecase.clone()));
        let validate_token_handler = Arc::new(ValidateTokenCommandHandler::new(user_usecase));
        let user_error_mapper = Arc::new(UserErrorMapper);

        builder = builder
            .register::<GetUserCommand, _>("get_user".to_string(), get_user_handler, user_error_mapper.clone())
            .register::<ValidateTokenCommand, _>("validate_token".to_string(), validate_token_handler, user_error_mapper);

        // Register auth commands
        let signup_handler = Arc::new(SignupCommandHandler::new(auth_usecase.clone()));
        let password_login_handler = Arc::new(PasswordLoginCommandHandler::new(auth_usecase.clone()));
        let verify_email_handler = Arc::new(VerifyEmailCommandHandler::new(auth_usecase));
        let auth_error_mapper = Arc::new(AuthErrorMapper);

        builder = builder
            .register::<SignupCommand, _>("signup".to_string(), signup_handler, auth_error_mapper.clone())
            .register::<PasswordLoginCommand, _>("password_login".to_string(), password_login_handler, auth_error_mapper.clone())
            .register::<VerifyEmailCommand, _>("verify_email".to_string(), verify_email_handler, auth_error_mapper);

        builder.build()
    }

    /// Create an empty registry builder for custom command registration
    pub fn create_empty_builder() -> CommandRegistryBuilder {
        CommandRegistryBuilder::new()
    }

    /// Create a registry builder with only specific command groups
    pub fn create_builder_with_login(
        login_usecase: Arc<dyn LoginUseCase>,
    ) -> CommandRegistryBuilder {
        let login_handler = Arc::new(LoginCommandHandler::new(login_usecase.clone()));
        let login_start_url_handler = Arc::new(GenerateLoginStartUrlCommandHandler::new(login_usecase));
        let login_error_mapper = Arc::new(LoginErrorMapper);

        CommandRegistryBuilder::new()
            .register::<LoginCommand, _>("login".to_string(), login_handler, login_error_mapper.clone())
            .register::<GenerateLoginStartUrlCommand, _>("generate_login_start_url".to_string(), login_start_url_handler, login_error_mapper)
    }

    /// Create a registry builder with only auth commands
    pub fn create_builder_with_auth(
        auth_usecase: Arc<dyn AuthUseCase>,
    ) -> CommandRegistryBuilder {
        let signup_handler = Arc::new(SignupCommandHandler::new(auth_usecase.clone()));
        let password_login_handler = Arc::new(PasswordLoginCommandHandler::new(auth_usecase.clone()));
        let verify_email_handler = Arc::new(VerifyEmailCommandHandler::new(auth_usecase));
        let auth_error_mapper = Arc::new(AuthErrorMapper);

        CommandRegistryBuilder::new()
            .register::<SignupCommand, _>("signup".to_string(), signup_handler, auth_error_mapper.clone())
            .register::<PasswordLoginCommand, _>("password_login".to_string(), password_login_handler, auth_error_mapper.clone())
            .register::<VerifyEmailCommand, _>("verify_email".to_string(), verify_email_handler, auth_error_mapper)
    }

    /// Create a registry builder with only token commands
    pub fn create_builder_with_token(
        token_usecase: Arc<dyn TokenUseCase>,
    ) -> CommandRegistryBuilder {
        let refresh_token_handler = Arc::new(RefreshTokenCommandHandler::new(token_usecase.clone()));
        let revoke_token_handler = Arc::new(RevokeTokenCommandHandler::new(token_usecase.clone()));
        let revoke_all_tokens_handler = Arc::new(RevokeAllTokensCommandHandler::new(token_usecase));
        let token_error_mapper = Arc::new(TokenErrorMapper);

        CommandRegistryBuilder::new()
            .register::<RefreshTokenCommand, _>("refresh_token".to_string(), refresh_token_handler, token_error_mapper.clone())
            .register::<RevokeTokenCommand, _>("revoke_token".to_string(), revoke_token_handler, token_error_mapper.clone())
            .register::<RevokeAllTokensCommand, _>("revoke_all_tokens".to_string(), revoke_all_tokens_handler, token_error_mapper)
    }

    /// Create a registry builder with only user commands
    pub fn create_builder_with_user(
        user_usecase: Arc<dyn UserUseCase>,
    ) -> CommandRegistryBuilder {
        let get_user_handler = Arc::new(GetUserCommandHandler::new(user_usecase.clone()));
        let validate_token_handler = Arc::new(ValidateTokenCommandHandler::new(user_usecase));
        let user_error_mapper = Arc::new(UserErrorMapper);

        CommandRegistryBuilder::new()
            .register::<GetUserCommand, _>("get_user".to_string(), get_user_handler, user_error_mapper.clone())
            .register::<ValidateTokenCommand, _>("validate_token".to_string(), validate_token_handler, user_error_mapper)
    }

    /// Create a registry builder with only link provider commands
    pub fn create_builder_with_link_provider(
        link_provider_usecase: Arc<dyn LinkProviderUseCase>,
    ) -> CommandRegistryBuilder {
        let link_provider_handler = Arc::new(LinkProviderCommandHandler::new(link_provider_usecase.clone()));
        let link_provider_start_url_handler = Arc::new(GenerateLinkProviderStartUrlCommandHandler::new(link_provider_usecase));
        let link_provider_error_mapper = Arc::new(LinkProviderErrorMapper);

        CommandRegistryBuilder::new()
            .register::<LinkProviderCommand, _>("link_provider".to_string(), link_provider_handler, link_provider_error_mapper.clone())
            .register::<GenerateLinkProviderStartUrlCommand, _>("generate_link_provider_start_url".to_string(), link_provider_start_url_handler, link_provider_error_mapper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_empty_builder() {
        let builder = CommandRegistryFactory::create_empty_builder();
        let registry = builder.build();
        let command_types = registry.list_command_types();
        
        assert!(command_types.is_empty());
    }
} 