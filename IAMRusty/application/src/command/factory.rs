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
        RequestPasswordResetCommand, RequestPasswordResetCommandHandler,
        ValidateResetTokenCommand, ValidateResetTokenCommandHandler,
        ResetPasswordUnauthenticatedCommand, ResetPasswordUnauthenticatedCommandHandler,
        ResetPasswordAuthenticatedCommand, ResetPasswordAuthenticatedCommandHandler,
        PasswordResetErrorMapper,
    },
    provider::{
        GenerateLinkProviderStartUrlCommand, GenerateLinkProviderStartUrlCommandHandler,
        GetProviderTokenCommand, GetProviderTokenCommandHandler, LinkProviderCommand,
        LinkProviderCommandHandler, LinkProviderErrorMapper, ProviderErrorMapper,
        RevokeProviderTokenCommand, RevokeProviderTokenCommandHandler,
        RelinkProviderCommand, RelinkProviderCommandHandler,
        GenerateRelinkProviderStartUrlCommand, GenerateRelinkProviderStartUrlCommandHandler,
    },
    registration::{
        CheckUsernameCommand, CheckUsernameCommandHandler, CompleteRegistrationCommand,
        CompleteRegistrationCommandHandler, RegistrationErrorMapper,
    },
    signup::{AuthErrorMapper as SignupAuthErrorMapper, SignupCommand, SignupCommandHandler},
    token::{
        GetJwksCommand, GetJwksCommandHandler, RefreshTokenCommand, RefreshTokenCommandHandler, 
        RevokeAllTokensCommand, RevokeAllTokensCommandHandler, RevokeTokenCommand, 
        RevokeTokenCommandHandler, TokenErrorMapper,
    },
    user::{
        GetUserCommand, GetUserCommandHandler, UserErrorMapper, ValidateTokenCommand,
        ValidateTokenCommandHandler,
    },
    verify_email::{
        AuthErrorMapper as VerifyEmailAuthErrorMapper, VerifyEmailCommand,
        VerifyEmailCommandHandler,
    },
};
use crate::usecase::{
    link_provider::LinkProviderUseCase, login::LoginUseCase, oauth::OAuthUseCase,
    password_reset::PasswordResetUseCase, provider::ProviderUseCase, registration::RegistrationUseCase, token::TokenUseCase,
    user::UserUseCase,
};
use rustycog_command::{CommandRegistry, CommandRegistryBuilder};
use std::sync::Arc;

/// Factory for creating a command registry with all standard commands registered
pub struct CommandRegistryFactory;

impl CommandRegistryFactory {
    /// Create a command registry with all standard IAM commands registered
    pub fn create_iam_registry(
        oauth_usecase: Arc<dyn OAuthUseCase>,
        link_provider_usecase: Arc<dyn LinkProviderUseCase>,
        provider_usecase: Arc<dyn ProviderUseCase>,
        token_usecase: Arc<dyn TokenUseCase>,
        user_usecase: Arc<dyn UserUseCase>,
        login_auth_usecase: Arc<dyn LoginUseCase>,
        registration_usecase: Arc<dyn RegistrationUseCase>,
        password_reset_usecase: Arc<dyn PasswordResetUseCase>,
    ) -> CommandRegistry {
        let mut builder = CommandRegistryBuilder::new();

        // Register OAuth login commands
        let oauth_login_handler = Arc::new(OAuthLoginCommandHandler::new(oauth_usecase.clone()));
        let oauth_start_url_handler =
            Arc::new(GenerateOAuthStartUrlCommandHandler::new(oauth_usecase));
        let oauth_login_error_mapper = Arc::new(OAuthLoginErrorMapper);

        builder = builder
            .register::<OAuthLoginCommand, _>(
                "oauth_login".to_string(),
                oauth_login_handler,
                oauth_login_error_mapper.clone(),
            )
            .register::<GenerateOAuthStartUrlCommand, _>(
                "generate_oauth_start_url".to_string(),
                oauth_start_url_handler,
                oauth_login_error_mapper,
            );

        // Register link provider commands
        let link_provider_handler = Arc::new(LinkProviderCommandHandler::new(
            link_provider_usecase.clone(),
        ));
        let link_provider_start_url_handler = Arc::new(
            GenerateLinkProviderStartUrlCommandHandler::new(link_provider_usecase.clone()),
        );
        let link_provider_error_mapper = Arc::new(LinkProviderErrorMapper);

        builder = builder
            .register::<LinkProviderCommand, _>(
                "link_provider".to_string(),
                link_provider_handler,
                link_provider_error_mapper.clone(),
            )
            .register::<GenerateLinkProviderStartUrlCommand, _>(
                "generate_link_provider_start_url".to_string(),
                link_provider_start_url_handler,
                link_provider_error_mapper,
            );

        // Register provider token commands
        let get_provider_token_handler =
            Arc::new(GetProviderTokenCommandHandler::new(provider_usecase.clone()));
        let revoke_provider_token_handler =
            Arc::new(RevokeProviderTokenCommandHandler::new(provider_usecase));
        let provider_error_mapper = Arc::new(ProviderErrorMapper);

        builder = builder
            .register::<GetProviderTokenCommand, _>(
                "get_provider_token".to_string(),
                get_provider_token_handler,
                provider_error_mapper.clone(),
            )
            .register::<RevokeProviderTokenCommand, _>(
                "revoke_provider_token".to_string(),
                revoke_provider_token_handler,
                provider_error_mapper,
            );

        // Register relink provider commands
        let relink_provider_handler = Arc::new(RelinkProviderCommandHandler::new(
            link_provider_usecase.clone(),
        ));
        let relink_provider_start_url_handler = Arc::new(
            GenerateRelinkProviderStartUrlCommandHandler::new(link_provider_usecase.clone()),
        );
        let relink_provider_error_mapper = Arc::new(LinkProviderErrorMapper);

        builder = builder
            .register::<RelinkProviderCommand, _>(
                "relink_provider".to_string(),
                relink_provider_handler,
                relink_provider_error_mapper.clone(),
            )
            .register::<GenerateRelinkProviderStartUrlCommand, _>(
                "generate_relink_provider_start_url".to_string(),
                relink_provider_start_url_handler,
                relink_provider_error_mapper,
            );

        // Register token commands
        let refresh_token_handler =
            Arc::new(RefreshTokenCommandHandler::new(token_usecase.clone()));
        let revoke_token_handler = Arc::new(RevokeTokenCommandHandler::new(token_usecase.clone()));
        let revoke_all_tokens_handler = Arc::new(RevokeAllTokensCommandHandler::new(token_usecase.clone()));
        let get_jwks_handler = Arc::new(GetJwksCommandHandler::new(token_usecase));
        let token_error_mapper = Arc::new(TokenErrorMapper);

        builder = builder
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
            );

