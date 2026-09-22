//! Runner de conformance plateforme P4, indépendant du publisher.
//!
//! Un protocole unique ([`POLICY_ID`]) pour officiel et communautaire. Le
//! rapport versionné alimentera plus tard l'admission ; il n'accorde jamais
//! l'installabilité à lui seul.

use sha2::{Digest, Sha256};

use crate::build::{parse_manifest, reject_free_build_field};
use crate::digest::ReleaseDigest;

/// Identifiant de politique de conformance P4 (officiel **et** communautaire).
///
/// Un seul protocole : ADR-0005. Le publisher ne change pas la politique.
pub const POLICY_ID: &str = "apparatus-p4-conformance/v1";

const CHECK_FREE_BUILD: &str = "free_build";
const CHECK_PARSE: &str = "parse_manifest";
const CHECK_PROTOCOL: &str = "protocol";

/// Rapport de suite de conformance (pas une attestation d'admission).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceReport {
    /// Identifiant de politique versionné, identique officiel / communautaire.
    pub policy_id: &'static str,
    /// Empreinte `sha256:` + 64 hex minuscules du payload canonique.
    pub report_digest: ReleaseDigest,
    /// Tous les checks minimaux ont réussi.
    pub passed: bool,
}

/// Politique appliquée aux artifacts officiels.
#[must_use]
pub const fn official_policy_id() -> &'static str {
    POLICY_ID
}

/// Politique appliquée aux artifacts communautaires (même protocole).
#[must_use]
pub const fn community_policy_id() -> &'static str {
    POLICY_ID
}

/// Indique si le rapport autorise une admission / une installation.
///
/// Toujours `false` en T5 : une suite verte n'est pas installable.
#[must_use]
pub const fn report_grants_admission(_report: &ConformanceReport) -> bool {
    false
}

/// Évalue un artifact candidat et émet toujours un rapport versionné.
#[must_use]
pub fn evaluate_conformance(candidate_toml: &str) -> ConformanceReport {
    let parse_result = parse_manifest(candidate_toml);
    let parse_ok = parse_result.is_ok();
    let free_build_ok = reject_free_build_field(candidate_toml).is_ok();
    let protocol_ok = protocol_is_manifesto(candidate_toml);
    let candidate = match parse_result {
        Ok(validated) => validated.digest,
        Err(_) => apparatus_contracts::digest_str(candidate_toml),
    };
    let checks = [
        (CHECK_FREE_BUILD, free_build_ok),
        (CHECK_PARSE, parse_ok),
        (CHECK_PROTOCOL, protocol_ok),
    ];
    let all_ok = checks.iter().all(|(_, succeeded)| *succeeded);
    let payload = canonical_payload(POLICY_ID, &candidate, &checks);
    ConformanceReport {
        policy_id: POLICY_ID,
        report_digest: digest_report_payload(&payload),
        passed: all_ok,
    }
}

fn protocol_is_manifesto(toml: &str) -> bool {
    let Ok(value) = toml.parse::<toml::Value>() else {
        return false;
    };
    value
        .get("backend")
        .and_then(|backend| backend.get("protocol"))
        .and_then(toml::Value::as_str)
        .is_some_and(|protocol| protocol == apparatus_contracts::PROTOCOL_ID)
}

fn canonical_payload(
    policy_id: &str,
    candidate: &ReleaseDigest,
    checks: &[(&str, bool)],
) -> String {
    let mut payload = String::new();
    payload.push_str(policy_id);
    payload.push('\n');
    payload.push_str(candidate.as_str());
    payload.push('\n');
    for (name, ok) in checks {
        payload.push_str(name);
        payload.push('=');
        payload.push(if *ok { '1' } else { '0' });
        payload.push('\n');
    }
    payload
}

fn digest_report_payload(payload: &str) -> ReleaseDigest {
    let hash = Sha256::digest(payload.as_bytes());
    let hex_lower = hex::encode(hash);
    let formatted = format!("sha256:{hex_lower}");
    ReleaseDigest::new(&formatted).unwrap_or_else(|_| {
        let mut raw = [0_u8; 32];
        for (dst, src) in raw.iter_mut().zip(hash.iter()) {
            *dst = *src;
        }
        ReleaseDigest::from_bytes(&raw)
    })
}
