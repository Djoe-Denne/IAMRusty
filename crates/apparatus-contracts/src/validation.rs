//! Validateur unique et déterministe des manifestes Apparatus P0.
//!
//! Point d'entrée : [`validate_manifest_toml`]. Le validateur rejette
//! `latest` et les refs flottantes, les majeurs de schéma inconnus, les
//! capacités inconnues, les champs `trusted_*` / `credentials` et toute
//! version non `SemVer`. Aucune E/S, aucun accès réseau, résultat déterministe.

use crate::capabilities::Capability;
use crate::digest::digest_manifest;
use crate::error::ApparatusError;
use crate::ids::ReleaseDigest;
use crate::limits::{MAX_CAPABILITIES, MAX_DESCRIPTION_LEN, MAX_INSTALL_REF_LEN};
use crate::manifest::ApparatusManifest;
use crate::protocol::PROTOCOL_ID;

/// Majeur de schéma supporté par ce validateur P0.
pub const SUPPORTED_SCHEMA_MAJOR: u32 = 1;

/// Manifeste validé accompagné de son digest immuable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedManifest {
    /// Manifeste parsé et validé.
    pub manifest: ApparatusManifest,
    /// Digest immuable du descripteur canonique (identité d'installation).
    pub digest: ReleaseDigest,
}

/// Valide un manifeste `apparatus.toml` et calcule son digest.
///
/// La validation est déterministe : même entrée ⇒ même acceptation/refus et
/// même digest.
///
/// # Errors
///
/// Retourne [`ApparatusError::ForbiddenField`] si un champ interdit est
/// détecté, [`ApparatusError::TomlParse`] si le TOML est illisible,
/// [`ApparatusError::UnknownSchemaMajor`], [`ApparatusError::FloatingRef`],
/// [`ApparatusError::Semver`], [`ApparatusError::InvalidManifest`],
/// [`ApparatusError::UnknownCapability`] ou [`ApparatusError::Json`].
pub fn validate_manifest_toml(input: &str) -> Result<ValidatedManifest, ApparatusError> {
    scan_forbidden_toml(input)?;
    let manifest: ApparatusManifest =
        toml::from_str(input).map_err(|err| ApparatusError::TomlParse {
            message: ApparatusError::truncate(&err.to_string(), 300),
        })?;
    validate_manifest(&manifest)?;
    let digest = digest_manifest(&manifest)?;
    Ok(ValidatedManifest { manifest, digest })
}

/// Valide un manifeste déjà parsé (sans recalculer le digest).
///
/// # Errors
///
/// Retourne les mêmes erreurs métier que [`validate_manifest_toml`],
/// hors erreurs de parsing TOML et de digest.
pub fn validate_manifest(manifest: &ApparatusManifest) -> Result<(), ApparatusError> {
    if manifest.apparatus.schema_version != SUPPORTED_SCHEMA_MAJOR {
        return Err(ApparatusError::UnknownSchemaMajor {
            got: manifest.apparatus.schema_version,
            supported: SUPPORTED_SCHEMA_MAJOR,
        });
    }
    validate_declared_version(&manifest.apparatus.version)?;
    if manifest.backend.protocol != PROTOCOL_ID {
        return Err(ApparatusError::InvalidManifest {
            message: "backend.protocol must be manifesto-apparatus/1".to_owned(),
        });
    }
    if manifest.capabilities.requires.len() > MAX_CAPABILITIES {
        return Err(ApparatusError::InvalidManifest {
            message: "too many capabilities declared".to_owned(),
        });
    }
    Capability::parse_list(&manifest.capabilities.requires).map(|_| ())?;
    manifest.ui.validate()?;
    scan_manifest_values(manifest)?;
    Ok(())
}

/// Valide la version déclarative : `SemVer` stricte, jamais flottante.
///
/// # Errors
///
/// Retourne [`ApparatusError::FloatingRef`] pour `latest` et refs flottantes,
/// [`ApparatusError::Semver`] si le texte n'est pas une `SemVer` valide.
pub fn validate_declared_version(version: &str) -> Result<(), ApparatusError> {
    if is_floating_ref(version) {
        return Err(ApparatusError::FloatingRef {
            reference: ApparatusError::truncate(version, 100),
        });
    }
    semver::Version::parse(version)
        .map(|_| ())
        .map_err(|err| ApparatusError::Semver {
            message: ApparatusError::truncate(&err.to_string(), 200),
        })
}

