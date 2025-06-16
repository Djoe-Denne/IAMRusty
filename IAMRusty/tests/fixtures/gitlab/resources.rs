use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// GitLab user data structure matching the API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabUser {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

impl GitLabUser {
    /// Create a builder for custom user data
    pub fn create() -> GitLabUserBuilder {
        GitLabUserBuilder::default()
    }

    /// Pre-built user: Alice (test user)
    pub fn alice() -> Self {
        Self {
            id: 54321,
            username: "alice".to_string(),
            email: Some("alice@example.com".to_string()),
            avatar_url: Some(
                "https://gitlab.com/uploads/-/system/user/avatar/54321/avatar.png".to_string(),
            ),
        }
    }

    /// Pre-built user: Charlie (test user)
    pub fn charlie() -> Self {
        Self {
            id: 98765,
            username: "charlie".to_string(),
            email: Some("charlie@example.com".to_string()),
            avatar_url: Some(
                "https://gitlab.com/uploads/-/system/user/avatar/98765/avatar.png".to_string(),
            ),
        }
    }

    /// User without email (privacy settings)
    pub fn no_email_user() -> Self {
        Self {
            id: 11111,
            username: "private_gitlab_user".to_string(),
            email: None,
            avatar_url: Some(
                "https://gitlab.com/uploads/-/system/user/avatar/11111/avatar.png".to_string(),
            ),
        }
    }
}

/// Builder for GitLab user data
#[derive(Debug, Default)]
pub struct GitLabUserBuilder {
    id: Option<i64>,
    username: Option<String>,
    email: Option<Option<String>>,
    avatar_url: Option<Option<String>>,
}

impl GitLabUserBuilder {
    pub fn id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }

    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn email(mut self, email: Option<impl Into<String>>) -> Self {
        self.email = Some(email.map(|e| e.into()));
        self
    }

    pub fn avatar_url(mut self, avatar_url: Option<impl Into<String>>) -> Self {
        self.avatar_url = Some(avatar_url.map(|u| u.into()));
        self
    }

    pub fn build(self) -> GitLabUser {
        GitLabUser {
            id: self.id.unwrap_or(54321),
            username: self
                .username
                .unwrap_or_else(|| "test_gitlab_user".to_string()),
            email: self
                .email
                .unwrap_or_else(|| Some("test@gitlab.example.com".to_string())),
            avatar_url: self.avatar_url.unwrap_or_else(|| {
                Some("https://gitlab.com/uploads/-/system/user/avatar/54321/avatar.png".to_string())
            }),
        }
    }
}

/// GitLab OAuth token request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabTokenRequest {
    pub client_id: String,
    pub client_secret: String,
    pub code: String,
    pub grant_type: String,
    pub redirect_uri: Option<String>,
}

impl GitLabTokenRequest {
    /// Valid token request
    pub fn valid() -> Self {
        Self {
            client_id: "test_gitlab_client_id".to_string(),
            client_secret: "test_gitlab_client_secret".to_string(),
            code: "test_auth_code".to_string(),
            grant_type: "authorization_code".to_string(),
            redirect_uri: Some("http://localhost:3000/auth/gitlab/callback".to_string()),
        }
    }

    /// Token request with invalid code
    pub fn invalid_code() -> Self {
        Self {
            client_id: "test_gitlab_client_id".to_string(),
            client_secret: "test_gitlab_client_secret".to_string(),
            code: "invalid_gitlab_code".to_string(),
            grant_type: "authorization_code".to_string(),
            redirect_uri: Some("http://localhost:3000/auth/gitlab/callback".to_string()),
        }
    }

    /// Token request with invalid client
    pub fn invalid_client() -> Self {
        Self {
            client_id: "invalid_gitlab_client_id".to_string(),
            client_secret: "invalid_gitlab_client_secret".to_string(),
            code: "test_auth_code".to_string(),
            grant_type: "authorization_code".to_string(),
            redirect_uri: Some("http://localhost:3000/auth/gitlab/callback".to_string()),
        }
    }
}

