//! Runtime projected-token verification. Keys come from trusted platform config,
//! never from JWT headers (`jku`/`x5u`) or a caller-selected OIDC endpoint.
//! Reconciled with contract §13.1-B (HEAD fd94467, 2026-10-20).

use std::collections::HashMap;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use jsonwebtoken::jwk::{AlgorithmParameters, Jwk};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use lazaret_domain::runtime::{RuntimeEnrollmentProofVerifier, RuntimeError, RuntimePodProof};
use serde::Deserialize;

/// The enrollment audience is deliberately not an API-server audience.
pub const RUNTIME_ENROLLMENT_AUDIENCE: &str = "urn:aiforall:apparatus:runtime:enrollment:v1";
pub const RUNTIME_ENROLLMENT_ISSUER: &str = "https://kubernetes.default.svc.cluster.local";

/// Pinned trusted OIDC verification keys, with no discovery/network fallback.
pub struct RuntimeOidcVerifier {
    issuer: String,
    keys: HashMap<String, DecodingKey>,
}

impl RuntimeOidcVerifier {
    /// Load approved public RSA JWKs from platform configuration. Issuer changes
    /// require human configuration ratification, not discovery from a token.
    pub fn new(
        issuer: String,
        public_keys: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, RuntimeError> {
        if issuer != RUNTIME_ENROLLMENT_ISSUER {
            return Err(RuntimeError::RuntimeInternal);
        }
        let mut keys = HashMap::new();
        for (kid, jwk_json) in public_keys {
            if !valid_enrollment_kid(&kid) || keys.contains_key(&kid) {
                return Err(RuntimeError::RuntimeInternal);
            }
            let raw: serde_json::Value =
                serde_json::from_str(&jwk_json).map_err(|_| RuntimeError::RuntimeInternal)?;
            // Public catalogs must not carry RSA private parameters.
            if ["d", "p", "q", "dp", "dq", "qi", "oth"]
                .iter()
                .any(|name| raw.get(name).is_some())
            {
                return Err(RuntimeError::RuntimeInternal);
            }
            let jwk: Jwk =
                serde_json::from_value(raw).map_err(|_| RuntimeError::RuntimeInternal)?;
            if jwk.common.key_id.as_deref() != Some(kid.as_str()) {
                return Err(RuntimeError::RuntimeInternal);
            }
            let AlgorithmParameters::RSA(ref rsa) = jwk.algorithm else {
                return Err(RuntimeError::RuntimeInternal);
            };
            validate_rsa_public(&rsa.n, &rsa.e, 2048, None)?;
            let key = DecodingKey::from_jwk(&jwk).map_err(|_| RuntimeError::RuntimeInternal)?;
            keys.insert(kid, key);
        }
        if keys.is_empty() {
            return Err(RuntimeError::RuntimeInternal);
        }
        Ok(Self { issuer, keys })
    }
}

#[derive(Deserialize)]
struct ProjectedClaims {
    sub: String,
    aud: Vec<String>,
    exp: i64,
    iat: i64,
    #[serde(rename = "kubernetes.io")]
    kubernetes: KubernetesClaims,
}

#[derive(Deserialize)]
struct KubernetesClaims {
    namespace: String,
    pod: PodClaims,
    serviceaccount: ServiceAccountClaims,
}

#[derive(Deserialize)]
struct PodClaims {
    uid: String,
}

#[derive(Deserialize)]
struct ServiceAccountClaims {
    name: String,
    uid: String,
}

impl RuntimeEnrollmentProofVerifier for RuntimeOidcVerifier {
    fn verify(&self, token: &str) -> Result<RuntimePodProof, RuntimeError> {
        if token.len() > 16 * 1024 {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        let header = decode_header(token).map_err(|_| RuntimeError::RuntimeInvalidProof)?;
        if header.alg != Algorithm::RS256 {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        if !header.kid.as_deref().is_some_and(valid_enrollment_kid) {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        let key = header
            .kid
            .as_ref()
            .and_then(|kid| self.keys.get(kid))
            .ok_or(RuntimeError::RuntimeInvalidProof)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.leeway = 0;
        validation.validate_nbf = true;
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[RUNTIME_ENROLLMENT_AUDIENCE]);
        validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);
        let claims = decode::<ProjectedClaims>(token, key, &validation)
            .map_err(|_| RuntimeError::RuntimeInvalidProof)?
            .claims;
        let now = chrono::Utc::now().timestamp();
        let k = claims.kubernetes;
        if claims.aud.as_slice() != [RUNTIME_ENROLLMENT_AUDIENCE]
            || claims.iat > now
            || claims.exp <= now
            || claims.exp <= claims.iat
            || claims.exp.saturating_sub(claims.iat) > 600
            || k.pod.uid.is_empty()
            || k.pod.uid.len() > 253
            || k.namespace.is_empty()
            || k.namespace.len() > 63
            || k.serviceaccount.name.is_empty()
            || k.serviceaccount.name.len() > 253
            || k.serviceaccount.uid.is_empty()
            || claims.sub
                != format!(
                    "system:serviceaccount:{}:{}",
                    k.namespace, k.serviceaccount.name
                )
        {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        Ok(RuntimePodProof {
            pod_uid: k.pod.uid,
            namespace: k.namespace,
            service_account: k.serviceaccount.name,
            service_account_uid: k.serviceaccount.uid,
            expires_at_unix: claims.exp,
        })
    }
}

fn valid_enrollment_kid(kid: &str) -> bool {
    !kid.is_empty()
        && kid.len() <= 128
        && kid
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// RSA Base64urlUInt must be minimally encoded and exponent exactly 65537.
pub(super) fn validate_rsa_public(
    n: &str,
    e: &str,
    min_bits: usize,
    exact_bits: Option<usize>,
) -> Result<(), RuntimeError> {
    let modulus = URL_SAFE_NO_PAD
        .decode(n)
        .map_err(|_| RuntimeError::RuntimeInternal)?;
    let exponent = URL_SAFE_NO_PAD
        .decode(e)
        .map_err(|_| RuntimeError::RuntimeInternal)?;
    if modulus.first().is_none_or(|b| *b == 0)
        || exponent != [1, 0, 1]
        || URL_SAFE_NO_PAD.encode(&modulus) != n
        || URL_SAFE_NO_PAD.encode(&exponent) != e
    {
        return Err(RuntimeError::RuntimeInternal);
    }
    let bits = modulus.len() * 8 - modulus[0].leading_zeros() as usize;
    if bits < min_bits || exact_bits.is_some_and(|expected| expected != bits) {
        return Err(RuntimeError::RuntimeInternal);
    }
    Ok(())
}
