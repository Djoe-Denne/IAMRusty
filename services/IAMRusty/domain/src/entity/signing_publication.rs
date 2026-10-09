//! ADR-0312 candidate: prepared public entries and fixed, disjoint slot bounds.
//!
//! These numbers remain Proposed. The transaction/SQL snapshot cutover must
//! activate them together; a pure bound is not a PostgreSQL/runtime proof.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rsa::traits::PublicKeyParts;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

use super::signing_key::{
    admission_denied, parse_signing_public_key, require_transit_key_version, SigningKey,
    SigningKeyAdmissionReason, SigningKeyStatus, SigningProviderType, SigningScope, TrustScope,
};
use super::token::{Jwk, JwkSet};
use crate::error::DomainError;

pub const SLOT_BYTES_MAX: usize = 4096;
pub const SLOT_CAP_ORG: usize = 8;
pub const SLOT_COUNT_GLOBAL: usize = 191;
pub const SLOT_CAP_PLATFORM: usize = 16;
pub const SLOT_COUNT_GLOBAL_ORG: usize = 175;
pub const JWKS_ENVELOPE_BYTES: usize = 11;
pub const JWKS_WRITER_BYTES_MAX: usize = 786_432;
pub const JWKS_ORG_BYTES_MAX: usize = 720_896;
pub const JWKS_PLATFORM_RESERVE_BYTES: usize = 65_536;
pub const JWKS_CONSUMER_BYTES_MAX: usize = 1_048_576;
pub const JWKS_SLOT_FRAME_BYTES_MAX: usize =
    JWKS_ENVELOPE_BYTES + SLOT_COUNT_GLOBAL * (SLOT_BYTES_MAX + 1);

const fn invalid() -> DomainError {
    DomainError::InvalidSigningKeyMaterial
}
const fn capacity() -> DomainError {
    admission_denied(SigningKeyAdmissionReason::Capacity)
}

/// Counts supplied by the primary writer at its post-lock database clock.
/// `organization` is the target owner's count, not the number of organizations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SigningSlotCounts {
    pub global: usize,
    pub global_organization: usize,
    pub platform: usize,
    pub organization: usize,
}

impl SigningSlotCounts {
    /// # Errors
    ///
    /// Returns [`DomainError`] if the counts violate the ratified slot caps.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.global_organization.checked_add(self.platform) != Some(self.global)
            || self.organization > self.global_organization
            || self.global > SLOT_COUNT_GLOBAL
            || self.global_organization > SLOT_COUNT_GLOBAL_ORG
            || self.platform > SLOT_CAP_PLATFORM
        {
            return Err(capacity());
        }
        if self.organization > SLOT_CAP_ORG {
            return Err(admission_denied(
                SigningKeyAdmissionReason::TenantEpochLimit,
            ));
        }
        Ok(())
    }
}

/// Public n/e plus an immutable binding fingerprint, with no private material.
/// The sole PEM parse happens in `prepare`; status projections never reparse it.
#[derive(Clone, Debug)]
pub struct PreparedSigningPublicKey {
    n: String,
    e: String,
    binding_fingerprint: [u8; 32],
    longest_entry_bytes: usize,
}

impl PreparedSigningPublicKey {
    /// Effective provider binding comparison, independent of row/kid and PEM
    /// whitespace. Both components are already canonical; no RSA parse here.
    #[must_use]
    pub fn same_binding(&self, a: &SigningKey, other: &Self, b: &SigningKey) -> bool {
        a.algorithm == b.algorithm
            && a.issuer == b.issuer
            && a.trust_scope == b.trust_scope
            && a.organization_id == b.organization_id
            && a.provider_type == b.provider_type
            && a.provider_key_ref == b.provider_key_ref
            && a.provider_key_version == b.provider_key_version
            && a.credential_ref == b.credential_ref
            && self.n == other.n
            && self.e == other.e
    }

