//! SigningKey registry model (ADR-0304).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trust domain owning a signing key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustScope {
    Platform,
    Organization,
}

/// Lifecycle status for JWKS publication and signing eligibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningKeyStatus {
    /// Pre-published in JWKS before any signature (rotation N+1).
    Pending,
    /// Active signing + JWKS.
    Active,
    /// Still in JWKS for verification during rotation; do not mint new tokens.
    Retiring,
    /// Revoked; never in JWKS.
    Revoked,
}

impl SigningKeyStatus {
    /// Whether this key may appear in `GET /.well-known/jwks.json`.
    #[must_use]
    pub const fn in_jwks(&self) -> bool {
        matches!(self, Self::Pending | Self::Active | Self::Retiring)
    }

    /// Whether this key may mint new access tokens.
    #[must_use]
    pub const fn can_sign(&self) -> bool {
        matches!(self, Self::Active)
    }
}

impl From<&SigningKeyStatus> for String {
    fn from(status: &SigningKeyStatus) -> Self {
        match status {
            SigningKeyStatus::Pending => "pending".to_string(),
            SigningKeyStatus::Active => "active".to_string(),
            SigningKeyStatus::Retiring => "retiring".to_string(),
            SigningKeyStatus::Revoked => "revoked".to_string(),
        }
    }
}

impl std::str::FromStr for SigningKeyStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "retiring" => Ok(Self::Retiring),
            "revoked" => Ok(Self::Revoked),
            other => Err(format!("unknown signing key status: {other}")),
        }
    }
}

/// Provider backend type stored on a [`SigningKey`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningProviderType {
    PemFile,
    OpenBaoTransit,
    AwsKms,
    GcpKms,
    AzureKeyVault,
    /// HTTP Sign / GetPublicKey adapter (ADR-0309). Not a cloud BYOKMS.
    RemoteHttp,
}

impl SigningProviderType {
    /// Cloud BYOKMS types that IAM refuses to configure (no adapters yet).
    #[must_use]
    pub const fn is_unsupported_cloud_byokms(&self) -> bool {
        matches!(self, Self::AwsKms | Self::GcpKms | Self::AzureKeyVault)
    }
}

impl From<&SigningProviderType> for String {
    fn from(value: &SigningProviderType) -> Self {
        match value {
            SigningProviderType::PemFile => "pem_file".to_string(),
            SigningProviderType::OpenBaoTransit => "openbao_transit".to_string(),
            SigningProviderType::AwsKms => "aws_kms".to_string(),
            SigningProviderType::GcpKms => "gcp_kms".to_string(),
            SigningProviderType::AzureKeyVault => "azure_key_vault".to_string(),
            SigningProviderType::RemoteHttp => "remote_http".to_string(),
        }
    }
}

impl std::str::FromStr for SigningProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pem_file" => Ok(Self::PemFile),
            "openbao_transit" => Ok(Self::OpenBaoTransit),
            "aws_kms" => Ok(Self::AwsKms),
            "gcp_kms" => Ok(Self::GcpKms),
            "azure_key_vault" => Ok(Self::AzureKeyVault),
            "remote_http" | "remote" => Ok(Self::RemoteHttp),
            other => Err(format!("unknown signing provider type: {other}")),
        }
    }
}

/// Registry row for a platform or organization signing key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningKey {
    pub id: Uuid,
    /// Opaque public key id — never an org id, ARN, or OpenBao path.
    pub kid: String,
    pub algorithm: String,
    pub trust_scope: TrustScope,
    /// Issuer URL bound to this key (must match JWT `iss`).
    pub issuer: String,
    pub provider_type: SigningProviderType,
    pub provider_key_ref: String,
    pub credential_ref: Option<String>,
    pub public_key: String,
    pub status: SigningKeyStatus,
    pub organization_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One coherent primary SELECT, not an application-clock or bootstrap snapshot.