        // Register user commands
        let get_user_handler = Arc::new(GetUserCommandHandler::new(user_usecase.clone()));
        let validate_token_handler = Arc::new(ValidateTokenCommandHandler::new(user_usecase));
        let user_error_mapper = Arc::new(UserErrorMapper);

        builder = builder
            .register::<GetUserCommand, _>(
                "get_user".to_string(),
                get_user_handler,
                user_error_mapper.clone(),
            )
            .register::<ValidateTokenCommand, _>(
                "validate_token".to_string(),
                validate_token_handler,
                user_error_mapper,
            );

        // Register auth commands
        let signup_handler = Arc::new(SignupCommandHandler::new(login_auth_usecase.clone()));
        let password_login_handler =
            Arc::new(PasswordLoginCommandHandler::new(login_auth_usecase.clone()));
        let verify_email_handler =
            Arc::new(VerifyEmailCommandHandler::new(login_auth_usecase.clone()));
        let signup_auth_error_mapper = Arc::new(SignupAuthErrorMapper);
        let password_login_auth_error_mapper = Arc::new(PasswordLoginAuthErrorMapper);
        let verify_email_auth_error_mapper = Arc::new(VerifyEmailAuthErrorMapper);

        builder = builder
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
                verify_email_handler.clone(),
                verify_email_auth_error_mapper.clone(),
            );

        // Register resend verification email command
        use super::resend_verification_email::{
            ResendVerificationEmailCommand, ResendVerificationEmailCommandHandler,
        };
        let resend_verification_email_handler = Arc::new(
            ResendVerificationEmailCommandHandler::new(login_auth_usecase),
        );

        builder = builder.register::<ResendVerificationEmailCommand, _>(
            "resend_verification_email".to_string(),
            resend_verification_email_handler,
            verify_email_auth_error_mapper,
        );

        // Register registration commands
        let complete_registration_handler = Arc::new(CompleteRegistrationCommandHandler::new(
            registration_usecase.clone(),
        ));
        let check_username_handler =
            Arc::new(CheckUsernameCommandHandler::new(registration_usecase));
        let registration_error_mapper = Arc::new(RegistrationErrorMapper);

        builder = builder
            .register::<CompleteRegistrationCommand, _>(
                "complete_registration".to_string(),
                complete_registration_handler,
                registration_error_mapper.clone(),
            )
            .register::<CheckUsernameCommand, _>(
                "check_username".to_string(),
                check_username_handler,
                registration_error_mapper,
            );

        // Register password reset commands
        let request_password_reset_handler = Arc::new(RequestPasswordResetCommandHandler::new(password_reset_usecase.clone()));
        let validate_reset_token_handler = Arc::new(ValidateResetTokenCommandHandler::new(password_reset_usecase.clone()));
        let reset_password_unauthenticated_handler = Arc::new(ResetPasswordUnauthenticatedCommandHandler::new(password_reset_usecase.clone()));
        let reset_password_authenticated_handler = Arc::new(ResetPasswordAuthenticatedCommandHandler::new(password_reset_usecase));
        let password_reset_error_mapper = Arc::new(PasswordResetErrorMapper);

        builder = builder
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
            );

        builder.build()
    }

    /// Create an empty registry builder for custom command registration
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
        let validate_token_handler = Arc::new(ValidateTokenCommandHandler::new(user_usecase));
        let user_error_mapper = Arc::new(UserErrorMapper);

        CommandRegistryBuilder::new()
            .register::<GetUserCommand, _>(
                "get_user".to_string(),
                get_user_handler,
                user_error_mapper.clone(),
            )
            .register::<ValidateTokenCommand, _>(
                "validate_token".to_string(),
                validate_token_handler,
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
        let get_provider_token_handler =
            Arc::new(GetProviderTokenCommandHandler::new(provider_usecase.clone()));
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
