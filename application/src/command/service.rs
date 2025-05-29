use super::{
    bus::CommandBus,
    login::{LoginCommand, GenerateLoginStartUrlCommand, LoginCommandHandler, GenerateLoginStartUrlCommandHandler},
    link_provider::{LinkProviderCommand, GenerateLinkProviderStartUrlCommand, LinkProviderCommandHandler, GenerateLinkProviderStartUrlCommandHandler},
    token::{RefreshTokenCommand, RevokeTokenCommand, RevokeAllTokensCommand, RefreshTokenCommandHandler, RevokeTokenCommandHandler, RevokeAllTokensCommandHandler},
    user::{GetUserCommand, ValidateTokenCommand, GetUserCommandHandler, ValidateTokenCommandHandler},
    signup::{SignupCommand, SignupCommandHandler},
    password_login::{PasswordLoginCommand, PasswordLoginCommandHandler},
    verify_email::{VerifyEmailCommand, VerifyEmailCommandHandler},
    CommandContext, CommandError,
};
use crate::usecase::{
    login::{LoginUseCase, LoginResponse},
    link_provider::{LinkProviderUseCase, LinkProviderResponse},
    token::{TokenUseCase, RefreshTokenResponse},
    user::{UserUseCase, UserProfile},
    auth::{AuthUseCase, SignupResponse, LoginResponse as AuthLoginResponse, VerifyEmailResponse},
};
use domain::entity::provider::Provider;
use std::sync::Arc;
use uuid::Uuid;

/// Command service that works with trait objects and properly uses CommandBus
pub struct DynCommandService {
    command_bus: Arc<CommandBus>,
    login_handler: Arc<LoginCommandHandler<dyn LoginUseCase>>,
    login_start_url_handler: Arc<GenerateLoginStartUrlCommandHandler<dyn LoginUseCase>>,
    link_provider_handler: Arc<LinkProviderCommandHandler<dyn LinkProviderUseCase>>,
    link_provider_start_url_handler: Arc<GenerateLinkProviderStartUrlCommandHandler<dyn LinkProviderUseCase>>,
    refresh_token_handler: Arc<RefreshTokenCommandHandler<dyn TokenUseCase>>,
    revoke_token_handler: Arc<RevokeTokenCommandHandler<dyn TokenUseCase>>,
    revoke_all_tokens_handler: Arc<RevokeAllTokensCommandHandler<dyn TokenUseCase>>,
    get_user_handler: Arc<GetUserCommandHandler<dyn UserUseCase>>,
    validate_token_handler: Arc<ValidateTokenCommandHandler<dyn UserUseCase>>,
    signup_handler: Arc<SignupCommandHandler<dyn AuthUseCase>>,
    password_login_handler: Arc<PasswordLoginCommandHandler<dyn AuthUseCase>>,
    verify_email_handler: Arc<VerifyEmailCommandHandler<dyn AuthUseCase>>,
}

impl DynCommandService {
    /// Create a new DynCommandService
    pub fn new(
        command_bus: Arc<CommandBus>,
        login_usecase: Arc<dyn LoginUseCase>,
        link_provider_usecase: Arc<dyn LinkProviderUseCase>,
        token_usecase: Arc<dyn TokenUseCase>,
        user_usecase: Arc<dyn UserUseCase>,
        auth_usecase: Arc<dyn AuthUseCase>,
    ) -> Self {
        let login_handler = Arc::new(LoginCommandHandler::new(login_usecase.clone()));
        let login_start_url_handler = Arc::new(GenerateLoginStartUrlCommandHandler::new(login_usecase));
        let link_provider_handler = Arc::new(LinkProviderCommandHandler::new(link_provider_usecase.clone()));
        let link_provider_start_url_handler = Arc::new(GenerateLinkProviderStartUrlCommandHandler::new(link_provider_usecase));
        let refresh_token_handler = Arc::new(RefreshTokenCommandHandler::new(token_usecase.clone()));
        let revoke_token_handler = Arc::new(RevokeTokenCommandHandler::new(token_usecase.clone()));
        let revoke_all_tokens_handler = Arc::new(RevokeAllTokensCommandHandler::new(token_usecase));
        let get_user_handler = Arc::new(GetUserCommandHandler::new(user_usecase.clone()));
        let validate_token_handler = Arc::new(ValidateTokenCommandHandler::new(user_usecase));
        let signup_handler = Arc::new(SignupCommandHandler::new(auth_usecase.clone()));
        let password_login_handler = Arc::new(PasswordLoginCommandHandler::new(auth_usecase.clone()));
        let verify_email_handler = Arc::new(VerifyEmailCommandHandler::new(auth_usecase));

        Self {
            command_bus,
            login_handler,
            login_start_url_handler,
            link_provider_handler,
            link_provider_start_url_handler,
            refresh_token_handler,
            revoke_token_handler,
            revoke_all_tokens_handler,
            get_user_handler,
            validate_token_handler,
            signup_handler,
            password_login_handler,
            verify_email_handler,
        }
    }

