//! Workload identity types for Lazaret (ADR-0004 claims, ADR-0007 hybrid transport).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Implementation default for session token TTL (not an Accepted frozen number).
pub const DEFAULT_SESSION_TTL_MINUTES: u64 = 15;

/// Implementation default for issued client-certificate TTL (not an Accepted frozen number).
pub const DEFAULT_CERT_TTL_HOURS: u64 = 24;

/// Software product name of the in-process platform CA (not a commercial PKI).
pub const PLATFORM_INTERNAL_CA_PRODUCT: &str = "platform-internal-ca";

/// Dedicated session audience (never IAM `aiforall`).
pub const SESSION_AUDIENCE: &str = "lazaret";

/// Dedicated session issuer (never IAM `iamrusty`).
pub const SESSION_ISSUER: &str = "lazaret";

/// A cryptographically valid session is never sufficient authorization.
pub const AUTHORIZATION_FROM_SESSION: &str = "LiveManifestoCheckRequired";

/// Workload identity claims (binding = `project_components.id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadIdentity {
    /// Apparatus instance id.
    pub instance: Uuid,
    /// Binding id (`project_components.id`).
    pub binding: Uuid,
    /// Admitted release identifier.
    pub release: String,
    /// Desired generation from the P2 controller.
    pub generation: i64,
    /// Grant revision from Manifesto (distinct from generation).
    pub grant_revision: i64,
}

impl WorkloadIdentity {
    /// Build a workload identity after validating non-negative revisions and a non-empty release.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::InvalidIdentity`] when `release` is empty or generation / grant
    /// revision is negative.
    pub fn try_new(
        instance: Uuid,
        binding: Uuid,
        release: String,
        generation: i64,
        grant_revision: i64,
    ) -> Result<Self, IdentityError> {
        if release.is_empty() || generation < 0 || grant_revision < 0 {
            return Err(IdentityError::InvalidIdentity);
        }
        Ok(Self {
            instance,
            binding,
            release,
            generation,
            grant_revision,
        })
    }
}

/// JWT claims for a Lazaret session. No user `sub`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    /// Apparatus instance id.
    pub instance: Uuid,
    /// Binding id (`project_components.id`).
    pub binding: Uuid,
    /// Admitted release identifier.
    pub release: String,
    /// Desired generation.
    pub generation: i64,
    /// Grant revision.
    pub grant_revision: i64,
    /// Audience — always [`SESSION_AUDIENCE`].
    pub aud: String,
    /// Issuer — always [`SESSION_ISSUER`].
    pub iss: String,
    /// Expiration (unix seconds).
    pub exp: i64,
    /// Issued-at (unix seconds).
    pub iat: i64,
}

/// Verified proof of a session. Does not carry capability grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionProof {
    /// Workload identity copied from the token (not grants).
    pub identity: WorkloadIdentity,
    /// Token audience.
    pub audience: String,
    /// Token issuer.
    pub issuer: String,
    /// Expiration (unix seconds).
    pub expires_at_unix: i64,
}

/// Client certificate already authenticated by the TLS terminator (or a typed test double).
#[derive(Debug, Clone)]
pub struct VerifiedClientCertificate {
    der: Vec<u8>,
}

impl VerifiedClientCertificate {
    /// Wrap certificate DER bytes (handshake or test double).
    #[must_use]
    pub const fn from_der(der: Vec<u8>) -> Self {
        Self { der }
    }

    /// Parse a PEM certificate into DER.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::InvalidCertificate`] when the PEM is not a certificate.
    pub fn from_pem(pem_str: &str) -> Result<Self, IdentityError> {
        let parsed =
            pem::parse(pem_str.as_bytes()).map_err(|_| IdentityError::InvalidCertificate)?;
        if !parsed.tag().eq_ignore_ascii_case("CERTIFICATE") {
            return Err(IdentityError::InvalidCertificate);
        }
        Ok(Self {
            der: parsed.into_contents(),
        })
    }

    /// Certificate DER.
    #[must_use]
    pub fn der(&self) -> &[u8] {
        &self.der
    }

    /// SHA-256 fingerprint of the certificate DER (lowercase hex).
    #[must_use]
    pub fn fingerprint_sha256(&self) -> String {
        fingerprint_sha256(&self.der)
    }
}

/// SHA-256 fingerprint of certificate DER (lowercase hex).
#[must_use]
pub fn fingerprint_sha256(der: &[u8]) -> String {
    let digest = Sha256::digest(der);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(hex_digit(byte >> 4));
        out.push(hex_digit(byte & 0x0f));
    }
    out
}

