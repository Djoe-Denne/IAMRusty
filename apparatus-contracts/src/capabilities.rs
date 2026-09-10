//! Taxonomie minimale des capacités Apparatus P0.
//!
//! Une capacité inconnue est un refus ferme ([`ApparatusError::UnknownCapability`]).
//! P0 ne connaît que la lecture projet et le KV plateforme ; aucune capacité
//! réseau n'existe, donc la référence KV ne peut pas en déclarer.

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ApparatusError;

/// Capacité déclarable par un manifeste P0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Capability {
    /// Lecture des métadonnées du projet hôte (`project.read`).
    ProjectRead,
    /// Lecture du KV plateforme namespacé par binding (`storage.kv.read`).
    StorageKvRead,
    /// Écriture du KV plateforme namespacé par binding (`storage.kv.write`).
    StorageKvWrite,
}

impl Capability {
    /// Nom canonique sur le wire (`project.read`, …).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ProjectRead => "project.read",
            Self::StorageKvRead => "storage.kv.read",
            Self::StorageKvWrite => "storage.kv.write",
        }
    }

    /// Liste exhaustive des capacités P0.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::ProjectRead, Self::StorageKvRead, Self::StorageKvWrite]
    }

    /// Indique si la capacité accorde un accès réseau.
    ///
    /// Toujours `false` en P0 : aucune capacité réseau n'est définie.
    /// Toute chaîne réseau inconnue est rejetée au parsing.
    #[must_use]
    pub const fn is_network(&self) -> bool {
        false
    }

    /// Parse une liste de noms de capacités.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::UnknownCapability`] dès le premier nom inconnu.
    pub fn parse_list(names: &[String]) -> Result<Vec<Self>, ApparatusError> {
        names.iter().map(|name| name.parse()).collect()
    }
}

impl FromStr for Capability {
    type Err = ApparatusError;

    /// Parse un nom de capacité P0.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::UnknownCapability`] si le nom est inconnu.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "project.read" => Ok(Self::ProjectRead),
            "storage.kv.read" => Ok(Self::StorageKvRead),
            "storage.kv.write" => Ok(Self::StorageKvWrite),
            other => Err(ApparatusError::UnknownCapability {
                name: ApparatusError::truncate(other, 100),
            }),
        }
    }
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(self.as_str())
    }
}

impl Serialize for Capability {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Capability {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(DeError::custom)
    }
}

/// Indique si une liste de capacités contient un accès réseau.
///
/// Retourne toujours `false` en P0 (aucune capacité réseau définie).
/// Conservé comme point d'extension documenté pour P3.
#[must_use]
pub const fn requires_network(caps: &[Capability]) -> bool {
    let mut index = 0;
    while index < caps.len() {
        if caps[index].is_network() {
            return true;
        }
        index += 1;
    }
    false
}