    /// Execute login command through CommandBus
    pub async fn login(
        &self,
        provider: Provider,
        code: String,
        redirect_uri: String,
        context: CommandContext,
    ) -> Result<LoginResponse, CommandError> {
        let command = LoginCommand::new(provider, code, redirect_uri);
        self.command_bus
            .execute(command, self.login_handler.clone(), context)
            .await
    }

    /// Generate login start URL through CommandBus
    pub async fn generate_login_start_url(
        &self,
        provider: Provider,
        context: CommandContext,
    ) -> Result<String, CommandError> {
        let command = GenerateLoginStartUrlCommand::new(provider);
        self.command_bus
            .execute(command, self.login_start_url_handler.clone(), context)
            .await
    }

    /// Execute link provider command through CommandBus
    pub async fn link_provider(
        &self,
        user_id: Uuid,
        provider: Provider,
        code: String,
        redirect_uri: String,
        context: CommandContext,
    ) -> Result<LinkProviderResponse, CommandError> {
        let command = LinkProviderCommand::new(user_id, provider, code, redirect_uri);
        self.command_bus
            .execute(command, self.link_provider_handler.clone(), context)
            .await
    }

    /// Generate link provider start URL through CommandBus
    pub async fn generate_link_provider_start_url(
        &self,
        provider: Provider,
        context: CommandContext,
    ) -> Result<String, CommandError> {
        let command = GenerateLinkProviderStartUrlCommand::new(provider);
        self.command_bus
            .execute(command, self.link_provider_start_url_handler.clone(), context)
            .await
    }

    /// Refresh token through CommandBus
    pub async fn refresh_token(
        &self,
        refresh_token: String,
        context: CommandContext,
    ) -> Result<RefreshTokenResponse, CommandError> {
        let command = RefreshTokenCommand::new(refresh_token);
        self.command_bus
            .execute(command, self.refresh_token_handler.clone(), context)
            .await
    }

    /// Revoke token through CommandBus
    pub async fn revoke_token(
        &self,
        refresh_token: String,
        context: CommandContext,
    ) -> Result<(), CommandError> {
        let command = RevokeTokenCommand::new(refresh_token);
        self.command_bus
            .execute(command, self.revoke_token_handler.clone(), context)
            .await
    }

    /// Revoke all tokens through CommandBus
    pub async fn revoke_all_tokens(
        &self,
        user_id: Uuid,
        context: CommandContext,
    ) -> Result<u64, CommandError> {
        let command = RevokeAllTokensCommand::new(user_id);
        self.command_bus
            .execute(command, self.revoke_all_tokens_handler.clone(), context)
            .await
    }

    /// Get user through CommandBus
    pub async fn get_user(
        &self,
        user_id: Uuid,
        context: CommandContext,
    ) -> Result<UserProfile, CommandError> {
        let command = GetUserCommand::new(user_id);
        self.command_bus
            .execute(command, self.get_user_handler.clone(), context)
            .await
    }

    /// Validate token through CommandBus
    pub async fn validate_token(
        &self,
        token: String,
        context: CommandContext,
    ) -> Result<Uuid, CommandError> {
        let command = ValidateTokenCommand::new(token);
        self.command_bus
            .execute(command, self.validate_token_handler.clone(), context)
            .await
    }

    /// Execute signup command through CommandBus
    pub async fn signup(
        &self,
        username: String,
        email: String,
        password: String,
        context: CommandContext,
    ) -> Result<SignupResponse, CommandError> {
        let command = SignupCommand::new(username, email, password);
        self.command_bus
            .execute(command, self.signup_handler.clone(), context)
            .await
    }

    /// Execute password login command through CommandBus
    pub async fn password_login(
        &self,
        email: String,
        password: String,
        context: CommandContext,
    ) -> Result<AuthLoginResponse, CommandError> {
        let command = PasswordLoginCommand::new(email, password);
        self.command_bus
            .execute(command, self.password_login_handler.clone(), context)
            .await
    }

    /// Execute email verification command through CommandBus
    pub async fn verify_email(
        &self,
        email: String,
        verification_token: String,
        context: CommandContext,
    ) -> Result<VerifyEmailResponse, CommandError> {
        let command = VerifyEmailCommand::new(email, verification_token);
        self.command_bus
            .execute(command, self.verify_email_handler.clone(), context)
            .await
    }
} 