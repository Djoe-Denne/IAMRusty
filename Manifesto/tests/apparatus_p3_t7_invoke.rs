//! Apparatus P3 — T7 Manifesto boundary (no new runtime method, frozen component routes).

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
            if name == "tests" || name == "target" {
                continue;
            }
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn t7_components_route_count_stays_five() {
    let lib =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("http/src/lib.rs"))
            .expect("lib.rs");
    let registrations = lib.matches("/api/projects/{project_id}/components").count();
    assert_eq!(
        registrations, 5,
        "0006 E: /components stays at 5 registrations"
    );
}

#[test]
fn t7_manifesto_src_still_forbids_gateway() {
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
    let mut hits = Vec::new();
    for file in files {
        let content = std::fs::read_to_string(&file).expect("fichier");
        for (idx, line) in content.lines().enumerate() {
            if line.to_lowercase().contains("gateway") {
                hits.push(format!("{}:{}: {}", file.display(), idx + 1, line.trim()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "gateway leaked into Manifesto src:\n{}",
        hits.join("\n")
    );
}

#[test]
fn t7_runtime_trait_unchanged() {
    let ports = std::fs::read_to_string(workspace_root().join("apparatus-contracts/src/ports.rs"))
        .expect("ports");
    let start = ports.find("trait ApparatusRuntime").expect("trait");
    let block = &ports[start..];
    assert!(block.contains("fn bind"));
    assert!(block.contains("fn configure"));
    assert!(block.contains("fn unbind"));
    assert!(block.contains("fn observe"));
    assert!(block.contains("fn teardown"));
    let trait_end = block.find("fn teardown").expect("teardown");
    let trait_slice = &block[..trait_end + 40];
    let forbidden = format!("{}{}", "fn inv", "oke");
    assert!(
        !trait_slice.contains(&forbidden),
        "ApparatusRuntime must not grow a call method"
    );
}
