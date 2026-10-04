//! Exact public authentication matrix for the mesh allowlist (service prefix included).
//! `{provider_name}` means one registered connector slug, not an `/iam/` wildcard.
//! GET routes also support HEAD implicitly in Axum. Browser callbacks require a
//! consumed, cookie-bound transaction in phase 2; public does not mean unchecked.
//! Relink callbacks are additional exact registry-derived exemptions (contract §9).

pub const PUBLIC_AUTH_ROUTES: &[(&str, &str)] = &[
    ("GET", "/iam/.well-known/jwks.json"),
    ("POST", "/iam/api/auth/signup"),
    ("POST", "/iam/api/auth/login"),
    ("GET", "/iam/api/auth/verify"),
    ("POST", "/iam/api/auth/resend-verification"),
    ("POST", "/iam/api/auth/complete-registration"),
    ("GET", "/iam/api/auth/username/check"),
    ("POST", "/iam/api/auth/password/reset-request"),
    ("POST", "/iam/api/auth/password/reset-validate"),
    ("POST", "/iam/api/auth/password/reset-confirm"),
    ("GET", "/iam/api/auth/{provider_name}/login"),
    ("GET", "/iam/api/auth/{provider_name}/callback"),
    ("POST", "/iam/api/token/refresh"),
];

/// Distinct probe exemptions: GET/HEAD only, never an auth-route prefix exception.
pub const PUBLIC_PROBE_PATHS: &[&str] = &[
    "/iam/health",
    "/iam/ready",
    "/github-connect/health",
    "/github-connect/ready",
    "/gitlab-connect/health",
    "/gitlab-connect/ready",
];

/// Exact additional paths only; never export a provider wildcard for relink.
/// GET also permits Axum's implicit HEAD. Consumers must enforce both methods exactly.
///
/// # Errors
/// Rejects invalid registry configuration rather than producing a permissive matrix.
pub fn public_relink_callback_paths(
    idp: &iam_configuration::IdpConfig,
) -> Result<Vec<String>, String> {
    idp.validate()?;
    idp.connectors
        .iter()
        .map(|connector| {
            let provider = iam_domain::entity::provider::Provider::parse_slug(&connector.id)
                .map_err(|_| "invalid public relink provider".to_string())?;
            Ok(format!(
                "/iam/api/auth/{}/relink-callback",
                provider.as_str()
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_matrix_has_no_account_mutation_or_internal_exemption() {
        assert_eq!(PUBLIC_AUTH_ROUTES.len(), 13);
        assert_eq!(PUBLIC_PROBE_PATHS.len(), 6);
        assert!(PUBLIC_PROBE_PATHS
            .iter()
            .all(|path| path.ends_with("/health") || path.ends_with("/ready")));
        for (_, path) in PUBLIC_AUTH_ROUTES {
            assert!(path.starts_with("/iam/"));
            assert!(!path.contains("/internal/") && !path.contains("/api/me"));
            assert!(
                !path.ends_with("/link")
                    && !path.contains("relink")
                    && !path.ends_with("reset-authenticated")
            );
        }
    }

    #[test]
    fn relink_exemptions_are_exact_validated_registry_paths() {
        let registry = iam_configuration::IdpConfig {
            connectors: ["github", "gitlab"]
                .into_iter()
                .map(|id| iam_configuration::IdpConnectorConfig {
                    id: id.into(),
                    base_url: format!("https://{id}-connect"),
                    hmac_secret: "isolated-test-hmac-secret".into(),
                    pkce_supported: false,
                    redirect_uris: vec![
                        format!("https://platform/iam/api/auth/{id}/callback"),
                        format!("https://platform/iam/api/auth/{id}/relink-callback"),
                    ],
                })
                .collect(),
        };
        assert_eq!(
            public_relink_callback_paths(&registry).unwrap(),
            [
                "/iam/api/auth/github/relink-callback",
                "/iam/api/auth/gitlab/relink-callback"
            ]
        );
        assert!(public_relink_callback_paths(&iam_configuration::IdpConfig::default()).is_err());
        let mut invalid = registry;
        invalid.connectors[0].id = "github/../../mesh/echo-headers".into();
        assert!(public_relink_callback_paths(&invalid).is_err());
    }
}
