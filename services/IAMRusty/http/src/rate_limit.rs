//! In-process rate limiting for public IAM auth endpoints.

use axum::{
    body::{to_bytes, Body},
    extract::{ConnectInfo, Request},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use hmac::{Hmac, Mac};
use iam_configuration::security::{AuthRateLimitConfig, SecurityMode};
use rustycog::http::PeerClientCertificate;
use sha2::Sha256;
use tokio::sync::Semaphore;

const MAX_AUTH_BODY: usize = 16 * 1024;
const MAX_XFF_BYTES: usize = 1024;
const MAX_XFF_HOPS: usize = 8;

static INTERNAL_TOKEN: OnceLock<String> = OnceLock::new();

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

pub(crate) fn is_limited_path(path: &str) -> bool {
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
            | "/api/auth/password/reset-authenticated"
            | "/api/auth/complete-registration"
            | "/api/auth/username/check"
            | "/api/token/refresh"
    ) || path.ends_with("/login") && path.contains("/api/auth/")
        || path.ends_with("/callback") && path.contains("/api/auth/")
        || path.ends_with("/relink-callback") && path.contains("/api/auth/")
        || path.ends_with("/link") && path.contains("/api/auth/")
        || path.ends_with("/relink-start") && path.contains("/api/auth/")
}

#[derive(Hash, PartialEq, Eq)]
enum BucketKey {
    Source(IpAddr),
    Account([u8; 32]),
}

/// Per-router limiter, without process environment bypasses or a global mutable map.
pub struct AuthRateLimiter {
    config: AuthRateLimitConfig,
    buckets: Mutex<HashMap<BucketKey, (Instant, u32)>>,
    concurrency: Arc<Semaphore>,
    account_key: [u8; 32],
}

impl AuthRateLimiter {
    /// Construct at boot; excessive bounds and implicit exemptions are refused.
    ///
    /// # Errors
    /// Rejects invalid budgets, unbounded proxy configuration or a non-test disable.
    pub fn new(config: AuthRateLimitConfig, mode: SecurityMode) -> Result<Self, &'static str> {
        if config.source_limit == 0
            || config.account_limit == 0
            || !(1..=3600).contains(&config.window_seconds)
            || !(2..=100_000).contains(&config.max_buckets)
            || !(1..=256).contains(&config.max_concurrent)
            || config.trusted_proxies.len() > 32
            || config.trusted_proxies.iter().any(|proxy| !proxy.is_valid())
            || (config.disabled && mode != SecurityMode::IsolatedTest)
        {
            return Err("invalid IAM rate-limit policy");
        }
        let mut account_key = [0; 32];
        account_key[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        account_key[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        Ok(Self {
            concurrency: Arc::new(Semaphore::new(config.max_concurrent)),
            config,
            buckets: Mutex::new(HashMap::new()),
            account_key,
        })
    }

    fn take_slot(&self, key: BucketKey, limit: u32, now: Instant) -> bool {
        let mut store = self.buckets.lock().unwrap_or_else(PoisonError::into_inner);
        let window = Duration::from_secs(self.config.window_seconds);
        // Evict only expired buckets: random sources cannot evict a live account budget.
        store.retain(|_, (start, _)| now.saturating_duration_since(*start) < window);
        if !store.contains_key(&key) && store.len() >= self.config.max_buckets {
            return false;
        }
        let entry = store.entry(key).or_insert((now, 0));
        entry.1 = entry.1.saturating_add(1);
        entry.1 <= limit
    }

    fn account_digest(&self, body: &[u8]) -> Option<[u8; 32]> {
        let value: serde_json::Value = serde_json::from_slice(body).ok()?;
        let (namespace, account) =
            if let Some(email) = value.get("email").and_then(serde_json::Value::as_str) {
                ("email:", email)
            } else {
                ("username:", value.get("username")?.as_str()?)
            };
        self.digest_identifier(namespace, account)
    }

    fn query_account_digest(&self, query: &str) -> Option<[u8; 32]> {
        url::form_urlencoded::parse(query.as_bytes()).find_map(|(name, value)| {
            match name.as_ref() {
                "email" => self.digest_identifier("email:", &value),
                "username" => self.digest_identifier("username:", &value),
                _ => None,
            }
        })
    }

    fn digest_identifier(&self, namespace: &str, account: &str) -> Option<[u8; 32]> {
        if account.len() > 320 || account.trim().is_empty() {
            return None;
        }
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.account_key).ok()?;
        mac.update(namespace.as_bytes());
        mac.update(account.trim().to_lowercase().as_bytes());
        Some(mac.finalize().into_bytes().into())
    }