fn hex_digit(nibble: u8) -> char {
    char::from(b"0123456789abcdef"[usize::from(nibble)])
}

/// Issued client certificate (PEM + DER).
#[derive(Debug, Clone)]
pub struct IssuedCertificate {
    /// PEM encoding returned to the workload.
    pub pem: String,
    /// DER used for fingerprinting.
    pub der: Vec<u8>,
}

/// Decision after a crypto-valid session: never allow by token alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationDecision {
    /// Live Manifesto checks are still required (T4+).
    LiveManifestoCheckRequired,
}

/// Map a session proof to an authorization decision.
///
/// A crypto-valid token is never authorization.
#[must_use]
pub const fn authorization_from_session(_proof: &SessionProof) -> AuthorizationDecision {
    AuthorizationDecision::LiveManifestoCheckRequired
}

/// Certificate authority that signs workload CSRs.
pub trait CertificateAuthority: Send + Sync {
    /// Sign a PEM CSR and return the issued certificate bound to `binding`.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] when the CSR is invalid, requests a CA, or signing fails.
    fn sign_csr(
        &self,
        csr_pem: &str,
        ttl_hours: u64,
        binding: Uuid,
    ) -> Result<IssuedCertificate, IdentityError>;
}

/// Dedicated session token signer (never IAM HMAC).
pub trait SessionTokenSigner: Send + Sync {
    /// Sign session claims.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::SigningMaterial`] or [`IdentityError::InvalidSession`] on failure.
    fn sign(&self, claims: &SessionClaims) -> Result<String, IdentityError>;

    /// Verify a session token with the dedicated key and audience.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::InvalidSession`] or [`IdentityError::UserClaimForbidden`].
    fn verify(&self, token: &str) -> Result<SessionClaims, IdentityError>;
}

/// In-memory enrollment registry (fingerprint → identity). No SQL.
pub trait EnrollmentStore: Send + Sync {
    /// Store enrollment for a certificate fingerprint.
    ///
    /// First-wins: a binding already enrolled with a **different** fingerprint
    /// cannot be stolen (`BindingAlreadyEnrolled`). The same fingerprint is a
    /// no-op (`Ok`).
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::RegistryFull`] when a new binding would exceed
    /// capacity, or [`IdentityError::BindingAlreadyEnrolled`] when the binding
    /// is already enrolled under a different fingerprint.
    fn put(&self, fingerprint: String, identity: WorkloadIdentity) -> Result<(), IdentityError>;

    /// Lookup enrollment by fingerprint.
    fn get(&self, fingerprint: &str) -> Option<WorkloadIdentity>;

    /// `true` when this binding already has a stored fingerprint (first-wins).
    fn binding_enrolled(&self, binding: Uuid) -> bool;
}

/// Workload identity failures.
#[derive(Debug, Error)]
pub enum IdentityError {
    /// Private key material was present in an enrollment request.
    #[error("private key material is not accepted")]
    PrivateKeyNotAccepted,
    /// CSR PEM could not be parsed or signed.
    #[error("invalid certificate signing request: {0}")]
    InvalidCsr(String),
    /// Certificate PEM/DER could not be parsed.
    #[error("invalid client certificate")]
    InvalidCertificate,
    /// Identity fields are invalid.
    #[error("invalid workload identity")]
    InvalidIdentity,
    /// No enrollment for this client certificate.
    #[error("session refused: client certificate is not enrolled")]
    NotEnrolled,
    /// Session token is not a Lazaret session.
    #[error("invalid session token")]
    InvalidSession,
    /// User subject claim is forbidden on a workload session.
    #[error("user subject claim is forbidden on a workload session")]
    UserClaimForbidden,
    /// HTTP session without a verified client certificate.
    #[error("verified client certificate is required")]
    MissingClientCertificate,
    /// Live binding snapshot consult failed (transport).
    #[error("binding grant snapshot consult failed")]
    ConsultFailed,
    /// In-memory enrollment registry reached capacity.
    #[error("enrollment registry is full")]
    RegistryFull,
    /// Binding is already enrolled under a different certificate fingerprint.
    #[error("binding already enrolled")]
    BindingAlreadyEnrolled,
    /// Dedicated signing material could not be loaded or used.
    #[error("session signing material: {0}")]
    SigningMaterial(String),
    /// Internal CA failed to issue a certificate.
    #[error("platform internal CA: {0}")]
    CaFailure(String),
}
