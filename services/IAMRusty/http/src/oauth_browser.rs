//! Browser-only OAuth binding. No cookies or credentials are accepted from forwarding headers.

use axum::http::{header, HeaderMap, HeaderValue};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use iam_application::usecase::oauth::OAuthUseCase;
use iam_configuration::security::SecurityMode;
use iam_domain::entity::{
    oauth_transaction::{
        BeginOAuthTransaction, BegunOAuthTransaction, ConsumeOAuthTransaction,
        ConsumedOAuthTransaction,
    },
    provider::Provider,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{
    error::AuthError,
    oauth_state::{OAuthOperation, OAuthState},
};

const MAX_COOKIE_BYTES: usize = 8192;

/// Resolved from trusted application configuration, not request scheme/header guesses.
#[derive(Clone, Copy)]
pub struct OAuthBrowserPolicy {
    local_http: bool,
    allow_http_authorization: bool,
}

impl OAuthBrowserPolicy {
    /// Only explicit local/test HTTP selects the non-`__Host` cookie.
    ///
    /// # Errors
    /// Rejects a malformed origin or plaintext without an explicit exemption.
    pub fn new(mode: SecurityMode, public_base_url: &str) -> Result<Self, &'static str> {
        let origin =
            url::Url::parse(public_base_url).map_err(|_| "invalid OAuth browser origin")?;
        if public_base_url.trim() != public_base_url
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            return Err("invalid OAuth browser origin");
        }
        let local_http = origin.scheme() == "http" && mode != SecurityMode::Verified;
        if origin.scheme() != "https" && !local_http {
            return Err("OAuth browser requires HTTPS");
        }
        Ok(Self {
            local_http,
            allow_http_authorization: mode != SecurityMode::Verified,
        })
    }

    /// A connector response cannot silently downgrade a verified browser authorization URL.
    ///
    /// # Errors
    /// Rejects non-HTTP(S), embedded credentials, fragments or unapproved plaintext.
    pub fn validate_authorization_url(self, url: &url::Url) -> Result<(), AuthError> {
        if url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || !matches!(url.scheme(), "https" | "http")
            || (url.scheme() == "http" && !self.allow_http_authorization)
        {
            return Err(AuthError::oauth_invalid_url("start"));
        }
        Ok(())
    }

    const fn cookie_name(self) -> &'static str {
        if self.local_http {
            "iam-oauth-browser"
        } else {
            "__Host-iam-oauth-browser"
        }
    }

    fn cookie_value(self, headers: &HeaderMap) -> Result<Option<&str>, &'static str> {
        let mut total = 0;
        let mut found = None;
        let mut pairs = 0;
        for header in headers.get_all(header::COOKIE) {
            total += header.as_bytes().len();
            if total > MAX_COOKIE_BYTES {
                return Err("invalid OAuth browser cookie");
            }
            let header = header
                .to_str()
                .map_err(|_| "invalid OAuth browser cookie")?;
            for pair in header.split(';') {
                pairs += 1;
                if pairs > 64 {
                    return Err("invalid OAuth browser cookie");
                }
                let Some((name, value)) = pair.trim().split_once('=') else {
                    continue;
                };
                if name == self.cookie_name() {
                    if found.is_some() {
                        return Err("invalid OAuth browser cookie");
                    }
                    found = Some(value);
                }
            }
        }
        Ok(found)
    }

    /// Reuse a valid shared browser nonce: separate states keep concurrent tabs independent.
    ///
    /// # Errors
    /// Rejects ambiguous/unbounded headers or an OS randomness failure.
    pub fn start_nonce(self, headers: &HeaderMap) -> Result<BrowserNonce, &'static str> {
        if let Some(nonce) = self.cookie_value(headers)?.and_then(BrowserNonce::decode) {
            return Ok(nonce);
        }
        let mut bytes = [0; 32];
        rand::rngs::OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|_| "OAuth browser nonce unavailable")?;
        Ok(BrowserNonce(bytes))
    }

    /// Callbacks never manufacture a browser nonce or accept a non-browser fallback.
    ///
    /// # Errors
    /// Rejects a missing, malformed or ambiguous cookie.
    pub fn callback_nonce(self, headers: &HeaderMap) -> Result<BrowserNonce, &'static str> {
        self.cookie_value(headers)?
            .and_then(BrowserNonce::decode)
            .ok_or("invalid OAuth browser cookie")
    }

    /// Emit only after persistent begin and successful authorize generation.
    ///
    /// # Errors
    /// Rejects a header encoding failure without disclosing the nonce.
    pub fn cookie_headers(self, nonce: &BrowserNonce) -> Result<HeaderMap, &'static str> {
        let attributes = if self.local_http {
            "Path=/iam; HttpOnly; SameSite=Lax; Max-Age=600"
        } else {
            "Path=/; HttpOnly; SameSite=Lax; Max-Age=600; Secure"
        };
        let cookie = format!(
            "{}={}; {attributes}",
            self.cookie_name(),
            URL_SAFE_NO_PAD.encode(nonce.0)
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookie).map_err(|_| "OAuth browser cookie unavailable")?,
        );
        Ok(headers)
    }
}

