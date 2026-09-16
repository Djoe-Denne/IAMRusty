//! Apparatus P3 — T2 gate Lazaret (garde-fou négatif, pur, sans Docker).
//!
//! Tokens P4+ interdits sous `Lazaret/*/src`. Le jeton de frontière de
//! capacités n'est **pas** interdit ici (Lazaret est ce BC). Manifesto T7
//! continue de l'interdire sous `Manifesto/*/src`.

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

fn lazaret_prod_files() -> Vec<PathBuf> {
    let dir = workspace_root().join("Lazaret");
    assert!(dir.is_dir(), "T2 RED : dossier Lazaret/ absent");
    let mut files = Vec::new();
    collect_rs(&dir, &mut files);
    assert!(!files.is_empty(), "T2 RED : Lazaret/*/src vide");
    files
}

fn hits_case_insensitive(files: &[PathBuf], needle: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for file in files {
        let content = std::fs::read_to_string(file).expect("fichier lisible");
        for (idx, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(needle) {
                hits.push(format!("{}:{}: {}", file.display(), idx + 1, line.trim()));
            }
        }
    }
    hits
}

#[test]
fn t2_p4_tokens_forbidden_in_lazaret_src() {
    let files = lazaret_prod_files();
    let mut all_hits = Vec::new();
    for token in [
        "kubernetes",
        "k8s",
        "wasm",
        "wasi",
        "wasmtime",
        "iframe",
        "messagechannel",
        "apparatus_host",
        "ui_host",
    ] {
        all_hits.extend(hits_case_insensitive(&files, token));
    }
    assert!(
        all_hits.is_empty(),
        "T2 RED : tokens P4+ dans Lazaret/*/src :\n{}",
        all_hits.join("\n")
    );
}
