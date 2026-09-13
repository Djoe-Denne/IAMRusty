//! Apparatus P2 — T7 gate P3+ (garde-fou négatif, pur, sans Docker).
//!
//! Tokens P2 (`poll`, `worker`, `lease`, `fencing`, `controller`,
//! `desired_state`) uniquement sur l'allowlist L. Tokens P3+ interdits
//! partout, y compris l'allowlist P2.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("workspace lisible")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("dossier prod lisible");
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "tests" || name == "target" {
                continue;
            }
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn prod_files() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut files = Vec::new();
    for sub in [
        "Manifesto/domain/src",
        "Manifesto/application/src",
        "Manifesto/infra/src",
        "Manifesto/http/src",
        "Manifesto/setup/src",
        "Manifesto/configuration/src",
        "Manifesto/migration/src",
    ] {
        collect_rs(&root.join(sub), &mut files);
    }
    assert!(!files.is_empty(), "scope T7 non vide");
    files
}

fn hits_case_insensitive(
    files: &[PathBuf],
    needle: &str,
    allow_line: &[&str],
    allow_path: &[&str],
) -> Vec<String> {
    let mut hits = Vec::new();
    for file in files {
        let normalized = file.to_string_lossy().replace('\\', "/");
        if allow_path.iter().any(|a| normalized.contains(a)) {
            continue;
        }
        let content = std::fs::read_to_string(file).expect("fichier lisible");
        for (idx, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(needle) && !allow_line.iter().any(|a| line.contains(a))
            {
                hits.push(format!("{}:{}: {}", file.display(), idx + 1, line.trim()));
            }
        }
    }
    hits
}

const P2_RUNTIME_ALLOW_PATH: &[&str] = &[
    "Manifesto/migration/src/m20260912_000013_apparatus_p2_runtime.rs",
    "Manifesto/infra/src/apparatus_runtime/",
];

#[test]
fn t7_p2_tokens_only_on_allowlist_l() {
    let files = prod_files();
    let mut all_hits = Vec::new();
    for token in [
        "poll",
        "worker",
        "lease",
        "fencing",
        "controller",
        "desired_state",
    ] {
        all_hits.extend(hits_case_insensitive(
            &files,
            token,
            &[],
            P2_RUNTIME_ALLOW_PATH,
        ));
    }
    assert!(
        all_hits.is_empty(),
        "T7 RED : tokens P2 hors allowlist L :\n{}",
        all_hits.join("\n")
    );
}

#[test]
fn t7_p3_tokens_forbidden_even_on_p2_allowlist() {
    let files = prod_files();
    let mut all_hits = Vec::new();
    for token in [
        "wasm",
        "wasi",
        "kubernetes",
        "k8s",
        "iframe",
        "messagechannel",
        "gateway",
        "wasmtime",
        "apparatus_host",
        "ui_host",
    ] {
        all_hits.extend(hits_case_insensitive(&files, token, &[], &[]));
    }
    assert!(
        all_hits.is_empty(),
        "T7 RED : tokens P3+ dans Manifesto/*/src :\n{}",
        all_hits.join("\n")
    );
}

#[test]
fn t7_factory_allowlist_unchanged() {
    let files = prod_files();
    let hits = hits_case_insensitive(
        &files,
        "factory",
        &["Factory outcome", "ManifestoCommandRegistryFactory"],
        &["application/src/command/"],
    );
    assert!(
        hits.is_empty(),
        "T7 RED : factory hors allowlist :\n{}",
        hits.join("\n")
    );
}

#[test]
fn t7_no_valid_verified_status() {
    let files = prod_files();
    let mut hits = Vec::new();
    for file in &files {
        let content = std::fs::read_to_string(file).expect("fichier lisible");
        for (idx, line) in content.lines().enumerate() {
            if line.contains("\"VALID\"")
                || line.contains("\"VERIFIED\"")
                || line.contains("'VALID'")
                || line.contains("'VERIFIED'")
            {
                hits.push(format!("{}:{}: {}", file.display(), idx + 1, line.trim()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "T7 RED : statut VALID/VERIFIED détecté :\n{}",
        hits.join("\n")
    );
}

#[test]
fn t7_component_routes_unchanged_five_registrations() {
    let lib = workspace_root().join("Manifesto/http/src/lib.rs");
    let content = std::fs::read_to_string(&lib).expect("http lib lisible");
    let count = content
        .matches("/api/projects/{project_id}/components")
        .count();
    assert_eq!(
        count, 5,
        "T7 : les 5 routes /components (GET,GET,POST,PATCH,DELETE) doivent rester inchangées, trouvé {count}"
    );
}