    /// Charge the verified human subject for authenticated mutation endpoints.
    pub(crate) fn take_authenticated_account(&self, user_id: uuid::Uuid) -> bool {
        if self.config.disabled {
            return true;
        }
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&self.account_key) else {
            return false;
        };
        mac.update(b"subject:");
        mac.update(user_id.as_bytes());
        self.take_slot(
            BucketKey::Account(mac.finalize().into_bytes().into()),
            self.config.account_limit,
            Instant::now(),
        )
    }

    fn source(&self, request: &Request) -> Result<IpAddr, StatusCode> {
        let peer = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
            .0
            .ip();
        let Some(proxy) = self
            .config
            .trusted_proxies
            .iter()
            .find(|proxy| proxy.matches_peer(peer))
        else {
            // Even a syntactically valid XFF from an ordinary client is untrusted.
            return Ok(peer);
        };
        let certificate = request
            .extensions()
            .get::<PeerClientCertificate>()
            .ok_or(StatusCode::FORBIDDEN)?;
        if !peer_has_san(&certificate.der, &proxy.dns_san) {
            return Err(StatusCode::FORBIDDEN);
        }
        forwarded_source(request.headers())
    }
}

fn peer_has_san(der: &[u8], expected: &str) -> bool {
    if der.len() > 16 * 1024 {
        return false;
    }
    let Ok((_, cert)) = x509_parser::parse_x509_certificate(der) else {
        return false;
    };
    let Ok(Some(san)) = cert.subject_alternative_name() else {
        return false;
    };
    san.value.general_names.iter().any(|name| {
        matches!(name,
        x509_parser::extensions::GeneralName::DNSName(dns) if dns.eq_ignore_ascii_case(expected))
    })
}

