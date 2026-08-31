//! In-process rate limiting for public IAM auth endpoints.

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(60);
const LIMIT: u32 = 30;

static INTERNAL_TOKEN: OnceLock<String> = OnceLock::new();
static BUCKETS: OnceLock<Mutex<HashMap<String, (Instant, u32)>>> = OnceLock::new();

/// Configure the shared secret required by internal IdP token routes.
pub fn configure_internal_service_token(token: impl Into<String>) {
    let _ = INTERNAL_TOKEN.set(token.into());
}

/// Required header value for `/internal/{provider}/token`.
pub fn require_internal_service_token(headers: &HeaderMap) -> Result<(), StatusCode> {
    let expected = INTERNAL_TOKEN.get().map(String::as_str).unwrap_or("");
    if expected.is_empty() {
        return Err(StatusCode::FORBIDDEN);
    }
    let presented = headers
        .get("x-iam-internal-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if presented == expected {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

fn buckets() -> &'static Mutex<HashMap<String, (Instant, u32)>> {
    BUCKETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn rate_limit_disabled() -> bool {
    matches!(
        std::env::var("IAM_RATE_LIMIT_DISABLED").ok().as_deref(),
        Some("1") | Some("true")
    ) || std::env::var("APP_ENV").ok().as_deref() == Some("test")
}

fn is_limited_path(path: &str) -> bool {
    let path = path.strip_prefix("/iam").unwrap_or(path);
    matches!(
        path,
        "/api/auth/signup"
            | "/api/auth/login"
            | "/api/auth/verify"
            | "/api/auth/resend-verification"
            | "/api/auth/password/reset-request"
            | "/api/auth/password/reset-confirm"
            | "/api/auth/password/reset-validate"
    ) || path.ends_with("/login") && path.contains("/api/auth/")
}

/// Middleware that rate-limits public auth endpoints.
pub async fn rate_limit_auth(request: Request, next: Next) -> Response {
    if rate_limit_disabled() || !is_limited_path(request.uri().path()) {
        return next.run(request).await;
    }

    let key = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("local")
        .to_string();
    let key = format!("{key}:{}", request.uri().path());

    let allowed = {
        let mut store = buckets().lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let entry = store.entry(key).or_insert((now, 0));
        if now.duration_since(entry.0) > WINDOW {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= LIMIT
    };

    if allowed {
        next.run(request).await
    } else {
        StatusCode::TOO_MANY_REQUESTS.into_response()
    }
}
