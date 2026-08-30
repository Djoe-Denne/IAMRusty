use super::{
    oauth_login::{
        GenerateOAuthStartUrlCommand, GenerateOAuthStartUrlCommandHandler, OAuthLoginCommand,
        OAuthLoginCommandHandler, OAuthLoginErrorMapper,
    },
    password_login::{
        AuthErrorMapper as PasswordLoginAuthErrorMapper, PasswordLoginCommand,
        PasswordLoginCommandHandler,
    },
    password_reset::{
        PasswordResetErrorMapper, RequestPasswordResetCommand, RequestPasswordResetCommandHandler,
        ResetPasswordAuthenticatedCommand, ResetPasswordAuthenticatedCommandHandler,
        ResetPasswordUnauthenticatedCommand, ResetPasswordUnauthenticatedCommandHandler,
        ValidateResetTokenCommand, ValidateResetTokenCommandHandler,
    },
    provider::{
        GenerateLinkProviderStartUrlCommand, GenerateLinkProviderStartUrlCommandHandler,
        GenerateRelinkProviderStartUrlCommand, GenerateRelinkProviderStartUrlCommandHandler,
        GetProviderTokenCommand, GetProviderTokenCommandHandler, LinkProviderCommand,
        LinkProviderCommandHandler, LinkProviderErrorMapper, ProviderErrorMapper,
        RelinkProviderCommand, RelinkProviderCommandHandler, RevokeProviderTokenCommand,
        RevokeProviderTokenCommandHandler,
    },
    registration::{
        CheckUsernameCommand, CheckUsernameCommandHandler, CompleteRegistrationCommand,
        CompleteRegistrationCommandHandler, RegistrationErrorMapper,
    },
    resend_verification_email::{
        ResendVerificationEmailCommand, ResendVerificationEmailCommandHandler,
    },
    signup::{AuthErrorMapper as SignupAuthErrorMapper, SignupCommand, SignupCommandHandler},
    token::{
        GetJwksCommand, GetJwksCommandHandler, RefreshTokenCommand, RefreshTokenCommandHandler,
        RevokeAllTokensCommand, RevokeAllTokensCommandHandler, RevokeTokenCommand,
        RevokeTokenCommandHandler, TokenErrorMapper,
    },
    user::{GetUserCommand, GetUserCommandHandler, UserErrorMapper},
    verify_email::{
        AuthErrorMapper as VerifyEmailAuthErrorMapper, VerifyEmailCommand,
        VerifyEmailCommandHandler,
    },
};
use crate::usecase::{
    link_provider::LinkProviderUseCase, login::LoginUseCase, oauth::OAuthUseCase,
    password_reset::PasswordResetUseCase, provider::ProviderUseCase,
    registration::RegistrationUseCase, token::TokenUseCase, user::UserUseCase,
};
use iam_configuration::CommandConfig;
use rustycog::command::{CommandRegistry, CommandRegistryBuilder};
use std::sync::Arc;

/// Factory for creating a command registry with all standard commands registered
pub struct CommandRegistryFactory;

pub struct IamRegistryUseCases {
    pub oauth: Arc<dyn OAuthUseCase>,
    pub link_provider: Arc<dyn LinkProviderUseCase>,
    pub provider: Arc<dyn ProviderUseCase>,
    pub token: Arc<dyn TokenUseCase>,
    pub user: Arc<dyn UserUseCase>,
    pub login_auth: Arc<dyn LoginUseCase>,
    pub registration: Arc<dyn RegistrationUseCase>,
    pub password_reset: Arc<dyn PasswordResetUseCase>,
}

impl CommandRegistryFactory {
    /// Create a command registry with all standard IAM commands registered
    #[must_use]
    pub fn create_iam_registry(
        usecases: IamRegistryUseCases,
        command_config: CommandConfig,
    ) -> CommandRegistry {
        // Create registry config from the loaded configuration
        let registry_config =
            rustycog::command::registry::RegistryConfig::from_retry_config(&command_config.retry);

        let builder = CommandRegistryBuilder::with_config(registry_config);
        let builder = Self::register_oauth_commands(builder, usecases.oauth);
        let builder = Self::register_link_provider_commands(builder, usecases.link_provider.clone());
        let builder = Self::register_provider_commands(builder, usecases.provider);
        let builder = Self::register_relink_commands(builder, usecases.link_provider);
        let builder = Self::register_token_commands(builder, usecases.token);
        let builder = Self::register_user_commands(builder, usecases.user);
        let builder = Self::register_auth_commands(builder, usecases.login_auth);
        let builder = Self::register_registration_commands(builder, usecases.registration);
        Self::register_password_reset_commands(builder, usecases.password_reset).build()
    }

