//! Apparatus P4 — T12 gate P5 + régression (scan fichiers, sans Kind).
//!
//! Redéclare les tokens des gates Manifesto T1 / P2-T7 / P3-T2 pour le lot
//! operator : host UI / iframe / CLI / `messagechannel` / `ui_host` /
//! `apparatus_host` restent interdits dans Manifesto prod. K8s (`k8s`,
//! `kubernetes`, deps `kube` / `k8s-openapi`) uniquement sous
//! `apparatus-operator/`. Pas de P5 UI. Pas d'`invoke` sur `ApparatusRuntime`.

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

fn collect_named(dir: &Path, file_name: &str, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("dossier lisible");
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == ".git" || name == "tests" {
                continue;
            }
            collect_named(&path, file_name, out);
        } else if path.file_name().and_then(|n| n.to_str()) == Some(file_name) {
            out.push(path);
        }
    }
}

fn manifesto_prod_files() -> Vec<PathBuf> {
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
    assert!(!files.is_empty(), "scope Manifesto/*/src non vide");
    files
}

fn lazaret_prod_files() -> Vec<PathBuf> {
    let dir = workspace_root().join("Lazaret");
    assert!(dir.is_dir(), "T12 RED : dossier Lazaret/ absent");
    let mut files = Vec::new();
    collect_rs(&dir, &mut files);
    assert!(!files.is_empty(), "T12 RED : Lazaret/*/src vide");
    files
}

fn operator_src_files() -> Vec<PathBuf> {
    let dir = workspace_root().join("apparatus-operator/src");
    let mut files = Vec::new();
    collect_rs(&dir, &mut files);
    assert!(!files.is_empty(), "T12 RED : apparatus-operator/src vide");
    files
}