/// No Debug/Serialize: this browser secret is not diagnostic metadata.
pub struct BrowserNonce([u8; 32]);

impl BrowserNonce {
    fn decode(value: &str) -> Option<Self> {
        if value.len() != 43 {
            return None;
        }
        let bytes: [u8; 32] = URL_SAFE_NO_PAD.decode(value).ok()?.try_into().ok()?;
        (URL_SAFE_NO_PAD.encode(bytes) == value).then_some(Self(bytes))
    }

    fn hash(&self) -> [u8; 32] {
        Sha256::digest(self.0).into()
    }
}

/// Immutable per-router DI: use the existing OAuth use case and its writer, never a second registry.
pub struct OAuthRouteContext {
    pub use_case: Arc<dyn OAuthUseCase>,
    pub browser: OAuthBrowserPolicy,
}

/// Intention selected by the router, never a request parameter.
pub enum CallbackIntent {
    LoginOrLink,
    Relink,
}

impl OAuthRouteContext {
    /// Persist all bindings before an authorize request or successful cookie response.
    ///
    /// # Errors
    /// Returns a generic binding error or a non-secret storage-unavailable response.
    pub async fn begin(
        &self,
        state: &OAuthState,
        signed: &str,
        nonce: &BrowserNonce,
        provider: Provider,
        redirect_uri: String,
        pkce_required: bool,
    ) -> Result<BegunOAuthTransaction, AuthError> {
        self.use_case
            .begin_oauth_transaction(BeginOAuthTransaction {
                nonce_hash: Sha256::digest(state.nonce.as_bytes()).into(),
                state_hash: Sha256::digest(signed.as_bytes()).into(),
                browser_nonce_hash: nonce.hash(),
                provider,
                operation: state.operation.clone(),
                redirect_uri,
                expires_at: state.exp,
                pkce_required,
            })
            .await
            .map_err(|error| transaction_error("start", &error))
    }

    /// HMAC, route/provider/intention and browser binding precede the writer consume.
    ///
    /// # Errors
    /// Returns a generic state error, without exposing which binding mismatched.
    pub async fn consume_callback(
        &self,
        signed: &str,
        headers: &HeaderMap,
        provider: Provider,
        redirect_uri: String,
        intention: CallbackIntent,
    ) -> Result<ConsumedOAuthTransaction, AuthError> {
        let state =
            OAuthState::inspect(signed).map_err(|_| AuthError::oauth_invalid_state("callback"))?;
        let permitted = matches!(
            (intention, &state.operation),
            (
                CallbackIntent::LoginOrLink,
                OAuthOperation::Login | OAuthOperation::Link { .. }
            ) | (CallbackIntent::Relink, OAuthOperation::Relink { .. })
        );
        if !permitted || state.provider != provider.as_str() {
            return Err(AuthError::oauth_invalid_state("callback"));
        }
        self.consume(&state, signed, headers, provider, redirect_uri)
            .await
    }

