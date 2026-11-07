//! OAuth-only adapter using a trusted startup-injected HTTP client.

use std::{collections::BTreeMap, fmt};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use idp_connect_contract::{
    dto::{valid_pkce_verifier, valid_s256_challenge},
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    FederatedOAuthClient, FederatedOAuthError, ProviderAuthentication, ProviderTokens,
    ProviderUserProfile,
};
use oauth_connect_configuration::{
    validate_provider, ProviderConfig, ProviderProfile, TokenAuthMethod,
};
use reqwest::Client;
use rustycog::config::secrets::SecretValue;
use url::Url;

use crate::{compile_mapping, ApiId, CompiledMapping, DomainError};

mod wire;

/// One OAuth instance with its startup-injected HTTP client and resolved secret.
/// No tokens, response JSON or projected profile are retained between operations.
pub struct OAuthClient {
    client_id: String,
    client_secret: SecretValue,
    http: Client,
    authorization_url: Url,
    token_url: Url,
    token_auth_method: TokenAuthMethod,
    redirect_uris: Vec<String>,
    scope: String,
    pkce_supported: bool,
    apis: BTreeMap<ApiId, wire::JsonApi>,
    mapping: CompiledMapping,
}

impl fmt::Debug for OAuthClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthClient")
            .finish_non_exhaustive()
    }
}

impl OAuthClient {
    /// Construct an OAuth-only adapter, consuming the SDK secret and HTTP client.
    /// The caller supplies a trusted setup-built client: verified transport/roots,
    /// redirect none, retry never, total/connect limits and gzip policy. Options
    /// of an arbitrary raw `reqwest` client cannot be introspected here.
    ///
    /// # Errors
    /// Refuses invalid declarations/mappings and unsupported OIDC declarations.
    /// Performs no I/O or secret resolution. The supplied SDK secret stays exact.
    pub fn new(
        provider: &ProviderConfig,
        client_secret: SecretValue,
        http: Client,
    ) -> Result<Self, DomainError> {
        validate_provider(provider)?;
        let ProviderProfile::Oauth(profile) = &provider.profile else {
            return Err(invalid_declaration());
        };
        Ok(Self {
            client_id: provider.client_id.clone(),
            client_secret,
            http,
            authorization_url: Url::parse(&profile.authorization_url)
                .map_err(|_| invalid_declaration())?,
            token_url: Url::parse(&profile.token_url).map_err(|_| invalid_declaration())?,
            token_auth_method: provider.token_auth_method,
            redirect_uris: provider.redirect_uris.clone(),
            scope: provider.scopes.join(" "),
            pkce_supported: provider.pkce_supported,
            apis: wire::compile_apis(&profile.apis)?,
            mapping: compile_mapping(&profile.claims)?,
        })
    }

    fn authorize_uri(
        &self,
        request: &AuthorizeAuthenticationRequest,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        let error = FederatedOAuthError::Authorize;
        if !self.allowed_redirect(&request.redirect_uri)
            || request.state.trim().is_empty()
            || !valid_nonce(&request.nonce)
        {
            return Err(error);
        }
        let challenge = match (
            request.code_challenge.as_deref(),
            request.code_challenge_method.as_deref(),
        ) {
            (None, None) if !self.pkce_supported => None,
            (Some(challenge), Some("S256"))
                if self.pkce_supported && valid_s256_challenge(challenge) =>
            {
                Some(challenge)
            }
            _ => return Err(error),
        };
        let mut url = self.authorization_url.clone();
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("response_type", "code")
                .append_pair("client_id", &self.client_id)
                .append_pair("redirect_uri", &request.redirect_uri)
                .append_pair("state", &request.state)
                .append_pair("scope", &self.scope);
            if let Some(challenge) = challenge {
                query
                    .append_pair("code_challenge", challenge)
                    .append_pair("code_challenge_method", "S256");
            }
        }
        // Nonce is validated as v2 context only; OAuth never sends it to a vendor.
        Ok(AuthorizeResponse {
            authorization_url: url.into(),
            scope: self.scope.clone(),
        })
    }

    fn validate_completion(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<(), FederatedOAuthError> {
        let pkce = match request.code_verifier.as_deref() {
            Some(verifier) => self.pkce_supported && valid_pkce_verifier(verifier),
            None => !self.pkce_supported,
        };
        if !self.allowed_redirect(&request.redirect_uri)
            || request.code.trim().is_empty()
            || !valid_nonce(&request.expected_nonce)
            || !pkce
        {
            return Err(FederatedOAuthError::ExchangeCode);
        }
        Ok(())
    }

    fn allowed_redirect(&self, redirect: &str) -> bool {
        self.redirect_uris.iter().any(|allowed| allowed == redirect)
    }
}

fn invalid_declaration() -> DomainError {
    DomainError::InvalidInput {
        message: "Invalid OAuthConnect OAuth declaration".to_owned(),
    }
}

// Same canonical wire constraint as the contract receiver's private nonce check;
// reuse the public DTO PKCE validators above rather than adding a PKCE dialect.
fn valid_nonce(nonce: &str) -> bool {
    if nonce.len() != 43 {
        return false;
    }
    let Ok(bytes) = URL_SAFE_NO_PAD.decode(nonce) else {
        return false;
    };
    bytes.len() == 32 && URL_SAFE_NO_PAD.encode(&bytes) == nonce
}

#[async_trait]
impl FederatedOAuthClient for OAuthClient {
    async fn authorize(
        &self,
        _redirect_uri: &str,
        _state: &str,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        Err(FederatedOAuthError::Authorize)
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _redirect_uri: &str,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        Err(FederatedOAuthError::ExchangeCode)
    }

    async fn user_profile(
        &self,
        _access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError> {
        Err(FederatedOAuthError::UserProfile)
    }

    async fn authorize_with_pkce(
        &self,
        _redirect_uri: &str,
        _state: &str,
        _code_challenge: Option<&str>,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        Err(FederatedOAuthError::Authorize)
    }

    async fn exchange_code_with_pkce(
        &self,
        _code: &str,
        _redirect_uri: &str,
        _code_verifier: Option<&str>,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        Err(FederatedOAuthError::ExchangeCode)
    }

    async fn authorize_authentication(
        &self,
        request: &AuthorizeAuthenticationRequest,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.authorize_uri(request)
    }

    async fn complete_authorization(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<ProviderAuthentication, FederatedOAuthError> {
        self.validate_completion(request)?;
        let tokens = self.exchange(request).await?;
        let profile = self.profile(&tokens.access_token).await?;
        Ok(ProviderAuthentication { tokens, profile })
    }
}