fn crate_cargo_tomls(crate_dir: &str) -> Vec<PathBuf> {
    let dir = workspace_root().join(crate_dir);
    assert!(dir.is_dir(), "T12 RED : {crate_dir}/ absent");
    let mut files = Vec::new();
    collect_named(&dir, "Cargo.toml", &mut files);
    assert!(
        !files.is_empty(),
        "T12 RED : aucun Cargo.toml sous {crate_dir}/"
    );
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

const fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn line_has_ident(line: &str, needle: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let n = needle.as_bytes();
    let mut start = 0;
    while start + n.len() <= bytes.len() {
        if let Some(rel) = lower[start..].find(&needle) {
            let i = start + rel;
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let after = i + n.len();
            let after_ok = after >= bytes.len() || !is_ident_byte(bytes[after]);
            if before_ok && after_ok {
                return true;
            }
            start = i + 1;
        } else {
            break;
        }
    }
    false
}

fn hits_ident(files: &[PathBuf], needle: &str, allow_line: &[&str]) -> Vec<String> {
    let mut hits = Vec::new();
    for file in files {
        let content = std::fs::read_to_string(file).expect("fichier lisible");
        for (idx, line) in content.lines().enumerate() {
            if line_has_ident(line, needle) && !allow_line.iter().any(|a| line.contains(a)) {
                hits.push(format!("{}:{}: {}", file.display(), idx + 1, line.trim()));
            }
        }
    }
    hits
}

fn looks_like_dep_key(line: &str, name: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return false;
    }
    let key = trimmed
        .trim_start_matches('[')
        .split([' ', '=', '.', ']', '"'])
        .next()
        .unwrap_or("");
    key == name
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

/// Tokens T1 / P2-T7 : ne pas relâcher (y compris P5 `iframe` / `ui_host` / …).
const MANIFESTO_FORBIDDEN_TOKENS: &[&str] = &[
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

/// P5 tel que T12 le nomme (host UI / CLI en identifiant, pas substring `client`).
const P5_SUBSTRING_TOKENS: &[&str] = &["iframe", "messagechannel", "ui_host", "apparatus_host"];

const K8S_TOKENS: &[&str] = &["k8s", "kubernetes"];

const KUBE_DEP_KEYS: &[&str] = &["kube", "k8s-openapi"];

#[test]
fn t12_p5_host_iframe_cli_tokens_forbidden_in_manifesto() {
    let files = manifesto_prod_files();
    let mut all_hits = Vec::new();
    for token in P5_SUBSTRING_TOKENS {
        all_hits.extend(hits_case_insensitive(&files, token, &[], &[]));
    }
    all_hits.extend(hits_ident(&files, "host", &[]));
    // sea-orm migrator (`cli::run_cli`) n'est pas le CLI P5 host/iframe.
    all_hits.extend(hits_ident(&files, "cli", &["run_cli"]));
    assert!(
        all_hits.is_empty(),
        "T12 RED : tokens P5 host/iframe/CLI dans Manifesto/*/src :\n{}",
        all_hits.join("\n")
    );
}

#[test]
fn t12_p2_t7_tokens_still_forbidden_in_manifesto_prod() {
    let files = manifesto_prod_files();
    let mut all_hits = Vec::new();
    for token in MANIFESTO_FORBIDDEN_TOKENS {
        all_hits.extend(hits_case_insensitive(&files, token, &[], &[]));
    }
    assert!(
        all_hits.is_empty(),
        "T12 RED : tokens T1/T7 relâchés dans Manifesto/*/src :\n{}",
        all_hits.join("\n")
    );
}

#[test]
fn t12_k8s_tokens_forbidden_in_manifesto_and_lazaret_prod() {
    let mut files = manifesto_prod_files();
    files.extend(lazaret_prod_files());
    files.extend(crate_cargo_tomls("Manifesto"));
    files.extend(crate_cargo_tomls("Lazaret"));
    let mut all_hits = Vec::new();
    for token in K8S_TOKENS {
        all_hits.extend(hits_case_insensitive(&files, token, &[], &[]));
    }
    assert!(
        all_hits.is_empty(),
        "T12 RED : k8s/kubernetes hors apparatus-operator (Manifesto/Lazaret Cargo.toml+src) :\n{}",
        all_hits.join("\n")
    );
}

#[test]
fn t12_kube_deps_forbidden_outside_operator() {
    let mut tomls = crate_cargo_tomls("Manifesto");
    tomls.extend(crate_cargo_tomls("Lazaret"));
    let mut hits = Vec::new();
    for toml in &tomls {
        let content = std::fs::read_to_string(toml).expect("Cargo.toml lisible");
        for line in content.lines() {
            for key in KUBE_DEP_KEYS {
                if looks_like_dep_key(line, key) {
                    hits.push(format!("{}: {line}", toml.display()));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "T12 RED : deps kube/k8s-openapi hors apparatus-operator :\n{}",
        hits.join("\n")
    );
}

#[test]
fn t12_factory_absent() {
    let root = workspace_root();
    assert!(
        !root.join("Factory").exists(),
        "T12 RED : Factory/ présent à la racine workspace"
    );
    let entries = std::fs::read_dir(&root).expect("racine workspace lisible");
    for entry in entries {
        let name = entry.expect("entrée lisible").file_name();
        let name = name.to_string_lossy();
        assert!(
            !name.eq_ignore_ascii_case("Factory"),
            "T12 RED : dossier {name} à la racine workspace"
        );
    }
    for member in workspace_members() {
        let lower = member.replace('\\', "/").to_ascii_lowercase();
        let forbidden = lower
            .split('/')
            .any(|seg| seg == "factory" || seg == "factory-service");
        assert!(!forbidden, "T12 RED : membre Cargo {member}");
    }
}

#[test]
fn t12_no_gateway_in_manifesto_prod() {
    let files = manifesto_prod_files();
    let hits = hits_case_insensitive(&files, "gateway", &[], &[]);
    assert!(
        hits.is_empty(),
        "T12 RED : token gateway dans Manifesto/*/src :\n{}",
        hits.join("\n")
    );
}

#[test]
fn t12_apparatus_runtime_has_no_invoke() {
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
            "T12 : méthode {method} absente du trait ApparatusRuntime"
        );
    }
    assert!(
        !block.contains("fn invoke"),
        "T12 RED : fn invoke sur ApparatusRuntime"
    );
}

#[test]
fn t12_component_routes_unchanged_five_registrations() {
    let lib = workspace_root().join("Manifesto/http/src/lib.rs");
    let content = std::fs::read_to_string(&lib).expect("http lib lisible");
    let count = content
        .matches("/api/projects/{project_id}/components")
        .count();
    assert_eq!(
        count, 5,
        "T12 : les 5 routes /components (GET,GET,POST,PATCH,DELETE) doivent rester inchangées, trouvé {count}"
    );
}

#[test]
fn t12_p5_tokens_absent_from_operator_src() {
    let files = operator_src_files();
    let mut all_hits = Vec::new();
    for token in P5_SUBSTRING_TOKENS {
        all_hits.extend(hits_case_insensitive(&files, token, &[], &[]));
    }
    assert!(
        all_hits.is_empty(),
        "T12 RED : P5 UI ouvert sous apparatus-operator/src :\n{}",
        all_hits.join("\n")
    );
}

#[test]
fn t12_manifesto_gates_still_declare_p5_tokens() {
    let p2 = workspace_root().join("Manifesto/tests/apparatus_p2_t7_gate.rs");
    let p2_content = std::fs::read_to_string(&p2).expect("T7 gate lisible");
    assert!(
        p2_content.contains("fn t7_p3_tokens_forbidden_even_on_p2_allowlist"),
        "T12 : t7_p3_tokens_forbidden_even_on_p2_allowlist absent du gate P2"
    );
    for token in MANIFESTO_FORBIDDEN_TOKENS {
        let quoted = format!("\"{token}\"");
        assert!(
            p2_content.contains(&quoted),
            "T12 : token {token} absent de la liste interdite T7"
        );
    }

    let p3 = workspace_root().join("Manifesto/tests/apparatus_p3_t2_gate.rs");
    let p3_content = std::fs::read_to_string(&p3).expect("T2 gate lisible");
    assert!(
        p3_content.contains("fn t2_p4_tokens_forbidden_in_lazaret_src"),
        "T12 : t2_p4_tokens_forbidden_in_lazaret_src absent du gate P3 T2"
    );
    for token in [
        "k8s",
        "kubernetes",
        "iframe",
        "messagechannel",
        "apparatus_host",
        "ui_host",
    ] {
        let quoted = format!("\"{token}\"");
        assert!(
            p3_content.contains(&quoted),
            "T12 : token {token} absent de la liste interdite P3 T2"
        );
    }

    let t1 = workspace_root().join("Manifesto/tests/apparatus_p4_t1_absence.rs");
    let t1_content = std::fs::read_to_string(&t1).expect("T1 absence lisible");
    for token in MANIFESTO_FORBIDDEN_TOKENS {
        let quoted = format!("\"{token}\"");
        assert!(
            t1_content.contains(&quoted),
            "T12 : token {token} absent de la liste interdite T1"
        );
    }
}
