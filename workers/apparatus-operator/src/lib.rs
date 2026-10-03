//! Îlot ops Apparatus P4 (BC-A), hors Manifesto.
//!
//! Scaffold T2 : trois binaires sans listener HTTP. T3 : wrap Git → digest via
//! [`apparatus_contracts`]. T4 : worker de build sans identité privilégiée.
//! T5 : runner de conformance plateforme, rapport versionné hors admission.
//! T6 (`feature = "admit"`) : push ORAS d'enveloppe non-CRI + Cosign/Transit.
//! T7 : store d'admission in-memory (register ≠ admit). M2 : JSON produit
//! [`PersistentAdmissionStore`]. T8 : refus ADR-0005.
//! T9 : worker malveillant sans clés (même confinement officiel / communautaire).
//! T10 (`feature = "controller"`) : Kind + JSON VALID M3 + CR → Pod piné.
//! M4 : observer Manifesto `desired_state` (HTTP `/components`) hors crate Manifesto.

pub mod admission;
pub mod build;
pub mod conformance;
pub mod digest;

#[cfg(feature = "admit")]
pub mod admit;

#[cfg(feature = "controller")]
pub mod controller;

#[cfg(feature = "controller")]
pub mod desired_state;

pub use admission::{
    admission_store_path_from_env, evaluate_admit, manifesto_register_does_not_admit,
    register_manifesto_release, would_schedule, AdmissionRecord, AdmissionStatus, AdmissionStore,
    AdmitInput, AdmitRefuse, InMemoryAdmissionStore, PersistentAdmissionStore,
    ADMISSION_STORE_PATH_ENV,
};
pub use build::{
    cargo_build_scripts_trusted, parse_manifest, refuse_privileged_identity,
    refuse_privileged_identity_from, reject_free_build_field, PRIVILEGED_IDENTITY_ENV,
};
pub use conformance::{
    community_policy_id, evaluate_conformance, official_policy_id, report_grants_admission,
    ConformanceReport, POLICY_ID,
};
pub use digest::{
    digest_manifest, is_floating_ref, resolve_install_ref, validate_install_ref, ApparatusError,
    ReleaseDigest,
};

#[cfg(feature = "controller")]
pub use desired_state::{
    reconcile_ready, DesiredStateBridge, DesiredStateSource, HttpComponentsClient, ReadyBinding,
    MANIFESTO_BASE_URL_ENV, MANIFESTO_BEARER_ENV, MANIFESTO_PROJECT_ID_ENV,
};

/// Version du paquet, affichée par `--version` / `-V`.
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Affiche [`PKG_VERSION`] sur stdout si `--version` ou `-V` est demandé.
pub fn print_version_if_requested() {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("{PKG_VERSION}");
    }
}