#[derive(Clone)]
pub struct SigningKeyPublicationSnapshot {
    pub keys: Vec<SigningKey>,
    pub as_of: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningKeyAdmissionReason {
    Capacity,
    TenantEpochLimit,
    ChurnRate,
    EpochConflict,
}

pub fn admission_denied(reason: SigningKeyAdmissionReason) -> crate::error::DomainError {
    crate::error::DomainError::SigningKeyAdmissionDenied {
        reason,
        retry_after_seconds: None,
    }
}

/// RSA SPKI public parser with an explicit 8192-bit backend limit. rsa 0.9's
/// DecodePublicKey default silently caps at 4096. Legacy small valid keys remain
/// readable for auth; the NEW-material 2048 floor is enforced by admission only.
pub fn parse_signing_public_key(pem: &str) -> Result<rsa::RsaPublicKey, crate::error::DomainError> {
    let invalid = || crate::error::DomainError::InvalidSigningKeyMaterial;
    let (label, document) = rsa::pkcs8::Document::from_pem(pem).map_err(|_| invalid())?;
    if label != "PUBLIC KEY" {
        return Err(crate::error::DomainError::InvalidSigningKeyMaterial);
    }
    let spki = rsa::pkcs8::SubjectPublicKeyInfoRef::try_from(document.as_bytes())
        .map_err(|_| invalid())?;
    spki.algorithm
        .assert_algorithm_oid(rsa::pkcs1::ALGORITHM_OID)
        .map_err(|_| invalid())?;
    if spki.algorithm.parameters_any().map_err(|_| invalid())? != rsa::pkcs8::der::asn1::Null.into()
    {
        return Err(crate::error::DomainError::InvalidSigningKeyMaterial);
    }
    let der = spki
        .subject_public_key
        .as_bytes()
        .ok_or(crate::error::DomainError::InvalidSigningKeyMaterial)?;
    let key = rsa::pkcs1::RsaPublicKey::try_from(der).map_err(|_| invalid())?;
    if key.modulus.as_bytes().len() > 1024 || key.public_exponent.as_bytes().len() > 8 {
        return Err(invalid());
    }
    rsa::RsaPublicKey::new_with_max_size(
        rsa::BigUint::from_bytes_be(key.modulus.as_bytes()),
        rsa::BigUint::from_bytes_be(key.public_exponent.as_bytes()),
        8192,
    )
    .map_err(|_| invalid())
}

/// Immutable, validated numeric policy explicitly ratified by the user (0310).
/// No unbounded/legacy constructor or environment disabling admission exists.
#[derive(Clone, Debug)]
pub struct SigningKeyLifecyclePolicy {
    max_jwks_bytes: usize,
    platform_reserve_bytes: usize,
    max_reserved_jwk_bytes: usize,
    max_organization_epochs: usize,
    max_platform_epochs: usize,
    max_new_organization_epochs: usize,
    churn_window_seconds: i64,
}

#[derive(Clone)]
pub struct SigningKeyAdmissionHistory {
    pub organization_id: Option<Uuid>,
    pub admitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningKeyPublicationUsage {
    pub actual_bytes: usize,
    pub reserved_bytes: usize,
    pub organization_reserved_bytes: usize,
    pub platform_epochs: usize,
    pub organization_epochs: std::collections::BTreeMap<Uuid, usize>,
}

/// Non-secret publication report. An unmeasurable snapshot requires recovery,
/// not omission/eviction of keys or a global Active-authentication policy gate.
#[derive(Debug, Clone)]
pub struct SigningKeyLifecyclePreflight {
    pub snapshot_key_count: usize,
    pub usage: Option<SigningKeyPublicationUsage>,
    pub recovery_required: bool,
}

impl SigningKeyLifecyclePolicy {
    /// Bounded evidence suffices: the fourth newest admission determines the
    /// same release deadline even when conservative legacy history exceeds four.
    pub fn organization_churn_evidence_limit(&self) -> usize {
        self.max_new_organization_epochs
    }

    /// Shared TTL validation for root and pure snapshot checks.
    pub fn access_token_retention(
        &self,
        ttl: u64,
        as_of: DateTime<Utc>,
    ) -> Result<chrono::Duration, crate::error::DomainError> {
        let invalid = || crate::error::DomainError::InvalidSigningKeyMaterial;
        let retention = i64::try_from(ttl)
            .ok()
            .filter(|_| ttl != 0)
            .and_then(|ttl| ttl.checked_add(JWKS_RETIRE_SKEW_SECONDS))
            .and_then(chrono::Duration::try_seconds)
            .ok_or_else(invalid)?;
        as_of.checked_add_signed(retention).ok_or_else(invalid)?;
        as_of.checked_sub_signed(retention).ok_or_else(invalid)?;
        Ok(retention)
    }

    /// Constructor-only range check; snapshot evaluation uses its supplied DB clock.
    pub fn validate_access_token_ttl(&self, ttl: u64) -> Result<(), crate::error::DomainError> {
        self.access_token_retention(ttl, Utc::now()).map(|_| ())
    }

