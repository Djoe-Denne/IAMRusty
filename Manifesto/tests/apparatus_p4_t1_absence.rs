//! Apparatus P4 — T1 caractérisation d'absence (invariants P2/P3 Accepted, pur, sans Docker).
//!
//! Gèle ADR-0005 (pas d'auto-admission, pas de `VALID`/`VERIFIED` harness, pas de
//! `trusted_skip_gateway` DTO), ADR-0003 (harness ≠ production) et ADR-0002
//! (Factory absente). Ne crée pas `Factory/`, ne retargete pas les gates P2/P3,
//! n'ajoute pas de SQL.

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

fn workspace_members() -> Vec<String> {
    let cargo =
        std::fs::read_to_string(workspace_root().join("Cargo.toml")).expect("Cargo.toml lisible");
    let start = cargo
        .find("members = [")
        .expect("workspace.members présent");
    let from = &cargo[start..];
    let end = from.find(']').expect("workspace.members fermé");
    from[..end]
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                return None;
            }
            let q = trimmed.find('"')?;
            let rest = &trimmed[q + 1..];
            let close = rest.find('"')?;
            Some(rest[..close].to_string())
        })
        .collect()
}

const P2_T7_P3_TOKENS: &[&str] = &[
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
];

const P3_T2_FORBIDDEN_TOKENS: &[&str] = &[
    "k8s",
    "kubernetes",
    "wasm",
    "wasi",
    "wasmtime",
    "iframe",
    "messagechannel",
    "apparatus_host",
    "ui_host",
];

#[test]
fn t1_no_git_package_admission_prod() {
    let root = workspace_root();
    assert!(
        !root.join("Factory").exists(),
        "T1 RED : Factory/ présent à la racine workspace"
    );
    let entries = std::fs::read_dir(&root).expect("racine workspace lisible");
    for entry in entries {
        let name = entry.expect("entrée lisible").file_name();
        let name = name.to_string_lossy();
        assert!(
            !name.eq_ignore_ascii_case("Factory"),
            "T1 RED : dossier {name} à la racine workspace"
        );
    }

    for member in workspace_members() {
        let lower = member.replace('\\', "/").to_ascii_lowercase();
        let forbidden = lower
            .split('/')
            .any(|seg| seg == "factory" || seg == "factory-service");
        assert!(!forbidden, "T1 RED : membre Cargo {member}");
    }

    let files = prod_files();
    let adapter_files: Vec<_> = files
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("kubernetes_adapter.rs"))
        })
        .collect();
    assert!(
        adapter_files.is_empty(),
        "T1 RED : kubernetes_adapter.rs sous Manifesto/*/src :\n{}",
        adapter_files
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
    let type_hits = hits_case_insensitive(&files, "kubernetesadapter", &[], &[]);
    assert!(
        type_hits.is_empty(),
        "T1 RED : type KubernetesAdapter sous Manifesto/*/src :\n{}",
        type_hits.join("\n")
    );

    for needle in ["admission", "conformance", "signer"] {
        let hits = hits_case_insensitive(&files, needle, &[], &[]);
        assert!(
            hits.is_empty(),
            "T1 RED : token {needle} dans Manifesto/*/src :\n{}",
            hits.join("\n")
        );
    }
}

#[test]
fn t1_harness_has_no_valid_verified() {
    let path = workspace_root().join("apparatus-contracts/src/harness.rs");
    let content = std::fs::read_to_string(&path).expect("harness.rs lisible");
    let mut hits = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if line.contains("\"VALID\"")
            || line.contains("\"VERIFIED\"")
            || line.contains("'VALID'")
            || line.contains("'VERIFIED'")
        {
            hits.push(format!("{}:{}: {}", path.display(), idx + 1, line.trim()));
        }
    }
    assert!(
        hits.is_empty(),
        "T1 RED : statut VALID/VERIFIED dans harness.rs :\n{}",
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
fn t1_no_gateway_in_manifesto_prod() {
    let files = prod_files();
    let hits = hits_case_insensitive(&files, "gateway", &[], &[]);
    assert!(
        hits.is_empty(),
        "T1 RED : token gateway dans Manifesto/*/src :\n{}",
        hits.join("\n")
    );
}

#[test]
fn t1_component_routes_unchanged_five_registrations() {
    let lib = workspace_root().join("Manifesto/http/src/lib.rs");
    let content = std::fs::read_to_string(&lib).expect("http lib lisible");
    let count = content
        .matches("/api/projects/{project_id}/components")
        .count();
    assert_eq!(
        count, 5,
        "T1 : les 5 routes /components (GET,GET,POST,PATCH,DELETE) doivent rester inchangées, trouvé {count}"
    );
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
    files.retain(|p| {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name != "apparatus_p3_t1_absence.rs" && name != "apparatus_p4_t1_absence.rs"
    });
    assert!(!files.is_empty(), "scope Manifesto .rs non vide");
    let hits = hits_case_insensitive(&files, "invoke", &[], &[]);
    assert!(
        hits.is_empty(),
        "T1 RED : invoke dans Manifesto/**/*.rs :\n{}",
        hits.join("\n")
    );
}

#[test]
fn t1_p4_tokens_forbidden_in_manifesto_prod() {
    let files = prod_files();
    let mut all_hits = Vec::new();
    for token in P2_T7_P3_TOKENS {
        all_hits.extend(hits_case_insensitive(&files, token, &[], &[]));
    }
    assert!(
        all_hits.is_empty(),
        "T1 RED : tokens P4 interdits dans Manifesto/*/src :\n{}",
        all_hits.join("\n")
    );
}

#[test]
fn t1_p2_and_p3_gates_not_retargeted() {
    let p2 = workspace_root().join("Manifesto/tests/apparatus_p2_t7_gate.rs");
    let p2_content = std::fs::read_to_string(&p2).expect("T7 gate lisible");
    assert!(
        p2_content.contains("fn t7_p3_tokens_forbidden_even_on_p2_allowlist"),
        "T1 : t7_p3_tokens_forbidden_even_on_p2_allowlist absent du gate P2"
    );
    for token in P2_T7_P3_TOKENS {
        let quoted = format!("\"{token}\"");
        assert!(
            p2_content.contains(&quoted),
            "T1 : token {token} absent de la liste interdite T7"
        );
    }

    let p3 = workspace_root().join("Manifesto/tests/apparatus_p3_t2_gate.rs");
    let p3_content = std::fs::read_to_string(&p3).expect("T2 gate lisible");
    assert!(
        p3_content.contains("fn t2_p4_tokens_forbidden_in_lazaret_src"),
        "T1 : t2_p4_tokens_forbidden_in_lazaret_src absent du gate P3 T2"
    );
    for token in P3_T2_FORBIDDEN_TOKENS {
        let quoted = format!("\"{token}\"");
        assert!(
            p3_content.contains(&quoted),
            "T1 : token {token} absent de la liste interdite P3 T2"
        );
    }
}

#[test]
fn t1_no_apparatus_p4_migration_filenames() {
    let dir = workspace_root().join("Manifesto/migration/src");
    let entries = std::fs::read_dir(&dir).expect("Manifesto/migration/src lisible");
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(
            !name.to_ascii_lowercase().contains("apparatus_p4"),
            "T1 RED : fichier migration {name}"
        );
    }
}
