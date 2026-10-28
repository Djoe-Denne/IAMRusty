//! HTTP forwarding only: TLS is provided by the existing t11b live-server helper.
//! The authoritative C collaborator is still WireMock, not an in-process port.

use axum::{
    body::{to_bytes, Body},
    http::{Request, Response, StatusCode},
    Router,
};

pub fn wiremock_proxy_router(upstream: String) -> Router {
    let client = reqwest::Client::new();
    Router::new().fallback(move |request: Request<Body>| {
        let upstream = upstream.clone();
        let client = client.clone();
        async move { forward(request, &upstream, &client).await }
    })
}

async fn forward(
    request: Request<Body>,
    upstream: &str,
    client: &reqwest::Client,
) -> Result<Response<Body>, StatusCode> {
    let (mut parts, body) = request.into_parts();
    let suffix = parts
        .uri
        .path_and_query()
        .map_or("/", |value| value.as_str());
    let bytes = to_bytes(body, 262_144)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
    // The relay is transport-only. Preserve path/query/body for WireMock's exact
    // matchers; do not claim service-origin authorization from forwarded headers.
    parts.headers.remove("host");
    let response = client
        .request(parts.method, format!("{upstream}{suffix}"))
        .headers(parts.headers)
        .body(bytes)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let mut result = Response::builder().status(response.status());
    for (name, value) in response.headers() {
        result = result.header(name, value);
    }
    result
        .body(Body::from(
            response
                .bytes()
                .await
                .map_err(|_| StatusCode::BAD_GATEWAY)?,
        ))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
