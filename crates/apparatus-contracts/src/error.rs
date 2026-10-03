//! Erreurs stables et versionnées des contrats Apparatus P0.
//!
//! Chaque variante expose un [`ApparatusError::code`] stable (`APPARATUS_*`).
//! Les messages [`core::fmt::Display`] ne contiennent jamais de secret, de bearer,
//! de mot de passe ni de contenu de payload : seules des longueurs, des raisons
//! et des noms de champs tronqués sont rapportés.

use thiserror::Error;

/// Erreur de contrat Apparatus.
///
/// Les codes retournés par [`ApparatusError::code`] font partie du contrat stable P0.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ApparatusError {
    /// Identifiant invalide (caractères interdits, vide, format inattendu).
    #[error("invalid id: {reason}")]
    InvalidId {
        /// Raison courte et stable, sans contenu sensible.
        reason: String,
    },
    /// Identifiant trop long.
    #[error("id too long: {actual} chars, max {max}")]
    IdTooLong {
        /// Borne appliquée.
        max: usize,
        /// Longueur constatée (en caractères).
        actual: usize,
    },
    /// Digest de release invalide.
    #[error("invalid release digest: {reason}")]
    InvalidDigest {
        /// Raison courte et stable.
        reason: String,
    },
    /// Manifeste invalide (structure ou valeur rejetée).
    #[error("invalid manifest: {message}")]
    InvalidManifest {
        /// Message tronqué, sans secret.
        message: String,
    },
    /// Version majeure de schéma inconnue.
    #[error("unknown schema major: got {got}, supported {supported}")]
    UnknownSchemaMajor {
        /// Majeur constaté.
        got: u32,
        /// Majeur supporté par ce validateur.
        supported: u32,
    },
    /// Capacité inconnue : refus ferme (aucune capacité implicite).
    #[error("unknown capability: {name}")]
    UnknownCapability {
        /// Nom de la capacité rejetée (tronqué).
        name: String,
    },
    /// Référence d'installation flottante (`latest`, branche, tag) : interdite.
    #[error("floating install ref rejected: {reference}")]
    FloatingRef {
        /// Référence rejetée (tronquée, jamais un secret).
        reference: String,
    },
    /// Champ interdit (`trusted_*`, `credentials`, secrets).
    #[error("forbidden field: {field}")]
    ForbiddenField {
        /// Nom du champ rejeté (tronqué).
        field: String,
    },
    /// Payload trop volumineux.
    #[error("payload too large: {actual} bytes, max {max}")]
    PayloadTooLarge {
        /// Borne appliquée.
        max: usize,
        /// Taille constatée (en octets).
        actual: usize,
    },
    /// Configuration trop volumineuse.
    #[error("config too large: {actual} bytes, max {max}")]
    ConfigTooLarge {
        /// Borne appliquée.
        max: usize,
        /// Taille constatée (en octets).
        actual: usize,
    },
    /// Opération `invoke` invalide (nom vide, trop long, paramètres hors bornes).
    #[error("invalid operation: {reason}")]
    InvalidOperation {
        /// Raison courte et stable.
        reason: String,
    },
    /// Échec de parsing TOML (message tronqué pour ne pas fuir le contenu).
    #[error("toml parse error: {message}")]
    TomlParse {
        /// Message d'erreur tronqué.
        message: String,
    },
    /// Échec de parsing ou de sérialisation JSON (message tronqué).
    #[error("json error: {message}")]
    Json {
        /// Message d'erreur tronqué.
        message: String,
    },
    /// Version `SemVer` invalide.
    #[error("invalid semver: {message}")]
    Semver {
        /// Message d'erreur tronqué.
        message: String,
    },
}

impl ApparatusError {
    /// Code d'erreur stable (`APPARATUS_*`), partie du contrat versionné.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidId { .. } => "APPARATUS_INVALID_ID",
            Self::IdTooLong { .. } => "APPARATUS_ID_TOO_LONG",
            Self::InvalidDigest { .. } => "APPARATUS_INVALID_DIGEST",
            Self::InvalidManifest { .. } => "APPARATUS_INVALID_MANIFEST",
            Self::UnknownSchemaMajor { .. } => "APPARATUS_UNKNOWN_SCHEMA_MAJOR",
            Self::UnknownCapability { .. } => "APPARATUS_UNKNOWN_CAPABILITY",
            Self::FloatingRef { .. } => "APPARATUS_FLOATING_REF",
            Self::ForbiddenField { .. } => "APPARATUS_FORBIDDEN_FIELD",
            Self::PayloadTooLarge { .. } => "APPARATUS_PAYLOAD_TOO_LARGE",
            Self::ConfigTooLarge { .. } => "APPARATUS_CONFIG_TOO_LARGE",
            Self::InvalidOperation { .. } => "APPARATUS_INVALID_OPERATION",
            Self::TomlParse { .. } => "APPARATUS_TOML_PARSE",
            Self::Json { .. } => "APPARATUS_JSON",
            Self::Semver { .. } => "APPARATUS_SEMVER",
        }
    }

    /// Tronque un fragment libre à `max_chars` caractères pour éviter toute fuite.
    #[must_use]
    pub fn truncate(fragment: &str, max_chars: usize) -> String {
        if fragment.chars().count() <= max_chars {
            fragment.to_owned()
        } else {
            let kept: String = fragment.chars().take(max_chars).collect();
            format!("{kept}…")
        }
    }
}