    fn register_oauth_commands(
        builder: CommandRegistryBuilder,
        oauth: Arc<dyn OAuthUseCase>,
    ) -> CommandRegistryBuilder {
        let oauth_login_handler = Arc::new(OAuthLoginCommandHandler::new(oauth.clone()));
        let oauth_start_url_handler = Arc::new(GenerateOAuthStartUrlCommandHandler::new(oauth));
        let oauth_login_error_mapper = Arc::new(OAuthLoginErrorMapper);
        builder
            .register::<OAuthLoginCommand, _>(
                "oauth_login".to_string(),
                oauth_login_handler,
                oauth_login_error_mapper.clone(),
            )
            .register::<GenerateOAuthStartUrlCommand, _>(
                "generate_oauth_start_url".to_string(),
                oauth_start_url_handler,
                oauth_login_error_mapper,
            )
    }

    fn register_link_provider_commands(
        builder: CommandRegistryBuilder,
        link_provider: Arc<dyn LinkProviderUseCase>,
    ) -> CommandRegistryBuilder {
        let link_provider_handler = Arc::new(LinkProviderCommandHandler::new(link_provider.clone()));
        let link_provider_start_url_handler = Arc::new(
            GenerateLinkProviderStartUrlCommandHandler::new(link_provider),
        );
        let link_provider_error_mapper = Arc::new(LinkProviderErrorMapper);
        builder
            .register::<LinkProviderCommand, _>(
                "link_provider".to_string(),
                link_provider_handler,
                link_provider_error_mapper.clone(),
            )
            .register::<GenerateLinkProviderStartUrlCommand, _>(
                "generate_link_provider_start_url".to_string(),
                link_provider_start_url_handler,
                link_provider_error_mapper,
            )
    }

    fn register_provider_commands(
        builder: CommandRegistryBuilder,
        provider: Arc<dyn ProviderUseCase>,
    ) -> CommandRegistryBuilder {
        let get_provider_token_handler =
            Arc::new(GetProviderTokenCommandHandler::new(provider.clone()));
        let revoke_provider_token_handler =
            Arc::new(RevokeProviderTokenCommandHandler::new(provider));
        let provider_error_mapper = Arc::new(ProviderErrorMapper);
        builder
            .register::<GetProviderTokenCommand, _>(
                "get_provider_token".to_string(),
                get_provider_token_handler,
                provider_error_mapper.clone(),
            )
            .register::<RevokeProviderTokenCommand, _>(
                "revoke_provider_token".to_string(),
                revoke_provider_token_handler,
                provider_error_mapper,
            )
    }

    fn register_relink_commands(
        builder: CommandRegistryBuilder,
        link_provider: Arc<dyn LinkProviderUseCase>,
    ) -> CommandRegistryBuilder {
        let relink_provider_handler =
            Arc::new(RelinkProviderCommandHandler::new(link_provider.clone()));
        let relink_provider_start_url_handler = Arc::new(
            GenerateRelinkProviderStartUrlCommandHandler::new(link_provider),
        );
        let relink_provider_error_mapper = Arc::new(LinkProviderErrorMapper);
        builder
            .register::<RelinkProviderCommand, _>(
                "relink_provider".to_string(),
                relink_provider_handler,
                relink_provider_error_mapper.clone(),
            )
            .register::<GenerateRelinkProviderStartUrlCommand, _>(
                "generate_relink_provider_start_url".to_string(),
                relink_provider_start_url_handler,
                relink_provider_error_mapper,
            )
    }

