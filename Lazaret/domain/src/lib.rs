//! Domain layer for Lazaret.

pub mod connectors;
pub mod grants;
pub mod identity;
pub mod kv;
pub mod secrets;

pub use connectors::{ConnectorError, ConnectorProxy, ConnectorRegistry};
pub use grants::{
    evaluate_grant, BindingGrantSnapshot, BindingGrantSnapshotPort, CallOrigin, CapabilityConsent,
    GrantAuthorizationRequest, GrantDecision, GrantDenyReason, GrantFetchError,
    PrincipalMembership,
};
pub use identity::{
    authorization_from_session, fingerprint_sha256, AuthorizationDecision, CertificateAuthority,
    EnrollmentStore, IdentityError, IssuedCertificate, SessionClaims, SessionProof,
    SessionTokenSigner, VerifiedClientCertificate, WorkloadIdentity, AUTHORIZATION_FROM_SESSION,
    DEFAULT_CERT_TTL_HOURS, DEFAULT_SESSION_TTL_MINUTES, PLATFORM_INTERNAL_CA_PRODUCT,
    SESSION_AUDIENCE, SESSION_ISSUER,
};
pub use kv::AsyncKvStore;
pub use secrets::{parse_secret_reference, SecretError, SecretResolver};
