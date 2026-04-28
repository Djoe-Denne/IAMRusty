use super::resources::*;
use rustycog_testing::wiremock::MockServerFixture;
use serde::Serialize;
use std::sync::Arc;
use wiremock::{
    matchers::{body_string_contains, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

/// GitLab service for mocking GitLab OAuth endpoints
pub struct GitLabService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture, // Keeps the fixture alive for automatic cleanup
}

impl GitLabService {
    /// Create a new GitLab service instance with automatic mock cleanup
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();

        Self {
            server,
            _fixture: fixture,
        }
    }

    /// Get the base URL for GitLab API mocking
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Manual reset of all mocks (also happens automatically when service is dropped)
    pub async fn reset(&self) {
        self._fixture.reset().await;
    }

    /// Mock OAuth token exchange endpoint
    /// POST /oauth/token
    pub async fn oauth_token(
        &self,
        status_code: u16,
        request: GitLabTokenRequest,
        response: impl Serialize,
    ) -> &Self {
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(header("content-type", "application/x-www-form-urlencoded"))
            .and(body_string_contains(&request.code))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&self.server)
            .await;

        self
    }

    /// Mock user profile endpoint
    /// GET /api/v4/user
    pub async fn user_profile(
        &self,
        status_code: u16,
        request: GitLabUserRequest,
        response: impl Serialize,
    ) -> &Self {
        Mock::given(method("GET"))
            .and(path("/api/v4/user"))
            .and(header("user-agent", &request.user_agent))
            .and(header(
                "authorization",
                format!("Bearer {}", request.access_token),
            ))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&self.server)
            .await;

        self
    }

    /// Mock any custom endpoint with flexible matching
    pub async fn custom_endpoint(
        &self,
        method_name: &str,
        path_pattern: &str,
        status_code: u16,
        response: impl Serialize,
    ) -> &Self {
        Mock::given(method(method_name))
            .and(path(path_pattern))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&self.server)
            .await;

        self
    }

    // Convenience methods for common scenarios

    /// Setup successful OAuth token exchange
    pub async fn setup_successful_token_exchange(&self) -> &Self {
        self.oauth_token(
            200,
            GitLabTokenRequest::valid(),
            GitLabTokenResponse::success(),
        )
        .await
    }

    /// Setup failed OAuth token exchange (invalid code)
    pub async fn setup_failed_token_exchange_invalid_code(&self) -> &Self {
        self.oauth_token(
            400,
            GitLabTokenRequest::invalid_code(),
            GitLabError::invalid_grant(),
        )
        .await
    }

    /// Setup failed OAuth token exchange (invalid client)
    pub async fn setup_failed_token_exchange_invalid_client(&self) -> &Self {
        self.oauth_token(
            401,
            GitLabTokenRequest::invalid_client(),
            GitLabError::invalid_client(),
        )
        .await
    }

    /// Setup successful user profile fetch for Alice
    pub async fn setup_successful_user_profile_alice(&self) -> &Self {
        self.user_profile(200, GitLabUserRequest::authenticated(), GitLabUser::alice())
            .await
    }

    /// Setup successful user profile fetch for Charlie
    pub async fn setup_successful_user_profile_charlie(&self) -> &Self {
        self.user_profile(
            200,
            GitLabUserRequest::authenticated(),
            GitLabUser::charlie(),
        )
        .await
    }

    /// Setup failed user profile fetch (unauthorized)
    pub async fn setup_failed_user_profile_unauthorized(&self) -> &Self {
        self.user_profile(
            401,
            GitLabUserRequest::invalid_token(),
            GitLabError::unauthorized(),
        )
        .await
    }

    /// Setup forbidden error (insufficient scope)
    pub async fn setup_forbidden_error(&self) -> &Self {
        self.user_profile(
            403,
            GitLabUserRequest::authenticated(),
            GitLabError::forbidden(),
        )
        .await
    }

    /// Setup rate limit exceeded error
    pub async fn setup_rate_limit_exceeded(&self) -> &Self {
        self.user_profile(
            429,
            GitLabUserRequest::authenticated(),
            GitLabError::rate_limit_exceeded(),
        )
        .await
    }

    /// Setup server error
    pub async fn setup_server_error(&self) -> &Self {
        self.user_profile(
            500,
            GitLabUserRequest::authenticated(),
            GitLabError::server_error(),
        )
        .await
    }
}

// Note: Automatic cleanup happens when GitLabService is dropped via the MockServerFixture