    /// Pure evaluator for publication reporting.
    /// An unknown usage is recovery, never a successful truncated publication.
    pub fn preflight_publication(
        &self,
        snapshot: &SigningKeyPublicationSnapshot,
        access_ttl_seconds: u64,
    ) -> Result<SigningKeyLifecyclePreflight, crate::error::DomainError> {
        self.validate()?;
        self.access_token_retention(access_ttl_seconds, snapshot.as_of)?;
        let usage = self
            .publication_usage(&snapshot.keys, access_ttl_seconds, snapshot.as_of)
            .ok();
        let recovery_required = self
            .check_admission(
                &snapshot.keys,
                &[],
                None,
                access_ttl_seconds,
                snapshot.as_of,
            )
            .is_err();
        Ok(SigningKeyLifecyclePreflight {
            snapshot_key_count: snapshot.keys.len(),
            usage,
            recovery_required,
        })
    }
    /// # Errors
    /// Fails if the ratified finite policy is inconsistent with the consumer bound.
    pub fn new() -> Result<Self, crate::error::DomainError> {
        let policy = Self {
            max_jwks_bytes: 786_432,
            platform_reserve_bytes: 65_536,
            max_reserved_jwk_bytes: 4096,
            max_organization_epochs: 8,
            max_platform_epochs: 16,
            max_new_organization_epochs: 4,
            churn_window_seconds: 3600,
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), crate::error::DomainError> {
        if self.max_jwks_bytes >= 1_048_576
            || self.max_jwks_bytes == 0
            || self.platform_reserve_bytes == 0
            || self.platform_reserve_bytes >= self.max_jwks_bytes
            || self.max_reserved_jwk_bytes == 0
            || self.max_organization_epochs == 0
            || self.max_platform_epochs == 0
            || self.max_new_organization_epochs == 0
            || self.churn_window_seconds <= 0
        {
            return Err(crate::error::DomainError::InvalidSigningKeyMaterial);
        }
        Ok(())
    }

    /// New rows only; no legacy Active authentication check is added.
    pub fn validate_new_key(&self, key: &SigningKey) -> Result<(), crate::error::DomainError> {
        use rsa::traits::PublicKeyParts;
        let invalid = || crate::error::DomainError::InvalidSigningKeyMaterial;
        let kid = Uuid::parse_str(&key.kid).map_err(|_| invalid())?;
        if key.algorithm != "RS256"
            || kid.get_version() != Some(uuid::Version::Random)
            || key.kid != kid.simple().to_string()
            || key.issuer.len() > 1024
            || url::Url::parse(&key.issuer).is_err()
            || (key.trust_scope == TrustScope::Platform) != key.organization_id.is_none()
        {
            return Err(invalid());
        }
        let public = parse_signing_public_key(&key.public_key)?;
        let exponent = public.e().to_bytes_be();
        if !(2048..=8192).contains(&public.n().bits()) || exponent.len() > 8 || exponent.is_empty()
        {
            return Err(invalid());
        }
        let e = exponent
            .iter()
            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte));
        if e < 3 || e % 2 == 0 {
            return Err(invalid());
        }
        // JWK conversion uses canonical minimal n/e base64url, never raw PEM size.
        if self.reserved_entry_bytes(key)? > self.max_reserved_jwk_bytes {
            return Err(invalid());
        }
        Ok(())
    }

    pub fn reserved_entry_bytes(
        &self,
        key: &SigningKey,
    ) -> Result<usize, crate::error::DomainError> {
        let mut maximum = 0;
        for status in [
            SigningKeyStatus::Pending,
            SigningKeyStatus::Active,
            SigningKeyStatus::Retiring,
        ] {
            let mut variant = key.clone();
            variant.status = status;
            let set = crate::entity::token::JwkSet::from_registry_keys_checked(&[variant])?;
            let size = serde_json::to_vec(&set.keys[0])
                .map_err(|_| crate::error::DomainError::InvalidSigningKeyMaterial)?
                .len();
            maximum = maximum.max(size);
        }
        Ok(maximum)
    }

    /// Exact non-secret preflight/accounting, independent of policy acceptance.
    pub fn publication_usage(
        &self,
        keys: &[SigningKey],
        ttl: u64,
        as_of: DateTime<Utc>,
    ) -> Result<SigningKeyPublicationUsage, crate::error::DomainError> {
        let keys = filter_jwks_publication_keys_at(keys.to_vec(), ttl, as_of);
        let actual_bytes = crate::entity::token::JwkSet::from_registry_keys_checked(&keys)?
            .compact_bytes()?
            .len();
        let mut usage = SigningKeyPublicationUsage {
            actual_bytes,
            reserved_bytes: crate::entity::token::JwkSet { keys: vec![] }
                .compact_bytes()?
                .len(),
            organization_reserved_bytes: 0,
            platform_epochs: 0,
            organization_epochs: std::collections::BTreeMap::new(),
        };
        for key in &keys {
            let cost = self
                .reserved_entry_bytes(key)?
                .checked_add(1)
                .ok_or_else(|| admission_denied(SigningKeyAdmissionReason::Capacity))?;
            usage.reserved_bytes = usage
                .reserved_bytes
                .checked_add(cost)
                .ok_or_else(|| admission_denied(SigningKeyAdmissionReason::Capacity))?;
            if let Some(org) = key.organization_id {
                usage.organization_reserved_bytes = usage
                    .organization_reserved_bytes
                    .checked_add(cost)
                    .ok_or_else(|| admission_denied(SigningKeyAdmissionReason::Capacity))?;
                *usage.organization_epochs.entry(org).or_default() += 1;
            } else {
                usage.platform_epochs += 1;
            }
        }
        Ok(usage)
    }

