//! In-memory enrollment, platform-internal CA, dedicated session signer.

use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazaret_application::IdentityService;
use lazaret_configuration::IdentityConfig;
use lazaret_domain::{
    BindingGrantSnapshotPort, CertificateAuthority, EnrollmentStore, IdentityError,
    IssuedCertificate, SessionClaims, SessionTokenSigner, WorkloadIdentity, DEFAULT_CA_TTL_HOURS,
    DEFAULT_CERT_TTL_HOURS, DEFAULT_SESSION_TTL_MINUTES, PLATFORM_INTERNAL_CA_PRODUCT,
    SESSION_AUDIENCE, SESSION_ISSUER,
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
) -> Result<(Arc<IdentityService>, Arc<PlatformInternalCa>), IdentityError> {
    let ca = Arc::new(PlatformInternalCa::from_identity_config(config)?);
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
    Ok((
        Arc::new(IdentityService::new(
            ca.clone(),
            signer,
            enrollments,
            snapshots,
            session_ttl,
            cert_ttl,
        )),
        ca,
    ))
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
    /// In-memory issuer used only for `signed_by` / CSR signing.
    cert: Certificate,
    key: KeyPair,
    /// Original trust-anchor PEM (certificate only). Never the reconstructed issuer DER.
    trust_anchor_pem: String,
}

