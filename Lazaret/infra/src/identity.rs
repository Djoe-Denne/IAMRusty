//! In-memory enrollment, platform-internal CA, dedicated session signer.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazaret_application::IdentityService;
use lazaret_configuration::IdentityConfig;
use lazaret_domain::{
    BindingGrantSnapshotPort, CertificateAuthority, EnrollmentStore, IdentityError,
    IssuedCertificate, SessionClaims, SessionTokenSigner, WorkloadIdentity, DEFAULT_CERT_TTL_HOURS,
    DEFAULT_SESSION_TTL_MINUTES, PLATFORM_INTERNAL_CA_PRODUCT, SESSION_AUDIENCE, SESSION_ISSUER,
};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CertificateSigningRequestParams,
    DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose, SanType,
};
use sea_orm::DatabaseConnection;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// Maximum distinct bindings remembered by the in-process registry.
const MAX_ENROLLMENTS: usize = 10_000;

/// Build the identity service from `[identity]` config, a live snapshot port, and Lazaret DB.
///
/// Production enrollments use Postgres (`apparatus_enrollments`). Empty signing PEM →
/// ephemeral Ed25519 at boot. Never uses `[auth.jwt].hs256_secret`.
///
/// # Errors
///
/// Returns [`IdentityError`] if the CA or session signer cannot be constructed.
pub fn build_identity_service(
    config: &IdentityConfig,
    snapshots: Arc<dyn BindingGrantSnapshotPort>,
    db: DatabaseConnection,
) -> Result<Arc<IdentityService>, IdentityError> {
    let ca = Arc::new(PlatformInternalCa::new()?);
    let signer = Arc::new(DedicatedSessionSigner::from_config(config)?);
    let enrollments = Arc::new(crate::enrollment_postgres::PostgresEnrollmentRegistry::new(
        db,
    ));
    let session_ttl = if config.session_ttl_minutes == 0 {
        DEFAULT_SESSION_TTL_MINUTES
    } else {
        config.session_ttl_minutes
    };
    let cert_ttl = if config.cert_ttl_hours == 0 {
        DEFAULT_CERT_TTL_HOURS
    } else {
        config.cert_ttl_hours
    };
    Ok(Arc::new(IdentityService::new(
        ca,
        signer,
        enrollments,
        snapshots,
        session_ttl,
        cert_ttl,
    )))
}

/// Mutex-backed fingerprint → identity map. No SQL table.
pub struct InMemoryEnrollmentRegistry {
    inner: Mutex<HashMap<String, WorkloadIdentity>>,
}

impl InMemoryEnrollmentRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryEnrollmentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EnrollmentStore for InMemoryEnrollmentRegistry {
    async fn put(
        &self,
        fingerprint: String,
        identity: WorkloadIdentity,
    ) -> Result<(), IdentityError> {
        let mut map = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let existing_fp = map
            .iter()
            .find_map(|(fp, enrolled)| (enrolled.binding == identity.binding).then(|| fp.clone()));
        match existing_fp.as_deref() {
            Some(existing) if existing == fingerprint => return Ok(()),
            Some(_) => return Err(IdentityError::BindingAlreadyEnrolled),
            None => {}
        }
        if map.len() >= MAX_ENROLLMENTS {
            return Err(IdentityError::RegistryFull);
        }
        map.insert(fingerprint, identity);
        Ok(())
    }

    async fn get(&self, fingerprint: &str) -> Option<WorkloadIdentity> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(fingerprint)
            .cloned()
    }

    async fn binding_enrolled(&self, binding: Uuid) -> Result<bool, IdentityError> {
        let enrolled = self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .any(|enrolled| enrolled.binding == binding);
        Ok(enrolled)
    }

    async fn revoke_binding(&self, binding: Uuid) -> Result<(), IdentityError> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|_, enrolled| enrolled.binding != binding);
        Ok(())
    }
}

/// Software CA named [`PLATFORM_INTERNAL_CA_PRODUCT`].
pub struct PlatformInternalCa {
    cert: Certificate,
    key: KeyPair,
}

