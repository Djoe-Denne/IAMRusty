//! In-process rate limiting for public IAM auth endpoints.

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(60);
const LIMIT: u32 = 30;

static INTERNAL_TOKEN: OnceLock<String> = OnceLock::new();
static BUCKETS: OnceLock<Mutex<HashMap<String, (Instant, u32)>>> = OnceLock::new();

/// Configure the shared secret required by internal `IdP` token routes.
pub fn configure_internal_service_token(token: impl Into<String>) {
    let _ = INTERNAL_TOKEN.set(token.into());
}

/// Required header value for `/internal/{provider}/token` and `/revoke`.
///
/// # Errors
///
/// Returns [`StatusCode::FORBIDDEN`] when the shared secret is unset or the header does not match.
pub fn require_internal_service_token(headers: &HeaderMap) -> Result<(), StatusCode> {
    let expected = INTERNAL_TOKEN.get().map_or("", String::as_str);
    if expected.is_empty() {
        return Err(StatusCode::FORBIDDEN);
    }
    let presented = headers
        .get("x-iam-internal-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if constant_time_token_eq(presented.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Compare two buffers without exiting on the first differing byte.
/// Length mismatch is folded into the accumulator; both sides are always scanned.
fn constant_time_token_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut acc = left.len() ^ right.len();
    for i in 0..max_len {
        let l = left.get(i).copied().unwrap_or(0);
        let r = right.get(i).copied().unwrap_or(0);
        acc |= usize::from(l ^ r);
    }
    acc == 0
}

fn buckets() -> &'static Mutex<HashMap<String, (Instant, u32)>> {
    BUCKETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn rate_limit_disabled() -> bool {
    matches!(
        std::env::var("IAM_RATE_LIMIT_DISABLED").ok().as_deref(),
        Some("1" | "true")
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

    let allowed = take_slot(key);

    if allowed {
        next.run(request).await
    } else {
        StatusCode::TOO_MANY_REQUESTS.into_response()
    }
}

fn take_slot(key: String) -> bool {
    let mut store = buckets().lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    let entry = store.entry(key).or_insert((now, 0));
    if now.duration_since(entry.0) > WINDOW {
        *entry = (now, 0);
    }
    entry.1 += 1;
    let allowed = entry.1 <= LIMIT;
    drop(store);
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_token(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-iam-internal-token", token.parse().expect("header"));
        headers
    }

    #[test]
    fn internal_token_equal() {
        configure_internal_service_token("iam-handler-unit-token");
        assert!(constant_time_token_eq(b"same-secret", b"same-secret"));
        assert_eq!(
            require_internal_service_token(&headers_with_token("iam-handler-unit-token")),
            Ok(())
        );
    }

    #[test]
    fn internal_token_different() {
        configure_internal_service_token("iam-handler-unit-token");
        assert!(!constant_time_token_eq(b"same-secret", b"other-secret"));
        assert_eq!(
            require_internal_service_token(&headers_with_token("wrong-handler-unit-token")),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn internal_token_different_length() {
        configure_internal_service_token("iam-handler-unit-token");
        assert!(!constant_time_token_eq(b"short", b"much-longer-token"));
        assert!(!constant_time_token_eq(b"much-longer-token", b"short"));
        assert_eq!(
            require_internal_service_token(&headers_with_token("short")),
            Err(StatusCode::FORBIDDEN)
        );
    }
}
