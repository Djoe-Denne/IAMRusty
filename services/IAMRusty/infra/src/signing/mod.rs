//! `SigningProvider` adapters and `WorkloadIdentity` `StaticCredential` / `WIF`.

mod pem;
mod probe;
mod remote;
mod rotate;
mod scoped;
mod static_credential;
mod transit;
pub mod wif;

pub use pem::PemSigningProvider;
pub use probe::DefaultOrganizationSignerProbe;
pub use probe::ProviderBindingProbe;
pub use remote::RemoteSigningProvider;
pub use rotate::{
    insert_pending_rotation, promote_pending_rotation, rotate_organization_signer,
    rotate_signing_scope, DefaultOrganizationSignerRotator, PendingRotation, RotateContext,
    TransitClientConfig,
};
pub use scoped::ScopedSigningProvider;
pub use static_credential::StaticCredential;
pub use transit::{TransitSigningProvider, FORBIDDEN_TRANSIT_KEY_NAME};
pub use wif::{compose_workload_identity, AwsWif, AzureWif, GcpWif};