    /// # Errors
    ///
    /// Returns [`DomainError`] if metadata or public material cannot be prepared.
    pub fn prepare(key: &SigningKey) -> Result<Self, DomainError> {
        validate_metadata(key)?;
        let public = parse_signing_public_key(&key.public_key)?;
        let n = URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public.e().to_bytes_be());
        Self::from_components(key, n, e)
    }

    /// Restore a writer-prepared record without parsing its PEM again. A changed
    /// binding/material or malformed canonical components fails closed.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if persisted components fail closed.
    pub fn from_persisted(
        key: &SigningKey,
        n: String,
        e: String,
        binding_fingerprint: &[u8],
        longest_entry_bytes: usize,
    ) -> Result<Self, DomainError> {
        validate_metadata(key)?;
        validate_components(&n, &e)?;
        if longest_entry_bytes == 0 || longest_entry_bytes > SLOT_BYTES_MAX {
            return Err(invalid());
        }
        let expected = fingerprint(key, &n, &e, longest_entry_bytes)?;
        if binding_fingerprint != expected {
            return Err(invalid());
        }
        // The size attestation is bound to the immutable record; do not measure
        // Retiring or parse its PEM again on every mutation. Final DTO assembly
        // still checks actual entry/frame bytes independently.
        Ok(Self {
            n,
            e,
            binding_fingerprint: expected,
            longest_entry_bytes,
        })
    }

    fn from_components(key: &SigningKey, n: String, e: String) -> Result<Self, DomainError> {
        validate_metadata(key)?;
        validate_components(&n, &e)?;
        let entry = public_entry(key, &n, &e, SigningKeyStatus::Retiring);
        // Retiring is the longest reachable status spelling. No three-variant
        // reservation/tariff; this measurement validates this one binding only.
        let longest_entry_bytes = checked_entry_bytes(&entry)?;
        Ok(Self {
            binding_fingerprint: fingerprint(key, &n, &e, longest_entry_bytes)?,
            n,
            e,
            longest_entry_bytes,
        })
    }

    #[must_use]
    pub fn n(&self) -> &str {
        &self.n
    }
    #[must_use]
    pub fn e(&self) -> &str {
        &self.e
    }
    #[must_use]
    pub const fn binding_fingerprint(&self) -> &[u8; 32] {
        &self.binding_fingerprint
    }
    #[must_use]
    pub const fn longest_entry_bytes(&self) -> usize {
        self.longest_entry_bytes
    }

    /// # Errors
    ///
    /// Returns [`DomainError`] if the key is revoked or the binding no longer matches.
    pub fn project(&self, key: &SigningKey) -> Result<Jwk, DomainError> {
        if key.status == SigningKeyStatus::Revoked
            || fingerprint(key, &self.n, &self.e, self.longest_entry_bytes)?
                != self.binding_fingerprint
        {
            return Err(invalid());
        }
        Ok(public_entry(key, &self.n, &self.e, key.status.clone()))
    }
}

fn validate_metadata(key: &SigningKey) -> Result<(), DomainError> {
    let kid = Uuid::parse_str(&key.kid).map_err(|_| invalid())?;
    SigningScope::of(key).validate()?;
    if key.algorithm != "RS256"
        || kid.get_version() != Some(uuid::Version::Random)
        || key.kid != kid.simple().to_string()
        || key.issuer.len() > 1024
        || url::Url::parse(&key.issuer).is_err()
    {
        return Err(invalid());
    }
    if key.provider_type == SigningProviderType::OpenBaoTransit
        || key.provider_key_version.is_some()
    {
        require_transit_key_version(key.provider_key_version)?;
    }
    Ok(())
}

fn canonical_bytes(encoded: &str, max_bytes: usize) -> Result<Vec<u8>, DomainError> {
    // Bound allocation before decoding stored/adversarial data.
    if encoded.len() > (max_bytes * 4).div_ceil(3) {
        return Err(invalid());
    }
    let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| invalid())?;
    if bytes.is_empty()
        || bytes[0] == 0
        || bytes.len() > max_bytes
        || URL_SAFE_NO_PAD.encode(&bytes) != encoded
    {
        return Err(invalid());
    }
    Ok(bytes)
}

fn validate_components(n: &str, e: &str) -> Result<(), DomainError> {
    let modulus = canonical_bytes(n, 1024)?;
    let bits = (modulus.len() - 1) * 8 + (8 - modulus[0].leading_zeros() as usize);
    let exponent = canonical_bytes(e, 8)?;
    let exponent = exponent
        .iter()
        .fold(0u64, |value, byte| (value << 8) | u64::from(*byte));
    if !(2048..=8192).contains(&bits)
        || exponent < 3
        || exponent % 2 == 0
        || exponent > rsa::RsaPublicKey::MAX_PUB_EXPONENT
    {
        return Err(invalid());
    }
    Ok(())
}