fn forwarded_source(headers: &HeaderMap) -> Result<IpAddr, StatusCode> {
    let mut values = headers.get_all("x-forwarded-for").iter();
    let raw = values.next().ok_or(StatusCode::BAD_REQUEST)?;
    if values.next().is_some() || raw.as_bytes().len() > MAX_XFF_BYTES {
        return Err(StatusCode::BAD_REQUEST);
    }
    let raw = raw.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut source = None;
    for (index, hop) in raw.split(',').enumerate() {
        if index >= MAX_XFF_HOPS {
            return Err(StatusCode::BAD_REQUEST);
        }
        source = Some(hop.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    }
    // Rightmost is the real peer appended by the trusted proxy, not the leftmost caller claim.
    source.ok_or(StatusCode::BAD_REQUEST)
}

/// Middleware that rate-limits authentication work. A reliable transport peer is mandatory.
/// Holds only a semaphore permit across awaits, never a blocking map lock.
pub async fn rate_limit_auth(request: Request, next: Next) -> Response {
    if !is_limited_path(request.uri().path()) {
        return next.run(request).await;
    }
    let Some(limiter) = request.extensions().get::<Arc<AuthRateLimiter>>().cloned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    if limiter.config.disabled {
        return next.run(request).await;
    }
    let source = match limiter.source(&request) {
        Ok(source) => source,
        Err(status) => return status.into_response(),
    };
    if !limiter.take_slot(
        BucketKey::Source(source),
        limiter.config.source_limit,
        Instant::now(),
    ) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let Ok(_permit) = limiter.concurrency.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    let (parts, body) = request.into_parts();
    let bytes =
        match tokio::time::timeout(Duration::from_secs(5), to_bytes(body, MAX_AUTH_BODY)).await {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(_)) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
            Err(_) => return StatusCode::REQUEST_TIMEOUT.into_response(),
        };
    if parts.uri.query().is_some_and(|query| query.len() > 4096) {
        return StatusCode::URI_TOO_LONG.into_response();
    }
    let account =
        if parts.method == axum::http::Method::GET || parts.method == axum::http::Method::HEAD {
            parts
                .uri
                .query()
                .and_then(|query| limiter.query_account_digest(query))
        } else {
            limiter.account_digest(&bytes)
        };
    if let Some(account) = account {
        if !limiter.take_slot(
            BucketKey::Account(account),
            limiter.config.account_limit,
            Instant::now(),
        ) {
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        }
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter(max_buckets: usize) -> AuthRateLimiter {
        AuthRateLimiter::new(
            AuthRateLimitConfig {
                source_limit: 2,
                account_limit: 2,
                max_buckets,
                ..AuthRateLimitConfig::default()
            },
            SecurityMode::Verified,
        )
        .unwrap()
    }

    fn request_from(peer: &str, forwarded: &str) -> Request {
        let mut request = Request::builder()
            .uri("/api/auth/login")
            .header("x-forwarded-for", forwarded)
            .body(Body::empty())
            .unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
        request
    }

    fn mesh_limiter() -> AuthRateLimiter {
        AuthRateLimiter::new(
            AuthRateLimitConfig {
                source_limit: 2,
                trusted_proxies: vec![iam_configuration::security::TrustedAuthProxy {
                    ip: "10.244.0.0".parse().unwrap(),
                    prefix_len: Some(16),
                    dns_san: "envoy-mesh".into(),
                }],
                ..AuthRateLimitConfig::default()
            },
            SecurityMode::Verified,
        )
        .unwrap()
    }

    fn mesh_request(forwarded: &str) -> Request {
        use rcgen::{CertificateParams, KeyPair};
        let mut request = request_from("10.244.2.19:3000", forwarded);
        let cert = CertificateParams::new(vec!["envoy-mesh".to_string()])
            .unwrap()
            .self_signed(&KeyPair::generate().unwrap())
            .unwrap();
        // Transport certificate verification happens before middleware; no network fixture.
        request.extensions_mut().insert(PeerClientCertificate {
            der: cert.der().to_vec(),
        });
        request
    }

    #[test]
    fn declared_mesh_proxy_clients_have_independent_source_quotas() {
        let limiter = mesh_limiter();
        let now = Instant::now();
        for client in ["192.0.2.1", "192.0.2.2"] {
            let request = mesh_request(client);
            let source = limiter.source(&request).unwrap();
            assert_eq!(source, client.parse::<IpAddr>().unwrap());
            for attempt in 0..3 {
                assert_eq!(
                    limiter.take_slot(BucketKey::Source(source), limiter.config.source_limit, now),
                    attempt < 2
                );
            }
        }
        assert_eq!(limiter.buckets.lock().unwrap().len(), 2);
    }

    #[test]
    fn declared_mesh_proxy_appended_peer_prevents_forged_xff_quota_bypass() {
        let limiter = mesh_limiter();
        let now = Instant::now();
        for (attempt, forged) in ["192.0.2.90", "192.0.2.91", "192.0.2.92"]
            .into_iter()
            .enumerate()
        {
            let request = mesh_request(&format!("{forged}, 192.0.2.1"));
            let source = limiter.source(&request).unwrap();
            assert_eq!(source, "192.0.2.1".parse::<IpAddr>().unwrap());
            assert_eq!(
                limiter.take_slot(BucketKey::Source(source), limiter.config.source_limit, now),
                attempt < 2
            );
        }
    }

    #[test]
    fn direct_peer_outside_declared_mesh_proxy_ignores_forged_xff() {
        let limiter = mesh_limiter();
        let now = Instant::now();
        for (attempt, forged) in ["192.0.2.1", "192.0.2.2", "invalid"]
            .into_iter()
            .enumerate()
        {
            let request = request_from("198.51.100.1:3000", forged);
            let source = limiter.source(&request).unwrap();
            assert_eq!(source, "198.51.100.1".parse::<IpAddr>().unwrap());
            assert_eq!(
                limiter.take_slot(BucketKey::Source(source), limiter.config.source_limit, now),
                attempt < 2
            );
        }
    }

    #[test]
    fn declared_mesh_proxy_still_requires_certificate_and_bounded_hops() {
        let limiter = mesh_limiter();
        let request = request_from("10.244.2.19:3000", "192.0.2.1");
        assert_eq!(limiter.source(&request), Err(StatusCode::FORBIDDEN));
        let at_bound = std::iter::repeat("192.0.2.1")
            .take(MAX_XFF_HOPS)
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            limiter.source(&mesh_request(&at_bound)),
            Ok("192.0.2.1".parse().unwrap())
        );
        let over_bound = format!("{at_bound},192.0.2.1");
        assert_eq!(
            limiter.source(&mesh_request(&over_bound)),
            Err(StatusCode::BAD_REQUEST)
        );
        assert!(limiter.buckets.lock().unwrap().is_empty());
    }

    #[test]
    fn invalid_proxy_network_is_rejected_at_limiter_boot() {
        let config = AuthRateLimitConfig {
            trusted_proxies: vec![iam_configuration::security::TrustedAuthProxy {
                ip: "0.0.0.0".parse().unwrap(),
                prefix_len: Some(0),
                dns_san: "envoy-mesh".into(),
            }],
            ..AuthRateLimitConfig::default()
        };
        assert!(AuthRateLimiter::new(config, SecurityMode::Verified).is_err());
    }

    #[test]
    fn spoofed_headers_do_not_change_untrusted_source_budget() {
        let limiter = limiter(8);
        let now = Instant::now();
        for (index, forwarded) in ["192.0.2.1", "192.0.2.2", "192.0.2.3"]
            .into_iter()
            .enumerate()
        {
            let request = request_from("198.51.100.1:3000", forwarded);
            let source = limiter.source(&request).unwrap();
            assert_eq!(source, "198.51.100.1".parse::<IpAddr>().unwrap());
            assert_eq!(
                limiter.take_slot(BucketKey::Source(source), 2, now),
                index < 2
            );
        }
    }

    #[test]
    fn missing_peer_never_becomes_a_shared_local_bucket() {
        let request = Request::new(Body::empty());
        assert_eq!(
            limiter(8).source(&request),
            Err(StatusCode::SERVICE_UNAVAILABLE)
        );
    }

    #[test]
    fn trusted_ip_without_verified_tls_certificate_is_not_enough() {
        let mut config = AuthRateLimitConfig::default();
        config
            .trusted_proxies
            .push(iam_configuration::security::TrustedAuthProxy {
                ip: "198.51.100.1".parse().unwrap(),
                prefix_len: None,
                dns_san: "envoy-mesh".into(),
            });
        let limiter = AuthRateLimiter::new(config, SecurityMode::Verified).unwrap();
        let mut request = request_from("198.51.100.1:3000", "192.0.2.1");
        assert_eq!(limiter.source(&request), Err(StatusCode::FORBIDDEN));
        request
            .extensions_mut()
            .insert(PeerClientCertificate { der: vec![0; 10] });
        assert_eq!(limiter.source(&request), Err(StatusCode::FORBIDDEN));
    }

    #[test]
    fn only_allowlisted_proxy_tls_san_can_forward_client_source() {
        use rcgen::{CertificateParams, KeyPair};
        let config = AuthRateLimitConfig {
            trusted_proxies: vec![iam_configuration::security::TrustedAuthProxy {
                ip: "198.51.100.1".parse().unwrap(),
                prefix_len: None,
                dns_san: "envoy-mesh".into(),
            }],
            ..AuthRateLimitConfig::default()
        };
        let limiter = AuthRateLimiter::new(config, SecurityMode::Verified).unwrap();
        let mut request = request_from("198.51.100.1:3000", "192.0.2.99, 192.0.2.1");
        for (san, expected) in [
            ("other-client", Err(StatusCode::FORBIDDEN)),
            ("envoy-mesh", Ok("192.0.2.1".parse::<IpAddr>().unwrap())),
        ] {
            let cert = CertificateParams::new(vec![san.to_string()])
                .unwrap()
                .self_signed(&KeyPair::generate().unwrap())
                .unwrap();
            request.extensions_mut().insert(PeerClientCertificate {
                der: cert.der().to_vec(),
            });
            assert_eq!(limiter.source(&request), expected);
        }
    }

    #[test]
    fn trusted_forwarding_is_single_bounded_and_uses_appended_peer() {
        let request = request_from("198.51.100.1:3000", "192.0.2.99, 192.0.2.1");
        assert_eq!(
            forwarded_source(request.headers()).unwrap(),
            "192.0.2.1".parse::<IpAddr>().unwrap()
        );
        let mut headers = request.headers().clone();
        headers.append("x-forwarded-for", "192.0.2.100".parse().unwrap());
        assert_eq!(forwarded_source(&headers), Err(StatusCode::BAD_REQUEST));
        for raw in [
            "unknown".to_owned(),
            "1".repeat(MAX_XFF_BYTES + 1),
            std::iter::repeat("192.0.2.1")
                .take(MAX_XFF_HOPS + 1)
                .collect::<Vec<_>>()
                .join(","),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert("x-forwarded-for", raw.parse().unwrap());
            assert_eq!(forwarded_source(&headers), Err(StatusCode::BAD_REQUEST));
        }
    }

    #[test]
    fn live_buckets_are_not_evicted_and_expiry_is_deterministic() {
        let limiter = limiter(2);
        let now = Instant::now();
        let a = "192.0.2.1".parse().unwrap();
        let b = "192.0.2.2".parse().unwrap();
        let c = "192.0.2.3".parse().unwrap();
        assert!(limiter.take_slot(BucketKey::Source(a), 1, now));
        assert!(limiter.take_slot(BucketKey::Source(b), 1, now));
        assert!(!limiter.take_slot(BucketKey::Source(c), 1, now));
        assert!(!limiter.take_slot(BucketKey::Source(a), 1, now));
        assert_eq!(limiter.buckets.lock().unwrap().len(), 2);
        assert!(limiter.take_slot(BucketKey::Source(c), 1, now + Duration::from_secs(60)));
        assert_eq!(limiter.buckets.lock().unwrap().len(), 1);
    }

    #[test]
    fn account_budget_is_normalized_private_and_independent_of_source() {
        let limiter = limiter(8);
        let a = limiter
            .account_digest(br#"{"email":" Victim@Example.COM "}"#)
            .unwrap();
        let b = limiter
            .account_digest(br#"{"email":"victim@example.com"}"#)
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a,
            limiter
                .query_account_digest("email=Victim%40Example.COM&token=opaque")
                .unwrap()
        );
        let now = Instant::now();
        for index in 1..=3 {
            assert!(limiter.take_slot(BucketKey::Source(IpAddr::from([192, 0, 2, index])), 2, now));
            assert_eq!(limiter.take_slot(BucketKey::Account(a), 2, now), index < 3);
        }
    }

    #[test]
    fn concurrent_admission_never_exceeds_budget() {
        let limiter = Arc::new(limiter(8));
        let now = Instant::now();
        let threads: Vec<_> = (0..32)
            .map(|_| {
                let limiter = limiter.clone();
                std::thread::spawn(move || {
                    limiter.take_slot(BucketKey::Source("192.0.2.1".parse().unwrap()), 2, now)
                })
            })
            .collect();
        assert_eq!(
            threads
                .into_iter()
                .map(|thread| usize::from(thread.join().unwrap()))
                .sum::<usize>(),
            2
        );
    }

    #[test]
    fn concurrent_expensive_work_is_bounded_and_permits_return_on_drop() {
        let limiter = limiter(8);
        let permits: Vec<_> = (0..8)
            .map(|_| limiter.concurrency.clone().try_acquire_owned().unwrap())
            .collect();
        assert!(limiter.concurrency.clone().try_acquire_owned().is_err());
        drop(permits);
        assert!(limiter.concurrency.clone().try_acquire_owned().is_ok());
    }

    #[test]
    fn disable_requires_explicit_isolated_test_mode() {
        let config = AuthRateLimitConfig {
            disabled: true,
            ..AuthRateLimitConfig::default()
        };
        assert!(AuthRateLimiter::new(config.clone(), SecurityMode::Verified).is_err());
        assert!(AuthRateLimiter::new(config, SecurityMode::IsolatedTest).is_ok());
    }

    #[tokio::test]
    async fn middleware_replays_bounded_body_and_cannot_be_bypassed_with_xff() {
        use axum::{middleware, routing::post, Extension, Json, Router};
        use tower::ServiceExt;
        let limiter = Arc::new(limiter(8));
        let app = Router::new()
            .route(
                "/api/auth/login",
                post(|Json(value): Json<serde_json::Value>| async move {
                    assert_eq!(value["email"], "victim@example.com");
                    StatusCode::OK
                }),
            )
            .layer(middleware::from_fn(rate_limit_auth))
            .layer(Extension(limiter));
        for index in 1..=3 {
            let mut request = request_from("198.51.100.1:3000", &format!("192.0.2.{index}"));
            *request.method_mut() = axum::http::Method::POST;
            request
                .headers_mut()
                .insert("content-type", "application/json".parse().unwrap());
            *request.body_mut() =
                Body::from(r#"{"email":"victim@example.com","password":"opaque"}"#);
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                if index <= 2 {
                    StatusCode::OK
                } else {
                    StatusCode::TOO_MANY_REQUESTS
                }
            );
        }
    }

    #[tokio::test]
    async fn get_body_cannot_override_verified_query_account_budget() {
        use axum::{middleware, routing::get, Extension, Router};
        use tower::ServiceExt;
        let app = Router::new()
            .route("/api/auth/verify", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(rate_limit_auth))
            .layer(Extension(Arc::new(limiter(16))));
        for index in 1..=3 {
            let mut request = Request::builder()
                .uri("/api/auth/verify?email=victim%40example.com&token=opaque")
                .body(Body::from(format!(
                    r#"{{"email":"spoof{index}@example.com"}}"#
                )))
                .unwrap();
            request.extensions_mut().insert(ConnectInfo(
                format!("192.0.2.{index}:3000")
                    .parse::<SocketAddr>()
                    .unwrap(),
            ));
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                if index <= 2 {
                    StatusCode::OK
                } else {
                    StatusCode::TOO_MANY_REQUESTS
                }
            );
        }
    }

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