    /// Validate the simulated complete publication BEFORE any mutation. New kids
    /// charge history once, including later-revoked/expired rows; promotions do not.
    pub fn check_admission(
        &self,
        proposed: &[SigningKey],
        history: &[SigningKeyAdmissionHistory],
        new_key: Option<&SigningKey>,
        ttl: u64,
        as_of: DateTime<Utc>,
    ) -> Result<(), crate::error::DomainError> {
        if let Some(key) = new_key {
            self.validate_new_key(key)?;
        }
        let retention = self
            .access_token_retention(ttl, as_of)
            .map_err(|_| admission_denied(SigningKeyAdmissionReason::Capacity))?;
        if as_of.checked_add_signed(retention).is_none()
            || proposed.iter().any(|key| {
                key.status == SigningKeyStatus::Retiring
                    && (key.updated_at > as_of
                        || key.updated_at.checked_add_signed(retention).is_none())
            })
        {
            // Corrupt legacy future retirements must not free capacity now and
            // republish later. Recovery/revocation, not an auth-wide policy gate.
            return Err(admission_denied(SigningKeyAdmissionReason::Capacity));
        }
        let published = filter_jwks_publication_keys_at(proposed.to_vec(), ttl, as_of);
        // Invalid legacy publication is explicit recovery, not silent eviction.
        if published
            .iter()
            .any(|key| self.validate_new_key(key).is_err())
        {
            return Err(admission_denied(SigningKeyAdmissionReason::Capacity));
        }
        let usage = self
            .publication_usage(proposed, ttl, as_of)
            .map_err(|_| admission_denied(SigningKeyAdmissionReason::Capacity))?;
        if usage.actual_bytes > usage.reserved_bytes
            || usage.reserved_bytes > self.max_jwks_bytes
            || usage.organization_reserved_bytes > self.max_jwks_bytes - self.platform_reserve_bytes
        {
            return Err(admission_denied(SigningKeyAdmissionReason::Capacity));
        }
        if usage.platform_epochs > self.max_platform_epochs {
            return Err(admission_denied(SigningKeyAdmissionReason::Capacity));
        }
        if usage
            .organization_epochs
            .values()
            .any(|count| *count > self.max_organization_epochs)
        {
            return Err(admission_denied(
                SigningKeyAdmissionReason::TenantEpochLimit,
            ));
        }
        if let Some(org) = new_key.and_then(|key| key.organization_id) {
            let cutoff = as_of
                .checked_sub_signed(chrono::Duration::seconds(self.churn_window_seconds))
                .ok_or_else(|| admission_denied(SigningKeyAdmissionReason::Capacity))?;
            let mut admissions: Vec<_> = history
                .iter()
                .filter(|entry| entry.organization_id == Some(org) && entry.admitted_at > cutoff)
                .map(|entry| entry.admitted_at)
                .collect();
            admissions.sort_unstable();
            if admissions.len() >= self.max_new_organization_epochs {
                let deadline = admissions[admissions.len() - self.max_new_organization_epochs]
                    .checked_add_signed(chrono::Duration::seconds(self.churn_window_seconds));
                let retry_after_seconds = deadline.and_then(|deadline| {
                    let seconds = (deadline - as_of).num_seconds();
                    let ceil =
                        seconds + i64::from(deadline > as_of + chrono::Duration::seconds(seconds));
                    u32::try_from(ceil.max(1)).ok()
                });
                return Err(crate::error::DomainError::SigningKeyAdmissionDenied {
                    reason: SigningKeyAdmissionReason::ChurnRate,
                    retry_after_seconds,
                });
            }
        }
        Ok(())
    }
}

/// Configure equality: all effective bindings, normalized n/e; candidate identity
/// and timestamps are deliberately not part of this no-op comparison.
pub fn same_effective_signing_binding(a: &SigningKey, b: &SigningKey) -> bool {
    if a.algorithm != b.algorithm
        || a.issuer != b.issuer
        || a.trust_scope != b.trust_scope
        || a.organization_id != b.organization_id
        || a.provider_type != b.provider_type
        || a.provider_key_ref != b.provider_key_ref
        || a.credential_ref != b.credential_ref
    {
        return false;
    }
    match (
        crate::entity::token::Jwk::from_rsa_pem(&a.public_key, &a.kid, &a.issuer),
        crate::entity::token::Jwk::from_rsa_pem(&b.public_key, &b.kid, &b.issuer),
    ) {
        (Ok(a), Ok(b)) => a.n == b.n && a.e == b.e,
        _ => false,
    }
}

/// Transit key name reserved for apparatus Cosign — never reuse for org JWT signing.
pub const FORBIDDEN_TRANSIT_KEY_NAME: &str = "apparatus-p4-cosign";

/// Transit names are a single opaque ASCII segment, never paths or URL escapes.
///
/// # Errors
/// Rejects empty/oversized names, separators, escapes and reserved key names.
pub fn require_transit_key_name(name: &str) -> Result<(), crate::error::DomainError> {
    if name == FORBIDDEN_TRANSIT_KEY_NAME
        || name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(crate::error::DomainError::AuthorizationError(
            "invalid or forbidden Transit key name (apparatus-p4-cosign is forbidden)".into(),
        ));
    }
    Ok(())
}

