//! GitLab `OAuth2` client implementing [`FederatedOAuthClient`].

use std::time::Duration;

use async_trait::async_trait;
use gitlab_connect_configuration::GitLabConfig;
use gitlab_connect_domain::{redirect_uri_allowed, DomainError};
use idp_connect_contract::{
    AuthorizeResponse, FederatedOAuthClient, FederatedOAuthError, ProviderTokens,
    ProviderUserProfile,
};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, HttpRequest,
    HttpResponse, PkceCodeVerifier, RedirectUrl, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use tracing::error;

const USER_AGENT: &str = "GitLab-Connect";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const SCOPE: &str = "read_user";

/// GitLab federated OAuth client (vendor HTTP only).
pub struct GitLabConnectClient {
    pkce_supported: bool,
    client_id: ClientId,
    client_secret: ClientSecret,
    auth_url: AuthUrl,
    token_url: TokenUrl,
    user_url: String,
    redirect_uris: Vec<String>,
    http: reqwest::Client,
}

impl std::fmt::Debug for GitLabConnectClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitLabConnectClient")
            .field("pkce_supported", &self.pkce_supported)
            .field("user_url", &self.user_url)
            .field("redirect_uris", &self.redirect_uris)
            .finish_non_exhaustive()
    }
}

impl GitLabConnectClient {
    /// Build a client from typed config. Vendor URLs are parsed, never unwrapped.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::OAuth2Error`] if `auth_url` or `token_url` is not a
    /// valid URL, or if the HTTP client cannot be constructed.
    pub fn from_config(config: &GitLabConfig) -> Result<Self, DomainError> {
        Self::from_config_with_transport(
            config,
            gitlab_connect_configuration::transport::VendorTransportSecurity::Verified,
        )
    }

    /// Build with an explicit local/test exemption. TLS certificates remain verified.
    ///
    /// # Errors
    /// Rejects unsafe vendor URLs or HTTP client construction failures.
    pub fn from_config_with_transport(
        config: &GitLabConfig,
        policy: gitlab_connect_configuration::transport::VendorTransportSecurity,
    ) -> Result<Self, DomainError> {
        Self::from_config_with_transport_and_pkce(config, policy, false)
    }