    fn register_token_commands(
        builder: CommandRegistryBuilder,
        token: Arc<dyn TokenUseCase>,
    ) -> CommandRegistryBuilder {
        let refresh_token_handler = Arc::new(RefreshTokenCommandHandler::new(token.clone()));
        let revoke_token_handler = Arc::new(RevokeTokenCommandHandler::new(token.clone()));
        let revoke_all_tokens_handler = Arc::new(RevokeAllTokensCommandHandler::new(token.clone()));
        let get_jwks_handler = Arc::new(GetJwksCommandHandler::new(token));
        let token_error_mapper = Arc::new(TokenErrorMapper);
        builder
            .register::<RefreshTokenCommand, _>(
                "refresh_token".to_string(),
                refresh_token_handler,
                token_error_mapper.clone(),
            )
            .register::<RevokeTokenCommand, _>(
                "revoke_token".to_string(),
                revoke_token_handler,
                token_error_mapper.clone(),
            )
            .register::<RevokeAllTokensCommand, _>(
                "revoke_all_tokens".to_string(),
                revoke_all_tokens_handler,
                token_error_mapper.clone(),
            )
            .register::<GetJwksCommand, _>(
                "get_jwks".to_string(),
                get_jwks_handler,
                token_error_mapper,
            )
    }

    fn register_user_commands(
        builder: CommandRegistryBuilder,
        user: Arc<dyn UserUseCase>,
    ) -> CommandRegistryBuilder {
        let get_user_handler = Arc::new(GetUserCommandHandler::new(user));
        builder.register::<GetUserCommand, _>(
            "get_user".to_string(),
            get_user_handler,
            Arc::new(UserErrorMapper),
        )
    }

    fn register_auth_commands(
        builder: CommandRegistryBuilder,
        login_auth: Arc<dyn LoginUseCase>,
    ) -> CommandRegistryBuilder {
        let signup_handler = Arc::new(SignupCommandHandler::new(login_auth.clone()));
        let password_login_handler = Arc::new(PasswordLoginCommandHandler::new(login_auth.clone()));
        let verify_email_handler = Arc::new(VerifyEmailCommandHandler::new(login_auth.clone()));
        let signup_auth_error_mapper = Arc::new(SignupAuthErrorMapper);
        let password_login_auth_error_mapper = Arc::new(PasswordLoginAuthErrorMapper);
        let verify_email_auth_error_mapper = Arc::new(VerifyEmailAuthErrorMapper);
        let resend_verification_email_handler =
            Arc::new(ResendVerificationEmailCommandHandler::new(login_auth));
        builder
            .register::<SignupCommand, _>(
                "signup".to_string(),
                signup_handler,
                signup_auth_error_mapper,
            )
            .register::<PasswordLoginCommand, _>(
                "password_login".to_string(),
                password_login_handler,
                password_login_auth_error_mapper,
            )
            .register::<VerifyEmailCommand, _>(
                "verify_email".to_string(),
                verify_email_handler,
                verify_email_auth_error_mapper.clone(),
            )
            .register::<ResendVerificationEmailCommand, _>(
                "resend_verification_email".to_string(),
                resend_verification_email_handler,
                verify_email_auth_error_mapper,
            )
    }

    fn register_registration_commands(
        builder: CommandRegistryBuilder,
        registration: Arc<dyn RegistrationUseCase>,
    ) -> CommandRegistryBuilder {
        let complete_registration_handler =
            Arc::new(CompleteRegistrationCommandHandler::new(registration.clone()));
        let check_username_handler = Arc::new(CheckUsernameCommandHandler::new(registration));
        let registration_error_mapper = Arc::new(RegistrationErrorMapper);
        builder
            .register::<CompleteRegistrationCommand, _>(
                "complete_registration".to_string(),
                complete_registration_handler,
                registration_error_mapper.clone(),
            )
            .register::<CheckUsernameCommand, _>(
                "check_username".to_string(),
                check_username_handler,
                registration_error_mapper,
            )
    }