impl PlatformInternalCa {
    /// Generate an ephemeral platform CA keypair in process.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::CaFailure`] if rcgen cannot build the CA.
    pub fn new() -> Result<Self, IdentityError> {
        let key = KeyPair::generate().map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        let mut params = CertificateParams::new(Vec::<String>::new())
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, PLATFORM_INTERNAL_CA_PRODUCT);
        params.distinguished_name = dn;
        let ttl_hours =
            i64::try_from(DEFAULT_CERT_TTL_HOURS).map_err(|_| IdentityError::InvalidIdentity)?;
        params.not_before = OffsetDateTime::now_utc();
        params.not_after = params
            .not_before
            .checked_add(Duration::hours(ttl_hours))
            .ok_or(IdentityError::InvalidIdentity)?;
        let cert = params
            .self_signed(&key)
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        Ok(Self { cert, key })
    }

    /// PEM of the CA certificate only (no private key).
    #[must_use]
    pub fn cert_pem(&self) -> String {
        self.cert.pem()
    }
}

impl CertificateAuthority for PlatformInternalCa {
    fn sign_csr(
        &self,
        csr_pem: &str,
        ttl_hours: u64,
        binding: Uuid,
    ) -> Result<IssuedCertificate, IdentityError> {
        let mut csr = CertificateSigningRequestParams::from_pem(csr_pem)
            .map_err(|e| IdentityError::InvalidCsr(e.to_string()))?;
        if matches!(csr.params.is_ca, IsCa::Ca(_)) {
            return Err(IdentityError::InvalidCsr(
                "csr must not request a ca certificate".to_owned(),
            ));
        }
        impose_client_identity(&mut csr.params, binding)?;
        let hours = i64::try_from(ttl_hours).map_err(|_| IdentityError::InvalidIdentity)?;
        let now = OffsetDateTime::now_utc();
        csr.params.not_before = now;
        csr.params.not_after = now
            .checked_add(Duration::hours(hours))
            .ok_or(IdentityError::InvalidIdentity)?;
        let cert = csr
            .signed_by(&self.cert, &self.key)
            .map_err(|e| IdentityError::InvalidCsr(e.to_string()))?;
        Ok(IssuedCertificate {
            pem: cert.pem(),
            der: cert.der().as_ref().to_vec(),
        })
    }
}

fn impose_client_identity(
    params: &mut CertificateParams,
    binding: Uuid,
) -> Result<(), IdentityError> {
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let id = binding.to_string();
    let ia5 = rcgen::Ia5String::try_from(id.as_str())
        .map_err(|e| IdentityError::InvalidCsr(e.to_string()))?;
    params.subject_alt_names = vec![SanType::DnsName(ia5.clone()), SanType::URI(ia5)];
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, id);
    params.distinguished_name = dn;
    Ok(())
}

enum SessionAlg {
    EdDsa {
        encoding: EncodingKey,
        decoding: DecodingKey,
    },
    DedicatedHs256 {
        secret: Vec<u8>,
    },
}

/// Session signer with a dedicated key (`EdDSA` preferred; `HS256` only if `Ed25519` PEM fails).
pub struct DedicatedSessionSigner {
    alg: SessionAlg,
}

