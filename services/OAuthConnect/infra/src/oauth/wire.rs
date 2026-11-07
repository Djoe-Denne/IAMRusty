//! Fixed OAuth POST/GET requests and typed token normalization, without retries.

use std::collections::BTreeMap;

use idp_connect_contract::{
    CompleteAuthorizationRequest, FederatedOAuthError, ProviderTokens, ProviderUserProfile,
};
use oauth_connect_configuration::{AccessTokenAuth, JsonGetApi, TokenAuthMethod};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::Value;
use url::{form_urlencoded, Url};

use super::{invalid_declaration, OAuthClient};
use crate::{transport::read_json, ApiId, ApiResponses, DomainError};

/// Immutable, already validated definition; no operation-specific data is stored.
pub(super) struct JsonApi {
    url: Url,
    auth: AccessTokenAuth,
    headers: HeaderMap,
}

/// # Errors
/// Refuses invalid URL/header representations without exposing declaration data.
pub(super) fn compile_apis(
    definitions: &BTreeMap<ApiId, JsonGetApi>,
) -> Result<BTreeMap<ApiId, JsonApi>, DomainError> {
    let mut apis = BTreeMap::new();
    for (id, definition) in definitions {
        let mut headers = HeaderMap::new();
        for (name, value) in &definition.headers {
            let name =
                HeaderName::from_bytes(name.as_bytes()).map_err(|_| invalid_declaration())?;
            let value = HeaderValue::from_str(value).map_err(|_| invalid_declaration())?;
            let _ = headers.insert(name, value);
        }
        let _ = apis.insert(
            id.clone(),
            JsonApi {
                url: Url::parse(&definition.url).map_err(|_| invalid_declaration())?,
                auth: definition.access_token_auth,
                headers,
            },
        );
    }
    Ok(apis)
}

impl OAuthClient {
    /// # Errors
    /// Returns the sanitized exchange category on request, body or token failure.
    pub(super) async fn exchange(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        let error = FederatedOAuthError::ExchangeCode;
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", request.code.as_str()),
            ("redirect_uri", request.redirect_uri.as_str()),
        ];
        if let Some(verifier) = request.code_verifier.as_deref() {
            form.push(("code_verifier", verifier));
        }
        let mut post = self
            .http
            .post(self.token_url.clone())
            .header(ACCEPT, "application/json");
        match self.token_auth_method {
            TokenAuthMethod::ClientSecretPost => {
                form.push(("client_id", &self.client_id));
                form.push(("client_secret", self.client_secret.expose_secret()));
            }
            TokenAuthMethod::ClientSecretBasic => {
                // RFC 6749 §2.3.1: form-encode each credential BEFORE Basic base64.
                // reqwest marks this Authorization header sensitive itself.
                let id: String =
                    form_urlencoded::byte_serialize(self.client_id.as_bytes()).collect();
                let secret: String =
                    form_urlencoded::byte_serialize(self.client_secret.expose_secret().as_bytes())
                        .collect();
                post = post.basic_auth(id, Some(secret));
            }
        }
        let response = post.form(&form).send().await.map_err(|_| error)?;
        let value = read_json(response, error).await?;
        parse_tokens(&value)
    }

    /// # Errors
    /// Returns the sanitized profile category without returning partial tokens.
    pub(super) async fn profile(
        &self,
        access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError> {
        let error = FederatedOAuthError::UserProfile;
        let mut responses = ApiResponses::new();
        // Declarative validation requires every definition to be used. This loop
        // is local to one completion; no response/token cache or chaining exists.
        for (id, api) in &self.apis {
            let scheme = match api.auth {
                AccessTokenAuth::Bearer => "Bearer",
                AccessTokenAuth::Token => "token",
            };
            let mut authorization =
                HeaderValue::from_str(&format!("{scheme} {access_token}")).map_err(|_| error)?;
            authorization.set_sensitive(true);
            let response = self
                .http
                .get(api.url.clone())
                .headers(api.headers.clone())
                .header(AUTHORIZATION, authorization)
                .send()
                .await
                .map_err(|_| error)?;
            let value = read_json(response, error).await?;
            let _ = responses.insert(id.clone(), value);
        }
        self.mapping.project(&responses).map_err(|_| error)
    }
}

fn parse_tokens(value: &Value) -> Result<ProviderTokens, FederatedOAuthError> {
    let error = FederatedOAuthError::ExchangeCode;
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or(error)?;
    let token_type = value
        .get("token_type")
        .and_then(Value::as_str)
        .ok_or(error)?;
    if access_token.trim().is_empty() || !token_type.eq_ignore_ascii_case("Bearer") {
        return Err(error);
    }
    let refresh_token = match value.get("refresh_token") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.clone()),
        _ => return Err(error),
    };
    let expires_in = match value.get("expires_in") {
        None | Some(Value::Null) => None,
        Some(value) => Some(value.as_u64().ok_or(error)?),
    };
    Ok(ProviderTokens {
        access_token: access_token.to_owned(),
        refresh_token,
        expires_in,
    })
}
