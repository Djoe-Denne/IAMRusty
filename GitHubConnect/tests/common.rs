//! Common test helpers for GitHub Connect (in-process `axum-test`, no Postgres).

#![allow(dead_code, missing_docs, unused_imports)]

#[path = "fixtures/mod.rs"]
pub mod fixtures;

use github_connect_configuration::{
    AppConfig, AuthConfig, GitHubConfig, JwtAuthConfig, LoggingConfig, ServerConfig,
};
use github_connect_setup::Application;

pub use fixtures::github::GitHubFixtures;

/// HMAC secret used by GitHub Connect integration tests.
pub const TEST_HMAC_SECRET: &str = "github-connect-test-hmac";
/// Allowlisted login callback (matches IAM test.toml).
pub const ALLOWED_REDIRECT: &str = "http://127.0.0.1:8081/iam/api/auth/github/callback";

/// Build typed test config pointing GitHub vendor URLs at `WireMock`.
#[must_use]
pub fn test_config(github_base: &str) -> AppConfig {
    let base = github_base.trim_end_matches('/');
    AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_owned(),
            port: 0,
            tls_enabled: false,
            ..ServerConfig::default()
        },
        auth: AuthConfig {
            jwt: JwtAuthConfig {
                hs256_secret: Some("rustycog-test-hs256-secret".to_owned()),
                issuer: Some("iamrusty".to_owned()),
                audience: Some("aiforall".to_owned()),
            },
        },
        logging: LoggingConfig {
            level: "debug".to_owned(),
            ..LoggingConfig::default()
        },
        github: GitHubConfig {
            client_id: "test_github_client_id".to_owned(),
            client_secret: "test_github_client_secret".to_owned(),
            auth_url: format!("{base}/login/oauth/authorize"),
            token_url: format!("{base}/login/oauth/access_token"),
            user_url: format!("{base}/user"),
            hmac_secret: TEST_HMAC_SECRET.to_owned(),
            redirect_uris: vec![
                ALLOWED_REDIRECT.to_owned(),
                "http://127.0.0.1:8081/iam/api/auth/github/relink-callback".to_owned(),
            ],
        },
    }
}

/// Boot the prefixed router for `axum-test`.
///
/// # Errors
///
/// Returns an error if application initialization fails.
pub fn boot_app(github_base: &str) -> anyhow::Result<Application> {
    Application::new(test_config(github_base))
}