impl PlatformInternalCa {
    /// Generate an ephemeral platform CA keypair in process.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::CaFailure`] if rcgen cannot build the CA, or
    /// [`IdentityError::InvalidIdentity`] if the CA TTL cannot be applied.
    pub fn new() -> Result<Self, IdentityError> {
        let key = KeyPair::generate().map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        let mut params = CertificateParams::new(Vec::<String>::new())
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, PLATFORM_INTERNAL_CA_PRODUCT);
        params.distinguished_name = dn;
        let ttl_hours =
            i64::try_from(DEFAULT_CA_TTL_HOURS).map_err(|_| IdentityError::InvalidIdentity)?;
        params.not_before = OffsetDateTime::now_utc();
        params.not_after = params
            .not_before
            .checked_add(Duration::hours(ttl_hours))
            .ok_or(IdentityError::InvalidIdentity)?;
        let cert = params
            .self_signed(&key)
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        let trust_anchor_pem = cert.pem();
        Ok(Self {
            cert,
            key,
            trust_anchor_pem,
        })
    }

    /// Load a persisted platform CA, or generate one when both files are absent.
    ///
    /// On load, [`CertificateParams::from_ca_cert_pem`] and [`KeyPair::from_pem`] reconstruct an
    /// in-memory issuer via `params.self_signed(&key)` **only as a signing object**.
    /// [`Self::cert_pem`] always returns the **original file PEM bytes**, never the re-self-signed
    /// DER. This method does not overwrite `cert_path` on load.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::CaFailure`] if exactly one file exists, PEM parse fails, the key
    /// does not match the certificate, the certificate PEM contains a private key or more than one
    /// certificate, or files cannot be written. Returns [`IdentityError::InvalidIdentity`] when
    /// generating if the CA TTL cannot be applied.
    pub fn load_or_create(cert_path: &Path, key_path: &Path) -> Result<Self, IdentityError> {
        match pem_pair_present(cert_path, key_path)? {
            true => Self::load_from_files(cert_path, key_path),
            false => {
                let ca = Self::new()?;
                write_pem_file(cert_path, &ca.trust_anchor_pem)?;
                write_key_file(key_path, &ca.key.serialize_pem())?;
                Ok(ca)
            }
        }
    }

    /// Build from `[identity]` paths: both empty/whitespace → [`Self::new`], else [`Self::load_or_create`].
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::CaFailure`] if only one path is set, or if loading/creating fails.
    /// See [`Self::load_or_create`] and [`Self::new`].
    pub fn from_identity_config(config: &IdentityConfig) -> Result<Self, IdentityError> {
        let cert = config.ca_cert_pem_path.trim();
        let key = config.ca_key_pem_path.trim();
        match (cert.is_empty(), key.is_empty()) {
            (true, true) => Self::new(),
            (false, false) => Self::load_or_create(Path::new(cert), Path::new(key)),
            _ => Err(IdentityError::CaFailure(
                "ca_cert_pem_path and ca_key_pem_path must both be set or both empty".to_owned(),
            )),
        }
    }

    /// Generate a CA-signed serverAuth leaf when both files are absent.
    ///
    /// SAN DNS `lazaret-service` + `localhost` + IP `127.0.0.1`; CN `lazaret-service`;
    /// leaf TTL [`DEFAULT_CERT_TTL_HOURS`]. Existing cert+key pairs are left unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::CaFailure`] if exactly one file exists, signing fails, or files
    /// cannot be written. Returns [`IdentityError::InvalidIdentity`] if the leaf TTL cannot be
    /// applied.
    pub fn ensure_server_leaf(
        &self,
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<(), IdentityError> {
        if pem_pair_present(cert_path, key_path)? {
            return Ok(());
        }
        let mut params = CertificateParams::new(Vec::<String>::new())
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        let dns_service = rcgen::Ia5String::try_from("lazaret-service")
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        let dns_localhost = rcgen::Ia5String::try_from("localhost")
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        params.subject_alt_names = vec![
            SanType::DnsName(dns_service),
            SanType::DnsName(dns_localhost),
            SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        ];
        params.is_ca = IsCa::NoCa;
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "lazaret-service");
        params.distinguished_name = dn;
        let ttl_hours =
            i64::try_from(DEFAULT_CERT_TTL_HOURS).map_err(|_| IdentityError::InvalidIdentity)?;
        params.not_before = OffsetDateTime::now_utc();
        params.not_after = params
            .not_before
            .checked_add(Duration::hours(ttl_hours))
            .ok_or(IdentityError::InvalidIdentity)?;
        let key = KeyPair::generate().map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        let cert = params
            .signed_by(&key, &self.cert, &self.key)
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        let cert_pem = cert.pem();
        if cert_pem.contains("PRIVATE KEY") {
            return Err(IdentityError::CaFailure(
                "generated server certificate contained a private key".to_owned(),
            ));
        }
        write_pem_file(cert_path, &cert_pem)?;
        write_key_file(key_path, &key.serialize_pem())?;
        Ok(())
    }

    /// Write [`Self::cert_pem`] (certificate only) to `path`.
    ///
    /// Rewriting the CA cert path is safe: bytes are the original trust-anchor PEM, never the key.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::CaFailure`] if the file cannot be written.
    pub fn write_trust_anchor(&self, path: &Path) -> Result<(), IdentityError> {
        write_pem_file(path, &self.trust_anchor_pem)
    }

    /// PEM of the CA certificate only (no private key).
    #[must_use]
    pub fn cert_pem(&self) -> String {
        self.trust_anchor_pem.clone()
    }

    fn load_from_files(cert_path: &Path, key_path: &Path) -> Result<Self, IdentityError> {
        let trust_anchor_pem =
            fs::read_to_string(cert_path).map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        if trust_anchor_pem.contains("PRIVATE KEY") {
            return Err(IdentityError::CaFailure(
                "ca certificate file contains a private key".to_owned(),
            ));
        }
        if !trust_anchor_pem.contains("BEGIN CERTIFICATE") {
            return Err(IdentityError::CaFailure(
                "ca certificate file is not a certificate pem".to_owned(),
            ));
        }
        if trust_anchor_pem
            .matches("-----BEGIN CERTIFICATE-----")
            .count()
            > 1
        {
            return Err(IdentityError::CaFailure(
                "ca certificate file must contain exactly one certificate".to_owned(),
            ));
        }
        let key_pem =
            fs::read_to_string(key_path).map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        let key =
            KeyPair::from_pem(&key_pem).map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        ca_public_key_matches(&trust_anchor_pem, &key)?;
        let params = CertificateParams::from_ca_cert_pem(&trust_anchor_pem)
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        let cert = params
            .self_signed(&key)
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        Ok(Self {
            cert,
            key,
            trust_anchor_pem,
        })
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

fn pem_pair_present(cert_path: &Path, key_path: &Path) -> Result<bool, IdentityError> {
    match (cert_path.exists(), key_path.exists()) {
        (true, true) => Ok(true),
        (false, false) => Ok(false),
        _ => Err(IdentityError::CaFailure(
            "exactly one of certificate or key files exists".to_owned(),
        )),
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), IdentityError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        }
    }
    Ok(())
}

fn write_pem_file(path: &Path, pem: &str) -> Result<(), IdentityError> {
    ensure_parent_dir(path)?;
    fs::write(path, pem).map_err(|e| IdentityError::CaFailure(e.to_string()))
}

fn write_key_file(path: &Path, pem: &str) -> Result<(), IdentityError> {
    ensure_parent_dir(path)?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
        file.write_all(pem.as_bytes())
            .map_err(|e| IdentityError::CaFailure(e.to_string()))
    }
    #[cfg(not(unix))]
    {
        fs::write(path, pem).map_err(|e| IdentityError::CaFailure(e.to_string()))
    }
}

