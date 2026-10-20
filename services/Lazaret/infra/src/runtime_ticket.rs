//! Invoke-only RS256 JWS adapter (§13.1-B). Private signing is delegated to the
//! trusted platform provider; public catalogs never come from a ticket/header.

use std::{sync::Arc, time::Duration};

use apparatus_contracts::runtime::*;
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use lazaret_domain::runtime::{
    RuntimeError, RuntimeInvokePublicKey, RuntimeRsaSigningProvider, RuntimeTicketSigner,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::runtime_identity::validate_rsa_public;

pub struct ProviderRuntimeTicketSigner {
    provider: Arc<dyn RuntimeRsaSigningProvider>,
}

impl ProviderRuntimeTicketSigner {
    pub fn new(provider: Arc<dyn RuntimeRsaSigningProvider>) -> Self {
        Self { provider }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvokeHeader {
    alg: String,
    typ: String,
    kid: String,
}

/// RFC7638 preimage has exactly e/kty/n, in that order, with public integers.
/// Its SHA-256 is encoded once, not hashed a second time.
fn key_id(key: &RuntimeInvokePublicKey) -> Result<String, RuntimeError> {
    validate_rsa_public(
        &key.modulus_base64url,
        &key.exponent_base64url,
        3072,
        Some(3072),
    )
    .map_err(|_| RuntimeError::RuntimeKeyUnavailable)?;
    // Base64urlUInt validation above excludes characters requiring JSON escapes.
    let preimage = format!(
        "{{\"e\":\"{}\",\"kty\":\"RSA\",\"n\":\"{}\"}}",
        key.exponent_base64url, key.modulus_base64url
    );
    Ok(format!(
        "ari-v1-{}",
        URL_SAFE_NO_PAD.encode(Sha256::digest(preimage.as_bytes()))
    ))
}

fn check_key(key: &RuntimeInvokePublicKey, now: i64, signing: bool) -> Result<(), RuntimeError> {
    let invalid = if signing {
        RuntimeError::RuntimeKeyUnavailable
    } else {
        RuntimeError::RuntimeInvalidProof
    };
    if key.revoked
        || key.purpose != INVOKE_PURPOSE
        || key.kid.len() != 50
        || key.kid != key_id(key).map_err(|_| invalid)?
        || key.published_and_acknowledged_at_unix_ms < 0
        || key.activated_at_unix_ms < 60_000
        || key.published_and_acknowledged_at_unix_ms > key.activated_at_unix_ms - 60_000
        || key.signing_until_unix_ms <= key.activated_at_unix_ms
        || key.signing_until_unix_ms > key.activated_at_unix_ms.saturating_add(86_400_000)
        || now < key.activated_at_unix_ms
    {
        return Err(invalid);
    }
    if signing {
        if now >= key.signing_until_unix_ms {
            return Err(invalid);
        }
    } else {
        let last = key.last_issued_at_unix_ms.ok_or(invalid)?;
        if last < key.activated_at_unix_ms
            || last >= key.signing_until_unix_ms
            || last > now
            || key.verification_until_unix_ms <= last
            || key.verification_until_unix_ms > last.saturating_add(60_000)
            || now >= key.verification_until_unix_ms
        {
            return Err(invalid);
        }
    }
    Ok(())
}

fn check_ticket(ticket: &RuntimeInvocationTicketV1, now_ms: i64) -> Result<(), RuntimeError> {
    if ticket.schema_version != 1 {
        return Err(RuntimeError::RuntimeUnsupportedVersion);
    }
    let now = now_ms / 1000;
    if now_ms < 0
        || ticket.iss != INVOKE_ISSUER
        || ticket.aud != INVOKE_AUDIENCE
        || ticket.purpose != INVOKE_PURPOSE
        || ticket.iat < 0
        || ticket.iat > now
        || ticket.exp <= ticket.iat
        || ticket.exp.saturating_sub(ticket.iat) > 30
        || ticket.exp != ticket.deadline_unix_ms.get() / 1000
        || now >= ticket.exp
        || now_ms >= ticket.deadline_unix_ms.get()
        || ticket.source.binding == ticket.target.binding_id
    {
        return Err(RuntimeError::RuntimeInvalidProof);
    }
    Ok(())
}

#[async_trait]
impl RuntimeTicketSigner for ProviderRuntimeTicketSigner {
    async fn sign(&self, ticket: &RuntimeInvocationTicketV1) -> Result<String, RuntimeError> {
        let now = chrono::Utc::now().timestamp_millis();
        check_ticket(ticket, now)?;
        let key = self.provider.active_key()?;
        check_key(&key, now, true)?;
        let header = InvokeHeader {
            alg: "RS256".to_owned(),
            typ: INVOKE_TYP.to_owned(),
            kid: key.kid.clone(),
        };
        let header = serde_json::to_vec(&header).map_err(|_| RuntimeError::RuntimeInternal)?;
        let payload = serde_json::to_vec(ticket).map_err(|_| RuntimeError::RuntimeInternal)?;
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header),
            URL_SAFE_NO_PAD.encode(payload)
        );
        let signature = tokio::time::timeout(
            Duration::from_secs(2),
            self.provider
                .sign_rs256(INVOKE_PURPOSE, &key.kid, input.as_bytes()),
        )
        .await
        .map_err(|_| RuntimeError::RuntimeKeyUnavailable)??;
        if signature.len() != 384 {
            return Err(RuntimeError::RuntimeKeyUnavailable);
        }
        let token = format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature));
        // The provider must publish last_issued metadata before returning. This
        // also detects a wrong key/algorithm or a corrupted remote signature.
        if self.verify(&token)? != *ticket {
            return Err(RuntimeError::RuntimeKeyUnavailable);
        }
        Ok(token)
    }

    fn verify(&self, token: &str) -> Result<RuntimeInvocationTicketV1, RuntimeError> {
        if token.len() > 16_384 {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        let parts: Vec<_> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        for part in &parts {
            let decoded = URL_SAFE_NO_PAD
                .decode(part)
                .map_err(|_| RuntimeError::RuntimeInvalidProof)?;
            if URL_SAFE_NO_PAD.encode(decoded) != *part {
                return Err(RuntimeError::RuntimeInvalidProof);
            }
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|_| RuntimeError::RuntimeInvalidProof)?;
        let header: InvokeHeader =
            parse_runtime_json(&bytes).map_err(|_| RuntimeError::RuntimeInvalidProof)?;
        if header.alg != "RS256"
            || header.typ != INVOKE_TYP
            || header.kid.len() != 50
            || !header.kid.starts_with("ari-v1-")
        {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        let key = self.provider.verification_key(&header.kid)?;
        let now = chrono::Utc::now().timestamp_millis();
        if key.kid != header.kid {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        check_key(&key, now, false)?;
        let decoding =
            DecodingKey::from_rsa_components(&key.modulus_base64url, &key.exponent_base64url)
                .map_err(|_| RuntimeError::RuntimeInvalidProof)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.leeway = 0;
        validation.validate_nbf = true;
        validation.set_issuer(&[INVOKE_ISSUER]);
        validation.set_audience(&[INVOKE_AUDIENCE]);
        validation.set_required_spec_claims(&["iss", "aud", "iat", "exp", "jti"]);
        let verified = decode::<RuntimeInvocationTicketV1>(token, &decoding, &validation)
            .map_err(|_| RuntimeError::RuntimeInvalidProof)?
            .claims;
        // The shared parser rejects duplicate names and the restricted numeric
        // profile before relying on any semantic claim from the signed payload.
        let payload = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|_| RuntimeError::RuntimeInvalidProof)?;
        let canonical: RuntimeInvocationTicketV1 =
            parse_runtime_json(&payload).map_err(|_| RuntimeError::RuntimeInvalidProof)?;
        if verified != canonical {
            return Err(RuntimeError::RuntimeInvalidProof);
        }
        check_ticket(&verified, now)?;
        Ok(verified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic public parameters exercise encoding/schedule only; this is NOT
    // a valid signing fixture or evidence of a successfully verified RSA token.
    fn public_metadata() -> RuntimeInvokePublicKey {
        RuntimeInvokePublicKey {
            kid: "ari-v1-CTGuZ9GFvVyKZbZQ6BcI36ngIyBwTaouUA_NfnDedK0".to_owned(),
            purpose: INVOKE_PURPOSE.to_owned(),
            modulus_base64url: URL_SAFE_NO_PAD.encode([255_u8; 384]),
            exponent_base64url: "AQAB".to_owned(),
            published_and_acknowledged_at_unix_ms: 940_000,
            activated_at_unix_ms: 1_000_000,
            signing_until_unix_ms: 87_400_000,
            last_issued_at_unix_ms: Some(1_000_500),
            verification_until_unix_ms: 1_060_500,
            revoked: false,
        }
    }

    #[test]
    fn invoke_kid_is_the_single_rfc7638_hash_with_exact_size() {
        let key = public_metadata();
        let kid = key_id(&key).expect("public parameter vector");
        assert_eq!(kid, key.kid);
        assert_eq!(kid.len(), 50);
    }

    #[test]
    fn invoke_catalogue_requires_ack_rotation_retirement_and_emergency_revocation() {
        let mut key = public_metadata();
        assert!(check_key(&key, 1_001_000, true).is_ok());
        assert!(check_key(&key, 1_001_000, false).is_ok());
        assert!(check_key(&key, 1_060_500, false).is_err());
        key.published_and_acknowledged_at_unix_ms += 1;
        assert!(check_key(&key, 1_001_000, true).is_err());
        key = public_metadata();
        key.signing_until_unix_ms += 1;
        assert!(check_key(&key, 1_001_000, true).is_err());
        key = public_metadata();
        key.verification_until_unix_ms += 1;
        assert!(check_key(&key, 1_001_000, false).is_err());
        key = public_metadata();
        key.revoked = true;
        assert!(check_key(&key, 1_001_000, true).is_err());
        assert!(check_key(&key, 1_001_000, false).is_err());
    }
}
