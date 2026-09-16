//! Apparatus P3 — T6 absence: no KV rows on Manifesto DB.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("workspace lisible")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("dossier lisible");
    for entry in entries {
        let path = entry.expect("entrée").path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" {
                continue;
            }
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn t6_manifesto_src_has_no_kv_table() {
    let mut files = Vec::new();
    for sub in [
        "Manifesto/domain/src",
        "Manifesto/application/src",
        "Manifesto/infra/src",
        "Manifesto/http/src",
        "Manifesto/setup/src",
        "Manifesto/migration/src",
    ] {
        collect_rs(&workspace_root().join(sub), &mut files);
    }
    for file in files {
        let content = std::fs::read_to_string(&file).expect("fichier");
        assert!(
            !content.contains("apparatus_kv_entries"),
            "{} must not own platform KV",
            file.display()
        );
    }
}

#[test]
fn t6_openfga_model_has_no_apparatus() {
    let path = workspace_root().join("openfga/model.fga");
    let content = std::fs::read_to_string(path).expect("model.fga");
    assert!(!content.to_ascii_lowercase().contains("apparatus"));
}
