//! Identités Apparatus P0 : `apparatus_id`, `binding_id`, `operation_id`, digest.
//!
//! Tous les parseurs sont non paniquants (`FromStr` / `TryFrom`) et appliquent
//! les bornes de [`crate::limits`]. Les implémentations `Deserialize` valident
//! à la frontière wire : toute valeur invalide est rejetée sans panic.

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::error::ApparatusError;
use crate::limits::MAX_ID_LEN;

/// Préfixe canonique d'un digest de release.
pub const DIGEST_PREFIX: &str = "sha256:";

/// Longueur hexadécimale attendue après le préfixe `sha256:` (SHA-256 = 32 octets).
pub const DIGEST_HEX_LEN: usize = 64;

/// Identité d'un Apparatus (ex. `io.aiforall.reference-kv`).
///
/// Règles P0 : non vide, au plus [`MAX_ID_LEN`] caractères, minuscules ASCII,
/// chiffres, `.`, `_`, `-`, contient au moins un `.`, ne commence ni ne finit
/// par un point.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApparatusId(String);

impl ApparatusId {
    /// Construit un identifiant après validation.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::IdTooLong`] ou [`ApparatusError::InvalidId`]
    /// si la valeur viole les règles P0.
    pub fn new(value: &str) -> Result<Self, ApparatusError> {
        value.parse()
    }

    /// Retourne la valeur validée.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ApparatusId {
    type Err = ApparatusError;

    /// Parse un `apparatus_id` reverse-domain minuscule.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::IdTooLong`] ou [`ApparatusError::InvalidId`]
    /// si la valeur viole les règles P0.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let len = value.chars().count();
        if len > MAX_ID_LEN {
            return Err(ApparatusError::IdTooLong {
                max: MAX_ID_LEN,
                actual: len,
            });
        }
        if value.is_empty() {
            return Err(ApparatusError::InvalidId {
                reason: "apparatus id is empty".to_owned(),
            });
        }
        let well_formed = value.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-'
        }) && value.contains('.')
            && !value.starts_with('.')
            && !value.ends_with('.');
        if well_formed {
            Ok(Self(value.to_owned()))
        } else {
            Err(ApparatusError::InvalidId {
                reason: "apparatus id must be lowercase reverse-domain".to_owned(),
            })
        }
    }
}

impl Display for ApparatusId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ApparatusId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Serialize for ApparatusId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ApparatusId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(DeError::custom)
    }
}

/// Identité d'un binding (opaque, namespacée par Manifesto).
///
/// Règles P0 : non vide, au plus [`MAX_ID_LEN`] caractères,
/// `[A-Za-z0-9._-]`. Les UUID sont acceptés car ils respectent cet alphabet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BindingId(String);

impl BindingId {
    /// Construit un identifiant après validation.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::IdTooLong`] ou [`ApparatusError::InvalidId`]
    /// si la valeur viole les règles P0.
    pub fn new(value: &str) -> Result<Self, ApparatusError> {
        value.parse()
    }

    /// Retourne la valeur validée.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for BindingId {
    type Err = ApparatusError;

    /// Parse un `binding_id` opaque `[A-Za-z0-9._-]`.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::IdTooLong`] ou [`ApparatusError::InvalidId`]
    /// si la valeur viole les règles P0.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let len = value.chars().count();
        if len > MAX_ID_LEN {
            return Err(ApparatusError::IdTooLong {
                max: MAX_ID_LEN,
                actual: len,
            });
        }
        if value.is_empty() {
            return Err(ApparatusError::InvalidId {
                reason: "binding id is empty".to_owned(),
            });
        }
        let well_formed = value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-');
        if well_formed {
            Ok(Self(value.to_owned()))
        } else {
            Err(ApparatusError::InvalidId {
                reason: "binding id must match [A-Za-z0-9._-]".to_owned(),
            })
        }
    }
}

impl Display for BindingId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for BindingId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Serialize for BindingId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for BindingId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(DeError::custom)
    }
}

/// Identifiant d'opération pour l'idempotence (`bind`/`configure`/`invoke`/`unbind`).
///
/// UUID v4 sérialisé en string canonique. Le rejeu du même `operation_id`
/// rejoue la même réponse sans nouvel effet de bord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperationId(Uuid);

impl OperationId {
    /// Génère un nouvel identifiant aléatoire (UUID v4).
    ///
    /// Réservé à la production : les tests déterministes utilisent
    /// [`OperationId::from_bytes`].
    ///
    /// # Panics
    ///
    /// Ne panique jamais en pratique ; utilise le RNG système via
    /// `Uuid::new_v4`.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construit un identifiant déterministe depuis 16 octets.
    ///
    /// Les bits de version (4) et de variante (RFC 4122) sont forcés pour
    /// produire un UUID bien formé : même entrée ⇒ même sortie, sans RNG.
    /// Utilisé par les tests déterministes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        let mut shaped = bytes;
        shaped[6] = (shaped[6] & 0x0f) | 0x40;
        shaped[8] = (shaped[8] & 0x3f) | 0x80;
        Self(Uuid::from_bytes(shaped))
    }

    /// Retourne l'UUID sous-jacent.
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for OperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for OperationId {
    type Err = ApparatusError;

    /// Parse un `operation_id` UUID canonique.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidId`] si le texte n'est pas un UUID.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| ApparatusError::InvalidId {
                reason: "operation id must be an uuid".to_owned(),
            })
    }
}

impl Display for OperationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

/// Digest immuable d'une release : identité d'installation.
///
/// Format canonique : `sha256:` suivi de 64 caractères hexadécimaux minuscules.
/// Voir [`crate::digest`] pour l'algorithme et la canonicalisation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReleaseDigest(String);

impl ReleaseDigest {
    /// Construit un digest après validation du format.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidDigest`] si le format n'est pas canonique.
    pub fn new(value: &str) -> Result<Self, ApparatusError> {
        value.parse()
    }

    /// Construit un digest depuis 32 octets bruts (SHA-256).
    #[must_use]
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let mut out = String::with_capacity(DIGEST_PREFIX.len() + DIGEST_HEX_LEN);
        out.push_str(DIGEST_PREFIX);
        for byte in bytes {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        Self(out)
    }

    /// Retourne la forme canonique (`sha256:` + hex).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Retourne la partie hexadécimale (sans préfixe).
    #[must_use]
    pub fn hex_part(&self) -> &str {
        self.0
            .strip_prefix(DIGEST_PREFIX)
            .map_or(self.0.as_str(), |hex| hex)
    }
}

impl FromStr for ReleaseDigest {
    type Err = ApparatusError;

    /// Parse un digest `sha256:` + 64 hex minuscules.
    ///
    /// # Errors
    ///
    /// Retourne [`ApparatusError::InvalidDigest`] si le format n'est pas canonique.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some(hex) = value.strip_prefix(DIGEST_PREFIX) else {
            return Err(ApparatusError::InvalidDigest {
                reason: "digest must start with sha256:".to_owned(),
            });
        };
        let valid = hex.len() == DIGEST_HEX_LEN && hex.chars().all(|c| c.is_ascii_hexdigit());
        if !valid {
            return Err(ApparatusError::InvalidDigest {
                reason: "digest must carry 64 hex chars".to_owned(),
            });
        }
        if hex.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(ApparatusError::InvalidDigest {
                reason: "digest hex must be lowercase".to_owned(),
            });
        }
        Ok(Self(value.to_owned()))
    }
}

impl Display for ReleaseDigest {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ReleaseDigest {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Serialize for ReleaseDigest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ReleaseDigest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(DeError::custom)
    }
}
