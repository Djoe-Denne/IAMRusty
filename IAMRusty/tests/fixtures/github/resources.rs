use serde::{Deserialize, Serialize};
use serde_json::{json, Value};


/// GitHub user data structure matching the API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

impl GitHubUser {
    /// Create a builder for custom user data
    pub fn create() -> GitHubUserBuilder {
        GitHubUserBuilder::default()
    }

    /// Pre-built user: Arthur (test user)
    pub fn arthur() -> Self {
        Self {
            id: 12345,
            login: "arthur".to_string(),
            email: Some("arthur@example.com".to_string()),
            avatar_url: Some("https://avatars.githubusercontent.com/u/12345?v=4".to_string()),
        }
    }

    /// Pre-built user: Bob (test user)
    pub fn bob() -> Self {
        Self {
            id: 67890,
            login: "bob".to_string(),
            email: Some("bob@example.com".to_string()),
            avatar_url: Some("https://avatars.githubusercontent.com/u/67890?v=4".to_string()),
        }
    }

    /// User without email (privacy settings)
    pub fn no_email_user() -> Self {
        Self {
            id: 99999,
            login: "private_user".to_string(),
            email: None,
            avatar_url: Some("https://avatars.githubusercontent.com/u/99999?v=4".to_string()),
        }
    }
}

/// Builder for GitHub user data
#[derive(Debug, Default)]
pub struct GitHubUserBuilder {
    id: Option<i64>,
    login: Option<String>,
    email: Option<Option<String>>,
    avatar_url: Option<Option<String>>,
}

impl GitHubUserBuilder {
    pub fn id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }

    pub fn login(mut self, login: impl Into<String>) -> Self {
        self.login = Some(login.into());
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

    pub fn build(self) -> GitHubUser {
        GitHubUser {
            id: self.id.unwrap_or(12345),
            login: self.login.unwrap_or_else(|| "test_user".to_string()),
            email: self.email.unwrap_or_else(|| Some("test@example.com".to_string())),
            avatar_url: self.avatar_url.unwrap_or_else(|| Some("https://avatars.githubusercontent.com/u/12345?v=4".to_string())),
        }
    }
}

/// GitHub OAuth authorization request data (for the /login/oauth/authorize endpoint)
#[derive(Debug, Clone)]
pub struct GitHubAuthRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub response_type: String,
}

impl GitHubAuthRequest {
    /// Standard OAuth authorization request
    pub fn standard() -> Self {
        Self {
            client_id: "test_client_id".to_string(),
            redirect_uri: "http://127.0.0.1:8081/api/auth/github/callback".to_string(),
            scope: "user:email".to_string(),
            state: "test_state_12345".to_string(),
            response_type: "code".to_string(),
        }
    }

    /// Authorization request for login flow
    pub fn login_flow() -> Self {
        Self {
            client_id: "test_client_id".to_string(),
            redirect_uri: "http://127.0.0.1:8081/api/auth/github/callback".to_string(),
            scope: "user:email".to_string(),
            state: "login_state_67890".to_string(),
            response_type: "code".to_string(),
        }
    }

    /// Authorization request for linking flow
    pub fn linking_flow() -> Self {
        Self {
            client_id: "test_client_id".to_string(),
            redirect_uri: "http://127.0.0.1:8081/api/auth/github/callback".to_string(),
            scope: "user:email".to_string(),
            state: "link_state_99999".to_string(),
            response_type: "code".to_string(),
        }
    }
}

/// GitHub OAuth token request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubTokenRequest {
    pub client_id: String,
    pub client_secret: String,
    pub code: String,
    pub redirect_uri: Option<String>,
}

impl GitHubTokenRequest {
    /// Valid token request
    pub fn valid() -> Self {
        Self {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            code: "test_auth_code".to_string(),
            redirect_uri: Some("http://localhost:3000/auth/github/callback".to_string()),
        }
    }

    /// Token request with invalid code
    pub fn invalid_code() -> Self {
        Self {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            code: "invalid_code".to_string(),
            redirect_uri: Some("http://localhost:3000/auth/github/callback".to_string()),
        }
    }

    /// Token request with invalid client
    pub fn invalid_client() -> Self {
        Self {
            client_id: "invalid_client_id".to_string(),
            client_secret: "invalid_client_secret".to_string(),
            code: "valid_auth_code".to_string(),
            redirect_uri: Some("http://localhost:3000/auth/github/callback".to_string()),
        }
    }
}

/// GitHub OAuth token response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
}

impl GitHubTokenResponse {
    /// Successful token response
    pub fn success() -> Self {
        Self {
            access_token: "gho_test_access_token_12345".to_string(),
            token_type: "bearer".to_string(),
            scope: Some("user:email".to_string()),
            refresh_token: None, // GitHub doesn't use refresh tokens
        }
    }

    /// Token response with limited scope
    pub fn limited_scope() -> Self {
        Self {
            access_token: "gho_limited_token_67890".to_string(),
            token_type: "bearer".to_string(),
            scope: Some("user".to_string()),
            refresh_token: None,
        }
    }

    /// Expired token response (for testing refresh scenarios)
    pub fn expired() -> Self {
        Self {
            access_token: "gho_expired_token_99999".to_string(),
            token_type: "bearer".to_string(),
            scope: Some("user:email".to_string()),
            refresh_token: None,
        }
    }
}

/// GitHub user request context (for matching requests)
#[derive(Debug, Clone)]
pub struct GitHubUserRequest {
    pub access_token: String,
    pub user_agent: String,
    pub accept: String,
}

impl GitHubUserRequest {
    /// Authenticated user request
    pub fn authenticated() -> Self {
        Self {
            access_token: "gho_test_access_token_12345".to_string(),
            user_agent: "IAM-Service".to_string(),
            accept: "application/vnd.github.v3+json".to_string(),
        }
    }

    /// Request with invalid token
    pub fn invalid_token() -> Self {
        Self {
            access_token: "invalid_token".to_string(),
            user_agent: "IAM-Service".to_string(),
            accept: "application/vnd.github.v3+json".to_string(),
        }
    }

    /// Request with expired token
    pub fn expired_token() -> Self {
        Self {
            access_token: "gho_expired_token_99999".to_string(),
            user_agent: "IAM-Service".to_string(),
            accept: "application/vnd.github.v3+json".to_string(),
        }
    }
}

/// GitHub API error responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubError {
    pub error: String,
    pub error_description: Option<String>,
    pub error_uri: Option<String>,
}

impl GitHubError {
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
            error_description: Some("Bad credentials".to_string()),
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
            error_description: Some("API rate limit exceeded".to_string()),
            error_uri: Some("https://docs.github.com/rest/overview/resources-in-the-rest-api#rate-limiting".to_string()),
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

    /// Convert to JSON Value for response body
    pub fn to_json(&self) -> Value {
        json!(self)
    }
} 