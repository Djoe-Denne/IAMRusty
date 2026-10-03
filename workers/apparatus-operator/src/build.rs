//! Worker de build Apparatus P4 (identité OS distincte).
//!
//! Ce module n'est **pas** un script Cargo `build.rs` : il porte la logique du
//! binaire `apparatus-build`. Les scripts Cargo et proc-macros d'un plugin
//! s'exécutent avec cette identité ; ils ne sont jamais traités comme sûrs
//! (aucune liste de builders « connus »).

use std::path::Path;

use apparatus_contracts::{validate_manifest_toml, ApparatusError, ValidatedManifest};

/// Variables d'environnement qui constituent une identité privilégiée.
///
/// Le worker ne les consomme pas : si l'une est définie, il refuse.
pub const PRIVILEGED_IDENTITY_ENV: &[&str] = &[
    "VAULT_TOKEN",
    "BAO_TOKEN",
    "BAO_DEV_ROOT_TOKEN_ID",
    "VAULT_DEV_ROOT_TOKEN_ID",
    "TRANSIT_TOKEN",
    "COSIGN_PASSWORD",
    "COSIGN_KEY",
    "REGISTRY_PASSWORD",
    "ORAS_PASSWORD",
    "DOCKER_CONFIG",
    "KUBECONFIG",
    "KUBERNETES_SERVICE_HOST",
    "KUBERNETES_SERVICE_PORT",
];

const SERVICE_ACCOUNT_TOKEN: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";

/// Indique si les scripts Cargo / proc-macros sont considérés fiables.
///
/// Toujours `false`. Faire confiance à un builder « connu » reviendrait à une
/// allowlist : le code auteur (y compris `build.rs`) reste non fiable. L'îlot
/// isole le Job ; il ne blanchit pas le graphe de compilation.
#[must_use]
pub const fn cargo_build_scripts_trusted() -> bool {
    false
}

/// Refuse si une identité privilégiée est déjà injectée dans le processus.
///
/// Le worker ne lit pas ces valeurs pour signer, pousser ou parler au
/// cluster : les voir suffit à s'arrêter.
///
/// # Errors
///
/// [`ApparatusError::InvalidOperation`] si une variable de
/// [`PRIVILEGED_IDENTITY_ENV`] est définie, ou si le fichier jeton
/// in-cluster est présent.
pub fn refuse_privileged_identity() -> Result<(), ApparatusError> {
    let present: Vec<&str> = PRIVILEGED_IDENTITY_ENV
        .iter()
        .copied()
        .filter(|name| std::env::var_os(name).is_some())
        .collect();
    refuse_privileged_identity_from(&present, Path::new(SERVICE_ACCOUNT_TOKEN).exists())
}

/// Refuse une photographie d'identité (tests et [`refuse_privileged_identity`]).
///
/// `unsafe_code = forbid` empêche les tests de muter l'environnement du
/// processus ; le binaire `apparatus-build` exerce le chemin réel via
/// `Command::env`.
///
/// # Errors
///
/// [`ApparatusError::InvalidOperation`] si `present_env` contient un nom de
/// [`PRIVILEGED_IDENTITY_ENV`], ou si `service_account_token_present` est vrai.
pub fn refuse_privileged_identity_from(
    present_env: &[&str],
    service_account_token_present: bool,
) -> Result<(), ApparatusError> {
    for name in present_env {
        if PRIVILEGED_IDENTITY_ENV.contains(name) {
            return Err(ApparatusError::InvalidOperation {
                reason: format!("privileged identity injected: {name}"),
            });
        }
    }
    if service_account_token_present {
        return Err(ApparatusError::InvalidOperation {
            reason: "privileged identity injected: service-account token".to_owned(),
        });
    }
    Ok(())
}

/// Rejette un manifeste qui déclare un champ libre `build`.
///
/// ADR-0003 : pas de `build = "…"` dans `apparatus.toml`. La clé est cherchée
/// à tous les niveaux de tables TOML (le validateur P0 ne liste pas `build`
/// dans ses champs interdits exacts).
///
/// # Errors
///
/// [`ApparatusError::TomlParse`] si l'entrée n'est pas du TOML,
/// [`ApparatusError::ForbiddenField`] si une clé `build` est présente.
pub fn reject_free_build_field(toml: &str) -> Result<(), ApparatusError> {
    let value: toml::Value = toml::from_str(toml).map_err(|err| ApparatusError::TomlParse {
        message: ApparatusError::truncate(&err.to_string(), 300),
    })?;
    if toml_has_build_key(&value) {
        return Err(ApparatusError::ForbiddenField {
            field: "build".to_owned(),
        });
    }
    Ok(())
}

/// Parse un manifeste en refusant d'abord `build`, puis le validateur P0.
///
/// # Errors
///
/// Les erreurs de [`reject_free_build_field`] et de [`validate_manifest_toml`].
pub fn parse_manifest(toml: &str) -> Result<ValidatedManifest, ApparatusError> {
    reject_free_build_field(toml)?;
    validate_manifest_toml(toml)
}

fn toml_has_build_key(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(map) => {
            map.contains_key("build") || map.values().any(toml_has_build_key)
        }
        toml::Value::Array(items) => items.iter().any(toml_has_build_key),
        toml::Value::String(_)
        | toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => false,
    }
}