fn public_entry(key: &SigningKey, n: &str, e: &str, status: SigningKeyStatus) -> Jwk {
    Jwk {
        kty: "RSA".into(),
        kid: key.kid.clone(),
        use_: "sig".into(),
        alg: "RS256".into(),
        n: n.into(),
        e: e.into(),
        iss: key.issuer.clone(),
        status: Some(status),
        trust_scope: Some(key.trust_scope.clone()),
        organization_id: key.organization_id,
    }
}

fn fingerprint(
    key: &SigningKey,
    n: &str,
    e: &str,
    longest_entry_bytes: usize,
) -> Result<[u8; 32], DomainError> {
    // Status/timestamps deliberately excluded; every immutable binding field,
    // including recorded PEM bytes and provider version, is bound. Never public.
    let bytes = serde_json::to_vec(&(
        key.id,
        &key.kid,
        &key.algorithm,
        &key.trust_scope,
        &key.issuer,
        &key.provider_type,
        &key.provider_key_ref,
        key.provider_key_version,
        &key.credential_ref,
        &key.public_key,
        key.organization_id,
        n,
        e,
        longest_entry_bytes,
    ))
    .map_err(|_| invalid())?;
    Ok(Sha256::digest(bytes).into())
}

/// Validate the actual compact entry, not a bound inferred from issuer bytes.
///
/// # Errors
///
/// Returns [`DomainError`] if the compact entry exceeds [`SLOT_BYTES_MAX`].
pub fn checked_entry_bytes(entry: &Jwk) -> Result<usize, DomainError> {
    let bytes = serde_json::to_vec(entry).map_err(|_| invalid())?.len();
    if bytes > SLOT_BYTES_MAX {
        return Err(invalid());
    }
    Ok(bytes)
}

/// Canonical immutable public payload for the later transactional snapshot writer.
/// No fallback to registry keys, omission, eviction or PEM conversion is possible.
#[derive(Clone, Debug)]
pub struct ValidatedJwksPublication {
    jwks: JwkSet,
    compact_payload: String,
    counts: SigningSlotCounts,
}

impl ValidatedJwksPublication {
    #[must_use]
    pub fn usage(&self) -> super::signing_key::SigningKeyPublicationUsage {
        let mut owners = BTreeMap::new();
        for key in &self.jwks.keys {
            if let Some(org) = key.organization_id {
                *owners.entry(org).or_default() += 1;
            }
        }
        super::signing_key::SigningKeyPublicationUsage {
            actual_bytes: self.compact_payload.len(),
            reserved_bytes: JWKS_ENVELOPE_BYTES + self.counts.global * (SLOT_BYTES_MAX + 1),
            organization_reserved_bytes: self.counts.global_organization * (SLOT_BYTES_MAX + 1),
            platform_epochs: self.counts.platform,
            organization_epochs: owners,
        }
    }

    /// # Errors
    ///
    /// Returns [`DomainError`] if the entry set exceeds slot caps or is inconsistent.
    pub fn from_entries(entries: Vec<Jwk>) -> Result<Self, DomainError> {
        if entries.len() > SLOT_COUNT_GLOBAL {
            return Err(capacity());
        }
        let mut kids = HashSet::new();
        let mut organizations = BTreeMap::<Uuid, usize>::new();
        let mut counts = SigningSlotCounts {
            global: entries.len(),
            ..Default::default()
        };
        for entry in &entries {
            let kid = Uuid::parse_str(&entry.kid).map_err(|_| invalid())?;
            if entry.kty != "RSA"
                || entry.alg != "RS256"
                || entry.use_ != "sig"
                || kid.get_version() != Some(uuid::Version::Random)
                || entry.kid != kid.simple().to_string()
                || !kids.insert(&entry.kid)
                || entry.iss.len() > 1024
                || url::Url::parse(&entry.iss).is_err()
                || !matches!(
                    entry.status,
                    Some(
                        SigningKeyStatus::Pending
                            | SigningKeyStatus::Active
                            | SigningKeyStatus::Retiring
                    )
                )
            {
                return Err(invalid());
            }
            match (&entry.trust_scope, entry.organization_id) {
                (Some(TrustScope::Platform), None) => counts.platform += 1,
                (Some(TrustScope::Organization), Some(org)) => {
                    counts.global_organization += 1;
                    *organizations.entry(org).or_default() += 1;
                }
                _ => return Err(invalid()),
            }
            validate_components(&entry.n, &entry.e)?;
            checked_entry_bytes(entry)?;
        }
        counts.organization = organizations.values().copied().max().unwrap_or(0);
        counts.validate()?;
        let jwks = JwkSet { keys: entries };
        let bytes = jwks.compact_bytes()?;
        if bytes.len() > JWKS_SLOT_FRAME_BYTES_MAX || bytes.len() > JWKS_WRITER_BYTES_MAX {
            return Err(capacity());
        }
        let compact_payload = String::from_utf8(bytes).map_err(|_| invalid())?;
        Ok(Self {
            jwks,
            compact_payload,
            counts,
        })
    }