/// GitLab OAuth token response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

impl GitLabTokenResponse {
    /// Successful token response
    pub fn success() -> Self {
        Self {
            access_token: "glpat-test_access_token_12345".to_string(),
            token_type: "Bearer".to_string(),
            scope: Some("read_user".to_string()),
            refresh_token: Some("glprt-test_refresh_token_12345".to_string()),
            expires_in: Some(7200), // 2 hours
        }
    }

    /// Token response with limited scope
    pub fn limited_scope() -> Self {
        Self {
            access_token: "glpat-limited_token_67890".to_string(),
            token_type: "Bearer".to_string(),
            scope: Some("read_api".to_string()),
            refresh_token: Some("glprt-limited_refresh_67890".to_string()),
            expires_in: Some(3600), // 1 hour
        }
    }

    /// Expired token response (for testing refresh scenarios)
    pub fn expired() -> Self {
        Self {
            access_token: "glpat-expired_token_99999".to_string(),
            token_type: "Bearer".to_string(),
            scope: Some("read_user".to_string()),
            refresh_token: Some("glprt-expired_refresh_99999".to_string()),
            expires_in: Some(0), // Already expired
        }
    }
}

/// GitLab user request context (for matching requests)
#[derive(Debug, Clone)]
pub struct GitLabUserRequest {
    pub access_token: String,
    pub user_agent: String,
}

impl GitLabUserRequest {
    /// Authenticated user request
    pub fn authenticated() -> Self {
        Self {
            access_token: "glpat-test_access_token_12345".to_string(),
            user_agent: "IAM-Service".to_string(),
        }
    }

    /// Request with invalid token
    pub fn invalid_token() -> Self {
        Self {
            access_token: "invalid_gitlab_token".to_string(),
            user_agent: "IAM-Service".to_string(),
        }
    }

    /// Request with expired token
    pub fn expired_token() -> Self {
        Self {
            access_token: "glpat-expired_token_99999".to_string(),
            user_agent: "IAM-Service".to_string(),
        }
    }
}

/// GitLab API error responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabError {
    pub error: String,
    pub error_description: Option<String>,
    pub error_uri: Option<String>,
}

impl GitLabError {
    /// Invalid grant error (bad authorization code)
    pub fn invalid_grant() -> Self {
        Self {
            error: "invalid_grant".to_string(),
            error_description: Some("The provided authorization grant is invalid, expired, revoked, does not match the redirection URI used in the authorization request, or was issued to another client.".to_string()),
            error_uri: None,
        }
    }

    /// Unauthorized error (invalid token)
    pub fn unauthorized() -> Self {
        Self {
            error: "unauthorized".to_string(),
            error_description: Some("401 Unauthorized".to_string()),
            error_uri: None,
        }
    }

    /// Invalid client error
    pub fn invalid_client() -> Self {
        Self {
            error: "invalid_client".to_string(),
            error_description: Some("Client authentication failed".to_string()),
            error_uri: None,
        }
    }

    /// Rate limit exceeded error
    pub fn rate_limit_exceeded() -> Self {
        Self {
            error: "rate_limit_exceeded".to_string(),
            error_description: Some("API rate limit exceeded for this IP".to_string()),
            error_uri: Some("https://docs.gitlab.com/ee/user/gitlab_com/index.html#gitlabcom-specific-rate-limits".to_string()),
        }
    }

    /// Generic server error
    pub fn server_error() -> Self {
        Self {
            error: "server_error".to_string(),
            error_description: Some("The server encountered an unexpected condition".to_string()),
            error_uri: None,
        }
    }

    /// Forbidden error (insufficient scope)
    pub fn forbidden() -> Self {
        Self {
            error: "forbidden".to_string(),
            error_description: Some("Insufficient scope for this resource".to_string()),
            error_uri: None,
        }
    }

    /// Convert to JSON Value for response body
    pub fn to_json(&self) -> Value {
        json!(self)
    }
}
