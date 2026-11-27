//! Instance-local dependencies projected into IAM request extensions.

use std::sync::Arc;

use iam_application::usecase::{
    login::LoginUseCase, profile_email_verification::ProfileEmailVerification,
};

use rustycog::http::SessionLivenessChecker;

use crate::{
    handlers::session_checkpoint::SessionCheckpoint, oauth_browser::OAuthRouteContext,
    platform_user::PlatformIssuer, rate_limit::AuthRateLimiter,
};

/// Validated dependencies shared by every router of one IAM application.
pub struct IamHttpSecurityContext {
    platform_issuer: PlatformIssuer,
    rate_limiter: Arc<AuthRateLimiter>,
    oauth: Arc<OAuthRouteContext>,
    profile_email_verification: Arc<ProfileEmailVerification>,
    totp: Arc<dyn LoginUseCase>,
    session_liveness: Arc<dyn SessionLivenessChecker>,
    session_checkpoint: Option<Arc<SessionCheckpoint>>,
}

impl IamHttpSecurityContext {
    #[must_use]
    pub fn new(
        platform_issuer: PlatformIssuer,
        rate_limiter: Arc<AuthRateLimiter>,
        oauth: Arc<OAuthRouteContext>,
        profile_email_verification: Arc<ProfileEmailVerification>,
        totp: Arc<dyn LoginUseCase>,
        session_liveness: Arc<dyn SessionLivenessChecker>,
    ) -> Self {
        Self {
            platform_issuer,
            rate_limiter,
            oauth,
            profile_email_verification,
            totp,
            session_liveness,
            session_checkpoint: None,
        }
    }

    #[must_use]
    pub fn with_session_checkpoint(mut self, checkpoint: SessionCheckpoint) -> Self {
        self.session_checkpoint = Some(Arc::new(checkpoint));
        self
    }

    pub(crate) fn session_checkpoint(&self) -> Option<Arc<SessionCheckpoint>> {
        self.session_checkpoint.clone()
    }

    pub(crate) fn platform_issuer(&self) -> PlatformIssuer {
        self.platform_issuer.clone()
    }

    pub(crate) fn rate_limiter(&self) -> Arc<AuthRateLimiter> {
        self.rate_limiter.clone()
    }

    pub(crate) fn oauth(&self) -> Arc<OAuthRouteContext> {
        self.oauth.clone()
    }

    pub(crate) fn profile_email_verification(&self) -> Arc<ProfileEmailVerification> {
        self.profile_email_verification.clone()
    }

    pub(crate) fn totp(&self) -> Arc<dyn LoginUseCase> {
        self.totp.clone()
    }

    pub(crate) fn session_liveness(&self) -> Arc<dyn SessionLivenessChecker> {
        self.session_liveness.clone()
    }
}
