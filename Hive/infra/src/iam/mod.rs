//! IAM s2s adapters for Hive.

pub mod organization_signer_client;
pub mod static_credential;
pub use organization_signer_client::HttpIamOrganizationSignerClient;
pub use static_credential::StaticCredential;
