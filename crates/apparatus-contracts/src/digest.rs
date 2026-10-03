//! Digest SHA-256 déterministe du descripteur canonique.
//!
//! ## Canonicalisation P0
//!
//! 1. Le manifeste est converti en [`serde_json::Value`] via `serde`.
//! 2. Les objets JSON sont triés par clé : la fonction [`canonical_value`]
//!    reconstruit chaque objet dans un [`std::collections::BTreeMap`], donc
//!    l'ordre des clés de sortie est l'ordre lexicographique des octets UTF-8,
//!    indépendant de l'ordre TOML d'entrée.
//! 3. La valeur canonique est sérialisée avec [`serde_json::to_string`]
//!    (JSON compact, sans espaces ni retours ligne).
//! 4. L'empreinte est `SHA-256` de ces octets UTF-8.
//!
//! ## Format
//!
//! `sha256:` suivi de 64 caractères hexadécimaux minuscules (formatage manuel,
//! sans crate `hex`). Voir [`crate::ids::ReleaseDigest`].
//!
//! Toute mutation sémantique (id, version, protocole, capacités, UI) change le
//! digest ; un simple réordonnancement des clés TOML le conserve.

use std::collections::BTreeMap;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::ApparatusError;
use crate::ids::ReleaseDigest;
use crate::manifest::ApparatusManifest;

/// Retourne la forme canonique d'une valeur JSON (objets triés récursivement).
#[must_use]
pub fn canonical_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<&String, Value> = map
                .iter()
                .map(|(key, val)| (key, canonical_value(val)))
                .collect();
            let owned: serde_json::Map<String, Value> = sorted
                .into_iter()
                .map(|(key, val)| (key.clone(), val))
                .collect();
            Value::Object(owned)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_value).collect()),
        other => other.clone(),
    }
}

/// Sérialise une valeur en JSON canonique compact.
///
/// # Errors
///
/// Retourne [`ApparatusError::Json`] si la sérialisation échoue
/// (impossible en pratique pour nos types, mais gérée en `Result`).
pub fn canonical_json(value: &Value) -> Result<String, ApparatusError> {
    let canon = canonical_value(value);
    serde_json::to_string(&canon).map_err(|err| ApparatusError::Json {
        message: ApparatusError::truncate(&err.to_string(), 200),
    })
}

/// Calcule le digest d'octets bruts (SHA-256, format `sha256:` + hex).
#[must_use]
pub fn digest_bytes(bytes: &[u8]) -> ReleaseDigest {
    let hash = Sha256::digest(bytes);
    let mut raw = [0_u8; 32];
    raw.copy_from_slice(&hash);
    ReleaseDigest::from_bytes(&raw)
}

/// Calcule le digest d'une chaîne UTF-8.
#[must_use]
pub fn digest_str(content: &str) -> ReleaseDigest {
    digest_bytes(content.as_bytes())
}

/// Calcule le digest d'une valeur JSON après canonicalisation.
///
/// # Errors
///
/// Retourne [`ApparatusError::Json`] si la sérialisation canonique échoue.
pub fn digest_json(value: &Value) -> Result<ReleaseDigest, ApparatusError> {
    canonical_json(value).map(|canon| digest_str(&canon))
}

/// Calcule le digest immuable d'un manifeste parsé.
///
/// Le digest couvre l'intégralité du descripteur canonique (identité,
/// version, protocole, capacités, UI).
///
/// # Errors
///
/// Retourne [`ApparatusError::Json`] si la conversion du manifeste échoue
/// (impossible en pratique pour nos types, mais gérée en `Result`).
pub fn digest_manifest(manifest: &ApparatusManifest) -> Result<ReleaseDigest, ApparatusError> {
    let value = serde_json::to_value(manifest).map_err(|err| ApparatusError::Json {
        message: ApparatusError::truncate(&err.to_string(), 200),
    })?;
    digest_json(&value)
}