    async fn consume(
        &self,
        state: &OAuthState,
        signed: &str,
        headers: &HeaderMap,
        provider: Provider,
        redirect_uri: String,
    ) -> Result<ConsumedOAuthTransaction, AuthError> {
        let nonce = self
            .browser
            .callback_nonce(headers)
            .map_err(|_| AuthError::oauth_invalid_state("callback"))?;
        self.use_case
            .consume_oauth_transaction(ConsumeOAuthTransaction {
                nonce_hash: Sha256::digest(state.nonce.as_bytes()).into(),
                state_hash: Sha256::digest(signed.as_bytes()).into(),
                browser_nonce_hash: nonce.hash(),
                provider,
                operation: state.operation.clone(),
                redirect_uri,
                expires_at: state.exp,
            })
            .await
            .map_err(|error| transaction_error("callback", &error))
    }
}

fn transaction_error(
    operation: &str,
    error: &iam_application::usecase::oauth::OAuthError,
) -> AuthError {
    match error {
        iam_application::usecase::oauth::OAuthError::Transaction(
            iam_domain::entity::oauth_transaction::OAuthTransactionError::InvalidTransaction,
        ) => AuthError::oauth_invalid_state(operation),
        _ => AuthError::OAuth {
            operation: operation.into(),
            error_code: "oauth_transaction_unavailable".into(),
            message: "OAuth transaction unavailable".into(),
            status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
        },
    }
}

