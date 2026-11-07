//! OIDC through the maintained verifier and instance-local technical JWKS state.

use std::{fmt, sync::Arc};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use idp_connect_contract::{
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    FederatedOAuthClient, FederatedOAuthError, ProviderAuthentication, ProviderTokens,
    ProviderUserProfile,
};
use oauth_connect_configuration::{
    validate_provider, ProviderConfig, ProviderProfile, TokenAuthMethod,
};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreProviderMetadata, CoreTokenType},
    AuthorizationCode, ClaimsVerificationError, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeVerifier, RedirectUrl, Scope, SignatureVerificationError, TokenResponse,
};
use reqwest::Client;
use rustycog::config::secrets::SecretValue;
use tokio::time::Instant;

use crate::DomainError;

mod cache;
mod callback;
mod profile;
mod protocol;
mod verify;

// Private construction seams for colocated tests; neither is a public clock API.
// Advancing cache time must never advance the UTC time used by the verifier.
type MonotonicNow = Arc<dyn Fn() -> Instant + Send + Sync>;
type UtcNow = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// OIDC authority with an instance-local technical JWKS cache.
///
/// Clones share only this instance's immutable declaration and technical cache.
#[derive(Clone)]
pub struct OidcClient {
    instance: Arc<Instance>,
}

struct Instance {
    client_id: String,
    client_secret: SecretValue,
    redirect_uris: Vec<String>,
    scopes: Vec<String>,
    auth: TokenAuthMethod,
    metadata: CoreProviderMetadata,
    http: callback::BoundHttp,
    keys: cache::KeyCache,
    utc_now: UtcNow,
}

impl fmt::Debug for OidcClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("OidcClient").finish_non_exhaustive()
    }
}

impl OidcClient {
    /// Consume a declaration, resolved SDK secret and trusted setup HTTP client.
    ///
    /// # Errors
    /// Refuses invalid declarations, discovery/endpoints or initial JWKS without
    /// publishing a partial client. Network/parse diagnostics remain sanitized.
    pub async fn new(
        provider: &ProviderConfig,
        client_secret: SecretValue,
        http: Client,
    ) -> Result<Self, DomainError> {
        Self::new_with_clocks(
            provider,
            client_secret,
            http,
            Arc::new(Instant::now),
            Arc::new(Utc::now),
        )
        .await
    }

    /// Private independent clock injection for colocated cache/claims tests.
    ///
    /// # Errors
    /// Refuses invalid startup without publishing a partially hydrated cache.
    async fn new_with_clocks(
        provider: &ProviderConfig,
        client_secret: SecretValue,
        http: Client,
        monotonic_now: MonotonicNow,
        utc_now: UtcNow,
    ) -> Result<Self, DomainError> {
        validate_provider(provider)?;
        let ProviderProfile::Oidc(declaration) = &provider.profile else {
            return Err(invalid_startup());
        };
        let http = callback::BoundHttp::new(http, declaration, provider.transport_security)
            .map_err(|_| invalid_startup())?;
        let issuer = IssuerUrl::new(declaration.issuer.clone()).map_err(|_| invalid_startup())?;
        // The maintained discovery routine already fetches JWKS through this
        // same URL-guarded, byte-bounded callback. Never fetch it twice here.
        let metadata = http.discover(issuer).await.map_err(|_| invalid_startup())?;
        if metadata.issuer().as_str() != declaration.issuer.as_str() {
            return Err(invalid_startup());
        }
        protocol::validate_metadata(&metadata, &http, provider.token_auth_method)
            .map_err(|_| invalid_startup())?;
        let keys = cache::KeyCache::new(metadata.jwks().clone(), monotonic_now);
        Ok(Self {
            instance: Arc::new(Instance {
                client_id: provider.client_id.clone(),
                client_secret,
                redirect_uris: provider.redirect_uris.clone(),
                scopes: provider.scopes.clone(),
                auth: provider.token_auth_method,
                metadata,
                http,
                keys,
                utc_now,
            }),
        })
    }

    fn allowed_redirect(&self, redirect: &str) -> bool {
        self.instance
            .redirect_uris
            .iter()
            .any(|allowed| allowed == redirect)
    }

