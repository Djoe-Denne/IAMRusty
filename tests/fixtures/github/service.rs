use wiremock::{Mock, MockServer, ResponseTemplate, matchers::{method, path, header, body_string_contains}};
use serde::Serialize;
use std::sync::Arc;
use super::resources::*;
use crate::fixtures::common::get_mock_server;

/// GitHub service for mocking GitHub OAuth endpoints
pub struct GitHubService {
    server: Arc<MockServer>,
}

impl GitHubService {
    /// Create a new GitHub service instance
    pub async fn new() -> Self {
        let server = get_mock_server().await;
        Self { server }
    }

    /// Get the base URL for GitHub API mocking
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Mock OAuth token exchange endpoint
    /// POST /login/oauth/access_token
    pub async fn oauth_token(
        &self,
        status_code: u16,
        request: GitHubTokenRequest,
        response: impl Serialize,
    ) -> &Self {
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .and(header("accept", "application/json"))
            .and(body_string_contains(&request.code))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json")
            )
            .mount(&*self.server)
            .await;

        self
    }

    /// Mock user profile endpoint
    /// GET /user
    pub async fn user_profile(
        &self,
        status_code: u16,
        request: GitHubUserRequest,
        response: impl Serialize,
    ) -> &Self {
        Mock::given(method("GET"))
            .and(path("/user"))
            .and(header("user-agent", &request.user_agent))
            .and(header("accept", &request.accept))
            .and(header("authorization", format!("token {}", request.access_token)))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json")
            )
            .mount(&*self.server)
            .await;

        self
    }

    /// Mock user emails endpoint
    /// GET /user/emails
    pub async fn user_emails(
        &self,
        status_code: u16,
        request: GitHubUserRequest,
        response: impl Serialize,
    ) -> &Self {
        Mock::given(method("GET"))
            .and(path("/user/emails"))
            .and(header("user-agent", &request.user_agent))
            .and(header("accept", &request.accept))
            .and(header("authorization", format!("token {}", request.access_token)))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json")
            )
            .mount(&*self.server)
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
                    .insert_header("content-type", "application/json")
            )
            .mount(&*self.server)
            .await;

        self
    }

    // Convenience methods for common scenarios

    /// Setup successful OAuth token exchange
    pub async fn setup_successful_token_exchange(&self) -> &Self {
        self.oauth_token(
            200,
            GitHubTokenRequest::valid(),
            GitHubTokenResponse::success(),
        ).await
    }

    /// Setup failed OAuth token exchange (invalid code)
    pub async fn setup_failed_token_exchange_invalid_code(&self) -> &Self {
        self.oauth_token(
            400,
            GitHubTokenRequest::invalid_code(),
            GitHubError::invalid_grant(),
        ).await
    }

    /// Setup failed OAuth token exchange (invalid client)
    pub async fn setup_failed_token_exchange_invalid_client(&self) -> &Self {
        self.oauth_token(
            401,
            GitHubTokenRequest::invalid_client(),
            GitHubError::invalid_client(),
        ).await
    }

    /// Setup successful user profile fetch for Arthur
    pub async fn setup_successful_user_profile_arthur(&self) -> &Self {
        self.user_profile(
            200,
            GitHubUserRequest::authenticated(),
            GitHubUser::arthur(),
        ).await
    }

    /// Setup successful user profile fetch for Bob
    pub async fn setup_successful_user_profile_bob(&self) -> &Self {
        self.user_profile(
            200,
            GitHubUserRequest::authenticated(),
            GitHubUser::bob(),
        ).await
    }

    /// Setup failed user profile fetch (unauthorized)
    pub async fn setup_failed_user_profile_unauthorized(&self) -> &Self {
        self.user_profile(
            401,
            GitHubUserRequest::invalid_token(),
            GitHubError::unauthorized(),
        ).await
    }

    /// Setup rate limit exceeded error
    pub async fn setup_rate_limit_exceeded(&self) -> &Self {
        self.user_profile(
            403,
            GitHubUserRequest::authenticated(),
            GitHubError::rate_limit_exceeded(),
        ).await
    }

    /// Setup server error
    pub async fn setup_server_error(&self) -> &Self {
        self.user_profile(
            500,
            GitHubUserRequest::authenticated(),
            GitHubError::server_error(),
        ).await
    }

    /// Reset all mocks (clear all mounted mocks)
    pub async fn reset(&self) {
        self.server.reset().await;
    }
}

// Note: Removed chain methods due to Serialize trait not being dyn compatible
// Use individual method calls instead for better type safety 