use super::resources::*;
use rustycog_testing::wiremock::MockServerFixture;
use serde::Serialize;
use std::sync::Arc;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, header, method, path},
};

/// GitHub service for mocking GitHub OAuth endpoints
pub struct GitHubService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture, // Keeps the fixture alive for automatic cleanup
}

impl GitHubService {
    /// Create a new GitHub service instance with automatic mock cleanup
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();

        Self {
            server,
            _fixture: fixture,
        }
    }

    /// Get the base URL for GitHub API mocking
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Manual reset of all mocks (also happens automatically when service is dropped)
    pub async fn reset(&self) {
        self._fixture.reset().await;
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
            .and(header("content-type", "application/x-www-form-urlencoded"))
            .and(body_string_contains(&format!("code={}", &request.code)))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&*self.server)
            .await;

        self
    }

    /// Mock OAuth authorization endpoint
    /// GET /login/oauth/authorize
    pub async fn oauth_authorize(
        &self,
        status_code: u16,
        request: GitHubAuthRequest,
        redirect_location: Option<String>,
    ) -> &Self {
        let mut response_template = ResponseTemplate::new(status_code);

        if let Some(location) = redirect_location {
            response_template = response_template.insert_header("location", location);
        }

        Mock::given(method("GET"))
            .and(path("/login/oauth/authorize"))
            .and(wiremock::matchers::query_param(
                "client_id",
                &request.client_id,
            ))
            .and(wiremock::matchers::query_param(
                "redirect_uri",
                &request.redirect_uri,
            ))
            .and(wiremock::matchers::query_param("scope", &request.scope))
            .and(wiremock::matchers::query_param(
                "response_type",
                &request.response_type,
            ))
            // Don't match exact redirect_uri, scope, or state since they may vary
            .respond_with(response_template)
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
            .and(header(
                "authorization",
                format!("token {}", request.access_token),
            ))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json"),
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
            .and(header(
                "authorization",
                format!("token {}", request.access_token),
            ))
            .respond_with(
                ResponseTemplate::new(status_code)
                    .set_body_json(response)
                    .insert_header("content-type", "application/json"),
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
                    .insert_header("content-type", "application/json"),
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
        )
        .await
    }

    /// Setup successful OAuth authorization flow
    pub async fn setup_successful_oauth_authorization(&self) -> &Self {
        // Mock the authorization endpoint that redirects back with a code
        let callback_url = "http://127.0.0.1:8081/api/auth/github/callback?code=auth_code_from_github&state=login_state_67890";

        self.oauth_authorize(
            302, // Temporary redirect
            GitHubAuthRequest::login_flow(),
            Some(callback_url.to_string()),
        )
        .await
    }

    /// Setup failed OAuth token exchange (invalid code)
    pub async fn setup_failed_token_exchange_invalid_code(&self) -> &Self {
        self.oauth_token(
            400,
            GitHubTokenRequest::invalid_code(),
            GitHubError::invalid_grant(),
        )
        .await
    }

    /// Setup failed OAuth token exchange (invalid client)
    pub async fn setup_failed_token_exchange_invalid_client(&self) -> &Self {
        self.oauth_token(
            401,
            GitHubTokenRequest::invalid_client(),
            GitHubError::invalid_client(),
        )
        .await
    }

    /// Setup successful user profile fetch for Arthur
    pub async fn setup_successful_user_profile_arthur(&self) -> &Self {
        self.user_profile(
            200,
            GitHubUserRequest::authenticated(),
            GitHubUser::arthur(),
        )
        .await
    }

    /// Setup successful user profile fetch for Bob
    pub async fn setup_successful_user_profile_bob(&self) -> &Self {
        self.user_profile(200, GitHubUserRequest::authenticated(), GitHubUser::bob())
            .await
    }

    /// Setup failed user profile fetch (unauthorized)
    pub async fn setup_failed_user_profile_unauthorized(&self) -> &Self {
        self.user_profile(
            401,
            GitHubUserRequest::invalid_token(),
            GitHubError::unauthorized(),
        )
        .await
    }

    /// Setup rate limit exceeded error
    pub async fn setup_rate_limit_exceeded(&self) -> &Self {
        self.user_profile(
            403,
            GitHubUserRequest::authenticated(),
            GitHubError::rate_limit_exceeded(),
        )
        .await
    }

    /// Setup server error
    pub async fn setup_server_error(&self) -> &Self {
        self.user_profile(
            500,
            GitHubUserRequest::authenticated(),
            GitHubError::server_error(),
        )
        .await
    }
}

// Note: Automatic cleanup happens when GitHubService is dropped via the MockServerFixture