    /// Fail closed on corrupt, noncanonical, duplicate, unbounded or foreign
    /// fields. Missing/empty publication is not inferred from a parse error.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the payload is corrupt, noncanonical, or too large.
    pub fn from_compact_payload(payload: &str) -> Result<Self, DomainError> {
        if payload.len() > JWKS_SLOT_FRAME_BYTES_MAX {
            return Err(capacity());
        }
        let jwks: JwkSet = serde_json::from_str(payload).map_err(|_| invalid())?;
        let validated = Self::from_entries(jwks.keys)?;
        if validated.compact_payload != payload {
            return Err(invalid());
        }
        Ok(validated)
    }
    #[must_use]
    pub fn payload(&self) -> &str {
        &self.compact_payload
    }
    #[must_use]
    pub const fn jwks(&self) -> &JwkSet {
        &self.jwks
    }
    #[must_use]
    pub const fn counts(&self) -> SigningSlotCounts {
        self.counts
    }
    #[must_use]
    pub fn into_jwks(self) -> JwkSet {
        self.jwks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rsa::pkcs8::{EncodePublicKey, LineEnding};

    fn key(scope: TrustScope) -> SigningKey {
        let now = Utc::now();
        SigningKey {
            id: Uuid::new_v4(),
            kid: Uuid::new_v4().simple().to_string(),
            algorithm: "RS256".into(),
            organization_id: if scope == TrustScope::Organization {
                Some(Uuid::new_v4())
            } else {
                None
            },
            trust_scope: scope,
            issuer: "https://issuer.example/iam".into(),
            provider_type: SigningProviderType::PemFile,
            provider_key_ref: "provisioned-fixture.pem".into(),
            provider_key_version: None,
            credential_ref: None,
            public_key: include_str!("../../../config/keys/test-platform.pub").into(),
            status: SigningKeyStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }
    fn entry(scope: TrustScope) -> Jwk {
        let key = key(scope);
        PreparedSigningPublicKey::prepare(&key)
            .unwrap()
            .project(&key)
            .unwrap()
    }

    #[test]
    fn partitions_and_serializer_envelope_prove_candidate_frame_independently() {
        assert_eq!(
            serde_json::to_vec(&JwkSet { keys: vec![] }).unwrap(),
            b"{\"keys\":[]}"
        );
        // Independent literals/oracles, not expected values computed by policy.
        assert_eq!(SLOT_BYTES_MAX, 4096);
        assert_eq!(SLOT_CAP_ORG, 8);
        assert_eq!(SLOT_COUNT_GLOBAL, 191);
        assert_eq!(SLOT_CAP_PLATFORM, 16);
        assert_eq!(SLOT_COUNT_GLOBAL_ORG, 175);
        assert_eq!(11 + 191 * 4097, 782_538);
        assert_eq!(175 * 4097, 716_975);
        assert_eq!(786_432 - 11 - 716_975, 69_446);
        // Frame inequalities (literals, not runtime): 69_446 >= 64KiB, compact
        // envelope 782_538 <= 786_432 < 1MiB, org partition 716_975 <= 720_896,
        // sixteen worst entries (16*4097) sit between 64KiB and 69_446.
        assert_eq!(JWKS_SLOT_FRAME_BYTES_MAX, 782_538);
    }

    #[test]
    fn reserved_platform_partition_cannot_be_borrowed_by_orgs_and_last_slots_are_precise() {
        SigningSlotCounts {
            global: 191,
            global_organization: 175,
            platform: 16,
            organization: 8,
        }
        .validate()
        .unwrap();
        for counts in [
            SigningSlotCounts {
                global: 176,
                global_organization: 176,
                platform: 0,
                organization: 8,
            },
            SigningSlotCounts {
                global: 192,
                global_organization: 175,
                platform: 17,
                organization: 8,
            },
            SigningSlotCounts {
                global: 17,
                global_organization: 0,
                platform: 17,
                organization: 0,
            },
            SigningSlotCounts {
                global: 0,
                global_organization: 1,
                platform: 0,
                organization: 0,
            },
        ] {
            assert!(matches!(
                counts.validate(),
                Err(DomainError::SigningKeyAdmissionDenied {
                    reason: SigningKeyAdmissionReason::Capacity,
                    ..
                })
            ));
        }
        assert!(matches!(
            SigningSlotCounts {
                global: 9,
                global_organization: 9,
                platform: 0,
                organization: 9
            }
            .validate(),
            Err(DomainError::SigningKeyAdmissionDenied {
                reason: SigningKeyAdmissionReason::TenantEpochLimit,
                ..
            })
        ));
        SigningSlotCounts::default().validate().unwrap();
    }

    #[test]
    fn prepared_projection_survives_states_and_timestamps_but_not_any_binding_change() {
        let original = key(TrustScope::Organization);
        let prepared = PreparedSigningPublicKey::prepare(&original).unwrap();
        let expected_n = prepared.n().to_string();
        for status in [
            SigningKeyStatus::Pending,
            SigningKeyStatus::Active,
            SigningKeyStatus::Retiring,
        ] {
            let mut changed = original.clone();
            changed.status = status.clone();
            changed.updated_at += chrono::Duration::seconds(23);
            let jwk = prepared.project(&changed).unwrap();
            assert_eq!(jwk.status, Some(status));
            assert_eq!(jwk.n, expected_n);
            assert!(serde_json::to_vec(&jwk).unwrap().len() <= prepared.longest_entry_bytes());
            let json = serde_json::to_value(jwk).unwrap();
            for name in [
                "provider_key_ref",
                "provider_key_version",
                "credential_ref",
                "public_key",
                "binding_fingerprint",
            ] {
                assert!(json.get(name).is_none());
            }
        }
        for case in 0..9 {
            let mut changed = original.clone();
            match case {
                0 => changed.provider_key_ref.push('x'),
                1 => changed.provider_key_version = Some(7),
                2 => changed.credential_ref = Some("other".into()),
                3 => changed.issuer.push_str("/x"),
                4 => changed.kid = Uuid::new_v4().simple().to_string(),
                5 => changed.id = Uuid::new_v4(),
                6 => changed.public_key.push('\n'),
                7 => changed.organization_id = Some(Uuid::new_v4()),
                _ => changed.status = SigningKeyStatus::Revoked,
            }
            assert!(prepared.project(&changed).is_err());
        }
        PreparedSigningPublicKey::from_persisted(
            &original,
            prepared.n().into(),
            prepared.e().into(),
            prepared.binding_fingerprint(),
            prepared.longest_entry_bytes(),
        )
        .unwrap();
        assert!(PreparedSigningPublicKey::from_persisted(
            &original,
            format!("{}=", prepared.n()),
            prepared.e().into(),
            prepared.binding_fingerprint(),
            prepared.longest_entry_bytes()
        )
        .is_err());
        assert!(PreparedSigningPublicKey::from_persisted(
            &original,
            prepared.n().into(),
            prepared.e().into(),
            &[0; 32],
            prepared.longest_entry_bytes()
        )
        .is_err());
        assert!(PreparedSigningPublicKey::from_persisted(
            &original,
            prepared.n().into(),
            prepared.e().into(),
            prepared.binding_fingerprint(),
            prepared.longest_entry_bytes() + 1
        )
        .is_err());
    }

    fn public_8192() -> String {
        // Public-shape serializer oracle, NOT private crypto8192/PoP evidence.
        let mut n = vec![0; 1024];
        n[0] = 0x80;
        n[1023] = 1;
        rsa::RsaPublicKey::new_with_max_size(
            rsa::BigUint::from_bytes_be(&n),
            rsa::BigUint::from(rsa::RsaPublicKey::MAX_PUB_EXPONENT),
            8192,
        )
        .unwrap()
        .to_public_key_pem(LineEnding::LF)
        .unwrap()
    }

    #[test]
    fn issuer_byte_bound_does_not_replace_real_dto_escaping_and_exact4097_rejection() {
        let mut candidate = key(TrustScope::Organization);
        candidate.public_key = public_8192();
        candidate.issuer = format!("https://issuer.example/{}", "\u{0001}".repeat(800));
        assert!(candidate.issuer.len() < 1024);
        assert!(url::Url::parse(&candidate.issuer).is_ok());
        assert!(PreparedSigningPublicKey::prepare(&candidate).is_err());
        let mut raw = entry(TrustScope::Organization);
        raw.n = URL_SAFE_NO_PAD.encode(vec![255; 1024]);
        raw.e = URL_SAFE_NO_PAD.encode(u64::MAX.to_be_bytes());
        raw.status = Some(SigningKeyStatus::Retiring);
        // e64 is a conservative DTO-size oracle, not an accepted backend exponent.
        raw.iss = "https://issuer.example/".into();
        let baseline = serde_json::to_vec(&raw).unwrap().len();
        for wanted in [4096, 4097] {
            let added = wanted - baseline;
            raw.iss = format!(
                "https://issuer.example/{}{}",
                "\u{0001}".repeat(added / 6),
                "x".repeat(added % 6)
            );
            assert!(raw.iss.len() <= 1024);
            assert_eq!(serde_json::to_vec(&raw).unwrap().len(), wanted);
            assert_eq!(checked_entry_bytes(&raw).is_ok(), wanted == 4096);
        }
        raw.iss = format!("https://issuer.example/{}", "é\"\\\n".repeat(80));
        assert!(raw.iss.len() <= 1024);
        let retiring = serde_json::to_vec(&raw).unwrap().len();
        for status in [
            SigningKeyStatus::Pending,
            SigningKeyStatus::Active,
            SigningKeyStatus::Retiring,
        ] {
            raw.status = Some(status);
            assert!(serde_json::to_vec(&raw).unwrap().len() <= retiring);
        }
    }

    #[test]
    fn maximal_frame_uses_real_compact_dtos_without_duplicates_or_private_fields() {
        let mut entries = Vec::new();
        let mut maximum = entry(TrustScope::Organization);
        maximum.n = URL_SAFE_NO_PAD.encode(vec![255; 1024]);
        maximum.e = URL_SAFE_NO_PAD.encode(
            rsa::RsaPublicKey::MAX_PUB_EXPONENT
                .to_be_bytes()
                .into_iter()
                .skip_while(|b| *b == 0)
                .collect::<Vec<_>>(),
        );
        maximum.status = Some(SigningKeyStatus::Retiring);
        let base = serde_json::to_vec(&maximum).unwrap().len();
        let extra = 4096 - base;
        maximum.iss = format!(
            "https://issuer.example/iam{}{}",
            "\u{0001}".repeat(extra / 6),
            "x".repeat(extra % 6)
        );
        assert_eq!(serde_json::to_vec(&maximum).unwrap().len(), 4096);
        for index in 0..191 {
            let mut value = maximum.clone();
            value.kid = Uuid::new_v4().simple().to_string();
            if index >= 175 {
                value.trust_scope = Some(TrustScope::Platform);
                value.organization_id = None;
            } else {
                value.organization_id = Some(Uuid::new_v4());
            }
            entries.push(value);
        }
        let publication = ValidatedJwksPublication::from_entries(entries).unwrap();
        assert!(publication.payload().len() <= 782_538);
        assert!(publication.payload().len() <= 786_432);
        assert_eq!(publication.counts().global, 191);
        assert_eq!(publication.counts().global_organization, 175);
        assert_eq!(publication.counts().platform, 16);
        let restored =
            ValidatedJwksPublication::from_compact_payload(publication.payload()).unwrap();
        assert_eq!(restored.into_jwks().keys.len(), 191);
        let dto: serde_json::Value = serde_json::from_str(publication.payload()).unwrap();
        assert_eq!(dto["keys"].as_array().unwrap().len(), 191);
    }

    #[test]
    fn payload_corruption_duplicates_or_foreign_fields_never_become_an_empty_success() {
        let empty = ValidatedJwksPublication::from_entries(vec![]).unwrap();
        assert_eq!(empty.payload(), "{\"keys\":[]}");
        ValidatedJwksPublication::from_compact_payload(empty.payload()).unwrap();
        for payload in ["", "{}", "{\"keys\":[],\"private\":true}", "{ \"keys\":[]}"] {
            assert!(ValidatedJwksPublication::from_compact_payload(payload).is_err());
        }
        let valid = entry(TrustScope::Platform);
        assert!(
            ValidatedJwksPublication::from_entries(vec![valid.clone(), valid.clone()]).is_err()
        );
        let mut corrupt = valid.clone();
        corrupt.e = URL_SAFE_NO_PAD.encode(u64::MAX.to_be_bytes());
        assert!(ValidatedJwksPublication::from_entries(vec![corrupt]).is_err());
        let mut corrupt = valid;
        corrupt.organization_id = Some(Uuid::new_v4());
        assert!(ValidatedJwksPublication::from_entries(vec![corrupt]).is_err());
    }
}
