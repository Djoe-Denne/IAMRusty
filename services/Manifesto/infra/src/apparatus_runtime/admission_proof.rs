//! Offline, pinned-key admission proof verification. No network or private key.

use apparatus_contracts::runtime::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rsa::{
    pkcs1v15::{Signature, VerifyingKey},
    signature::Verifier as _,
    traits::PublicKeyParts,
    RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::sync::Arc;
use tokio::sync::{OwnedRwLockReadGuard, RwLock};
use uuid::Uuid;

type PinnedKeys = Vec<(PinnedRuntimeAdmissionKey, String)>;

/// Local immutable proof, constructible only by this verifier module. A proof
/// does not bypass writer challenge consumption, negative guards or profile lock.
pub struct VerifiedRuntimeAdmissionV1 {
    claims: RuntimeAdmissionAttestationV1,
    kid: String,
    fingerprint: [u8; 32],
    // Held until the caller commits/rolls back its SQL transaction. Key removal
    // is serialized with the actual act, not just a stale preflight check.
    _key_policy_lease: OwnedRwLockReadGuard<PinnedKeys>,
}
impl VerifiedRuntimeAdmissionV1 {
    pub fn claims(&self) -> &RuntimeAdmissionAttestationV1 {
        &self.claims
    }
    pub fn kid(&self) -> &str {
        &self.kid
    }
    pub fn fingerprint(&self) -> &[u8; 32] {
        &self.fingerprint
    }
}

pub trait RuntimeAdmissionProofVerifier: Send + Sync {
    fn verify(
        &self,
        expected: &RuntimeAdmissionChallengeV1,
        compact_jws: &str,
        db_now: RuntimeUnixMsV1,
    ) -> Result<VerifiedRuntimeAdmissionV1, RuntimeErrorCodeV1>;
}

/// Default wiring: the existence of a challenge never enables maintenance.
pub struct RejectRuntimeAdmissionProofVerifier;
impl RuntimeAdmissionProofVerifier for RejectRuntimeAdmissionProofVerifier {
    fn verify(
        &self,
        _: &RuntimeAdmissionChallengeV1,
        _: &str,
        _: RuntimeUnixMsV1,
    ) -> Result<VerifiedRuntimeAdmissionV1, RuntimeErrorCodeV1> {
        Err(RuntimeErrorCodeV1::RuntimeKeyUnavailable)
    }
}

/// Setup-provisioned public trust entry, never populated from the JWS.
pub struct PinnedRuntimeAdmissionKey {
    pub authority_id: Uuid,
    pub installation_id: Uuid,
    pub public_key: RsaPublicKey,
}

pub struct PinnedRuntimeAdmissionProofVerifier {
    keys: Arc<RwLock<PinnedKeys>>,
    allowed_repositories: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    alg: String,
    typ: String,
    kid: String,
}

impl PinnedRuntimeAdmissionProofVerifier {
    /// Build once from pinned configuration. Duplicate/ambiguous keys close the
    /// configuration, rather than selecting the first matching candidate.
    pub fn new(
        keys: Vec<PinnedRuntimeAdmissionKey>,
        allowed_repositories: Vec<String>,
    ) -> Result<Self, RuntimeErrorCodeV1> {
        let unavailable = RuntimeErrorCodeV1::RuntimeKeyUnavailable;
        if keys.is_empty()
            || allowed_repositories.is_empty()
            || allowed_repositories.iter().any(|r| {
                r.is_empty()
                    || r.contains('@')
                    || !r.is_ascii()
                    || !r.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b".-_/:".contains(&b)
                    })
                    || !r.contains('/')
                    || r.split('/')
                        .skip(1)
                        .any(|segment| segment.is_empty() || segment.contains(':'))
                    || r.contains('*')
                    || r.ends_with('/')
            })
        {
            return Err(unavailable);
        }
        let mut configured = Vec::new();
        for key in keys {
            if key.authority_id.is_nil()
                || key.installation_id.is_nil()
                || key.public_key.n().bits() != 3072
                || key.public_key.e().to_bytes_be() != [1, 0, 1]
            {
                return Err(unavailable);
            }
            let n = URL_SAFE_NO_PAD.encode(key.public_key.n().to_bytes_be());
            // RFC7638 canonical RSA public JWK, exact lexicographic member order.
            let thumbprint = format!("{{\"e\":\"AQAB\",\"kty\":\"RSA\",\"n\":\"{n}\"}}");
            let kid = format!(
                "ara-v1-{}",
                URL_SAFE_NO_PAD.encode(Sha256::digest(thumbprint.as_bytes()))
            );
            if configured
                .iter()
                .any(|(_, old): &(PinnedRuntimeAdmissionKey, String)| old == &kid)
            {
                return Err(unavailable);
            }
            configured.push((key, kid));
        }
        Ok(Self {
            keys: Arc::new(RwLock::new(configured)),
            allowed_repositories,
        })
    }

    /// Trusted setup/maintenance key withdrawal. Existing guarded transactions
    /// finish before this returns; subsequent verification cannot use the key.
    /// No expiry grace, discovery or alternate algorithm is installed.
    pub async fn revoke_key(&self, kid: &str) {
        self.keys
            .write()
            .await
            .retain(|(_, current)| current != kid);
    }

    fn pinned_reference(&self, value: &str) -> bool {
        if validate_runtime_oci_pin(value).is_err() {
            return false;
        }
        let Some((repository, digest)) = value.split_once('@') else {
            return false;
        };
        self.allowed_repositories
            .iter()
            .any(|allowed| repository == allowed)
            && RuntimeDigestV1::new(digest.to_owned()).is_ok()
    }
}

