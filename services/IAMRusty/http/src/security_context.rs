//! Instance-local dependencies projected into IAM request extensions.

use std::sync::Arc;

use crate::{
    oauth_browser::OAuthRouteContext, platform_user::PlatformIssuer, rate_limit::AuthRateLimiter,
};

/// Validated dependencies shared by every router of one IAM application.
pub struct IamHttpSecurityContext {
    platform_issuer: PlatformIssuer,
    rate_limiter: Arc<AuthRateLimiter>,
    oauth: Arc<OAuthRouteContext>,
}

impl IamHttpSecurityContext {
    #[must_use]
    pub const fn new(
        platform_issuer: PlatformIssuer,
        rate_limiter: Arc<AuthRateLimiter>,
        oauth: Arc<OAuthRouteContext>,
    ) -> Self {
        Self {
            platform_issuer,
            rate_limiter,
            oauth,
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
}
