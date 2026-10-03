//! Axum 0.8 HMAC middleware and three POST handlers.
//!
//! HMAC `PATH` is the **full received path**, including `SERVICE_PREFIX`
//! (example: `/github-connect/v1/token`, not `/v1/token`). This middleware
//! reads [`axum::extract::OriginalUri`] so verification stays correct when the
//! router is nested under a prefix. Callers that do not nest may still apply
//! the layer: `OriginalUri` then equals the request path.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::extract::{OriginalUri, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};

use crate::client::FederatedOAuthClient;
use crate::dto::{
    AuthorizeRequest, ProfileRequest, TokenRequest, AUTHORIZE_PATH, PROFILE_PATH, TOKEN_PATH,
};
use crate::hmac::{self, HmacKey, SIGNATURE_HEADER, TIMESTAMP_HEADER};

/// Combined state for HMAC + federated client handlers.
#[derive(Clone)]
pub struct ConnectState {
    /// Shared HMAC secret (never logged).
    pub hmac_key: HmacKey,
    /// Connector implementation.
    pub client: Arc<dyn FederatedOAuthClient>,
}

/// Router exposing `POST` `/v1/authorize`, `/v1/token`, `/v1/profile`.
///
/// Nest this under the service prefix (example: `/github-connect`). HMAC uses
/// the prefixed path via [`OriginalUri`].
pub fn router(state: ConnectState) -> Router {
    let hmac_key = state.hmac_key.clone();
    Router::new()
        .route(AUTHORIZE_PATH, post(handle_authorize))
        .route(TOKEN_PATH, post(handle_token))
        .route(PROFILE_PATH, post(handle_profile))
        .layer(middleware::from_fn_with_state(hmac_key, hmac_middleware))
        .with_state(state.client)
}

/// Verifies `IdP` Connect HMAC, then forwards the (re-assembled) request.
///
/// Missing headers, a stale timestamp, a bad signature, or a non-UTF-8 body
/// all become `401`. Secrets, codes, and tokens are not logged.
///
/// # Errors
///
/// Returns [`StatusCode::UNAUTHORIZED`] when HMAC verification fails, or
/// [`StatusCode::PAYLOAD_TOO_LARGE`] when the body exceeds the buffer limit.
pub async fn hmac_middleware(
    State(key): State<HmacKey>,
    OriginalUri(uri): OriginalUri,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method().as_str().to_owned();
    let path = uri.path().to_owned();
    let timestamp = parse_timestamp_header(request.headers())?;
    let signature = header_str(request.headers(), SIGNATURE_HEADER)?.to_owned();
    let now = hmac::unix_timestamp_secs().map_err(|_| StatusCode::UNAUTHORIZED)?;

    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, 64 * 1024)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
    let body_str = std::str::from_utf8(&bytes).map_err(|_| StatusCode::UNAUTHORIZED)?;

    hmac::verify(
        key.as_bytes(),
        &method,
        &path,
        timestamp,
        body_str,
        &signature,
        now,
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let request = Request::from_parts(parts, Body::from(bytes));
    Ok(next.run(request).await)
}

fn header_str<'a>(
    headers: &'a axum::http::HeaderMap,
    name: &'static str,
) -> Result<&'a str, StatusCode> {
    headers
        .get(name)
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

fn parse_timestamp_header(headers: &axum::http::HeaderMap) -> Result<i64, StatusCode> {
    header_str(headers, TIMESTAMP_HEADER)?
        .parse()
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

fn json_or_bad_gateway<T: serde::Serialize>(
    result: Result<T, crate::FederatedOAuthError>,
) -> Response {
    result.map_or_else(
        |_| StatusCode::BAD_GATEWAY.into_response(),
        |body| (StatusCode::OK, Json(body)).into_response(),
    )
}

async fn handle_authorize(
    State(client): State<Arc<dyn FederatedOAuthClient>>,
    Json(req): Json<AuthorizeRequest>,
) -> impl IntoResponse {
    json_or_bad_gateway(client.authorize(&req.redirect_uri, &req.state).await)
}

async fn handle_token(
    State(client): State<Arc<dyn FederatedOAuthClient>>,
    Json(req): Json<TokenRequest>,
) -> impl IntoResponse {
    json_or_bad_gateway(client.exchange_code(&req.code, &req.redirect_uri).await)
}

async fn handle_profile(
    State(client): State<Arc<dyn FederatedOAuthClient>>,
    Json(req): Json<ProfileRequest>,
) -> impl IntoResponse {
    json_or_bad_gateway(client.user_profile(&req.access_token).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{AuthorizeResponse, ProviderTokens, ProviderUserProfile};
    use crate::error::FederatedOAuthError;
    use crate::hmac::{sign, unix_timestamp_secs};
    use async_trait::async_trait;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    struct StubClient;

    #[async_trait]
    impl FederatedOAuthClient for StubClient {
        async fn authorize(
            &self,
            _redirect_uri: &str,
            _state: &str,
        ) -> Result<AuthorizeResponse, FederatedOAuthError> {
            Ok(AuthorizeResponse {
                authorization_url: "https://idp.example/authorize".into(),
                scope: "read:user".into(),
            })
        }

        async fn exchange_code(
            &self,
            _code: &str,
            _redirect_uri: &str,
        ) -> Result<ProviderTokens, FederatedOAuthError> {
            Ok(ProviderTokens {
                access_token: "tok".into(),
                refresh_token: None,
                expires_in: Some(3600),
            })
        }

        async fn user_profile(
            &self,
            _access_token: &str,
        ) -> Result<ProviderUserProfile, FederatedOAuthError> {
            Ok(ProviderUserProfile {
                id: "42".into(),
                username: "alice".into(),
                email: None,
                avatar_url: None,
                email_verified: false,
            })
        }
    }

    fn signed_server() -> (TestServer, HmacKey) {
        let key = HmacKey::new(b"server-test-secret");
        let inner = router(ConnectState {
            hmac_key: key.clone(),
            client: Arc::new(StubClient),
        });
        let app = Router::new().nest("/github-connect", inner);
        let server = TestServer::new(app).expect("test server");
        (server, key)
    }

    async fn post_signed(
        server: &TestServer,
        key: &HmacKey,
        path: &str,
        hmac_path: &str,
        body: &str,
    ) -> axum_test::TestResponse {
        let now = unix_timestamp_secs().expect("clock");
        let sig = sign(key.as_bytes(), "POST", hmac_path, now, body).expect("sign");
        server
            .post(path)
            .add_header(TIMESTAMP_HEADER, now.to_string())
            .add_header(SIGNATURE_HEADER, sig)
            .content_type("application/json")
            .bytes(axum::body::Bytes::from(body.to_owned()))
            .await
    }

    #[tokio::test]
    async fn nested_router_verifies_hmac_against_prefixed_path() {
        let (server, key) = signed_server();
        let body = r#"{"code":"x","redirect_uri":"https://app.example/cb"}"#;

        let ok = post_signed(
            &server,
            &key,
            "/github-connect/v1/token",
            "/github-connect/v1/token",
            body,
        )
        .await;
        ok.assert_status_ok();
        let tokens: ProviderTokens = ok.json();
        assert_eq!(tokens.access_token, "tok");

        let bare = post_signed(&server, &key, "/github-connect/v1/token", "/v1/token", body).await;
        bare.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn missing_hmac_headers_are_unauthorized() {
        let (server, _key) = signed_server();
        let response = server
            .post("/github-connect/v1/token")
            .content_type("application/json")
            .bytes(axum::body::Bytes::from_static(
                br#"{"code":"x","redirect_uri":"https://app.example/cb"}"#,
            ))
            .await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}
