use idp_connect_contract::dto::{ProviderTokens, ProviderUserProfile};

/// Shared HMAC secret for IAM test.toml `[[idp.connectors]]`.
pub const TEST_HMAC_SECRET: &str = "iam-idp-connect-test-hmac";

/// Successful token payload used by IAM callback tests.
#[must_use]
pub fn success_tokens() -> ProviderTokens {
    ProviderTokens {
        access_token: "idp_test_access_token".to_string(),
        refresh_token: None,
        expires_in: Some(3600),
    }
}

/// GitHub Arthur profile (matches previous GitHubFixtures).
#[must_use]
pub fn github_arthur() -> ProviderUserProfile {
    ProviderUserProfile {
        id: "12345".to_string(),
        username: "arthur".to_string(),
        email: Some("arthur@example.com".to_string()),
        avatar_url: Some("https://avatars.githubusercontent.com/u/12345?v=4".to_string()),
        email_verified: true,
    }
}

/// GitHub Bob profile.
#[must_use]
pub fn github_bob() -> ProviderUserProfile {
    ProviderUserProfile {
        id: "67890".to_string(),
        username: "bob".to_string(),
        email: Some("bob@example.com".to_string()),
        avatar_url: Some("https://avatars.githubusercontent.com/u/67890?v=4".to_string()),
        email_verified: true,
    }
}

/// GitLab Alice profile (matches previous GitLabFixtures).
#[must_use]
pub fn gitlab_alice() -> ProviderUserProfile {
    ProviderUserProfile {
        id: "54321".to_string(),
        username: "alice".to_string(),
        email: Some("alice@example.com".to_string()),
        avatar_url: Some(
            "https://gitlab.com/uploads/-/system/user/avatar/54321/avatar.png".to_string(),
        ),
        email_verified: true,
    }
}
