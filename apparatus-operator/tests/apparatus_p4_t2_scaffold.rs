//! Apparatus P4 — T2 scaffold BC-A (`apparatus-operator`).
//!
//! Prouve le member workspace après `apparatus-reference-kv`, l'absence de
//! `Factory/`, l'absence d'entrée monolith, trois binaires `--version` sans
//! listener HTTP, features `admit`/`controller` déclarées, et zéro dep `kube`
//! / `k8s-openapi` dans les `Cargo.toml` Manifesto. Pas de YAML Kind-cluster
//! (`kind.x-k8s.io` / `kind: Cluster`) sous `apparatus-operator/k8s/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const HTTP_LISTEN_TOKENS: [&str; 4] = ["axum", "create_router", "TcpListener", ".bind("];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR a un parent (racine du dépôt)")
        .to_path_buf()
}

fn collect_named(dir: &Path, file_name: &str, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("dossier lisible {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == ".git" {
                continue;
            }
            collect_named(&path, file_name, out);
        } else if path.file_name().and_then(|n| n.to_str()) == Some(file_name) {
            out.push(path);
        }
    }
}

fn members_quoted(root_cargo: &str) -> Vec<String> {
    let start = root_cargo
        .find("members")
        .and_then(|i| root_cargo[i..].find('[').map(|j| i + j + 1))
        .expect("table members");
    let rest = &root_cargo[start..];
    let end = rest.find(']').expect("members fermé");
    rest[..end]
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                return None;
            }
            let start = trimmed.find('"')?;
            let end = trimmed[start + 1..].find('"')?;
            Some(trimmed[start + 1..start + 1 + end].to_string())
        })
        .collect()
}

fn features_table(cargo: &str) -> &str {
    let start = cargo.find("[features]").expect("[features] déclaré");
    let rest = &cargo[start..];
    let end = rest[1..].find("\n[").map_or(rest.len(), |i| i + 1);
    &rest[..end]
}

fn feature_declared(features: &str, name: &str) -> bool {
    features.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            return false;
        }
        trimmed == name
            || trimmed.starts_with(&format!("{name} "))
            || trimmed.starts_with(&format!("{name}="))
            || trimmed.starts_with(&format!("{name} ="))
    })
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

fn bin_exe(cargo_bin_exe_key: &str) -> PathBuf {
    // rustc/cargo 1.94 : CARGO_BIN_EXE_* n'est plus visible via `env!` (runtime
    // seulement) et conserve les tirets du nom de binaire.
    let raw = std::env::var_os(cargo_bin_exe_key)
        .unwrap_or_else(|| panic!("{cargo_bin_exe_key} absent (binaire non construit)"));
    PathBuf::from(raw)
}

