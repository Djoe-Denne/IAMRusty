//! Runtime-specific trust boundaries. No cluster client and no IAM bearer.
//! Aligned to contract §13.1 and integrated contracts HEAD b399363.

use apparatus_contracts::runtime::{RuntimeExecutionContextV1, RuntimeInvocationTicketV1};
// Shared maintenance data from C; provenance must be verified by the runner.
pub use apparatus_contracts::runtime::{InstallRuntimeLegacyFenceV1, RuntimeCutoverFenceV1};
use async_trait::async_trait;
use uuid::Uuid;

use crate::WorkloadIdentity;

/// One shared v1 taxonomy, never a second Lazaret-specific wire enum.
pub use apparatus_contracts::runtime::RuntimeErrorCodeV1 as RuntimeError;

/// Verified projected-token claims, not caller-supplied registration data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePodProof {
    pub pod_uid: String,
    pub namespace: String,
    pub service_account: String,
    pub service_account_uid: String,
    pub expires_at_unix: i64,
}

/// Offline OIDC signature/audience validation; current Pod ownership is checked
/// separately against the authoritative runtime-control port on every use.
pub trait RuntimeEnrollmentProofVerifier: Send + Sync {
    fn verify(&self, token: &str) -> Result<RuntimePodProof, RuntimeError>;
}

/// Purpose-scoped remote RS256 signer. Implementations never expose private keys.
#[async_trait]
pub trait RuntimeTicketSigner: Send + Sync {
    async fn sign(&self, ticket: &RuntimeInvocationTicketV1) -> Result<String, RuntimeError>;
    fn verify(&self, token: &str) -> Result<RuntimeInvocationTicketV1, RuntimeError>;
}

/// Public invoke-only provider metadata. No private RSA material or discovery URL.
#[derive(Clone)]
pub struct RuntimeInvokePublicKey {
    pub kid: String,
    pub purpose: String,
    pub modulus_base64url: String,
    pub exponent_base64url: String,
    pub published_and_acknowledged_at_unix_ms: i64,
    pub activated_at_unix_ms: i64,
    pub signing_until_unix_ms: i64,
    pub last_issued_at_unix_ms: Option<i64>,
    pub verification_until_unix_ms: i64,
    pub revoked: bool,
}

/// Trusted platform provider seam, separate from IAM/session/enrollment signers.
/// Implementations enforce the fixed invoke purpose and atomically update the
/// public last-issuance metadata before returning a signature. The composition
/// root supplies the provider; requests cannot choose an endpoint or credential.
#[async_trait]
pub trait RuntimeRsaSigningProvider: Send + Sync {
    fn active_key(&self) -> Result<RuntimeInvokePublicKey, RuntimeError>;
    fn verification_key(&self, kid: &str) -> Result<RuntimeInvokePublicKey, RuntimeError>;
    async fn sign_rs256(
        &self,
        purpose: &str,
        kid: &str,
        signing_input: &[u8],
    ) -> Result<Vec<u8>, RuntimeError>;
}

/// Lazaret-owned durable identity record, separate from legacy binding uniqueness.
/// Certificate/CSR digests are bare SHA-256 hex; no private key or projected token.
#[derive(Clone)]
pub struct RuntimeEnrollment {
    pub project_id: Uuid,
    pub identity: WorkloadIdentity,
    pub pod_uid: String,
    pub revision_uid: String,
    pub namespace: String,
    pub service_account: String,
    pub service_account_uid: String,
    pub revocation_epoch: u64,
    pub csr_sha256: String,
    pub fingerprint: String,
    pub certificate_pem: String,
    pub issued_at_unix_ms: i64,
    pub expires_at_unix_ms: i64,
    pub closed: bool,
}

/// Per-process bootstrap plus the immutable negative-directory mapping. The
/// composition root must inject the real guard; no request can select/bypass it.
#[async_trait]
pub trait RuntimeAdmissionGuard: Send + Sync {
    async fn require_runtime(&self, enrollment: &RuntimeEnrollment) -> Result<(), RuntimeError>;
    async fn require_binding(&self, project_id: Uuid, binding_id: Uuid)
        -> Result<(), RuntimeError>;
    async fn require_legacy(&self, binding_id: Uuid) -> Result<(), RuntimeError>;
    async fn require_legacy_identity(
        &self,
        identity: &WorkloadIdentity,
    ) -> Result<(), RuntimeError>;
}

/// Durable registry and one-use authorization journal. Every successful issuance
/// is journaled before forwarding; every accept is a database CAS, not a mutex.
#[async_trait]
pub trait RuntimeStore: Send + Sync {
    async fn has_cutover_fences(&self) -> Result<bool, RuntimeError>;
    async fn list_cutover_fences(
        &self,
        project_id: Uuid,
        after: Option<Uuid>,
        limit: u16,
    ) -> Result<Vec<RuntimeCutoverFenceV1>, RuntimeError>;
    async fn cutover_fence(
        &self,
        binding_id: Uuid,
    ) -> Result<Option<RuntimeCutoverFenceV1>, RuntimeError>;
    /// Negative admission check under the same database lock as install. The
    /// application must additionally require a valid per-process bootstrap.
    async fn admit_legacy(&self, binding_id: Uuid) -> Result<(), RuntimeError>;
    /// Commit the negative directory and local closure before returning an ack.
    async fn install_legacy_fence(
        &self,
        command: &InstallRuntimeLegacyFenceV1,
    ) -> Result<RuntimeCutoverFenceV1, RuntimeError>;
    async fn enrollment_by_instance(
        &self,
        instance: Uuid,
    ) -> Result<Option<RuntimeEnrollment>, RuntimeError>;
    async fn enrollment_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<Option<RuntimeEnrollment>, RuntimeError>;
    /// First-wins on instance, Pod UID and certificate fingerprint. Identical
    /// mapping+CSR returns the original certificate, never activates a tombstone.
    async fn enroll(&self, record: &RuntimeEnrollment) -> Result<RuntimeEnrollment, RuntimeError>;
    /// Persist the typed public claims, never the raw signed bearer ticket.
    async fn issue(&self, ticket: &RuntimeInvocationTicketV1) -> Result<(), RuntimeError>;
    async fn issued(
        &self,
        authorization_id: Uuid,
    ) -> Result<Option<RuntimeInvocationTicketV1>, RuntimeError>;
    async fn accept(
        &self,
        authorization_id: Uuid,
        instance_id: Uuid,
    ) -> Result<RuntimeExecutionContextV1, RuntimeError>;
    async fn execution(
        &self,
        execution_id: Uuid,
    ) -> Result<RuntimeExecutionContextV1, RuntimeError>;
    /// Call only after the authoritative identity fence commits; local closure
    /// never substitutes for Manifesto's serialization with begin-effect.
    async fn close_binding(&self, project_id: Uuid, binding_id: Uuid) -> Result<(), RuntimeError>;
}
