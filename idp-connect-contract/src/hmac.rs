//! HMAC-SHA256 request authentication for IAM ↔ `IdP` Connect.
//!
//! Canonical string (literal newlines, no extra spaces):
//! `METHOD\nPATH\nTIMESTAMP\nBODY`
//!
//! `PATH` is supplied by the caller and is **not** stripped of a service prefix.
//! When a connector is nested (example: `/github-connect/v1/token`), pass that
//! full received path — not `/v1/token`. `BODY` is the raw request body; use
//! an empty string when there is no body.
//!
//! Signature header value is lowercase hex of HMAC-SHA256. Timestamp is unix
//! epoch seconds (decimal). Allowed skew is [`MAX_SKEW_SECS`].

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use thiserror::Error;

/// Header carrying the unix-epoch timestamp (seconds) used in the canonical string.
pub const TIMESTAMP_HEADER: &str = "X-IdP-Connect-Timestamp";
/// Header carrying the lowercase-hex HMAC-SHA256 of the canonical string.
pub const SIGNATURE_HEADER: &str = "X-IdP-Connect-Signature";
/// Maximum `|now - timestamp|` in seconds accepted by [`verify`].
pub const MAX_SKEW_SECS: u64 = 30;

type HmacSha256 = Hmac<Sha256>;

/// Shared HMAC secret. `Debug` does not print the key bytes.
#[derive(Clone)]
pub struct HmacKey(Arc<[u8]>);

impl std::fmt::Debug for HmacKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HmacKey(..)")
    }
}

impl HmacKey {
    /// Wraps raw secret bytes. Empty keys are cryptographically weak; callers
    /// must provision a real secret at deploy time.
    #[must_use]
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self(Arc::from(secret.as_ref()))
    }

    /// Borrow the secret bytes (do not log this).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// HMAC helper failure. Display strings never include secrets or signatures.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum HmacError {
    /// Required HMAC header is missing or not valid header text.
    #[error("missing HMAC header")]
    MissingHeader,
    /// Timestamp header is not a decimal i64 unix epoch.
    #[error("invalid HMAC timestamp")]
    InvalidTimestamp,
    /// `|now - timestamp|` is greater than [`MAX_SKEW_SECS`].
    #[error("HMAC timestamp outside allowed window")]
    StaleTimestamp,
    /// Signature is not 32-byte hex.
    #[error("invalid HMAC signature encoding")]
    InvalidSignatureEncoding,
    /// Constant-time compare failed.
    #[error("HMAC signature mismatch")]
    SignatureMismatch,
    /// HMAC key was rejected by the primitive (should not happen for SHA-256).
    #[error("invalid HMAC key")]
    InvalidKey,
    /// Body is not UTF-8, so the canonical string cannot be built.
    #[error("HMAC body is not UTF-8")]
    InvalidBody,
    /// System clock is before the unix epoch or overflows `i64`.
    #[error("system clock is unusable for HMAC")]
    Clock,
}

/// Builds `METHOD\nPATH\nTIMESTAMP\nBODY` with literal newlines and no extra spaces.
#[must_use]
pub fn canonical_string(method: &str, path: &str, timestamp: i64, body: &str) -> String {
    format!("{method}\n{path}\n{timestamp}\n{body}")
}

/// Signs the canonical string. Returns lowercase hex (HMAC-SHA256).
///
/// # Errors
///
/// Returns [`HmacError::InvalidKey`] if the HMAC primitive rejects the secret.
pub fn sign(
    secret: &[u8],
    method: &str,
    path: &str,
    timestamp: i64,
    body: &str,
) -> Result<String, HmacError> {
    let digest = hmac_digest(secret, method, path, timestamp, body)?;
    Ok(hex::encode(digest))
}

/// Verifies an HMAC against `now` (unix seconds). Tests inject `now` so they
/// are not wall-clock flaky.
///
/// # Errors
///
/// Returns [`HmacError::StaleTimestamp`] when skew exceeds [`MAX_SKEW_SECS`],
/// [`HmacError::InvalidSignatureEncoding`] when the header is not 32-byte hex,
/// [`HmacError::SignatureMismatch`] when the compare fails, or
/// [`HmacError::InvalidKey`] if signing the expected canonical string fails.
pub fn verify(
    secret: &[u8],
    method: &str,
    path: &str,
    timestamp: i64,
    body: &str,
    signature_hex: &str,
    now: i64,
) -> Result<(), HmacError> {
    if timestamp.abs_diff(now) > MAX_SKEW_SECS {
        return Err(HmacError::StaleTimestamp);
    }

    let expected = hmac_digest(secret, method, path, timestamp, body)?;
    let provided = parse_signature(signature_hex)?;
    if expected.ct_eq(&provided).into() {
        Ok(())
    } else {
        Err(HmacError::SignatureMismatch)
    }
}

