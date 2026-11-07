//! The standard protocol's sole HTTP path: guarded URLs, original bounded bytes.

use std::{collections::BTreeSet, future::Future, pin::Pin, sync::Arc};

use idp_connect_contract::FederatedOAuthError;
use oauth_connect_configuration::{
    is_reserved_authorization_parameter, OidcProfile, VendorTransportSecurity,
};
use openidconnect::{
    core::CoreProviderMetadata, AsyncHttpClient, HttpRequest, HttpResponse, IssuerUrl,
};
use reqwest::{header::AUTHORIZATION, Client};
use url::Url;

#[derive(Clone)]
pub(super) struct BoundHttp {
    client: Client,
    origins: Arc<BTreeSet<String>>,
    security: VendorTransportSecurity,
}

impl BoundHttp {
    pub(super) fn new(
        client: Client,
        declaration: &OidcProfile,
        security: VendorTransportSecurity,
    ) -> Result<Self, FederatedOAuthError> {
        let origins = declaration
            .allowed_endpoint_origins
            .iter()
            .map(|origin| {
                Url::parse(origin)
                    .map(|url| url.origin().ascii_serialization())
                    .map_err(|_| FederatedOAuthError::ExchangeCode)
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        Ok(Self {
            client,
            origins: Arc::new(origins),
            security,
        })
    }

    pub(super) fn guard(&self, raw: &str) -> Result<Url, FederatedOAuthError> {
        let error = FederatedOAuthError::ExchangeCode;
        let lower = raw.to_ascii_lowercase();
        if raw.chars().any(|c| c.is_whitespace() || c.is_control())
            || raw.contains(['\\', '{', '}'])
            || lower.contains("%7b")
            || lower.contains("%7d")
        {
            return Err(error);
        }
        let url = Url::parse(raw).map_err(|_| error)?;
        let authority = raw
            .split_once("://")
            .and_then(|(_, tail)| tail.split(['/', '?', '#']).next())
            .ok_or(error)?;
        if authority.contains('@')
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || url.host().is_none()
            || !(url.scheme() == "https"
                || (url.scheme() == "http" && self.security != VendorTransportSecurity::Verified))
            || !self.origins.contains(&url.origin().ascii_serialization())
        {
            return Err(error);
        }
        Ok(url)
    }

    pub(super) fn guard_endpoints(
        &self,
        metadata: &CoreProviderMetadata,
    ) -> Result<(), FederatedOAuthError> {
        let authorize = self.guard(metadata.authorization_endpoint().as_str())?;
        if authorize
            .query_pairs()
            .any(|(key, _)| is_reserved_authorization_parameter(key.as_ref()))
        {
            return Err(FederatedOAuthError::ExchangeCode);
        }
        self.guard(
            metadata
                .token_endpoint()
                .ok_or(FederatedOAuthError::ExchangeCode)?
                .as_str(),
        )?;
        self.guard(metadata.jwks_uri().as_str())?;
        if let Some(endpoint) = metadata.userinfo_endpoint() {
            self.guard(endpoint.as_str())?;
        }
        Ok(())
    }

    pub(super) async fn discover(
        &self,
        issuer: IssuerUrl,
    ) -> Result<CoreProviderMetadata, FederatedOAuthError> {
        let discovery_url = issuer
            .join(".well-known/openid-configuration")
            .map_err(|_| FederatedOAuthError::ExchangeCode)?
            .to_string();
        let callback = |request: HttpRequest| {
            let is_metadata = request.uri().to_string() == discovery_url;
            async move {
                let response = self.send(request).await?;
                if is_metadata {
                    // http::Uri drops fragments before AsyncHttpClient sees the
                    // internal JWKS request. Inspect the STANDARD metadata type's
                    // original URL strings before allowing that request to exist.
                    // The standard discovery parser still receives unchanged bytes
                    // (including duplicate fields); no Value/reserialization path.
                    let metadata: CoreProviderMetadata = serde_json::from_slice(response.body())
                        .map_err(|_| FederatedOAuthError::ExchangeCode)?;
                    self.guard_endpoints(&metadata)?;
                }
                Ok::<_, FederatedOAuthError>(response)
            }
        };
        CoreProviderMetadata::discover_async(issuer, &callback)
            .await
            .map_err(|_| FederatedOAuthError::ExchangeCode)
    }

    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, FederatedOAuthError> {
        let error = FederatedOAuthError::ExchangeCode;
        // Even Discovery's internal JWKS request goes through this pre-I/O guard.
        let url = self.guard(&request.uri().to_string())?;
        let (mut parts, body) = request.into_parts();
        if let Some(value) = parts.headers.get_mut(AUTHORIZATION) {
            value.set_sensitive(true);
        }
        let response = self
            .client
            .request(parts.method, url)
            .headers(parts.headers)
            .body(body)
            .send()
            .await
            .map_err(|_| error)?;
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = crate::transport::read_bytes(response, error).await?;
        let mut response = HttpResponse::new(bytes);
        *response.status_mut() = status;
        *response.headers_mut() = headers;
        Ok(response)
    }
}

impl<'a> AsyncHttpClient<'a> for BoundHttp {
    type Error = FederatedOAuthError;
    type Future = Pin<Box<dyn Future<Output = Result<HttpResponse, Self::Error>> + Send + 'a>>;
    fn call(&'a self, request: HttpRequest) -> Self::Future {
        Box::pin(self.send(request))
    }
}
