//! Schéma versionné de `apparatus.toml` (syntaxe unique P0).
//!
//! La version de schéma (`schema_version`) est distincte de la version SDK et
//! du protocole wire [`crate::protocol::PROTOCOL_ID`]. La version `SemVer` du
//! manifeste est déclarative ; l'identité d'installation est le digest
//! ([`crate::digest`]). Toute structure entrante applique `deny_unknown_fields`.

use serde::{Deserialize, Serialize};

use crate::ids::ApparatusId;
use crate::ui::UiDeclaration;

/// Manifeste `apparatus.toml` parsé (avant validation métier).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApparatusManifest {
    /// Identité et versions déclaratives.
    pub apparatus: ApparatusMeta,
    /// Déclaration du backend privé.
    pub backend: BackendDecl,
    /// Capacités requises.
    pub capabilities: CapabilitiesDecl,
    /// Déclaration UI.
    pub ui: UiDeclaration,
}

/// Métadonnées d'identité du manifeste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApparatusMeta {
    /// Identité reverse-domain (ex. `io.aiforall.reference-kv`).
    pub id: ApparatusId,
    /// Version déclarative `SemVer` (ex. `0.1.0`). Distincte du digest d'installation.
    pub version: String,
    /// Version majeure du schéma de manifeste (P0 : `1` uniquement).
    pub schema_version: u32,
    /// Nom d'affichage optionnel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Description optionnelle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Déclaration du backend privé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendDecl {
    /// Protocole wire, P0 : exactement `manifesto-apparatus/1`.
    pub protocol: String,
    /// Adaptateur de stockage déclaré (ex. `kv-v1` pour la référence).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
}

/// Capacités requises par la release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilitiesDecl {
    /// Noms de capacités (`project.read`, `storage.kv.read`, `storage.kv.write`).
    ///
    /// Stockés en `String` pour que le validateur unique produise l'erreur
    /// stable [`crate::error::ApparatusError::UnknownCapability`].
    #[serde(default)]
    pub requires: Vec<String>,
}
