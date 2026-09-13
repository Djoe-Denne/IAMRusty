//! Contrats Apparatus P0 : manifeste, protocole wire, digest, validation.
//!
//! Crate pure Rust, sans `Axum`, `SeaORM`, `JWT`, `AWS`, `reqwest` ni `Tokio` :
//! schéma versionné de `apparatus.toml`, identités, DTO release/binding/
//! configuration/opération, protocole backend privé [`protocol::PROTOCOL_ID`],
//! erreurs stables versionnées, UI déclarative, taxonomie minimale de
//! capacités, digest SHA-256 déterministe et harness in-process TEST-ONLY
//! (feature `test-harness`, jamais en production).
//!
//! ## Tests
//!
//! ```sh
//! cargo test -p apparatus-contracts --features test-harness
//! ```

pub mod capabilities;
pub mod digest;
pub mod error;
#[cfg(feature = "test-harness")]
pub mod harness;
pub mod ids;
pub mod limits;
pub mod manifest;
pub mod ports;
pub mod protocol;
pub mod ui;
pub mod validation;

pub use capabilities::{requires_network, Capability};
pub use digest::{digest_bytes, digest_json, digest_manifest, digest_str};
pub use error::ApparatusError;
#[cfg(feature = "test-harness")]
pub use harness::{InMemoryKv, TestHarness};
pub use ids::{ApparatusId, BindingId, OperationId, ReleaseDigest};
pub use limits::{
    MAX_CAPABILITIES, MAX_CONFIG_BYTES, MAX_DESCRIPTION_LEN, MAX_ID_LEN, MAX_INSTALL_REF_LEN,
    MAX_KV_KEY_LEN, MAX_KV_VALUE_BYTES, MAX_OPERATION_NAME_LEN, MAX_PAYLOAD_BYTES,
    MAX_SCHEMA_PATH_LEN,
};
pub use manifest::{ApparatusManifest, ApparatusMeta, BackendDecl, CapabilitiesDecl};
pub use ports::{ApparatusRuntime, KvStore, RuntimeObservation};
pub use protocol::{
    new_operation_id, BindRequest, BindResponse, ConfigureRequest, ConfigureResponse,
    DiscoveryDocument, DiscoveryEndpoints, HealthResponse, HealthStatus, InvokeRequest,
    InvokeResponse, ReadyResponse, UnbindRequest, UnbindResponse, BIND_PATH, CONFIGURE_PATH,
    HEALTH_PATH, INVOKE_PATH, PROTOCOL_ID, READY_PATH, UNBIND_PATH, WELL_KNOWN_PATH,
};
pub use ui::{UiDeclaration, UiMode};
#[cfg(feature = "test-harness")]
pub use validation::assert_no_secret_keys;
pub use validation::{
    is_floating_ref, validate_declared_version, validate_install_ref, validate_manifest,
    validate_manifest_toml, ValidatedManifest, SUPPORTED_SCHEMA_MAJOR,
};