fn run_bin_version(exe: &Path) -> String {
    let output = Command::new(exe)
        .arg("--version")
        .output()
        .unwrap_or_else(|e| panic!("échec exécution {}: {e}", exe.display()));
    assert!(
        output.status.success(),
        "{} --version exit={:?} stderr={}",
        exe.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn t2_workspace_member_after_reference_kv() {
    let cargo = fs::read_to_string(repo_root().join("Cargo.toml")).expect("Cargo.toml racine");
    let members = members_quoted(&cargo);
    let kv = members
        .iter()
        .position(|m| m == "apparatus-reference-kv")
        .expect("member apparatus-reference-kv");
    let op = members
        .iter()
        .position(|m| m == "apparatus-operator")
        .expect("member apparatus-operator");
    assert!(
        op > kv,
        "apparatus-operator (index {op}) doit suivre apparatus-reference-kv (index {kv})"
    );
}

#[test]
fn t2_operator_dir_exists_factory_absent() {
    let root = repo_root();
    assert!(
        root.join("apparatus-operator").is_dir(),
        "apparatus-operator/ doit exister à la racine"
    );
    assert!(
        !root.join("Factory").exists(),
        "Factory/ ne doit pas exister à la racine"
    );
}

#[test]
fn t2_monolith_does_not_list_operator() {
    let cargo = fs::read_to_string(repo_root().join("monolith").join("Cargo.toml"))
        .expect("monolith/Cargo.toml");
    if cargo.contains("[workspace]") || cargo.contains("members") {
        let members = if cargo.contains("members") {
            members_quoted(&cargo)
        } else {
            Vec::new()
        };
        assert!(
            !members.iter().any(|m| m == "apparatus-operator"),
            "monolith workspace ne doit pas lister apparatus-operator"
        );
    }
    let depends = cargo
        .lines()
        .any(|line| looks_like_dep_key(line, "apparatus-operator"));
    assert!(
        !depends,
        "monolith/Cargo.toml ne doit pas dépendre de apparatus-operator"
    );
}

#[test]
fn t2_three_bins_print_package_version() {
    let version = env!("CARGO_PKG_VERSION");
    for (label, key) in [
        ("controller", "CARGO_BIN_EXE_apparatus-controller"),
        ("admit", "CARGO_BIN_EXE_apparatus-admit"),
        ("build", "CARGO_BIN_EXE_apparatus-build"),
    ] {
        let exe = bin_exe(key);
        assert!(exe.is_file(), "{label} exe manquant: {}", exe.display());
        let stdout = run_bin_version(&exe);
        assert!(
            stdout.contains(version),
            "{label} --version stdout={stdout:?} attendait {version}"
        );
    }
}

#[test]
fn t2_operator_src_has_no_http_listener() {
    let root = repo_root().join("apparatus-operator").join("src");
    let files = [
        root.join("lib.rs"),
        root.join("bin").join("controller.rs"),
        root.join("bin").join("admit.rs"),
        root.join("bin").join("build.rs"),
    ];
    let mut hits = Vec::new();
    for file in &files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("source manquante {}: {e}", file.display()));
        for needle in HTTP_LISTEN_TOKENS {
            if content.contains(needle) {
                hits.push(format!("{}: {needle}", file.display()));
            }
        }
    }
    assert!(hits.is_empty(), "tokens HTTP interdits en T2: {hits:?}");
}

#[test]
fn t2_manifesto_cargo_has_no_kube_deps() {
    let manifesto = repo_root().join("Manifesto");
    assert!(manifesto.is_dir(), "Manifesto/ doit exister");
    let mut tomls = Vec::new();
    collect_named(&manifesto, "Cargo.toml", &mut tomls);
    assert!(!tomls.is_empty(), "au moins Manifesto/Cargo.toml");
    let mut hits = Vec::new();
    for toml in &tomls {
        let content = fs::read_to_string(toml).expect("Cargo.toml Manifesto lisible");
        for line in content.lines() {
            if looks_like_dep_key(line, "kube") || looks_like_dep_key(line, "k8s-openapi") {
                hits.push(format!("{}: {line}", toml.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "deps kube/k8s-openapi interdites dans Manifesto: {hits:?}"
    );
}

#[test]
fn t2_admit_and_controller_features_declared() {
    let cargo = fs::read_to_string(repo_root().join("apparatus-operator").join("Cargo.toml"))
        .expect("apparatus-operator/Cargo.toml");
    let features = features_table(&cargo);
    assert!(
        feature_declared(features, "admit"),
        "feature admit manquante dans [features]: {features}"
    );
    assert!(
        feature_declared(features, "controller"),
        "feature controller manquante dans [features]: {features}"
    );
}

fn is_yaml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
}

fn collect_yaml(dir: &Path, hits: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("dossier lisible {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("entrée lisible").path();
        if path.is_dir() {
            collect_yaml(&path, hits);
        } else if is_yaml_file(&path) {
            hits.push(path);
        }
    }
}

fn is_kind_cluster_yaml(content: &str) -> bool {
    if content.contains("kind.x-k8s.io") {
        return true;
    }
    content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "kind: Cluster" || trimmed.starts_with("kind: Cluster ")
    })
}

#[test]
fn t2_no_kind_yaml_under_operator_k8s() {
    let k8s = repo_root().join("apparatus-operator").join("k8s");
    if !k8s.exists() {
        return;
    }
    let mut files = Vec::new();
    collect_yaml(&k8s, &mut files);
    let mut hits = Vec::new();
    for path in &files {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("YAML lisible {}: {e}", path.display()));
        if is_kind_cluster_yaml(&content) {
            hits.push(path.display().to_string());
        }
    }
    assert!(
        hits.is_empty(),
        "aucun YAML Kind-cluster (kind.x-k8s.io / kind: Cluster) sous apparatus-operator/k8s/: {hits:?}"
    );
}