/// Current unix epoch seconds for production HMAC checks.
///
/// # Errors
///
/// Returns [`HmacError::Clock`] if the system clock is before 1970-01-01 or
/// the second count does not fit in `i64`.
pub fn unix_timestamp_secs() -> Result<i64, HmacError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| HmacError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| HmacError::Clock)
}

fn hmac_digest(
    secret: &[u8],
    method: &str,
    path: &str,
    timestamp: i64,
    body: &str,
) -> Result<[u8; 32], HmacError> {
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| HmacError::InvalidKey)?;
    mac.update(canonical_string(method, path, timestamp, body).as_bytes());
    let bytes = mac.finalize().into_bytes();
    let mut out = [0_u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn parse_signature(signature_hex: &str) -> Result<[u8; 32], HmacError> {
    let decoded = hex::decode(signature_hex).map_err(|_| HmacError::InvalidSignatureEncoding)?;
    decoded
        .try_into()
        .map_err(|_| HmacError::InvalidSignatureEncoding)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"unit-test-secret";
    const METHOD: &str = "POST";
    const PREFIXED: &str = "/github-connect/v1/token";
    const BARE: &str = "/v1/token";
    const TS: i64 = 1_700_000_000;
    const BODY: &str = r#"{"code":"x","redirect_uri":"https://app.example/cb"}"#;

    #[test]
    fn canonical_string_is_four_newline_separated_parts() {
        assert_eq!(
            canonical_string("POST", "/v1/token", 10, r#"{"a":1}"#),
            "POST\n/v1/token\n10\n{\"a\":1}"
        );
        assert_eq!(
            canonical_string("POST", "/v1/token", 10, ""),
            "POST\n/v1/token\n10\n"
        );
    }

    #[test]
    fn sign_emits_lowercase_hex() {
        let sig = sign(SECRET, METHOD, PREFIXED, TS, BODY).expect("sign");
        assert_eq!(sig.len(), 64);
        assert!(
            sig.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "signature must be lowercase hex"
        );
    }

    #[test]
    fn good_signature_verifies() {
        let sig = sign(SECRET, METHOD, PREFIXED, TS, BODY).expect("sign");
        verify(SECRET, METHOD, PREFIXED, TS, BODY, &sig, TS).expect("verify");
    }

    #[test]
    fn timestamp_inside_window_inclusive_passes() {
        let sig = sign(SECRET, METHOD, PREFIXED, TS, BODY).expect("sign");
        verify(
            SECRET,
            METHOD,
            PREFIXED,
            TS,
            BODY,
            &sig,
            TS + i64::try_from(MAX_SKEW_SECS).expect("skew fits i64"),
        )
        .expect("30s inclusive");
    }

    #[test]
    fn timestamp_outside_30s_window_fails() {
        let sig = sign(SECRET, METHOD, PREFIXED, TS, BODY).expect("sign");
        let err = verify(SECRET, METHOD, PREFIXED, TS, BODY, &sig, TS + 31).expect_err("stale");
        assert_eq!(err, HmacError::StaleTimestamp);
    }

    #[test]
    fn mutated_body_fails() {
        let sig = sign(SECRET, METHOD, PREFIXED, TS, BODY).expect("sign");
        let err = verify(SECRET, METHOD, PREFIXED, TS, "{\"nope\":true}", &sig, TS)
            .expect_err("mutated body");
        assert_eq!(err, HmacError::SignatureMismatch);
    }

    #[test]
    fn wrong_secret_fails() {
        let sig = sign(SECRET, METHOD, PREFIXED, TS, BODY).expect("sign");
        let err = verify(b"other-secret", METHOD, PREFIXED, TS, BODY, &sig, TS)
            .expect_err("wrong secret");
        assert_eq!(err, HmacError::SignatureMismatch);
    }

    #[test]
    fn prefixed_signature_does_not_verify_bare_path() {
        let sig = sign(SECRET, METHOD, PREFIXED, TS, BODY).expect("sign");
        let err = verify(SECRET, METHOD, BARE, TS, BODY, &sig, TS).expect_err("path mismatch");
        assert_eq!(err, HmacError::SignatureMismatch);
        verify(SECRET, METHOD, PREFIXED, TS, BODY, &sig, TS).expect("prefixed path");
    }

    #[test]
    fn hmac_key_debug_omits_secret() {
        let key = HmacKey::new(SECRET);
        assert_eq!(format!("{key:?}"), "HmacKey(..)");
    }
}
