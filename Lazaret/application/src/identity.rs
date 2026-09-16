//! Application service: enroll (CSR) then short dedicated session. Token ≠ authorization.

use std::sync::Arc;

use chrono::Utc;
use lazaret_domain::{
    authorization_from_session, AuthorizationDecision, BindingGrantSnapshotPort,
    CertificateAuthority, EnrollmentStore, GrantFetchError, IdentityError, IssuedCertificate,
    SessionClaims, SessionProof, SessionTokenSigner, VerifiedClientCertificate, WorkloadIdentity,
    SESSION_AUDIENCE, SESSION_ISSUER,
};
use uuid::Uuid;

/// CSR enrollment command. The private key must stay on the workload.
pub struct EnrollCommand {
    /// PEM-encoded certificate signing request (never a private key).
    pub csr_pem: String,
    /// Workload identity from the client (revision/generation are not authoritative).
    pub identity: WorkloadIdentity,
    /// Project used to fetch the live binding snapshot.
    pub project_id: Uuid,
}

/// Hybrid identity application service.
pub struct IdentityService {
    ca: Arc<dyn CertificateAuthority>,
    signer: Arc<dyn SessionTokenSigner>,
    enrollments: Arc<dyn EnrollmentStore>,
    snapshots: Arc<dyn BindingGrantSnapshotPort>,
    session_ttl_minutes: u64,
    cert_ttl_hours: u64,
}

impl IdentityService {
    /// Wire CA, dedicated session signer, enrollment store, and live snapshot port.
    #[must_use]
    pub fn new(
        ca: Arc<dyn CertificateAuthority>,
        signer: Arc<dyn SessionTokenSigner>,
        enrollments: Arc<dyn EnrollmentStore>,
        snapshots: Arc<dyn BindingGrantSnapshotPort>,
        session_ttl_minutes: u64,
        cert_ttl_hours: u64,
    ) -> Self {
        Self {
            ca,
            signer,
            enrollments,
            snapshots,
            session_ttl_minutes,
            cert_ttl_hours,
        }
    }

    /// Issue a client certificate from a workload CSR after a live snapshot consult.
    ///
    /// Generation, grant revision, and binding come from the snapshot. `instance`
    /// and `release` stay on the client body.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::PrivateKeyNotAccepted`] if private-key PEM is present,
    /// [`IdentityError::InvalidIdentity`] if the snapshot is missing or the binding
    /// does not match, [`IdentityError::ConsultFailed`] on transport failure,
    /// [`IdentityError::RegistryFull`] when the registry is at capacity,
    /// [`IdentityError::BindingAlreadyEnrolled`] when the binding is already enrolled
    /// under a different fingerprint, or CA errors.
    pub async fn enroll(&self, command: EnrollCommand) -> Result<IssuedCertificate, IdentityError> {
        reject_private_key_material(&command.csr_pem)?;
        let snapshot = self
            .snapshots
            .fetch(command.project_id, command.identity.binding, None)
            .await
            .map_err(|error| match error {
                GrantFetchError::NotFound => IdentityError::InvalidIdentity,
                GrantFetchError::Transport(_) => IdentityError::ConsultFailed,
            })?;
        if snapshot.component_id != command.identity.binding {
            return Err(IdentityError::InvalidIdentity);
        }
        let identity = WorkloadIdentity::try_new(
            command.identity.instance,
            snapshot.component_id,
            command.identity.release,
            snapshot.desired_generation,
            snapshot.grant_revision,
        )?;
        if self.enrollments.binding_enrolled(identity.binding) {
            return Err(IdentityError::BindingAlreadyEnrolled);
        }
        let issued = self
            .ca
            .sign_csr(&command.csr_pem, self.cert_ttl_hours, identity.binding)?;
        let fingerprint = lazaret_domain::fingerprint_sha256(&issued.der);
        self.enrollments.put(fingerprint, identity)?;
        Ok(issued)
    }

    /// Issue a short session token for an enrolled, verified client certificate.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::NotEnrolled`] when the certificate was not enrolled,
    /// or signing errors.
    pub fn issue_session(&self, cert: &VerifiedClientCertificate) -> Result<String, IdentityError> {
        let identity = self
            .enrollments
            .get(&cert.fingerprint_sha256())
            .ok_or(IdentityError::NotEnrolled)?;
        let now = Utc::now().timestamp();
        let ttl_minutes =
            i64::try_from(self.session_ttl_minutes).map_err(|_| IdentityError::InvalidIdentity)?;
        let exp = now.saturating_add(ttl_minutes.saturating_mul(60));
        let claims = SessionClaims {
            instance: identity.instance,
            binding: identity.binding,
            release: identity.release,
            generation: identity.generation,
            grant_revision: identity.grant_revision,
            aud: SESSION_AUDIENCE.to_owned(),
            iss: SESSION_ISSUER.to_owned(),
            exp,
            iat: now,
        };
        self.signer.sign(&claims)
    }

    /// Verify a dedicated Lazaret session token (rejects IAM JWTs).
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::InvalidSession`] or [`IdentityError::UserClaimForbidden`].
    pub fn verify_session(&self, token: &str) -> Result<SessionProof, IdentityError> {
        let claims = self.signer.verify(token)?;
        reject_user_subject_claim(token)?;
        if claims.iss != SESSION_ISSUER || claims.aud != SESSION_AUDIENCE {
            return Err(IdentityError::InvalidSession);
        }
        let identity = WorkloadIdentity::try_new(
            claims.instance,
            claims.binding,
            claims.release,
            claims.generation,
            claims.grant_revision,
        )?;
        Ok(SessionProof {
            identity,
            audience: claims.aud,
            issuer: claims.iss,
            expires_at_unix: claims.exp,
        })
    }

    /// A crypto-valid session is never authorization.
    #[must_use]
    pub const fn authorization_from_session(&self, proof: &SessionProof) -> AuthorizationDecision {
        authorization_from_session(proof)
    }
}

fn reject_private_key_material(pem: &str) -> Result<(), IdentityError> {
    let upper = pem.to_ascii_uppercase();
    if upper.contains("PRIVATE KEY") {
        return Err(IdentityError::PrivateKeyNotAccepted);
    }
    Ok(())
}

fn reject_user_subject_claim(token: &str) -> Result<(), IdentityError> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or(IdentityError::InvalidSession)?;
    let decoded = decode_b64url(payload).ok_or(IdentityError::InvalidSession)?;
    let value: serde_json::Value =
        serde_json::from_slice(&decoded).map_err(|_| IdentityError::InvalidSession)?;
    if value.get("sub").is_some() {
        return Err(IdentityError::UserClaimForbidden);
    }
    Ok(())
}

fn decode_b64url(input: &str) -> Option<Vec<u8>> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD
        .decode(input)
        .ok()
        .or_else(|| base64::engine::general_purpose::URL_SAFE.decode(input).ok())
}
