//! Mapping legacy→Apparatus injectif et erreur versionnée de collision (T3).
//!
//! Table canonique minimale : chaque type legacy (`taskboard`, `wiki`)
//! pointe vers une identité Apparatus distincte. Deux types legacy vers la
//! même cible = [`MappingCollision`] (code stable
//! [`APPARATUS_MAPPING_COLLISION_CODE`], contrat v1), jamais de fusion
//! silencieuse, jamais de second UUID public.
//!
//! Complète la contrainte DB `project_components_unique` : sous course réelle,
//! le second ajout concurrent est tranché par la base et [`is_unique_violation`]
//! le convertit en conflit 409 (pas un `if` applicatif, pas une erreur interne).

use sea_orm::DbErr;
use std::collections::HashMap;
use std::fmt;

/// Version du contrat d'erreur T3 (code stable ci-dessous).
pub const APPARATUS_MAPPING_CONTRACT_VERSION: u32 = 1;

/// Code d'erreur versionné stable : collision de mapping legacy→Apparatus.
pub const APPARATUS_MAPPING_COLLISION_CODE: &str = "APPARATUS_MAPPING_COLLISION";

/// Collision de mapping : deux types legacy distincts vers la même cible.
///
/// Refus ferme, sans fusion silencieuse et sans second UUID public :
/// l'identité d'installation reste `project_components.id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingCollision {
    /// Cible Apparatus convoitée par deux types legacy.
    target: String,
    /// Premier type legacy déclaré vers `target`.
    first_legacy: String,
    /// Second type legacy rejeté vers `target`.
    second_legacy: String,
}

impl MappingCollision {
    /// Construit une collision documentée (champs tronqués, sans secret).
    #[must_use]
    pub fn new(target: &str, first_legacy: &str, second_legacy: &str) -> Self {
        Self {
            target: truncate(target, 64),
            first_legacy: truncate(first_legacy, 64),
            second_legacy: truncate(second_legacy, 64),
        }
    }

    /// Code stable versionné (`APPARATUS_MAPPING_COLLISION`, contrat v1).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        APPARATUS_MAPPING_COLLISION_CODE
    }

    /// Version du contrat d'erreur.
    #[must_use]
    pub const fn contract_version(&self) -> u32 {
        APPARATUS_MAPPING_CONTRACT_VERSION
    }

    /// Cible Apparatus convoitée.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }
}

impl fmt::Display for MappingCollision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: legacy '{}' et '{}' vers '{}' (contrat v{})",
            self.code(),
            self.first_legacy,
            self.second_legacy,
            self.target,
            self.contract_version()
        )
    }
}

impl std::error::Error for MappingCollision {}

fn truncate(fragment: &str, max_chars: usize) -> String {
    if fragment.chars().count() <= max_chars {
        fragment.to_owned()
    } else {
        let kept: String = fragment.chars().take(max_chars).collect();
        format!("{kept}…")
    }
}

/// Table canonique legacy→Apparatus (injective : cibles distinctes).
///
/// Retourne `None` pour un type legacy inconnu (appelant : rejet ferme).
#[must_use]
pub fn legacy_to_apparatus(legacy: &str) -> Option<&'static str> {
    match legacy {
        "taskboard" => Some("io.aiforall.taskboard"),
        "wiki" => Some("io.aiforall.wiki"),
        _ => None,
    }
}

/// Vérifie qu'un lot de paires `(legacy, cible)` reste injectif.
///
/// Deux paires distinctes vers la même cible = `Err(MappingCollision)`.
/// Une paire répétée à l'identique reste acceptée (idempotence, pas une collision).
///
/// # Errors
///
/// Retourne [`MappingCollision`] si deux types legacy distincts visent la même cible.
pub fn check_pairs_injective(pairs: &[(&str, &str)]) -> Result<(), MappingCollision> {
    let mut seen: HashMap<&str, &str> = HashMap::new();
    for (legacy, target) in pairs {
        if let Some(first) = seen.insert(*target, *legacy) {
            if first != *legacy {
                return Err(MappingCollision::new(target, first, legacy));
            }
        }
    }
    Ok(())
}

/// Détecte une violation d'unicité Postgres (SQLSTATE 23505) dans `DbErr`.
///
/// Utilisé par le dépôt composants : sous course réelle, la contrainte
/// `project_components_unique` tranche et l'écriture fautive devient un
/// conflit 409 au lieu d'une erreur interne.
#[must_use]
pub fn is_unique_violation(err: &DbErr) -> bool {
    let runtime_err = match err {
        DbErr::Exec(err) | DbErr::Query(err) => err,
        _ => return false,
    };
    match runtime_err {
        sea_orm::RuntimeErr::SqlxError(sqlx_err) => sqlx_err
            .as_database_error()
            .and_then(|db_err| db_err.code())
            .is_some_and(|code| code.as_ref() == "23505"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_table_is_injective() {
        let a = legacy_to_apparatus("taskboard");
        let b = legacy_to_apparatus("wiki");
        assert!(a.is_some() && b.is_some(), "types legacy connus");
        assert_ne!(a, b, "pas de fusion silencieuse taskboard/wiki");
    }

    #[test]
    fn unknown_legacy_is_none() {
        assert_eq!(legacy_to_apparatus("forum"), None);
    }

    #[test]
    fn colliding_pairs_are_rejected_with_stable_code() {
        let err = check_pairs_injective(&[
            ("taskboard", "io.aiforall.generic"),
            ("wiki", "io.aiforall.generic"),
        ])
        .expect_err("2 legacy vers 1 cible = rejet");
        assert_eq!(err.code(), "APPARATUS_MAPPING_COLLISION");
        assert_eq!(err.contract_version(), 1);
        assert_eq!(err.target(), "io.aiforall.generic");
    }

    #[test]
    fn distinct_pairs_are_accepted() {
        check_pairs_injective(&[
            ("taskboard", "io.aiforall.taskboard"),
            ("wiki", "io.aiforall.wiki"),
        ])
        .expect("paires distinctes acceptées");
    }

    #[test]
    fn repeated_identical_pair_is_idempotent() {
        check_pairs_injective(&[
            ("taskboard", "io.aiforall.taskboard"),
            ("taskboard", "io.aiforall.taskboard"),
        ])
        .expect("paire répétée acceptée (idempotence)");
    }
}