impl RuntimeAdmissionProofVerifier for PinnedRuntimeAdmissionProofVerifier {
    fn verify(
        &self,
        expected: &RuntimeAdmissionChallengeV1,
        compact: &str,
        now: RuntimeUnixMsV1,
    ) -> Result<VerifiedRuntimeAdmissionV1, RuntimeErrorCodeV1> {
        let invalid = RuntimeErrorCodeV1::RuntimeInvalidProof;
        expected.validate().map_err(|_| invalid)?;
        if compact.len() > MAX_ADMISSION_PROOF_BYTES || !compact.is_ascii() {
            return Err(invalid);
        }
        let parts: Vec<_> = compact.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
            return Err(invalid);
        }
        let decode = |part: &str| -> Result<Vec<u8>, RuntimeErrorCodeV1> {
            let bytes = URL_SAFE_NO_PAD.decode(part).map_err(|_| invalid)?;
            if URL_SAFE_NO_PAD.encode(&bytes) != part {
                return Err(invalid);
            }
            Ok(bytes)
        };
        let header_bytes = decode(parts[0])?;
        let header: Header = parse_runtime_json(&header_bytes).map_err(|_| invalid)?;
        if header.alg != "RS256" || header.typ != ADMISSION_TYP {
            return Err(invalid);
        }
        let keys = self
            .keys
            .clone()
            .try_read_owned()
            .map_err(|_| RuntimeErrorCodeV1::RuntimeKeyUnavailable)?;
        let (key, _) = keys
            .iter()
            .find(|(_, kid)| kid == &header.kid)
            .ok_or(RuntimeErrorCodeV1::RuntimeKeyUnavailable)?;
        if key.authority_id != expected.authority_id
            || key.installation_id != expected.installation_id
        {
            return Err(invalid);
        }
        let signature = Signature::try_from(decode(parts[2])?.as_slice()).map_err(|_| invalid)?;
        let signed = format!("{}.{}", parts[0], parts[1]);
        VerifyingKey::<Sha256>::new(key.public_key.clone())
            .verify(signed.as_bytes(), &signature)
            .map_err(|_| invalid)?;
        let payload = decode(parts[1])?;
        let claims: RuntimeAdmissionAttestationV1 =
            parse_runtime_json(&payload).map_err(|_| invalid)?;
        // JCS is a signed protocol invariant, not merely equivalent JSON data.
        let canonical =
            serde_json::from_slice::<serde_json::Value>(&payload).map_err(|_| invalid)?;
        if serde_jcs::to_vec(&canonical).map_err(|_| invalid)? != payload {
            return Err(invalid);
        }
        if claims.schema_version != 1
            || claims.challenge != *expected
            || expected.schema_version != 1
            || claims.iss != ADMISSION_ISSUER
            || claims.aud != ADMISSION_AUDIENCE
            || claims.purpose != ADMISSION_PURPOSE
            || claims.jti != expected.challenge_id
            || claims.record_id.is_nil()
            || claims.admission_revision.get() == 0
        {
            return Err(invalid);
        }
        let observed = claims.observed_at_unix_ms.get();
        let until = claims.valid_until_unix_ms.get();
        let challenge_expiry = expected.expires_at_unix_ms.get();
        if expected
            .issued_at_unix_ms
            .get()
            .checked_add(ADMISSION_CHALLENGE_TTL_MS)
            != Some(challenge_expiry)
            || expected.issued_at_unix_ms.get() > observed
            || observed > now.get()
            || now.get() >= until
            || now.get() >= challenge_expiry
            || observed
                .checked_add(ADMISSION_PROOF_TTL_MS)
                .map(|v| v.min(challenge_expiry))
                != Some(until)
            || claims.iat != observed / 1000
            || claims.exp != until / 1000
            || claims.exp <= claims.iat
            || now.get() / 1000 >= claims.exp
        {
            return Err(invalid);
        }
        match expected.action {
            RuntimeAdmissionActionV1::Prepare
                if expected.service_uid.is_none() && expected.revision_uid.is_none() => {}
            RuntimeAdmissionActionV1::Open
                if expected
                    .service_uid
                    .as_deref()
                    .is_some_and(|s| validate_runtime_uid(s).is_ok())
                    && expected
                        .revision_uid
                        .as_deref()
                        .is_some_and(|s| validate_runtime_uid(s).is_ok()) => {}
            _ => return Err(invalid),
        }
        if claims.policy_id != "apparatus-p4-conformance/v1"
            || !self.pinned_reference(&claims.envelope_reference)
            || !self.pinned_reference(&claims.cri_image)
        {
            return Err(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted);
        }
        Ok(VerifiedRuntimeAdmissionV1 {
            claims,
            kid: header.kid,
            fingerprint: Sha256::digest(compact.as_bytes()).into(),
            _key_policy_lease: keys,
        })
    }
}