    fn register_password_reset_commands(
        builder: CommandRegistryBuilder,
        password_reset: Arc<dyn PasswordResetUseCase>,
    ) -> CommandRegistryBuilder {
        let request_password_reset_handler =
            Arc::new(RequestPasswordResetCommandHandler::new(password_reset.clone()));
        let validate_reset_token_handler =
            Arc::new(ValidateResetTokenCommandHandler::new(password_reset.clone()));
        let reset_password_unauthenticated_handler = Arc::new(
            ResetPasswordUnauthenticatedCommandHandler::new(password_reset.clone()),
        );
        let reset_password_authenticated_handler =
            Arc::new(ResetPasswordAuthenticatedCommandHandler::new(password_reset));
        let password_reset_error_mapper = Arc::new(PasswordResetErrorMapper);
        builder
            .register::<RequestPasswordResetCommand, _>(
                "request_password_reset".to_string(),
                request_password_reset_handler,
                password_reset_error_mapper.clone(),
            )
            .register::<ValidateResetTokenCommand, _>(
                "validate_reset_token".to_string(),
                validate_reset_token_handler,
                password_reset_error_mapper.clone(),
            )
            .register::<ResetPasswordUnauthenticatedCommand, _>(
                "reset_password_unauthenticated".to_string(),
                reset_password_unauthenticated_handler,
                password_reset_error_mapper.clone(),
            )
            .register::<ResetPasswordAuthenticatedCommand, _>(
                "reset_password_authenticated".to_string(),
                reset_password_authenticated_handler,
                password_reset_error_mapper,
            )
    }

    /// Create an empty registry builder for custom command registration
    #[must_use]
    pub fn create_empty_builder() -> CommandRegistryBuilder {
        CommandRegistryBuilder::new()
    }

    /// Create a registry builder with only OAuth login commands
    pub fn create_builder_with_oauth_login(
        oauth_usecase: Arc<dyn OAuthUseCase>,
    ) -> CommandRegistryBuilder {
        let oauth_login_handler = Arc::new(OAuthLoginCommandHandler::new(oauth_usecase.clone()));
        let oauth_start_url_handler =
            Arc::new(GenerateOAuthStartUrlCommandHandler::new(oauth_usecase));
        let oauth_login_error_mapper = Arc::new(OAuthLoginErrorMapper);

        CommandRegistryBuilder::new()
            .register::<OAuthLoginCommand, _>(
                "oauth_login".to_string(),
                oauth_login_handler,
                oauth_login_error_mapper.clone(),
            )
            .register::<GenerateOAuthStartUrlCommand, _>(
                "generate_oauth_start_url".to_string(),
                oauth_start_url_handler,
                oauth_login_error_mapper,
            )
    }

    /// Create a registry builder with only auth commands
    pub fn create_builder_with_auth(
        login_auth_usecase: Arc<dyn LoginUseCase>,
    ) -> CommandRegistryBuilder {
        let signup_handler = Arc::new(SignupCommandHandler::new(login_auth_usecase.clone()));
        let password_login_handler =
            Arc::new(PasswordLoginCommandHandler::new(login_auth_usecase.clone()));
        let verify_email_handler =
            Arc::new(VerifyEmailCommandHandler::new(login_auth_usecase.clone()));
        let signup_auth_error_mapper = Arc::new(SignupAuthErrorMapper);
        let password_login_auth_error_mapper = Arc::new(PasswordLoginAuthErrorMapper);
        let verify_email_auth_error_mapper = Arc::new(VerifyEmailAuthErrorMapper);

        CommandRegistryBuilder::new()
            .register::<SignupCommand, _>(
                "signup".to_string(),
                signup_handler,
                signup_auth_error_mapper,
            )
            .register::<PasswordLoginCommand, _>(
                "password_login".to_string(),
                password_login_handler,
                password_login_auth_error_mapper,
            )
            .register::<VerifyEmailCommand, _>(
                "verify_email".to_string(),
                verify_email_handler,
                verify_email_auth_error_mapper,
            )
    }