    fn authorization(
        &self,
        request: &AuthorizeAuthenticationRequest,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        let error = FederatedOAuthError::Authorize;
        let challenge = request.code_challenge.as_deref().ok_or(error)?;
        if !self.allowed_redirect(&request.redirect_uri)
            || request.state.trim().is_empty()
            || !valid_nonce(&request.nonce)
            || request.code_challenge_method.as_deref() != Some("S256")
            || !idp_connect_contract::dto::valid_s256_challenge(challenge)
        {
            return Err(error);
        }
        let client = self
            .instance
            .protocol(self.instance.metadata.jwks())
            .set_redirect_uri(RedirectUrl::new(request.redirect_uri.clone()).map_err(|_| error)?);
        let state = request.state.clone();
        let nonce = request.nonce.clone();
        let mut authorize = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                move || CsrfToken::new(state),
                move || Nonce::new(nonce),
            )
            .add_extra_param("code_challenge", challenge.to_owned())
            .add_extra_param("code_challenge_method", "S256");
        for scope in &self.instance.scopes {
            if scope != "openid" {
                authorize = authorize.add_scope(Scope::new(scope.clone()));
            }
        }
        let (url, _, _) = authorize.url();
        Ok(AuthorizeResponse {
            authorization_url: url.into(),
            scope: self.instance.scopes.join(" "),
        })
    }

    fn validate_completion(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<(), FederatedOAuthError> {
        if !self.allowed_redirect(&request.redirect_uri)
            || request.code.trim().is_empty()
            || !valid_nonce(&request.expected_nonce)
            || !request
                .code_verifier
                .as_deref()
                .is_some_and(idp_connect_contract::dto::valid_pkce_verifier)
        {
            return Err(FederatedOAuthError::ExchangeCode);
        }
        Ok(())
    }

    async fn completion(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<ProviderAuthentication, FederatedOAuthError> {
        self.validate_completion(request)?;
        let tokens = self.exchange_oidc(request).await?;
        let nonce = Nonce::new(request.expected_nonce.clone());
        let (protocol, mut claims) = self.verified_identity(&tokens, &nonce).await?;
        if claims.email.is_none() && protocol.user_info_url().is_some() {
            let info: openidconnect::core::CoreUserInfoClaims = protocol
                .user_info(
                    tokens.access_token().clone(),
                    Some(openidconnect::SubjectIdentifier::new(
                        claims.subject.clone(),
                    )),
                )
                .map_err(|_| FederatedOAuthError::UserProfile)?
                .request_async(&self.instance.http)
                .await
                .map_err(|_| FederatedOAuthError::UserProfile)?;
            claims.merge(profile::Profile::from_userinfo(&info)?)?;
        }
        Ok(ProviderAuthentication {
            tokens: ProviderTokens {
                access_token: tokens.access_token().secret().clone(),
                refresh_token: tokens.refresh_token().map(|token| token.secret().clone()),
                expires_in: tokens.expires_in().map(|duration| duration.as_secs()),
            },
            profile: claims.into_profile(),
        })
    }

    async fn exchange_oidc(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<protocol::Reply, FederatedOAuthError> {
        let error = FederatedOAuthError::ExchangeCode;
        let verifier = request.code_verifier.clone().ok_or(error)?;
        let client = self
            .instance
            .protocol(self.instance.metadata.jwks())
            .set_redirect_uri(RedirectUrl::new(request.redirect_uri.clone()).map_err(|_| error)?);
        let tokens = client
            .exchange_code(AuthorizationCode::new(request.code.clone()))
            .map_err(|_| error)?
            .set_pkce_verifier(PkceCodeVerifier::new(verifier))
            .request_async(&self.instance.http)
            .await
            .map_err(|_| error)?;
        if tokens.access_token().secret().trim().is_empty()
            || tokens.token_type() != &CoreTokenType::Bearer
        {
            return Err(error);
        }
        Ok(tokens)
    }

    async fn verified_identity(
        &self,
        tokens: &protocol::Reply,
        nonce: &Nonce,
    ) -> Result<(protocol::ProtocolClient, profile::Profile), FederatedOAuthError> {
        let id_token = tokens.id_token().ok_or(FederatedOAuthError::UserProfile)?;
        let snapshot = self
            .instance
            .keys
            .current(&self.instance.http, self.instance.metadata.jwks_uri())
            .await?;
        let mut protocol = self.instance.protocol(&snapshot.keys);
        let claims = match verify::validated(
            &protocol,
            id_token,
            tokens.access_token(),
            nonce,
            (self.instance.utc_now)(),
            &self.instance.client_id,
        ) {
            Ok(claims) => claims,
            Err(ClaimsVerificationError::SignatureVerification(
                SignatureVerificationError::NoMatchingKey,
            )) => {
                let fresh = self
                    .instance
                    .keys
                    .refresh(
                        snapshot.generation,
                        &self.instance.http,
                        self.instance.metadata.jwks_uri(),
                    )
                    .await?;
                protocol = self.instance.protocol(&fresh.keys);
                verify::validated(
                    &protocol,
                    id_token,
                    tokens.access_token(),
                    nonce,
                    (self.instance.utc_now)(),
                    &self.instance.client_id,
                )
                .map_err(|_| FederatedOAuthError::UserProfile)?
            }
            Err(_) => return Err(FederatedOAuthError::UserProfile),
        };
        Ok((protocol, claims))
    }
}

fn invalid_startup() -> DomainError {
    DomainError::InvalidInput {
        message: "Invalid OAuthConnect OIDC initialization".to_owned(),
    }
}

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
impl FederatedOAuthClient for OidcClient {
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
        self.authorization(request)
    }

    async fn complete_authorization(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<ProviderAuthentication, FederatedOAuthError> {
        self.completion(request).await
    }
}

#[cfg(test)]
mod tests;