/// Valide une référence d'installation : seul un digest est accepté.
///
/// L'identité d'installation est le digest du descripteur canonique
/// (ADR-0002). `latest`, branches, tags et `SemVer` sont rejetés comme
/// identité d'installation.
///
/// # Errors
///
/// Retourne [`ApparatusError::FloatingRef`] pour les refs flottantes,
/// [`ApparatusError::InvalidDigest`] sinon (y compris `SemVer`, qui n'est
/// qu'une version déclarative).
pub fn validate_install_ref(reference: &str) -> Result<(), ApparatusError> {
    if reference.chars().count() > MAX_INSTALL_REF_LEN {
        return Err(ApparatusError::InvalidDigest {
            reason: "install ref too long".to_owned(),
        });
    }
    if let Ok(_digest) = ReleaseDigest::new(reference) {
        return Ok(());
    }
    if is_floating_ref(reference) {
        return Err(ApparatusError::FloatingRef {
            reference: ApparatusError::truncate(reference, 100),
        });
    }
    Err(ApparatusError::InvalidDigest {
        reason: "install ref must be a sha256: digest".to_owned(),
    })
}

/// Indique si une référence est flottante (`latest`, branche, tag, …).
#[must_use]
pub fn is_floating_ref(reference: &str) -> bool {
    const FLOATING: &[&str] = &[
        "latest", "main", "master", "stable", "head", "trunk", "default", "next",
    ];
    if FLOATING.contains(&reference) {
        return true;
    }
    reference.starts_with("branches/")
        || reference.starts_with("tags/")
        || reference.starts_with("heads/")
        || reference.starts_with("refs/")
        || reference.contains("..")
        || reference.contains(' ')
        || reference.contains('\t')
        || reference.contains('\n')
}

/// Balaye le TOML brut à la recherche de champs interdits.
///
/// Complète `deny_unknown_fields` (erreur typée) par l'erreur stable
/// [`ApparatusError::ForbiddenField`], y compris pour des champs imbriqués.
fn scan_forbidden_toml(input: &str) -> Result<(), ApparatusError> {
    let table: toml::Table = toml::from_str(input).map_err(|err| ApparatusError::TomlParse {
        message: ApparatusError::truncate(&err.to_string(), 300),
    })?;
    scan_toml_value(&toml::Value::Table(table))
}

/// Indique si un nom de clé est interdit dans les contrats P0.
///
/// Comparaison insensible à la casse : `Trusted_Skip`, `CREDENTIALS` ou
/// `Secret` sont rejetés comme leurs formes minuscules.
fn is_forbidden_key(key: &str) -> bool {
    const FORBIDDEN_EXACT: &[&str] = &[
        "trusted_skip_gateway",
        "trusted_skip",
        "credentials",
        "credential",
        "secrets",
        "secret",
        "password",
        "token",
        "bearer",
        "hmac_secret",
        "jwt",
    ];
    let lowered = key.to_ascii_lowercase();
    lowered.starts_with("trusted_") || FORBIDDEN_EXACT.contains(&lowered.as_str())
}

/// Balaye récursivement une valeur TOML à la recherche de clés interdites.
fn scan_toml_value(value: &toml::Value) -> Result<(), ApparatusError> {
    match value {
        toml::Value::Table(map) => {
            for (key, nested) in map {
                if is_forbidden_key(key) {
                    return Err(ApparatusError::ForbiddenField {
                        field: ApparatusError::truncate(key, 100),
                    });
                }
                scan_toml_value(nested)?;
            }
            Ok(())
        }
        toml::Value::Array(items) => {
            for item in items {
                scan_toml_value(item)?;
            }
            Ok(())
        }
        toml::Value::String(_)
        | toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => Ok(()),
    }
}

/// Balaye les valeurs libres du manifeste (nom, description, stockage).
fn scan_manifest_values(manifest: &ApparatusManifest) -> Result<(), ApparatusError> {
    if let Some(name) = &manifest.apparatus.name {
        crate::protocol::validate_short_text(name)?;
    }
    if let Some(description) = &manifest.apparatus.description {
        if description.chars().count() > MAX_DESCRIPTION_LEN {
            return Err(ApparatusError::InvalidManifest {
                message: "apparatus.description too long".to_owned(),
            });
        }
    }
    if let Some(storage) = &manifest.backend.storage {
        crate::protocol::validate_short_text(storage)?;
    }
    Ok(())
}

/// Vérifie qu'une valeur JSON ne porte aucune clé à secret (TEST-ONLY).
///
/// Disponible uniquement avec la feature `test-harness`. Utilisé par les
/// tests pour prouver l'absence de secrets dans les DTO sérialisés.
/// Les clés inspectées : `password`, `secret`, `secrets`, `token`,
/// `bearer`, `credentials`, `hmac_secret`, `jwt` (insensible à la casse).
///
/// # Errors
///
/// Retourne [`ApparatusError::ForbiddenField`] si une clé à secret est trouvée.
#[cfg(feature = "test-harness")]
pub fn assert_no_secret_keys(value: &serde_json::Value) -> Result<(), ApparatusError> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, nested) in map {
                if is_forbidden_key(key) {
                    return Err(ApparatusError::ForbiddenField {
                        field: ApparatusError::truncate(key, 100),
                    });
                }
                assert_no_secret_keys(nested)?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => {
            for item in items {
                assert_no_secret_keys(item)?;
            }
            Ok(())
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => Ok(()),
    }
}