/// Reject a receiver that ignored requested PKCE metadata or returned unsolicited PKCE.
///
/// # Errors
/// Returns a generic URL error for any incomplete, duplicate or mismatched PKCE pair.
pub fn validate_authorization_pkce(
    url: &url::Url,
    begun: &BegunOAuthTransaction,
) -> Result<(), AuthError> {
    let challenges: Vec<_> = url
        .query_pairs()
        .filter(|(name, _)| name.as_ref() == "code_challenge")
        .map(|(_, value)| value.into_owned())
        .collect();
    let methods: Vec<_> = url
        .query_pairs()
        .filter(|(name, _)| name.as_ref() == "code_challenge_method")
        .map(|(_, value)| value.into_owned())
        .collect();
    let valid = match (
        begun.code_challenge.as_deref(),
        begun.code_challenge_method.as_deref(),
    ) {
        (None, None) => challenges.is_empty() && methods.is_empty(),
        (Some(challenge), Some("S256")) => {
            challenges.len() == 1
                && methods.len() == 1
                && challenges[0] == challenge
                && methods[0] == "S256"
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(AuthError::oauth_invalid_url("start"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iam_application::usecase::oauth::{OAuthError, OAuthResponse};
    use iam_domain::{
        entity::oauth_transaction::{OAuthTransaction, OAuthTransactionError},
        port::repository::OAuthTransactionWriteRepository,
    };
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        },
    };

    #[derive(Default)]
    struct Writer {
        rows: Mutex<HashMap<[u8; 32], OAuthTransaction>>,
        consumes: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl OAuthTransactionWriteRepository for Writer {
        async fn create(&self, tx: &OAuthTransaction) -> Result<(), OAuthTransactionError> {
            let mut rows = self.rows.lock().unwrap();
            if rows.contains_key(&tx.state_hash) {
                drop(rows);
                return Err(OAuthTransactionError::InvalidTransaction);
            }
            rows.insert(
                tx.state_hash,
                OAuthTransaction {
                    id: tx.id,
                    nonce_hash: tx.nonce_hash,
                    state_hash: tx.state_hash,
                    browser_nonce_hash: tx.browser_nonce_hash,
                    provider: tx.provider.clone(),
                    operation: tx.operation.clone(),
                    redirect_uri: tx.redirect_uri.clone(),
                    expires_at: tx.expires_at,
                    consumed_at: tx.consumed_at,
                    pkce_verifier: tx.pkce_verifier.clone(),
                },
            );
            drop(rows);
            Ok(())
        }
        async fn consume(
            &self,
            input: &ConsumeOAuthTransaction,
        ) -> Result<Option<OAuthTransaction>, OAuthTransactionError> {
            self.consumes.fetch_add(1, Ordering::SeqCst);
            let mut rows = self.rows.lock().unwrap();
            if !rows
                .get(&input.state_hash)
                .is_some_and(|row| row.consumed_at.is_none() && row.matches(input))
            {
                drop(rows);
                return Ok(None);
            }
            let mut row = rows.remove(&input.state_hash).unwrap();
            drop(rows);
            row.consumed_at = Some(chrono::Utc::now().timestamp());
            Ok(Some(row))
        }
        async fn purge_expired(&self, batch_size: u32) -> Result<u64, OAuthTransactionError> {
            if !(1..=1000).contains(&batch_size) {
                return Err(OAuthTransactionError::InvalidTransaction);
            }
            let now = chrono::Utc::now().timestamp();
            let mut rows = self.rows.lock().unwrap();
            let mut expired: Vec<_> = rows
                .values()
                .filter(|row| row.expires_at <= now)
                .map(|row| (row.expires_at, row.id, row.state_hash))
                .collect();
            expired.sort_unstable();
            let mut deleted = 0;
            for (_, _, hash) in expired.into_iter().take(batch_size as usize) {
                if rows.remove(&hash).is_some() {
                    deleted += 1;
                }
            }
            Ok(deleted)
        }
    }

    #[derive(Default)]
    struct Core {
        writer: Writer,
    }

    #[async_trait::async_trait]
    impl OAuthUseCase for Core {
        async fn begin_oauth_transaction(
            &self,
            input: BeginOAuthTransaction,
        ) -> Result<BegunOAuthTransaction, OAuthError> {
            Ok(OAuthTransaction::begin(&self.writer, input).await?)
        }
        async fn consume_oauth_transaction(
            &self,
            input: ConsumeOAuthTransaction,
        ) -> Result<ConsumedOAuthTransaction, OAuthError> {
            Ok(ConsumedOAuthTransaction::consume(&self.writer, input).await?)
        }
        async fn generate_start_url(
            &self,
            _provider: Provider,
            _redirect: String,
            _state: String,
            _begun: &BegunOAuthTransaction,
        ) -> Result<String, OAuthError> {
            Err(OAuthTransactionError::InvalidTransaction.into())
        }
        async fn oauth_login(
            &self,
            _provider: Provider,
            _code: String,
            _redirect: String,
            _consumed: ConsumedOAuthTransaction,
        ) -> Result<OAuthResponse, OAuthError> {
            panic!("browser binding unit tests must never exchange a vendor code")
        }
    }

    fn context() -> (OAuthRouteContext, Arc<Core>) {
        let config = iam_configuration::security::SecurityConfig {
            mode: SecurityMode::IsolatedTest,
            ..iam_configuration::security::SecurityConfig::default()
        };
        crate::oauth_state::configure_oauth_state_secret(
            &config
                .validate_oauth_state_secret("iam-oauth-state-hmac-test")
                .unwrap(),
        )
        .unwrap();
        let core = Arc::new(Core::default());
        (
            OAuthRouteContext {
                use_case: core.clone(),
                browser: OAuthBrowserPolicy::new(SecurityMode::Verified, "https://platform")
                    .unwrap(),
            },
            core,
        )
    }

    async fn begun(
        context: &OAuthRouteContext,
        nonce: &BrowserNonce,
        state: &OAuthState,
    ) -> String {
        let signed = state.encode().unwrap();
        context
            .begin(
                state,
                &signed,
                nonce,
                Provider::parse_slug("github").unwrap(),
                "https://platform/iam/api/auth/github/callback".into(),
                false,
            )
            .await
            .unwrap();
        signed
    }

    fn browser_cookie(policy: OAuthBrowserPolicy, nonce: &BrowserNonce) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            format!(
                "{}={}",
                policy.cookie_name(),
                URL_SAFE_NO_PAD.encode(nonce.0)
            )
            .parse()
            .unwrap(),
        );
        headers
    }

    #[test]
    fn cookie_is_secure_host_only_and_local_exception_is_explicit() {
        assert!(OAuthBrowserPolicy::new(SecurityMode::Verified, "http://localhost").is_err());
        let secure = OAuthBrowserPolicy::new(SecurityMode::Verified, "https://platform").unwrap();
        let nonce = secure.start_nonce(&HeaderMap::new()).unwrap();
        let headers = secure.cookie_headers(&nonce).unwrap();
        let cookie = headers[header::SET_COOKIE].to_str().unwrap();
        for bit in [
            "__Host-iam-oauth-browser=",
            "Path=/;",
            "HttpOnly",
            "SameSite=Lax",
            "Max-Age=600",
            "Secure",
        ] {
            assert!(cookie.contains(bit));
        }
        assert!(!cookie.contains("Domain="));
        assert!(secure
            .validate_authorization_url(&url::Url::parse("http://vendor/authorize").unwrap())
            .is_err());
        assert!(secure
            .validate_authorization_url(&url::Url::parse("https://vendor/authorize").unwrap())
            .is_ok());
        let local =
            OAuthBrowserPolicy::new(SecurityMode::LocalInsecure, "http://localhost").unwrap();
        assert!(local
            .validate_authorization_url(&url::Url::parse("http://vendor/authorize").unwrap())
            .is_ok());
        let cookie = local.cookie_headers(&nonce).unwrap();
        let value = cookie[header::SET_COOKIE].to_str().unwrap();
        assert!(value.starts_with("iam-oauth-browser="));
        assert!(value.contains("Path=/iam;"));
        assert!(!value.contains("Secure"));
        assert!(
            OAuthBrowserPolicy::new(SecurityMode::LocalInsecure, "https://localhost")
                .unwrap()
                .cookie_headers(&nonce)
                .unwrap()[header::SET_COOKIE]
                .to_str()
                .unwrap()
                .contains("Secure")
        );
    }

    #[test]
    fn tabs_reuse_browser_nonce_without_reusing_transaction_state() {
        let policy = OAuthBrowserPolicy::new(SecurityMode::Verified, "https://platform").unwrap();
        let first = policy.start_nonce(&HeaderMap::new()).unwrap();
        let cookies = browser_cookie(policy, &first);
        let second = policy.start_nonce(&cookies).unwrap();
        assert_eq!(first.hash(), second.hash());
        assert_eq!(
            first.hash(),
            policy.callback_nonce(&cookies).unwrap().hash()
        );
        assert_ne!(
            OAuthState::new_login("github").nonce,
            OAuthState::new_login("github").nonce
        );
        assert!(policy.callback_nonce(&HeaderMap::new()).is_err());
    }

    #[test]
    fn missing_malformed_and_duplicate_cookies_never_authorize_callbacks() {
        let policy = OAuthBrowserPolicy::new(SecurityMode::Verified, "https://platform").unwrap();
        for value in [
            "garbage".to_string(),
            "a".repeat(44),
            URL_SAFE_NO_PAD.encode([0; 31]),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::COOKIE,
                format!("__Host-iam-oauth-browser={value}").parse().unwrap(),
            );
            assert!(policy.callback_nonce(&headers).is_err());
        }
        let nonce = policy.start_nonce(&HeaderMap::new()).unwrap();
        let mut headers = browser_cookie(policy, &nonce);
        let duplicate = headers[header::COOKIE].clone();
        headers.append(header::COOKIE, duplicate);
        assert!(policy.callback_nonce(&headers).is_err());
        let mut non_browser = HeaderMap::new();
        non_browser.insert("x-forwarded-proto", "https".parse().unwrap());
        non_browser.insert(
            header::AUTHORIZATION,
            "Bearer verified-platform-token".parse().unwrap(),
        );
        assert!(policy.callback_nonce(&non_browser).is_err());
    }

    #[test]
    fn authorization_response_cannot_downgrade_or_invent_pkce() {
        let challenge = "a".repeat(43);
        let required = BegunOAuthTransaction {
            code_challenge: Some(challenge.clone()),
            code_challenge_method: Some("S256".into()),
        };
        let absent = BegunOAuthTransaction {
            code_challenge: None,
            code_challenge_method: None,
        };
        let plain = url::Url::parse("https://vendor/authorize?state=opaque").unwrap();
        assert!(validate_authorization_pkce(&plain, &required).is_err());
        assert!(validate_authorization_pkce(&plain, &absent).is_ok());
        let valid = url::Url::parse(&format!(
            "https://vendor/authorize?code_challenge={challenge}&code_challenge_method=S256"
        ))
        .unwrap();
        assert!(validate_authorization_pkce(&valid, &required).is_ok());
        assert!(validate_authorization_pkce(&valid, &absent).is_err());
    }

    #[tokio::test]
    async fn hmac_route_intention_and_browser_are_validated_before_writer_effects() {
        let (context, core) = context();
        let nonce = context.browser.start_nonce(&HeaderMap::new()).unwrap();
        let cookies = browser_cookie(context.browser, &nonce);
        let state = OAuthState::new_login("github");
        let signed = begun(&context, &nonce, &state).await;
        let provider = Provider::parse_slug("github").unwrap();
        let redirect = "https://platform/iam/api/auth/github/callback".to_string();
        for (value, headers, provider, intention) in [
            (
                format!("{signed}!"),
                cookies.clone(),
                provider.clone(),
                CallbackIntent::LoginOrLink,
            ),
            (
                signed.clone(),
                HeaderMap::new(),
                provider.clone(),
                CallbackIntent::LoginOrLink,
            ),
            (
                signed.clone(),
                cookies.clone(),
                Provider::parse_slug("gitlab").unwrap(),
                CallbackIntent::LoginOrLink,
            ),
            (
                signed.clone(),
                cookies.clone(),
                provider.clone(),
                CallbackIntent::Relink,
            ),
        ] {
            assert!(context
                .consume_callback(&value, &headers, provider, redirect.clone(), intention)
                .await
                .is_err());
        }
        let mut expired = state.clone();
        expired.exp = chrono::Utc::now().timestamp() - 1;
        assert!(context
            .consume_callback(
                &expired.encode().unwrap(),
                &cookies,
                provider.clone(),
                redirect.clone(),
                CallbackIntent::LoginOrLink
            )
            .await
            .is_err());
        assert_eq!(core.writer.consumes.load(Ordering::SeqCst), 0);
        let wrong_browser = context.browser.start_nonce(&HeaderMap::new()).unwrap();
        let wrong_cookie = browser_cookie(context.browser, &wrong_browser);
        assert!(context
            .consume_callback(
                &signed,
                &wrong_cookie,
                provider.clone(),
                redirect.clone(),
                CallbackIntent::LoginOrLink
            )
            .await
            .is_err());
        assert!(context
            .consume_callback(
                &signed,
                &cookies,
                provider.clone(),
                "https://platform/wrong-callback".into(),
                CallbackIntent::LoginOrLink
            )
            .await
            .is_err());
        let consumed = context
            .consume_callback(
                &signed,
                &cookies,
                provider.clone(),
                redirect.clone(),
                CallbackIntent::LoginOrLink,
            )
            .await
            .unwrap();
        assert_eq!(consumed.target_user_id(), None);
        assert!(context
            .consume_callback(
                &signed,
                &cookies,
                provider,
                redirect,
                CallbackIntent::LoginOrLink
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn two_tabs_consume_independent_states_with_one_cookie_and_canonical_target() {
        let (context, core) = context();
        let nonce = context.browser.start_nonce(&HeaderMap::new()).unwrap();
        let cookies = browser_cookie(context.browser, &nonce);
        let provider = Provider::parse_slug("github").unwrap();
        let target = uuid::Uuid::new_v4();
        let first = begun(&context, &nonce, &OAuthState::new_login("github")).await;
        let second_nonce = context.browser.start_nonce(&cookies).unwrap();
        let second = begun(
            &context,
            &second_nonce,
            &OAuthState::new_link(target, "github"),
        )
        .await;
        assert_eq!(core.writer.rows.lock().unwrap().len(), 2);
        let redirect = "https://platform/iam/api/auth/github/callback".to_string();
        context
            .consume_callback(
                &first,
                &cookies,
                provider.clone(),
                redirect.clone(),
                CallbackIntent::LoginOrLink,
            )
            .await
            .unwrap();
        let result = context
            .consume_callback(
                &second,
                &cookies,
                provider,
                redirect,
                CallbackIntent::LoginOrLink,
            )
            .await
            .unwrap();
        assert_eq!(result.target_user_id(), Some(target));
        assert_eq!(core.writer.rows.lock().unwrap().len(), 0);
        // No delete-cookie side effect: the same nonce can still start another transaction.
        assert!(context.browser.callback_nonce(&cookies).is_ok());
    }

    #[tokio::test]
    async fn relink_requires_signed_relink_state_browser_and_persisted_canonical_target() {
        let (context, core) = context();
        let nonce = context.browser.start_nonce(&HeaderMap::new()).unwrap();
        let cookies = browser_cookie(context.browser, &nonce);
        let target = uuid::Uuid::new_v4();
        let state = OAuthState::new_relink(target, "github");
        let signed = state.encode().unwrap();
        let provider = Provider::parse_slug("github").unwrap();
        let redirect = "https://platform/iam/api/auth/github/relink-callback".to_string();
        context
            .begin(
                &state,
                &signed,
                &nonce,
                provider.clone(),
                redirect.clone(),
                false,
            )
            .await
            .unwrap();
        for (signed, headers, intention) in [
            (String::new(), cookies.clone(), CallbackIntent::Relink),
            (signed.clone(), HeaderMap::new(), CallbackIntent::Relink),
            (signed.clone(), cookies.clone(), CallbackIntent::LoginOrLink),
        ] {
            assert!(context
                .consume_callback(
                    &signed,
                    &headers,
                    provider.clone(),
                    redirect.clone(),
                    intention
                )
                .await
                .is_err());
        }
        assert_eq!(core.writer.consumes.load(Ordering::SeqCst), 0);
        // Even a newly signed state cannot override the target stored at START.
        let mut wrong_target = state.clone();
        wrong_target.operation = OAuthOperation::Relink {
            user_id: uuid::Uuid::new_v4(),
        };
        assert!(context
            .consume_callback(
                &wrong_target.encode().unwrap(),
                &cookies,
                provider.clone(),
                redirect.clone(),
                CallbackIntent::Relink
            )
            .await
            .is_err());
        let consumed = context
            .consume_callback(
                &signed,
                &cookies,
                provider,
                redirect,
                CallbackIntent::Relink,
            )
            .await
            .unwrap();
        assert_eq!(consumed.target_user_id(), Some(target));
        assert!(matches!(
            consumed.operation(),
            OAuthOperation::Relink { .. }
        ));
    }

    #[tokio::test]
    async fn distinct_router_contexts_use_one_writer_and_yield_only_one_capability() {
        let (context, core) = context();
        let second_context = OAuthRouteContext {
            use_case: core.clone(),
            browser: context.browser,
        };
        let nonce = context.browser.start_nonce(&HeaderMap::new()).unwrap();
        let cookies = browser_cookie(context.browser, &nonce);
        let signed = begun(&context, &nonce, &OAuthState::new_login("github")).await;
        let provider = Provider::parse_slug("github").unwrap();
        let redirect = "https://platform/iam/api/auth/github/callback".to_string();
        let (first, second) = tokio::join!(
            context.consume_callback(
                &signed,
                &cookies,
                provider.clone(),
                redirect.clone(),
                CallbackIntent::LoginOrLink
            ),
            second_context.consume_callback(
                &signed,
                &cookies,
                provider,
                redirect,
                CallbackIntent::LoginOrLink
            ),
        );
        assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
        assert_eq!(core.writer.consumes.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn storage_failure_is_unavailable_not_a_credential_or_binding_oracle() {
        let storage = OAuthTransactionError::Storage.into();
        let unavailable = transaction_error("callback", &storage);
        assert!(matches!(
            unavailable,
            AuthError::OAuth {
                status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
                ..
            }
        ));
        let invalid = OAuthTransactionError::InvalidTransaction.into();
        let rejected = transaction_error("callback", &invalid);
        assert!(!format!("{rejected:?}").contains("nonce"));
    }
}