    /// Explicit receiver capability. Unsupported PKCE is refused rather than ignored.
    ///
    /// # Errors
    /// Rejects unsafe vendor URLs or HTTP client construction failures.
    pub fn from_config_with_transport_and_pkce(
        config: &GitLabConfig,
        policy: gitlab_connect_configuration::transport::VendorTransportSecurity,
        pkce_supported: bool,
    ) -> Result<Self, DomainError> {
        for raw in [&config.auth_url, &config.token_url, &config.user_url] {
            validate_vendor_url(raw, policy)?;
        }
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
            pkce_supported,
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

fn validate_vendor_url(
    raw: &str,
    policy: gitlab_connect_configuration::transport::VendorTransportSecurity,
) -> Result<(), DomainError> {
    let url = reqwest::Url::parse(raw)
        .map_err(|_| DomainError::OAuth2Error("invalid vendor URL".into()))?;
    if raw.trim() != raw
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.scheme(), "https" | "http")
        || (policy == gitlab_connect_configuration::transport::VendorTransportSecurity::Verified
            && url.scheme() != "https")
    {
        return Err(DomainError::OAuth2Error(
            "vendor requires verified HTTPS; plaintext needs explicit local/test policy".into(),
        ));
    }
    Ok(())
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

/// GitLab user response from the API.
#[derive(Debug, Deserialize)]
struct GitLabUser {
    id: i64,
    username: String,
    email: Option<String>,
    avatar_url: Option<String>,
    #[serde(default)]
    confirmed_at: Option<String>,
}

fn map_profile(gitlab_user: GitLabUser) -> ProviderUserProfile {
    ProviderUserProfile {
        id: gitlab_user.id.to_string(),
        username: gitlab_user.username,
        email: gitlab_user.email,
        avatar_url: gitlab_user.avatar_url,
        email_verified: gitlab_user
            .confirmed_at
            .as_ref()
            .is_some_and(|v| !v.is_empty()),
    }
}

#[async_trait]
impl FederatedOAuthClient for GitLabConnectClient {
    async fn authorize(
        &self,
        redirect_uri: &str,
        state: &str,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.authorize_with_pkce(redirect_uri, state, None).await
    }

    async fn authorize_with_pkce(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: Option<&str>,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        if code_challenge.is_some_and(|challenge| {
            !self.pkce_supported || !idp_connect_contract::dto::valid_s256_challenge(challenge)
        }) {
            return Err(FederatedOAuthError::Authorize);
        }
        self.ensure_redirect_allowed(redirect_uri, FederatedOAuthError::Authorize)?;
        let client = self.basic_client(redirect_uri, FederatedOAuthError::Authorize)?;
        let state = state.to_owned();
        let (mut auth_url, _) = client
            .authorize_url(move || CsrfToken::new(state))
            .add_scope(oauth2::Scope::new(SCOPE.to_owned()))
            .url();
        if let Some(challenge) = code_challenge {
            let mut pairs = auth_url.query_pairs_mut();
            pairs.append_pair("code_challenge", challenge);
            pairs.append_pair("code_challenge_method", "S256");
            pairs.finish();
        }
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
        self.exchange_code_with_pkce(code, redirect_uri, None).await
    }

    async fn exchange_code_with_pkce(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: Option<&str>,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        if code_verifier.is_some_and(|verifier| {
            !self.pkce_supported || !idp_connect_contract::dto::valid_pkce_verifier(verifier)
        }) {
            return Err(FederatedOAuthError::ExchangeCode);
        }
        self.ensure_redirect_allowed(redirect_uri, FederatedOAuthError::ExchangeCode)?;
        let client = self.basic_client(redirect_uri, FederatedOAuthError::ExchangeCode)?;
        let http = self.http.clone();
        let mut request = client.exchange_code(AuthorizationCode::new(code.to_owned()));
        if let Some(verifier) = code_verifier {
            request = request.set_pkce_verifier(PkceCodeVerifier::new(verifier.to_owned()));
        }
        let token_result = request
            .request_async(move |req| async move { send_oauth(&http, req).await })
            .await
            .map_err(|_| {
                error!("GitLab token exchange failed");
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
        let gitlab_user = self
            .http
            .get(&self.user_url)
            .header("User-Agent", USER_AGENT)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|_| {
                error!("GitLab user profile request failed");
                FederatedOAuthError::UserProfile
            })?
            .json::<GitLabUser>()
            .await
            .map_err(|_| {
                error!("GitLab user profile response was not valid JSON");
                FederatedOAuthError::UserProfile
            })?;

        Ok(map_profile(gitlab_user))
    }
}

#[cfg(test)]
mod transport_tests {
    use super::*;
    use gitlab_connect_configuration::transport::VendorTransportSecurity;

    #[test]
    fn plaintext_vendor_endpoints_require_explicit_isolated_policy() {
        let config = GitLabConfig {
            token_url: "http://localhost/token".into(),
            ..GitLabConfig::default()
        };
        assert!(GitLabConnectClient::from_config(&config).is_err());
        assert!(GitLabConnectClient::from_config_with_transport(
            &config,
            VendorTransportSecurity::IsolatedTest
        )
        .is_ok());
    }

    #[test]
    fn invalid_vendor_urls_never_echo_embedded_credentials() {
        let error = validate_vendor_url(
            "https://user:SENTINEL-VENDOR-SECRET@vendor/token",
            VendorTransportSecurity::Verified,
        )
        .unwrap_err();
        assert!(!error.to_string().contains("SENTINEL-VENDOR-SECRET"));
    }

    #[tokio::test]
    async fn pkce_is_explicit_s256_and_never_silently_ignored() {
        let redirect = "https://app.example/callback";
        let config = GitLabConfig {
            redirect_uris: vec![redirect.into()],
            ..GitLabConfig::default()
        };
        let disabled = GitLabConnectClient::from_config(&config).unwrap();
        let challenge = "a".repeat(43);
        assert!(disabled
            .authorize_with_pkce(redirect, "opaque", Some(&challenge))
            .await
            .is_err());
        assert!(disabled
            .exchange_code_with_pkce("code", redirect, Some(&challenge))
            .await
            .is_err());
        let enabled = GitLabConnectClient::from_config_with_transport_and_pkce(
            &config,
            VendorTransportSecurity::Verified,
            true,
        )
        .unwrap();
        let response = enabled
            .authorize_with_pkce(redirect, "opaque", Some(&challenge))
            .await
            .unwrap();
        let url = reqwest::Url::parse(&response.authorization_url).unwrap();
        assert_eq!(
            url.query_pairs()
                .filter(|(key, value)| key.as_ref() == "code_challenge"
                    && value.as_ref() == challenge.as_str())
                .count(),
            1
        );
        assert_eq!(
            url.query_pairs()
                .filter(|(key, value)| key.as_ref() == "code_challenge_method"
                    && value.as_ref() == "S256")
                .count(),
            1
        );
        let no_pkce = enabled.authorize(redirect, "opaque").await.unwrap();
        assert!(!no_pkce.authorization_url.contains("code_challenge"));
        assert!(enabled
            .authorize_with_pkce(redirect, "opaque", Some("short"))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn consumed_verifier_reaches_vendor_token_post() {
        use wiremock::{
            matchers::{body_string_contains, method, path},
            Mock, MockServer, ResponseTemplate,
        };
        let vendor = MockServer::start().await;
        let verifier = "v".repeat(43);
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains(format!("code_verifier={verifier}")))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "application/json")
                    .set_body_string(r#"{"access_token":"opaque-access","token_type":"Bearer"}"#),
            )
            .expect(1)
            .mount(&vendor)
            .await;
        let redirect = "https://app.example/callback";
        let config = GitLabConfig {
            token_url: format!("{}/token", vendor.uri()),
            redirect_uris: vec![redirect.into()],
            ..GitLabConfig::default()
        };
        let client = GitLabConnectClient::from_config_with_transport_and_pkce(
            &config,
            VendorTransportSecurity::IsolatedTest,
            true,
        )
        .unwrap();
        let tokens = client
            .exchange_code_with_pkce("opaque-code", redirect, Some(&verifier))
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "opaque-access");
        vendor.verify().await;
    }
}
