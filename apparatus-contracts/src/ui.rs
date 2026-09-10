//! Modes UI déclaratifs Apparatus P0 : `absent` | `schema` | `sandbox`.
//!
//! Déclaration seule : aucun host, iframe, bundle ni bridge n'est créé ici.
//! Le mode `sandbox` est accepté comme déclaration statique sans conclure à
//! une isolation de production (décidée avant P5).

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ApparatusError;
use crate::limits::MAX_SCHEMA_PATH_LEN;

/// Mode UI déclaré par le manifeste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum UiMode {
    /// Aucune UI.
    Absent,
    /// UI générée depuis un schéma JSON déclaré (`schema_path` requis).
    Schema,
    /// Bundle statique sandboxé (déclaration seule, sans host en P0).
    Sandbox,
}

impl Display for UiMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Absent => f.write_str("absent"),
            Self::Schema => f.write_str("schema"),
            Self::Sandbox => f.write_str("sandbox"),
        }
    }
}

impl FromStr for UiMode {
    type Err = ApparatusError;

    /// Parse un mode UI (`absent` | `schema` | `sandbox`).
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidManifest`] si le mode est inconnu.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "absent" => Ok(Self::Absent),
            "schema" => Ok(Self::Schema),
            "sandbox" => Ok(Self::Sandbox),
            _ => Err(ApparatusError::InvalidManifest {
                message: "ui.mode must be absent|schema|sandbox".to_owned(),
            }),
        }
    }
}

/// Déclaration UI du manifeste.
///
/// Aucun champ host/URL : `deny_unknown_fields` rejette tout ajout de ce type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiDeclaration {
    /// Mode UI.
    pub mode: UiMode,
    /// Chemin relatif du schéma JSON (requis si `mode = schema`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_path: Option<String>,
}

impl UiDeclaration {
    /// Valide la cohérence de la déclaration.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidManifest`] si `schema` n'a pas de
    /// `schema_path`, si `absent` en déclare un, ou si le chemin est
    /// absolu, remonte (`..`) ou dépasse [`MAX_SCHEMA_PATH_LEN`].
    pub fn validate(&self) -> Result<(), ApparatusError> {
        match (&self.mode, &self.schema_path) {
            (UiMode::Absent, Some(_)) => Err(ApparatusError::InvalidManifest {
                message: "ui.schema_path must be absent when mode is absent".to_owned(),
            }),
            (UiMode::Schema, None) => Err(ApparatusError::InvalidManifest {
                message: "ui.schema_path is required when mode is schema".to_owned(),
            }),
            (UiMode::Schema | UiMode::Sandbox, Some(path)) => Self::validate_schema_path(path),
            (UiMode::Absent | UiMode::Sandbox, None) => Ok(()),
        }
    }

    /// Valide un chemin de schéma : relatif, sans remontée, borné.
    fn validate_schema_path(path: &str) -> Result<(), ApparatusError> {
        if path.is_empty() || path.chars().count() > MAX_SCHEMA_PATH_LEN {
            return Err(ApparatusError::InvalidManifest {
                message: "ui.schema_path length out of bounds".to_owned(),
            });
        }
        if path.starts_with('/') || path.contains("..") || path.contains('\\') {
            return Err(ApparatusError::InvalidManifest {
                message: "ui.schema_path must be a relative path".to_owned(),
            });
        }
        Ok(())
    }
}