    /// Create a registry builder with only token commands
    pub fn create_builder_with_token(
        token_usecase: Arc<dyn TokenUseCase>,
    ) -> CommandRegistryBuilder {
        let refresh_token_handler =
            Arc::new(RefreshTokenCommandHandler::new(token_usecase.clone()));
        let revoke_token_handler = Arc::new(RevokeTokenCommandHandler::new(token_usecase.clone()));
        let revoke_all_tokens_handler = Arc::new(RevokeAllTokensCommandHandler::new(token_usecase));
        let token_error_mapper = Arc::new(TokenErrorMapper);

        CommandRegistryBuilder::new()
            .register::<RefreshTokenCommand, _>(
                "refresh_token".to_string(),
                refresh_token_handler,
                token_error_mapper.clone(),
            )
            .register::<RevokeTokenCommand, _>(
                "revoke_token".to_string(),
                revoke_token_handler,
                token_error_mapper.clone(),
            )
            .register::<RevokeAllTokensCommand, _>(
                "revoke_all_tokens".to_string(),
                revoke_all_tokens_handler,
                token_error_mapper,
            )
    }

    /// Create a registry builder with only user commands
    pub fn create_builder_with_user(user_usecase: Arc<dyn UserUseCase>) -> CommandRegistryBuilder {
        let get_user_handler = Arc::new(GetUserCommandHandler::new(user_usecase.clone()));
        let user_error_mapper = Arc::new(UserErrorMapper);

        CommandRegistryBuilder::new().register::<GetUserCommand, _>(
            "get_user".to_string(),
            get_user_handler,
            user_error_mapper,
        )
    }

    /// Create a registry builder with only link provider commands
    pub fn create_builder_with_link_provider(
        link_provider_usecase: Arc<dyn LinkProviderUseCase>,
    ) -> CommandRegistryBuilder {
        let link_provider_handler = Arc::new(LinkProviderCommandHandler::new(
            link_provider_usecase.clone(),
        ));
        let link_provider_start_url_handler = Arc::new(
            GenerateLinkProviderStartUrlCommandHandler::new(link_provider_usecase),
        );
        let link_provider_error_mapper = Arc::new(LinkProviderErrorMapper);

        CommandRegistryBuilder::new()
            .register::<LinkProviderCommand, _>(
                "link_provider".to_string(),
                link_provider_handler,
                link_provider_error_mapper.clone(),
            )
            .register::<GenerateLinkProviderStartUrlCommand, _>(
                "generate_link_provider_start_url".to_string(),
                link_provider_start_url_handler,
                link_provider_error_mapper,
            )
    }

    /// Create a registry builder with only provider token commands
    pub fn create_builder_with_provider(
        provider_usecase: Arc<dyn ProviderUseCase>,
    ) -> CommandRegistryBuilder {
        let get_provider_token_handler = Arc::new(GetProviderTokenCommandHandler::new(
            provider_usecase.clone(),
        ));
        let revoke_provider_token_handler =
            Arc::new(RevokeProviderTokenCommandHandler::new(provider_usecase));
        let provider_error_mapper = Arc::new(ProviderErrorMapper);

        CommandRegistryBuilder::new()
            .register::<GetProviderTokenCommand, _>(
                "get_provider_token".to_string(),
                get_provider_token_handler,
                provider_error_mapper.clone(),
            )
            .register::<RevokeProviderTokenCommand, _>(
                "revoke_provider_token".to_string(),
                revoke_provider_token_handler,
                provider_error_mapper,
            )
    }

    /// Create a registry builder with only relink provider commands
    pub fn create_builder_with_relink_provider(
        link_provider_usecase: Arc<dyn LinkProviderUseCase>,
    ) -> CommandRegistryBuilder {
        let relink_provider_handler = Arc::new(RelinkProviderCommandHandler::new(
            link_provider_usecase.clone(),
        ));
        let relink_provider_start_url_handler = Arc::new(
            GenerateRelinkProviderStartUrlCommandHandler::new(link_provider_usecase),
        );
        let relink_provider_error_mapper = Arc::new(LinkProviderErrorMapper);

        CommandRegistryBuilder::new()
            .register::<RelinkProviderCommand, _>(
                "relink_provider".to_string(),
                relink_provider_handler,
                relink_provider_error_mapper.clone(),
            )
            .register::<GenerateRelinkProviderStartUrlCommand, _>(
                "generate_relink_provider_start_url".to_string(),
                relink_provider_start_url_handler,
                relink_provider_error_mapper,
            )
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
