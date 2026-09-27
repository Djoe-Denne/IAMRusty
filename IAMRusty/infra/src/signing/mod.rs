//! SigningProvider adapters and WorkloadIdentity StaticCredential.

mod pem;
mod probe;
mod rotate;
mod static_credential;
mod transit;

pub use pem::PemSigningProvider;
pub use probe::DefaultOrganizationSignerProbe;
pub use rotate::{
    insert_pending_rotation, promote_pending_rotation, rotate_organization_signer,
    DefaultOrganizationSignerRotator, PendingRotation, RotateContext, TransitClientConfig,
};
pub use static_credential::StaticCredential;
pub use transit::{TransitSigningProvider, FORBIDDEN_TRANSIT_KEY_NAME};