fn ca_public_key_matches(cert_pem: &str, key: &KeyPair) -> Result<(), IdentityError> {
    let (_, pem) = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes())
        .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
    let (_, cert) = x509_parser::parse_x509_certificate(&pem.contents)
        .map_err(|e| IdentityError::CaFailure(e.to_string()))?;
    if cert.public_key().raw != key.public_key_der().as_slice() {
        return Err(IdentityError::CaFailure(
            "ca private key does not match certificate".to_owned(),
        ));
    }
    Ok(())
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
    use super::IdentityConfig;
    use super::*;

    fn assert_cert_only(pem: &str) {
        assert!(
            pem.contains("BEGIN CERTIFICATE"),
            "cert pem must include a certificate"
        );
        assert!(
            !pem.contains("PRIVATE KEY"),
            "cert pem must not include a private key"
        );
    }

    #[test]
    fn platform_ca_cert_pem_excludes_private_key() {
        let pem = PlatformInternalCa::new().expect("platform ca").cert_pem();
        assert_cert_only(&pem);
    }

    #[test]
    fn load_or_create_reload_preserves_cert_pem() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("ca.crt");
        let key_path = dir.path().join("ca.key");
        let first = PlatformInternalCa::load_or_create(&cert_path, &key_path).expect("create");
        let pem = first.cert_pem();
        assert_cert_only(&pem);
        let on_disk = std::fs::read_to_string(&cert_path).expect("read ca.crt");
        assert_eq!(pem, on_disk);
        assert_cert_only(&on_disk);
        let reloaded = PlatformInternalCa::load_or_create(&cert_path, &key_path).expect("reload");
        assert_eq!(pem, reloaded.cert_pem());
        assert_eq!(
            pem,
            std::fs::read_to_string(&cert_path).expect("re-read ca.crt")
        );
    }

    #[test]
    fn load_or_create_split_brain_is_fail_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("ca.crt");
        let key_path = dir.path().join("ca.key");
        PlatformInternalCa::load_or_create(&cert_path, &key_path).expect("create");
        let key_pem = std::fs::read_to_string(&key_path).expect("read key");
        std::fs::remove_file(&key_path).expect("remove key");
        match PlatformInternalCa::load_or_create(&cert_path, &key_path) {
            Err(IdentityError::CaFailure(_)) => {}
            Ok(_) => panic!("split-brain cert only must fail closed"),
            Err(other) => panic!("unexpected error: {other}"),
        }

        std::fs::write(&key_path, key_pem).expect("restore key");
        std::fs::remove_file(&cert_path).expect("remove cert");
        match PlatformInternalCa::load_or_create(&cert_path, &key_path) {
            Err(IdentityError::CaFailure(_)) => {}
            Ok(_) => panic!("split-brain key only must fail closed"),
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn generated_ca_crt_excludes_private_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("ca.crt");
        let key_path = dir.path().join("ca.key");
        PlatformInternalCa::load_or_create(&cert_path, &key_path).expect("create");
        let on_disk = std::fs::read_to_string(&cert_path).expect("read ca.crt");
        assert_cert_only(&on_disk);
    }

    #[test]
    fn platform_ca_not_after_uses_ca_ttl() {
        let pem = PlatformInternalCa::new().expect("platform ca").cert_pem();
        let params = CertificateParams::from_ca_cert_pem(&pem).expect("parse ca pem");
        let span = params.not_after - params.not_before;
        let expected =
            Duration::hours(i64::try_from(DEFAULT_CA_TTL_HOURS).expect("ca ttl fits i64"));
        assert_eq!(span, expected);
    }

    #[test]
    fn from_identity_config_with_paths_loads_persisted_ca() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("ca.crt");
        let key_path = dir.path().join("ca.key");
        let config = IdentityConfig {
            ca_cert_pem_path: cert_path.to_string_lossy().into_owned(),
            ca_key_pem_path: key_path.to_string_lossy().into_owned(),
            ..IdentityConfig::default()
        };
        let first = PlatformInternalCa::from_identity_config(&config).expect("generate");
        let pem = first.cert_pem();
        let second = PlatformInternalCa::from_identity_config(&config).expect("reload");
        assert_eq!(pem, second.cert_pem());
    }

    #[test]
    fn from_identity_config_cert_path_only_is_fail_closed() {
        let config = IdentityConfig {
            ca_cert_pem_path: "ca.crt".to_owned(),
            ..IdentityConfig::default()
        };
        match PlatformInternalCa::from_identity_config(&config) {
            Err(IdentityError::CaFailure(_)) => {}
            Ok(_) => panic!("cert path only must fail closed"),
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn from_identity_config_key_path_only_is_fail_closed() {
        let config = IdentityConfig {
            ca_key_pem_path: "ca.key".to_owned(),
            ..IdentityConfig::default()
        };
        match PlatformInternalCa::from_identity_config(&config) {
            Err(IdentityError::CaFailure(_)) => {}
            Ok(_) => panic!("key path only must fail closed"),
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn ensure_server_leaf_is_ca_signed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ca_cert = dir.path().join("ca.crt");
        let ca_key = dir.path().join("ca.key");
        let leaf_cert = dir.path().join("server.crt");
        let leaf_key = dir.path().join("server.key");
        let ca = PlatformInternalCa::load_or_create(&ca_cert, &ca_key).expect("ca");
        ca.ensure_server_leaf(&leaf_cert, &leaf_key).expect("leaf");

        let server_pem = std::fs::read_to_string(&leaf_cert).expect("server.crt");
        let ca_pem = std::fs::read_to_string(&ca_cert).expect("ca.crt");
        assert_cert_only(&server_pem);

        let (_, server_block) =
            x509_parser::pem::parse_x509_pem(server_pem.as_bytes()).expect("server pem");
        let (_, server_x509) =
            x509_parser::parse_x509_certificate(&server_block.contents).expect("server x509");
        let (_, ca_block) = x509_parser::pem::parse_x509_pem(ca_pem.as_bytes()).expect("ca pem");
        let (_, ca_x509) =
            x509_parser::parse_x509_certificate(&ca_block.contents).expect("ca x509");

        assert_eq!(
            server_x509.issuer(),
            ca_x509.subject(),
            "server leaf issuer must be the platform CA subject"
        );

        let san = server_x509
            .subject_alternative_name()
            .expect("san parse")
            .expect("san present");
        let mut has_dns_service = false;
        let mut has_dns_localhost = false;
        let mut has_ip_localhost = false;
        for name in &san.value.general_names {
            match name {
                x509_parser::extensions::GeneralName::DNSName("lazaret-service") => {
                    has_dns_service = true;
                }
                x509_parser::extensions::GeneralName::DNSName("localhost") => {
                    has_dns_localhost = true;
                }
                x509_parser::extensions::GeneralName::IPAddress(bytes)
                    if *bytes == [127, 0, 0, 1] =>
                {
                    has_ip_localhost = true;
                }
                _ => {}
            }
        }
        assert!(has_dns_service, "SAN missing DnsName lazaret-service");
        assert!(has_dns_localhost, "SAN missing DnsName localhost");
        assert!(has_ip_localhost, "SAN missing iPAddress 127.0.0.1");
    }

    #[cfg(unix)]
    #[test]
    fn load_or_create_writes_ca_key_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("ca.crt");
        let key_path = dir.path().join("ca.key");
        PlatformInternalCa::load_or_create(&cert_path, &key_path).expect("create");
        let mode = std::fs::metadata(&key_path)
            .expect("ca.key metadata")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o077,
            0,
            "ca.key must not be group/other readable, mode={mode:#o}"
        );
    }

    #[test]
    fn load_from_files_rejects_second_certificate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("ca.crt");
        let key_path = dir.path().join("ca.key");
        PlatformInternalCa::load_or_create(&cert_path, &key_path).expect("create");
        let extra = PlatformInternalCa::new().expect("other ca").cert_pem();
        let mut pem = std::fs::read_to_string(&cert_path).expect("read ca.crt");
        pem.push_str(&extra);
        std::fs::write(&cert_path, pem).expect("append second cert");
        match PlatformInternalCa::load_or_create(&cert_path, &key_path) {
            Err(IdentityError::CaFailure(msg)) => {
                assert!(
                    msg.contains("exactly one certificate"),
                    "unexpected message: {msg}"
                );
            }
            Ok(_) => panic!("second certificate must fail closed"),
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn load_or_create_rejects_mismatched_ca_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("ca.crt");
        let key_path = dir.path().join("ca.key");
        PlatformInternalCa::load_or_create(&cert_path, &key_path).expect("create");
        let foreign = KeyPair::generate().expect("foreign key");
        std::fs::write(&key_path, foreign.serialize_pem()).expect("overwrite key");
        match PlatformInternalCa::load_or_create(&cert_path, &key_path) {
            Err(IdentityError::CaFailure(msg)) => {
                assert!(msg.contains("does not match"), "unexpected message: {msg}");
            }
            Ok(_) => panic!("mismatched ca.key must fail closed"),
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn load_or_create_rejects_ca_cert_with_private_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("ca.crt");
        let key_path = dir.path().join("ca.key");
        PlatformInternalCa::load_or_create(&cert_path, &key_path).expect("create");
        let key_pem = std::fs::read_to_string(&key_path).expect("read key");
        let mut cert_pem = std::fs::read_to_string(&cert_path).expect("read cert");
        cert_pem.push_str(&key_pem);
        std::fs::write(&cert_path, cert_pem).expect("append key to cert");
        match PlatformInternalCa::load_or_create(&cert_path, &key_path) {
            Err(IdentityError::CaFailure(msg)) => {
                assert!(
                    msg.to_lowercase().contains("private key"),
                    "unexpected message: {msg}"
                );
            }
            Ok(_) => panic!("ca.crt with private key must fail closed"),
            Err(other) => panic!("unexpected error: {other}"),
        }
    }
}