/// Organization key and credential references must remain in that organization's
/// namespace. A platform/default credential must never be a fallback here.
///
/// # Errors
/// Rejects missing credentials, malformed segments or another organization's refs.
pub fn require_org_transit_binding(
    org: Uuid,
    key_name: &str,
    credential_ref: Option<&str>,
) -> Result<(), crate::error::DomainError> {
    require_transit_key_name(key_name)?;
    let prefix = format!("org-{org}-");
    let credential = credential_ref.ok_or_else(|| {
        crate::error::DomainError::AuthorizationError(
            "organization Transit credential required".into(),
        )
    })?;
    require_transit_key_name(credential)?;
    if !key_name.starts_with(&prefix)
        || key_name.len() == prefix.len()
        || !credential.starts_with(&prefix)
        || credential.len() == prefix.len()
    {
        return Err(crate::error::DomainError::AuthorizationError(
            "Transit key and credential must belong to the organization".into(),
        ));
    }
    Ok(())
}

/// Access-token typ header value (ADR-0304 / rustycog-http).
pub const ACCESS_TOKEN_TYP: &str = "aiforall-access+jwt";

/// Clock skew added to access-token TTL when retiring keys leave the JWKS window.
pub const JWKS_RETIRE_SKEW_SECONDS: i64 = 60;

/// Opaque public `kid` — UUID v4 simple hex, never `org-` / ARN / vault path / slug.
#[must_use]
pub fn opaque_kid() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Drop retiring keys whose verification window (`updated_at` + access TTL + skew) has elapsed.
#[must_use]
pub fn filter_jwks_publication_keys(
    keys: Vec<SigningKey>,
    expiration_seconds: u64,
) -> Vec<SigningKey> {
    filter_jwks_publication_keys_at(keys, expiration_seconds, Utc::now())
}

pub fn filter_jwks_publication_keys_at(
    keys: Vec<SigningKey>,
    expiration_seconds: u64,
    as_of: DateTime<Utc>,
) -> Vec<SigningKey> {
    let retain = i64::try_from(expiration_seconds)
        .ok()
        .and_then(|ttl| ttl.checked_add(JWKS_RETIRE_SKEW_SECONDS))
        .and_then(chrono::Duration::try_seconds);
    keys.into_iter()
        .filter(|key| match key.status {
            SigningKeyStatus::Pending | SigningKeyStatus::Active => true,
            SigningKeyStatus::Retiring => {
                key.updated_at <= as_of
                    && key
                        .updated_at
                        .checked_add_signed(match retain {
                            Some(retain) => retain,
                            None => return false,
                        })
                        .is_some_and(|expiry| expiry > as_of)
            }
            SigningKeyStatus::Revoked => false,
        })
        .collect()
}

#[cfg(test)]
mod admission_tests {
    #[test]
    fn public_parser_accepts_lf_and_crlf_with_identical_key_material() {
        use rsa::traits::PublicKeyParts;
        let lf = include_str!("../../../config/keys/test-platform.pub").replace("\r\n", "\n");
        let crlf = lf.replace('\n', "\r\n");
        let a = super::parse_signing_public_key(&lf).expect("LF SPKI");
        let b = super::parse_signing_public_key(&crlf).expect("CRLF SPKI");
        assert_eq!(a.n(), b.n());
        assert_eq!(a.e(), b.e());
        assert!(super::parse_signing_public_key("invalid\r\nPUBLIC KEY").is_err());
    }

    use super::*;
    use crate::error::DomainError;
    use base64::Engine;
    use chrono::TimeZone;
    use rsa::pkcs8::{EncodePublicKey, LineEnding};

