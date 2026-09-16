//! Apparatus P3 — T1 caractérisation d'absence (invariants P2 Accepted, pur, sans Docker).
//!
//! Gèle ADR-0006 G (pas d'`invoke` sur `ApparatusRuntime`, zéro `gateway` sous
//! `Manifesto/*/src`), ADR-0004 Réalité Partial et ADR-0005 (pas de
//! `trusted_skip_gateway` DTO). Ne retargete pas le gate P2.

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

fn collect_rs_including_tests(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("dossier lisible");
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" {
                continue;
            }
            collect_rs_including_tests(&path, out);
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

fn apparatus_runtime_trait_block() -> String {
    let path = workspace_root().join("apparatus-contracts/src/ports.rs");
    let content = std::fs::read_to_string(path).expect("ports.rs lisible");
    let start = content
        .find("trait ApparatusRuntime")
        .expect("trait ApparatusRuntime présent");
    let from_trait = &content[start..];
    let open = from_trait.find('{').expect("accolade ouvrante du trait");
    let mut depth = 0_i32;
    let mut end = 0;
    for (i, ch) in from_trait.char_indices() {
        if i < open {
            continue;
        }
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    assert!(end > open, "trait ApparatusRuntime sans accolade fermante");
    from_trait[..end].to_string()
}

#[test]
fn t1_apparatus_runtime_has_no_invoke() {
    let block = apparatus_runtime_trait_block();
    for method in [
        "fn bind",
        "fn configure",
        "fn unbind",
        "fn observe",
        "fn teardown",
    ] {
        assert!(
            block.contains(method),
            "T1 : méthode {method} absente du trait ApparatusRuntime"
        );
    }
    assert!(
        !block.contains("fn invoke"),
        "T1 RED : fn invoke sur ApparatusRuntime"
    );
}

#[test]
fn t1_no_invoke_in_manifesto_rs() {
    let mut files = Vec::new();
    collect_rs_including_tests(&workspace_root().join("Manifesto"), &mut files);
    files.retain(|p| p.file_name().and_then(|n| n.to_str()) != Some("apparatus_p3_t1_absence.rs"));
    assert!(!files.is_empty(), "scope Manifesto .rs non vide");
    let hits = hits_case_insensitive(&files, "invoke", &[], &[]);
    assert!(
        hits.is_empty(),
        "T1 RED : invoke dans Manifesto/**/*.rs :\n{}",
        hits.join("\n")
    );
}

#[test]
fn t1_p2_gate_still_forbids_gateway_in_prod() {
    let gate = workspace_root().join("Manifesto/tests/apparatus_p2_t7_gate.rs");
    let content = std::fs::read_to_string(&gate).expect("T7 gate lisible");
    assert!(
        content.contains("fn t7_p3_tokens_forbidden_even_on_p2_allowlist"),
        "T1 : t7_p3_tokens_forbidden_even_on_p2_allowlist absent du gate P2"
    );
    assert!(
        content.contains("\"gateway\""),
        "T1 : token gateway absent de la liste interdite T7"
    );

    let files = prod_files();
    let hits = hits_case_insensitive(&files, "gateway", &[], &[]);
    assert!(
        hits.is_empty(),
        "T1 RED : token gateway dans Manifesto/*/src :\n{}",
        hits.join("\n")
    );
}

#[test]
fn t1_dto_has_no_trusted_skip_gateway() {
    let mut files = Vec::new();
    collect_rs_including_tests(
        &workspace_root().join("apparatus-contracts/src"),
        &mut files,
    );
    assert!(!files.is_empty(), "scope apparatus-contracts/src non vide");
    let hits = hits_case_insensitive(
        &files,
        "trusted_skip_gateway",
        &[],
        &["apparatus-contracts/src/validation.rs"],
    );
    assert!(
        hits.is_empty(),
        "T1 RED : trusted_skip_gateway hors validation.rs :\n{}",
        hits.join("\n")
    );
}

#[test]
fn t1_openfga_model_has_no_apparatus() {
    let path = workspace_root().join("openfga/model.fga");
    let hits = hits_case_insensitive(&[path], "apparatus", &[], &[]);
    assert!(
        hits.is_empty(),
        "T1 RED : apparatus dans openfga/model.fga :\n{}",
        hits.join("\n")
    );
}
