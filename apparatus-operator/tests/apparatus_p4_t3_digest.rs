//! Apparatus P4 — T3 Git → digest canonique (wrap `apparatus-contracts`).
//!
//! Identité d'installation = `ReleaseDigest` (`sha256:` + 64 hex), jamais une
//! ref Git flottante. Pas de second schéma : l'operator délègue aux contrats P0.

use apparatus_contracts::{digest_manifest as contracts_digest_manifest, validate_manifest_toml};
use apparatus_operator::{digest_manifest, is_floating_ref, resolve_install_ref, ReleaseDigest};

/// Digest P0 connu (`contracts_p0` / vecteur SHA-256 de `abc`).
const VALID_DIGEST: &str =
    "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

/// Hex de [`VALID_DIGEST`] sans préfixe `sha256:` — digest invalide, pas flottant.
const UNPREFIXED_HEX: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

const VALID_MANIFEST_TOML: &str = r#"
[apparatus]
id = "io.aiforall.reference-kv"
version = "0.1.0"
schema_version = 1

[backend]
protocol = "manifesto-apparatus/1"
storage = "kv-v1"

[capabilities]
requires = ["project.read", "storage.kv.read", "storage.kv.write"]

[ui]
mode = "schema"
schema_path = "ui/settings.schema.json"
"#;

#[test]
fn t3_floating_refs_rejected_with_floating_code() {
    for floating in ["latest", "main", "tags/v1.0", "branches/foo"] {
        let err = resolve_install_ref(floating).expect_err("ref flottante rejetée");
        assert_eq!(err.code(), "APPARATUS_FLOATING_REF", "pour {floating}");
        assert!(is_floating_ref(floating), "pour {floating}");
    }
}

#[test]
fn t3_garbage_is_invalid_digest_not_floating() {
    for bad in ["garbage", "sha256:dead", UNPREFIXED_HEX] {
        assert!(!is_floating_ref(bad), "{bad} ne doit pas être flottant");
        let err = resolve_install_ref(bad).expect_err("digest invalide rejeté");
        assert_eq!(err.code(), "APPARATUS_INVALID_DIGEST", "pour {bad}");
    }
}

#[test]
fn t3_valid_sha256_digest_accepted() {
    let digest = resolve_install_ref(VALID_DIGEST).expect("digest valide");
    assert_eq!(digest.as_str(), VALID_DIGEST);
    let via_new = ReleaseDigest::new(VALID_DIGEST).expect("ReleaseDigest::new");
    assert_eq!(digest, via_new);
}

#[test]
fn t3_digest_manifest_matches_contracts() {
    let validated = validate_manifest_toml(VALID_MANIFEST_TOML).expect("manifeste P0 valide");
    let via_operator = digest_manifest(&validated.manifest).expect("digest operator");
    let via_contracts = contracts_digest_manifest(&validated.manifest).expect("digest contracts");
    assert_eq!(via_operator, via_contracts);
    assert_eq!(via_operator, validated.digest);
}

#[test]
fn t3_digest_rs_wraps_contracts_without_local_floating_table() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/digest.rs"));
    assert!(
        src.contains("apparatus_contracts"),
        "digest.rs doit appeler apparatus_contracts"
    );
    assert!(
        !src.contains("const FLOATING"),
        "digest.rs ne doit pas dupliquer const FLOATING"
    );
    assert!(
        !(src.contains("\"latest\"") && src.contains("\"main\"")),
        "digest.rs ne doit pas contenir une table locale latest/main"
    );
}
