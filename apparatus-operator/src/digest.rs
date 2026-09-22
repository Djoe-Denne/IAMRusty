//! Git → digest canonique (ADR-0002).
//!
//! Wrap de [`apparatus_contracts`] : l'identité d'installation est un
//! [`ReleaseDigest`], jamais une référence Git flottante. Pas de second schéma.

pub use apparatus_contracts::{
    digest_manifest, is_floating_ref, validate_install_ref, ApparatusError, ReleaseDigest,
};

/// Convertit une référence d'installation en [`ReleaseDigest`] canonique.
///
/// Délègue à [`validate_install_ref`] puis [`ReleaseDigest::new`]. Le futur
/// contrôleur ne consomme que ce digest.
///
/// # Errors
///
/// Retourne [`ApparatusError::FloatingRef`] (`APPARATUS_FLOATING_REF`) si la
/// référence est flottante, ou [`ApparatusError::InvalidDigest`]
/// (`APPARATUS_INVALID_DIGEST`) si ce n'est pas un digest `sha256:` suivi de
/// 64 hexadécimaux minuscules.
pub fn resolve_install_ref(reference: &str) -> Result<ReleaseDigest, ApparatusError> {
    apparatus_contracts::validate_install_ref(reference)?;
    apparatus_contracts::ReleaseDigest::new(reference)
}
