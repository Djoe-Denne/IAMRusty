//! Instance-local dependencies projected into IAM request extensions.

use std::sync::Arc;

use iam_application::usecase::{
    login::LoginUseCase, profile_email_verification::ProfileEmailVerification,
};

use crate::{
    oauth_browser::OAuthRouteContext, platform_user::PlatformIssuer, rate_limit::AuthRateLimiter,
};

/// Validated dependencies shared by every router of one IAM application.
pub struct IamHttpSecurityContext {
    platform_issuer: PlatformIssuer,
    rate_limiter: Arc<AuthRateLimiter>,
    oauth: Arc<OAuthRouteContext>,
    profile_email_verification: Arc<ProfileEmailVerification>,
    totp: Arc<dyn LoginUseCase>,
}

impl IamHttpSecurityContext {
    #[must_use]
    pub const fn new(
        platform_issuer: PlatformIssuer,
        rate_limiter: Arc<AuthRateLimiter>,
        oauth: Arc<OAuthRouteContext>,
        profile_email_verification: Arc<ProfileEmailVerification>,
        totp: Arc<dyn LoginUseCase>,
    ) -> Self {
        Self {
            platform_issuer,
            rate_limiter,
            oauth,
            profile_email_verification,
            totp,
        }
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
}