    fn clock() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 4, 12, 0, 0).unwrap()
    }
    fn key(org: Option<Uuid>) -> SigningKey {
        SigningKey {
            id: Uuid::new_v4(),
            kid: opaque_kid(),
            algorithm: "RS256".into(),
            trust_scope: if org.is_some() {
                TrustScope::Organization
            } else {
                TrustScope::Platform
            },
            issuer: "https://iam.example.test/iam/orgs/é".into(),
            provider_type: SigningProviderType::RemoteHttp,
            provider_key_ref: "key-ref".into(),
            credential_ref: Some("credential-ref".into()),
            public_key: include_str!("../../../config/keys/test-platform.pub").into(),
            status: SigningKeyStatus::Active,
            organization_id: org,
            created_at: clock(),
            updated_at: clock(),
        }
    }
    fn reason(error: DomainError) -> SigningKeyAdmissionReason {
        match error {
            DomainError::SigningKeyAdmissionDenied { reason, .. } => reason,
            _ => panic!("expected typed admission outcome"),
        }
    }
    fn public_bits(bits: usize) -> String {
        let n = (rsa::BigUint::from(1u8) << (bits - 1)) + rsa::BigUint::from(3u8);
        rsa::RsaPublicKey::new_with_max_size(n, rsa::BigUint::from(65537u32), 16_384)
            .unwrap()
            .to_public_key_pem(LineEnding::LF)
            .unwrap()
    }

    #[test]
    fn preflight_shared_evaluator_distinguishes_empty_normal_unknown_and_overbudget() {
        let policy = SigningKeyLifecyclePolicy::new().unwrap();
        let mut snapshot = SigningKeyPublicationSnapshot {
            keys: vec![],
            as_of: clock(),
        };
        let report = policy.preflight_publication(&snapshot, 1800).unwrap();
        assert!(!report.recovery_required);
        assert_eq!(report.snapshot_key_count, 0);
        assert_eq!(report.usage.unwrap().actual_bytes, b"{\"keys\":[]}".len());
        snapshot.keys.push(key(None));
        assert!(
            !policy
                .preflight_publication(&snapshot, 1800)
                .unwrap()
                .recovery_required
        );
        snapshot.keys[0].public_key = "invalid legacy public material".into();
        let report = policy.preflight_publication(&snapshot, 1800).unwrap();
        assert!(report.recovery_required);
        assert!(report.usage.is_none());
        snapshot.keys = (0..1600).map(|_| key(Some(Uuid::new_v4()))).collect();
        let report = policy.preflight_publication(&snapshot, 1800).unwrap();
        assert!(report.recovery_required);
        assert!(report.usage.unwrap().reserved_bytes > 786432);
        assert_eq!(report.snapshot_key_count, 1600);
        assert!(policy.preflight_publication(&snapshot, 0).is_err());
        assert!(policy.preflight_publication(&snapshot, u64::MAX).is_err());
    }

    #[test]
    fn preflight_uses_actual_non900_ttl_and_bounded_churn_evidence_is_equivalent() {
        let policy = SigningKeyLifecyclePolicy::new().unwrap();
        let org = Uuid::new_v4();
        let mut retiring = key(Some(org));
        retiring.status = SigningKeyStatus::Retiring;
        retiring.updated_at = clock() - chrono::Duration::seconds(1000);
        retiring.public_key = "invalid retained legacy material".into();
        let snapshot = SigningKeyPublicationSnapshot {
            keys: vec![retiring],
            as_of: clock(),
        };
        assert!(
            !policy
                .preflight_publication(&snapshot, 900)
                .unwrap()
                .recovery_required
        );
        assert!(
            policy
                .preflight_publication(&snapshot, 1800)
                .unwrap()
                .recovery_required
        );
        let history: Vec<_> = (1..=100)
            .map(|age| SigningKeyAdmissionHistory {
                organization_id: Some(org),
                admitted_at: clock() - chrono::Duration::seconds(age),
            })
            .collect();
        let candidate = key(Some(org));
        let full = policy
            .check_admission(
                &[candidate.clone()],
                &history,
                Some(&candidate),
                1800,
                clock(),
            )
            .unwrap_err();
        let bounded = policy
            .check_admission(
                &[candidate.clone()],
                &history[..policy.organization_churn_evidence_limit()],
                Some(&candidate),
                1800,
                clock(),
            )
            .unwrap_err();
        assert_eq!(format!("{full:?}"), format!("{bounded:?}"));
    }

    #[test]
    fn ratified_policy_is_finite_and_exact() {
        let p = SigningKeyLifecyclePolicy::new().unwrap();
        assert_eq!(
            (
                p.max_jwks_bytes,
                p.platform_reserve_bytes,
                p.max_reserved_jwk_bytes
            ),
            (786432, 65536, 4096)
        );
        assert_eq!(
            (
                p.max_organization_epochs,
                p.max_platform_epochs,
                p.max_new_organization_epochs,
                p.churn_window_seconds
            ),
            (8, 16, 4, 3600)
        );
    }

    #[test]
    fn compact_unicode_escaping_reservation_covers_every_reachable_status_and_envelope() {
        let p = SigningKeyLifecyclePolicy::new().unwrap();
        let mut k = key(Some(Uuid::new_v4()));
        k.issuer = "https://iam.example.test/é?quote=\"&backslash=\\".into();
        let b = p.reserved_entry_bytes(&k).unwrap();
        for status in [
            SigningKeyStatus::Pending,
            SigningKeyStatus::Active,
            SigningKeyStatus::Retiring,
        ] {
            k.status = status;
            let usage = p.publication_usage(&[k.clone()], 900, clock()).unwrap();
            let set =
                crate::entity::token::JwkSet::from_registry_keys_checked(&[k.clone()]).unwrap();
            assert_eq!(usage.actual_bytes, serde_json::to_vec(&set).unwrap().len());
            assert_eq!(usage.reserved_bytes, b + 1 + b"{\"keys\":[]}".len());
            assert_eq!(usage.organization_reserved_bytes, b + 1);
            assert!(usage.actual_bytes <= usage.reserved_bytes);
            let jwk = &set.keys[0];
            for value in [&jwk.n, &jwk.e] {
                assert!(!value.contains('='));
                let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(value)
                    .unwrap();
                assert_ne!(bytes[0], 0);
            }
        }
    }

    #[test]
    fn new_material_bounds_2048_through_8192_and_entry_bytes_are_authoritative() {
        let p = SigningKeyLifecyclePolicy::new().unwrap();
        for bits in [1024, 2048, 4096, 8192, 8193] {
            let mut k = key(None);
            k.public_key = public_bits(bits);
            assert_eq!(
                p.validate_new_key(&k).is_ok(),
                (2048..=8192).contains(&bits)
            );
        }
        let mut k = key(None);
        k.kid = "not-an-opaque-v4-kid".into();
        assert!(matches!(
            p.validate_new_key(&k),
            Err(DomainError::InvalidSigningKeyMaterial)
        ));
        k = key(None);
        k.issuer = "not-an-absolute-url".into();
        assert!(p.validate_new_key(&k).is_err());
        k.issuer = format!("https://iam.example.test/{}", "é".repeat(512));
        assert!(k.issuer.len() > 1024);
        assert!(p.validate_new_key(&k).is_err());
        k.issuer = format!("https://iam.example.test/{}", "\u{1}".repeat(900));
        assert!(k.issuer.len() <= 1024);
        assert!(url::Url::parse(&k.issuer).is_ok());
        assert!(p.reserved_entry_bytes(&k).unwrap() > 4096);
        assert!(p.validate_new_key(&k).is_err());
        let public = rsa::RsaPublicKey::new_unchecked(
            (rsa::BigUint::from(1u8) << 2047) + rsa::BigUint::from(3u8),
            (rsa::BigUint::from(1u8) << 64) + rsa::BigUint::from(1u8),
        );
        k = key(None);
        k.public_key = public.to_public_key_pem(LineEnding::LF).unwrap();
        assert!(p.validate_new_key(&k).is_err());
    }

    #[test]
    fn complete_binding_idempotence_normalizes_public_but_not_credentials_or_provider() {
        let original = key(Some(Uuid::new_v4()));
        let mut candidate = original.clone();
        candidate.id = Uuid::new_v4();
        candidate.kid = opaque_kid();
        candidate.created_at += chrono::Duration::hours(1);
        candidate.updated_at += chrono::Duration::hours(1);
        // The checked-in fixture is already CRLF; do not manufacture CRCRLF.
        candidate.public_key = candidate.public_key.replace("\r\n", "\n");
        assert!(same_effective_signing_binding(&original, &candidate));
        candidate.public_key = candidate.public_key.replace('\n', "\r\n");
        assert!(same_effective_signing_binding(&original, &candidate));
        for mutant in 0..7 {
            let mut changed = candidate.clone();
            match mutant {
                0 => changed.credential_ref = Some("different-credential".into()),
                1 => changed.provider_key_ref.push_str("different"),
                2 => changed.provider_type = SigningProviderType::PemFile,
                3 => changed.issuer.push_str("different"),
                4 => changed.algorithm = "HS256".into(),
                5 => changed.organization_id = Some(Uuid::new_v4()),
                _ => changed.public_key = public_bits(4096),
            }
            assert!(!same_effective_signing_binding(&original, &changed));
        }
    }

    #[test]
    fn sliding_churn_counts_revoked_history_and_promotion_is_not_a_second_charge() {
        let p = SigningKeyLifecyclePolicy::new().unwrap();
        let org = Uuid::new_v4();
        let k = key(Some(org));
        let mut history = vec![
            SigningKeyAdmissionHistory {
                organization_id: Some(org),
                admitted_at: clock() - chrono::Duration::seconds(3599)
            };
            4
        ];
        let err = p
            .check_admission(&[k.clone()], &history, Some(&k), 900, clock())
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::SigningKeyAdmissionDenied {
                reason: SigningKeyAdmissionReason::ChurnRate,
                retry_after_seconds: Some(1)
            }
        ));
        assert!(p
            .check_admission(&[k.clone()], &history, None, 900, clock())
            .is_ok());
        for entry in &mut history {
            entry.admitted_at = clock() - chrono::Duration::seconds(3600);
        }
        assert!(p
            .check_admission(&[k.clone()], &history, Some(&k), 900, clock())
            .is_ok());
        for entry in &mut history {
            entry.admitted_at += chrono::Duration::nanoseconds(1);
        }
        assert_eq!(
            reason(
                p.check_admission(&[k.clone()], &history, Some(&k), 900, clock())
                    .unwrap_err()
            ),
            SigningKeyAdmissionReason::ChurnRate
        );
    }

    #[test]
    fn organization_and_platform_caps_and_reserve_reject_only_admission() {
        let p = SigningKeyLifecyclePolicy::new().unwrap();
        let org = Uuid::new_v4();
        let orgs: Vec<_> = (0..9).map(|_| key(Some(org))).collect();
        assert!(p
            .check_admission(&orgs[..8], &[], None, 900, clock())
            .is_ok());
        assert_eq!(
            reason(
                p.check_admission(&orgs, &[], None, 900, clock())
                    .unwrap_err()
            ),
            SigningKeyAdmissionReason::TenantEpochLimit
        );
        let platform: Vec<_> = (0..17).map(|_| key(None)).collect();
        assert!(p
            .check_admission(&platform[..16], &[], None, 900, clock())
            .is_ok());
        assert_eq!(
            reason(
                p.check_admission(&platform, &[], None, 900, clock())
                    .unwrap_err()
            ),
            SigningKeyAdmissionReason::Capacity
        );
        let org_key = key(Some(org));
        let platform_key = key(None);
        let org_cost = p.reserved_entry_bytes(&org_key).unwrap() + 1;
        let platform_cost = p.reserved_entry_bytes(&platform_key).unwrap() + 1;
        let mut finite = p.clone();
        finite.max_jwks_bytes = 11 + 2 * org_cost + platform_cost;
        finite.platform_reserve_bytes = 11 + org_cost + platform_cost;
        finite.validate().unwrap();
        assert!(finite
            .check_admission(&[org_key.clone(), platform_key], &[], None, 900, clock())
            .is_ok());
        let another = key(Some(Uuid::new_v4()));
        assert_eq!(
            reason(
                finite
                    .check_admission(&[org_key, another], &[], None, 900, clock())
                    .unwrap_err()
            ),
            SigningKeyAdmissionReason::Capacity
        );
    }

    #[test]
    fn retirement_uses_supplied_db_clock_exact_expiry_and_never_reactivates_future_legacy() {
        let p = SigningKeyLifecyclePolicy::new().unwrap();
        let mut k = key(None);
        k.status = SigningKeyStatus::Retiring;
        k.updated_at = clock() - chrono::Duration::seconds(960);
        assert!(filter_jwks_publication_keys_at(vec![k.clone()], 900, clock()).is_empty());
        assert_eq!(
            filter_jwks_publication_keys_at(
                vec![k.clone()],
                900,
                clock() - chrono::Duration::nanoseconds(1)
            )
            .len(),
            1
        );
        k.updated_at = clock() + chrono::Duration::seconds(1);
        assert!(filter_jwks_publication_keys_at(vec![k.clone()], 900, clock()).is_empty());
        assert_eq!(
            reason(
                p.check_admission(&[k], &[], None, 900, clock())
                    .unwrap_err()
            ),
            SigningKeyAdmissionReason::Capacity
        );
        let active = key(None);
        assert_eq!(
            filter_jwks_publication_keys_at(vec![active], u64::MAX, clock()).len(),
            1
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transit_names_reject_paths_url_escapes_and_foreign_credentials() {
        for value in [
            "",
            "..",
            "a/b",
            "a\\b",
            "%2f",
            "a?x",
            "a#x",
            "a.b",
            "é",
            FORBIDDEN_TRANSIT_KEY_NAME,
        ] {
            assert!(require_transit_key_name(value).is_err(), "{value}");
        }
        assert!(require_transit_key_name(&"a".repeat(129)).is_err());
        let org = Uuid::new_v4();
        let other = Uuid::new_v4();
        let key = format!("org-{org}-jwt");
        let credential = format!("org-{org}-token");
        assert!(require_org_transit_binding(org, &key, Some(&credential)).is_ok());
        assert!(require_org_transit_binding(org, &key, None).is_err());
        for credential in ["", "../token", "token/path", "%2f", "platform-token"] {
            assert!(require_org_transit_binding(org, &key, Some(credential)).is_err());
        }
        assert!(require_org_transit_binding(org, &key, Some(&format!("org-{org}-"))).is_err());
        assert!(require_org_transit_binding(org, &key, Some("openbao-token")).is_err());
        assert!(
            require_org_transit_binding(org, &key, Some(&format!("org-{other}-token"))).is_err()
        );
        assert!(
            require_org_transit_binding(org, &format!("org-{other}-jwt"), Some(&credential))
                .is_err()
        );
    }

    #[test]
    fn pending_never_signs_and_only_bounded_retiring_is_published() {
        assert!(!SigningKeyStatus::Pending.can_sign());
        assert!(!SigningKeyStatus::Retiring.can_sign());
        assert!(SigningKeyStatus::Active.can_sign());
        let now = Utc::now();
        let mut key = SigningKey {
            id: Uuid::new_v4(),
            kid: "kid".into(),
            algorithm: "RS256".into(),
            trust_scope: TrustScope::Platform,
            issuer: "https://iam.example".into(),
            provider_type: SigningProviderType::PemFile,
            provider_key_ref: "platform.pem".into(),
            credential_ref: None,
            public_key: String::new(),
            status: SigningKeyStatus::Retiring,
            organization_id: None,
            created_at: now,
            updated_at: now - chrono::Duration::seconds(961),
        };
        assert!(filter_jwks_publication_keys(vec![key.clone()], 900).is_empty());
        key.updated_at = now - chrono::Duration::seconds(10);
        assert_eq!(
            filter_jwks_publication_keys(vec![key.clone()], 900).len(),
            1
        );
        key.status = SigningKeyStatus::Revoked;
        assert!(filter_jwks_publication_keys(vec![key], 900).is_empty());
    }
}