impl DedicatedSessionSigner {
    /// Load configured Ed25519 PEM or generate ephemeral material.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::SigningMaterial`] if no usable dedicated key can be built.
    pub fn from_config(config: &IdentityConfig) -> Result<Self, IdentityError> {
        if let Some(pem) = config
            .session_signing_key_pem
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if let Ok(signer) = Self::from_ed25519_private_pem(pem) {
                return Ok(signer);
            }
            tracing::warn!(
                "identity.session_signing_key_pem is not usable Ed25519; generating ephemeral key"
            );
        }
        if let Ok(signer) = Self::generate_ephemeral_ed25519() {
            return Ok(signer);
        }
        tracing::warn!("Ed25519 session PEM unavailable; using dedicated HS256 (not IAM secret)");
        Self::generate_dedicated_hs256()
    }

    fn from_ed25519_private_pem(private_pem: &str) -> Result<Self, IdentityError> {
        let key_pair = KeyPair::from_pem(private_pem)
            .map_err(|e| IdentityError::SigningMaterial(e.to_string()))?;
        Self::from_ed25519_key_pair(&key_pair)
    }

    fn generate_ephemeral_ed25519() -> Result<Self, IdentityError> {
        let key_pair = KeyPair::generate_for(&rcgen::PKCS_ED25519)
            .map_err(|e| IdentityError::SigningMaterial(e.to_string()))?;
        Self::from_ed25519_key_pair(&key_pair)
    }

    fn from_ed25519_key_pair(key_pair: &KeyPair) -> Result<Self, IdentityError> {
        let private_pem = key_pair.serialize_pem();
        let public_pem = key_pair.public_key_pem();
        let encoding = EncodingKey::from_ed_pem(private_pem.as_bytes())
            .map_err(|e| IdentityError::SigningMaterial(e.to_string()))?;
        let decoding = DecodingKey::from_ed_pem(public_pem.as_bytes())
            .map_err(|e| IdentityError::SigningMaterial(e.to_string()))?;
        let signer = Self {
            alg: SessionAlg::EdDsa { encoding, decoding },
        };
        signer.probe_roundtrip()?;
        Ok(signer)
    }

    fn generate_dedicated_hs256() -> Result<Self, IdentityError> {
        let mut secret = vec![0_u8; 32];
        fill_random(&mut secret)?;
        let signer = Self {
            alg: SessionAlg::DedicatedHs256 { secret },
        };
        signer.probe_roundtrip()?;
        Ok(signer)
    }

    fn probe_roundtrip(&self) -> Result<(), IdentityError> {
        let now = chrono::Utc::now().timestamp();
        let claims = SessionClaims {
            instance: uuid::Uuid::nil(),
            binding: uuid::Uuid::nil(),
            release: "probe".to_owned(),
            generation: 0,
            grant_revision: 0,
            aud: SESSION_AUDIENCE.to_owned(),
            iss: SESSION_ISSUER.to_owned(),
            exp: now.saturating_add(60),
            iat: now,
        };
        let token = SessionTokenSigner::sign(self, &claims)?;
        let verified = SessionTokenSigner::verify(self, &token)?;
        if verified.iss != SESSION_ISSUER || verified.aud != SESSION_AUDIENCE {
            return Err(IdentityError::SigningMaterial(
                "session round-trip issuer/audience mismatch".to_owned(),
            ));
        }
        Ok(())
    }
}

impl SessionTokenSigner for DedicatedSessionSigner {
    fn sign(&self, claims: &SessionClaims) -> Result<String, IdentityError> {
        match &self.alg {
            SessionAlg::EdDsa { encoding, .. } => {
                encode(&Header::new(Algorithm::EdDSA), claims, encoding)
                    .map_err(|e| IdentityError::SigningMaterial(e.to_string()))
            }
            SessionAlg::DedicatedHs256 { secret } => encode(
                &Header::new(Algorithm::HS256),
                claims,
                &EncodingKey::from_secret(secret),
            )
            .map_err(|e| IdentityError::SigningMaterial(e.to_string())),
        }
    }

    fn verify(&self, token: &str) -> Result<SessionClaims, IdentityError> {
        match &self.alg {
            SessionAlg::EdDsa { decoding, .. } => decode_session(token, decoding, Algorithm::EdDSA),
            SessionAlg::DedicatedHs256 { secret } => {
                let decoding = DecodingKey::from_secret(secret);
                decode_session(token, &decoding, Algorithm::HS256)
            }
        }
    }
}

fn decode_session(
    token: &str,
    decoding: &DecodingKey,
    algorithm: Algorithm,
) -> Result<SessionClaims, IdentityError> {
    let mut validation = Validation::new(algorithm);
    validation.set_audience(&[SESSION_AUDIENCE]);
    validation.set_issuer(&[SESSION_ISSUER]);
    validation.set_required_spec_claims(&["exp"]);
    let data = decode::<SessionClaims>(token, decoding, &validation)
        .map_err(|_| IdentityError::InvalidSession)?;
    Ok(data.claims)
}

fn fill_random(buf: &mut [u8]) -> Result<(), IdentityError> {
    use rand::RngCore;
    rand::thread_rng()
        .try_fill_bytes(buf)
        .map_err(|e| IdentityError::SigningMaterial(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_ca_cert_pem_excludes_private_key() {
        let pem = PlatformInternalCa::new().expect("platform ca").cert_pem();
        assert!(
            pem.contains("BEGIN CERTIFICATE"),
            "cert_pem must include a certificate"
        );
        assert!(
            !pem.contains("PRIVATE KEY"),
            "cert_pem must not include the CA private key"
        );
    }
}
