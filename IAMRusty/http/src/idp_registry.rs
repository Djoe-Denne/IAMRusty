//! IdP redirect helpers. Registry is injected per-router via axum `Extension`.

use iam_configuration::{IdpConfig, IdpRedirectFlow};

/// Redirect URI for `slug` and `flow`, if the connector is registered.
#[must_use]
pub fn redirect_uri_for(config: &IdpConfig, slug: &str, flow: IdpRedirectFlow) -> Option<String> {
    config.redirect_uri(slug, flow).map(str::to_string)
}
