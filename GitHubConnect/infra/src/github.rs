//! GitHub `OAuth2` client implementing [`FederatedOAuthClient`].

use std::time::Duration;

use async_trait::async_trait;
use github_connect_configuration::GitHubConfig;
use github_connect_domain::{redirect_uri_allowed, DomainError};
use idp_connect_contract::{
    AuthorizeResponse, FederatedOAuthClient, FederatedOAuthError, ProviderTokens,
    ProviderUserProfile,
};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, HttpRequest,
    HttpResponse, RedirectUrl, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use tracing::error;

const USER_AGENT: &str = "GitHub-Connect";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const SCOPE: &str = "user";

/// GitHub federated OAuth client (vendor HTTP only).
pub struct GitHubConnectClient {
    client_id: ClientId,
    client_secret: ClientSecret,
    auth_url: AuthUrl,
    token_url: TokenUrl,
    user_url: String,
    redirect_uris: Vec<String>,
    http: reqwest::Client,
}

impl std::fmt::Debug for GitHubConnectClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubConnectClient")
            .field("user_url", &self.user_url)
            .field("redirect_uris", &self.redirect_uris)
            .finish_non_exhaustive()
    }
}

impl GitHubConnectClient {
    /// Build a client from typed config. Vendor URLs are parsed, never unwrapped.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::OAuth2Error`] if `auth_url` or `token_url` is not a
    /// valid URL, or if the HTTP client cannot be constructed.
    pub fn from_config(config: &GitHubConfig) -> Result<Self, DomainError> {
        let auth_url = AuthUrl::new(config.auth_url.clone())
            .map_err(|e| DomainError::OAuth2Error(format!("invalid auth URL: {e}")))?;
        let token_url = TokenUrl::new(config.token_url.clone())
            .map_err(|e| DomainError::OAuth2Error(format!("invalid token URL: {e}")))?;
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| DomainError::OAuth2Error(format!("failed to build HTTP client: {e}")))?;
        Ok(Self {
            client_id: ClientId::new(config.client_id.clone()),
            client_secret: ClientSecret::new(config.client_secret.clone()),
            auth_url,
            token_url,
            user_url: config.user_url.clone(),
            redirect_uris: config.redirect_uris.clone(),
            http,
        })
    }

    fn ensure_redirect_allowed(
        &self,
        redirect_uri: &str,
        on_fail: FederatedOAuthError,
    ) -> Result<(), FederatedOAuthError> {
        if redirect_uri_allowed(redirect_uri, &self.redirect_uris) {
            Ok(())
        } else {
            Err(on_fail)
        }
    }

    fn basic_client(
        &self,
        redirect_uri: &str,
        on_fail: FederatedOAuthError,
    ) -> Result<BasicClient, FederatedOAuthError> {
        let redirect = RedirectUrl::new(redirect_uri.to_owned()).map_err(|_| on_fail)?;
        Ok(BasicClient::new(
            self.client_id.clone(),
            Some(self.client_secret.clone()),
            self.auth_url.clone(),
            Some(self.token_url.clone()),
        )
        .set_redirect_uri(redirect))
    }
}

async fn send_oauth(
    http: &reqwest::Client,
    request: HttpRequest,
) -> Result<HttpResponse, reqwest::Error> {
    let mut request_builder = http
        .request(request.method, request.url.as_str())
        .body(request.body);
    for (name, value) in &request.headers {
        request_builder = request_builder.header(name.as_str(), value.as_bytes());
    }
    let request = request_builder.build()?;
    let response = http.execute(request).await?;
    let status_code = response.status();
    let headers = response.headers().to_owned();
    let chunks = response.bytes().await?;
    Ok(HttpResponse {
        status_code,
        headers,
        body: chunks.to_vec(),
    })
}

/// GitHub user response from the API.
#[derive(Debug, Deserialize)]
struct GitHubUser {
    id: i64,
    login: String,
    email: Option<String>,
    avatar_url: Option<String>,
}

/// GitHub email response from the emails API.
#[derive(Debug, Clone, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
    #[serde(rename = "visibility")]
    _visibility: Option<String>,
}

fn map_profile(github_user: GitHubUser, github_emails: &[GitHubEmail]) -> ProviderUserProfile {
    let primary = github_emails.iter().find(|email| email.primary).cloned();
    let (email, email_verified) = if let Some(primary) = primary {
        (Some(primary.email), primary.verified)
    } else if let Some(public_email) = github_user.email.filter(|e| !e.is_empty()) {
        let verified = github_emails
            .iter()
            .find(|item| item.email == public_email)
            .is_some_and(|item| item.verified);
        (Some(public_email), verified)
    } else {
        (None, false)
    };
    ProviderUserProfile {
        id: github_user.id.to_string(),
        username: github_user.login,
        email,
        avatar_url: github_user.avatar_url,
        email_verified,
    }
}

#[async_trait]
impl FederatedOAuthClient for GitHubConnectClient {
    async fn authorize(
        &self,
        redirect_uri: &str,
        state: &str,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.ensure_redirect_allowed(redirect_uri, FederatedOAuthError::Authorize)?;
        let client = self.basic_client(redirect_uri, FederatedOAuthError::Authorize)?;
        let state = state.to_owned();
        let (auth_url, _) = client
            .authorize_url(move || CsrfToken::new(state))
            .add_scope(oauth2::Scope::new(SCOPE.to_owned()))
            .url();
        Ok(AuthorizeResponse {
            authorization_url: auth_url.to_string(),
            scope: SCOPE.to_owned(),
        })
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        self.ensure_redirect_allowed(redirect_uri, FederatedOAuthError::ExchangeCode)?;
        let client = self.basic_client(redirect_uri, FederatedOAuthError::ExchangeCode)?;
        let http = self.http.clone();
        let token_result = client
            .exchange_code(AuthorizationCode::new(code.to_owned()))
            .request_async(move |req| async move { send_oauth(&http, req).await })
            .await
            .map_err(|_| {
                error!("GitHub token exchange failed");
                FederatedOAuthError::ExchangeCode
            })?;
        Ok(ProviderTokens {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|r| r.secret().clone()),
            expires_in: token_result.expires_in().map(|duration| duration.as_secs()),
        })
    }

    async fn user_profile(
        &self,
        access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError> {
        let github_user = self
            .http
            .get(&self.user_url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("token {access_token}"))
            .send()
            .await
            .map_err(|_| {
                error!("GitHub user profile request failed");
                FederatedOAuthError::UserProfile
            })?
            .json::<GitHubUser>()
            .await
            .map_err(|_| {
                error!("GitHub user profile response was not valid JSON");
                FederatedOAuthError::UserProfile
            })?;

        let emails_url = format!("{}/emails", self.user_url);
        let github_emails = match self
            .http
            .get(&emails_url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("token {access_token}"))
            .send()
            .await
        {
            Ok(response) => response
                .json::<Vec<GitHubEmail>>()
                .await
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        Ok(map_profile(github_user, &github_emails))
    }
}
