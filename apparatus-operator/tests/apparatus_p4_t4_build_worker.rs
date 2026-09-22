//! Apparatus P4 — T4 worker de build (`apparatus-build`).
//!
//! Identité OS distincte : pas de Transit, pas de creds push, pas de token
//! cluster. `build.rs` / proc-macro hostiles = non fiables (jamais d'allowlist
//! de builders). Pas de champ libre `build = "…"` (ADR-0003). Pas de YAML Kind.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use apparatus_operator::{
    cargo_build_scripts_trusted, parse_manifest, refuse_privileged_identity_from,
    reject_free_build_field,
};

const FORBIDDEN_IDENTS: [&str; 9] = [
    "kube",
    "k8s_openapi",
    "reqwest",
    "cosign",
    "oras",
    "vault",
    "axum",
    "TcpListener",
    "create_router",
];

const FORBIDDEN_SUBSTRINGS: [&str; 2] = ["k8s-openapi", "TransitClient"];

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

fn repo_src(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn bin_exe(cargo_bin_exe_key: &str) -> PathBuf {
    // rustc/cargo 1.94 : CARGO_BIN_EXE_* n'est plus visible via `env!` (runtime
    // seulement) et conserve les tirets du nom de binaire.
    let raw = std::env::var_os(cargo_bin_exe_key)
        .unwrap_or_else(|| panic!("{cargo_bin_exe_key} absent (binaire non construit)"));
    PathBuf::from(raw)
}

const fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn contains_ident(haystack: &str, ident: &str) -> bool {
    let mut from = 0;
    while from < haystack.len() {
        let Some(rel) = haystack[from..].find(ident) else {
            return false;
        };
        let abs = from + rel;
        let before = abs
            .checked_sub(1)
            .and_then(|i| haystack.as_bytes().get(i).copied());
        let after = haystack.as_bytes().get(abs + ident.len()).copied();
        let left_ok = before.is_none_or(|c| !is_ident_char(c));
        let right_ok = after.is_none_or(|c| !is_ident_char(c));
        if left_ok && right_ok {
            return true;
        }
        from = abs + ident.len();
    }
    false
}

fn source_hits(path: &Path) -> Vec<String> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("source manquante {}: {e}", path.display()));
    let mut hits = Vec::new();
    for ident in FORBIDDEN_IDENTS {
        if contains_ident(&content, ident) {
            hits.push(format!("{}: ident {ident}", path.display()));
        }
    }
    for needle in FORBIDDEN_SUBSTRINGS {
        if content.contains(needle) {
            hits.push(format!("{}: {needle}", path.display()));
        }
    }
    hits
}

fn run_build_bin(args: &[&str], extra_env: &[(&str, Option<&str>)]) -> std::process::Output {
    let exe = bin_exe("CARGO_BIN_EXE_apparatus-build");
    let mut cmd = Command::new(&exe);
    cmd.args(args);
    for (name, value) in extra_env {
        match value {
            Some(v) => {
                cmd.env(name, v);
            }
            None => {
                cmd.env_remove(name);
            }
        }
    }
    cmd.output()
        .unwrap_or_else(|e| panic!("échec exécution {}: {e}", exe.display()))
}

#[test]
fn t4_build_sources_have_no_privileged_clients() {
    let files = [repo_src("src/bin/build.rs"), repo_src("src/build.rs")];
    let mut hits = Vec::new();
    for file in &files {
        hits.extend(source_hits(file));
    }
    assert!(
        hits.is_empty(),
        "clients / listeners interdits dans le worker build: {hits:?}"
    );
}

#[test]
fn t4_apparatus_build_metadata_omits_admit_controller() {
    let manifest = repo_src("Cargo.toml");
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .arg("--no-deps")
        .output()
        .expect("cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let meta: serde_json::Value = serde_json::from_slice(&output.stdout).expect("metadata JSON");
    let packages = meta["packages"].as_array().expect("packages");
    let pkg = packages
        .iter()
        .find(|p| p["name"] == "apparatus-operator")
        .expect("package apparatus-operator");
    let features = pkg["features"].as_object().expect("features");
    assert!(
        features.contains_key("admit"),
        "feature admit manquante: {features:?}"
    );
    assert!(
        features.contains_key("controller"),
        "feature controller manquante: {features:?}"
    );
    let default = features
        .get("default")
        .and_then(serde_json::Value::as_array)
        .expect("default features");
    for forbidden in ["admit", "controller"] {
        assert!(
            !default.iter().any(|v| v.as_str() == Some(forbidden)),
            "default ne doit pas activer {forbidden}: {default:?}"
        );
    }
    let targets = pkg["targets"].as_array().expect("targets");
    let build = targets
        .iter()
        .find(|t| t["name"] == "apparatus-build")
        .expect("cible bin apparatus-build");
    let required = build
        .get("required-features")
        .and_then(serde_json::Value::as_array);
    if let Some(required) = required {
        for forbidden in ["admit", "controller"] {
            assert!(
                !required.iter().any(|v| v.as_str() == Some(forbidden)),
                "apparatus-build required-features ne doit pas lister {forbidden}: {required:?}"
            );
        }
    }
}

#[test]
fn t4_refuse_privileged_identity_on_injected_env() {
    for name in ["VAULT_TOKEN", "KUBECONFIG", "KUBERNETES_SERVICE_HOST"] {
        let err =
            refuse_privileged_identity_from(&[name], false).expect_err("identité injectée rejetée");
        assert_eq!(err.code(), "APPARATUS_INVALID_OPERATION", "pour {name}");
        let output = run_build_bin(&[], &[(name, Some("injected"))]);
        assert!(
            !output.status.success(),
            "binaire doit refuser {name}, exit={:?} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    refuse_privileged_identity_from(&[], false).expect("aucune identité injectée");
}

#[test]
fn t4_cargo_build_scripts_untrusted_no_allowlist() {
    assert!(
        !cargo_build_scripts_trusted(),
        "build.rs / proc-macro ne sont jamais fiables"
    );
    let src = fs::read_to_string(repo_src("src/build.rs")).expect("src/build.rs");
    assert!(
        !src.contains("official-builder"),
        "pas d'allowlist de builders nommés"
    );
}

#[test]
fn t4_free_build_field_rejected() {
    let err = reject_free_build_field("build = \"make\"\n").expect_err("build libre rejeté");
    assert!(
        err.code() == "APPARATUS_FORBIDDEN_FIELD" || err.code() == "APPARATUS_INVALID_OPERATION",
        "code inattendu {}",
        err.code()
    );
}

#[test]
fn t4_wrapped_parse_rejects_build_command() {
    let toml = format!("build = \"cargo build\"\n{VALID_MANIFEST_TOML}");
    parse_manifest(&toml).expect_err("parse wrap doit rejeter build = cargo build");
}

#[test]
fn t4_bin_version_prints_package_version() {
    let version = env!("CARGO_PKG_VERSION");
    let output = run_build_bin(&["--version"], &[("VAULT_TOKEN", Some("injected"))]);
    assert!(
        output.status.success(),
        "--version doit réussir malgré VAULT_TOKEN, exit={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(version),
        "--version stdout={stdout:?} attendait {version}"
    );
    assert!(
        stdout.contains("0.1.0"),
        "--version stdout={stdout:?} attendait 0.1.0"
    );
}
