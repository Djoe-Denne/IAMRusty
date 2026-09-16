//! Apparatus P3 — T3 identité workload (pur, sans Docker).
//!
//! Manifesto n'a pas conscience de Lazaret. T7 n'est pas affaibli.
//! Lazaret expose l'identité hybride ; un jeton crypto-valide n'est pas une autorisation.

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
    assert!(dir.is_dir(), "T3 : dossier Lazaret/ absent");
    let mut files = Vec::new();
    collect_rs(&dir, &mut files);
    assert!(!files.is_empty(), "T3 : Lazaret/*/src vide");
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

fn concat_src(files: &[PathBuf]) -> String {
    files
        .iter()
        .map(|p| std::fs::read_to_string(file_or_empty(p)))
        .collect::<Result<Vec<_>, _>>()
        .expect("src lisible")
        .join("\n")
}

fn file_or_empty(path: &Path) -> &Path {
    path
}

#[test]
fn t3_manifesto_src_has_no_gateway_or_lazaret_or_session_issuance() {
    let files = manifesto_prod_files();
    let mut all = Vec::new();
    for token in [
        "gateway",
        "lazaret",
        "issue_session",
        "session_issuer",
        "lazaret_application",
        "lazaret_infra",
        "lazaret_http",
        "lazaret_domain",
        "lazaret_setup",
    ] {
        all.extend(hits_case_insensitive(&files, token));
    }
    assert!(
        all.is_empty(),
        "T3 RED : Manifesto conscient de Lazaret / session :\n{}",
        all.join("\n")
    );
}

#[test]
fn t3_does_not_weaken_p2_t7_gate() {
    let gate = workspace_root().join("Manifesto/tests/apparatus_p2_t7_gate.rs");
    let content = std::fs::read_to_string(&gate).expect("T7 gate lisible");
    assert!(
        content.contains("fn t7_p3_tokens_forbidden_even_on_p2_allowlist"),
        "T3 : t7_p3_tokens_forbidden_even_on_p2_allowlist absent"
    );
    assert!(
        content.contains("\"gateway\""),
        "T3 : token gateway retiré du gate P2"
    );
}

#[test]
fn t3_lazaret_src_has_no_skip_mtls_prod_flag() {
    let files = lazaret_prod_files();
    let mut all = Vec::new();
    for token in ["skip_mtls", "skip-mtls", "trusted_skip"] {
        all.extend(hits_case_insensitive(&files, token));
    }
    assert!(
        all.is_empty(),
        "T3 RED : drapeau skip mTLS prod dans Lazaret/*/src :\n{}",
        all.join("\n")
    );
}

#[test]
fn t3_lazaret_src_forbids_p4_tokens() {
    let files = lazaret_prod_files();
    let mut all = Vec::new();
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
        all.extend(hits_case_insensitive(&files, token));
    }
    assert!(
        all.is_empty(),
        "T3 RED : tokens P4+ dans Lazaret/*/src :\n{}",
        all.join("\n")
    );
}

#[test]
fn t3_lazaret_identity_types_and_constants_exist() {
    let blob = concat_src(&lazaret_prod_files());
    for needle in [
        "DEFAULT_SESSION_TTL_MINUTES",
        "DEFAULT_CERT_TTL_HOURS",
        "PLATFORM_INTERNAL_CA_PRODUCT",
        "SESSION_AUDIENCE",
        "SESSION_ISSUER",
        "VerifiedClientCertificate",
        "LiveManifestoCheckRequired",
        "IdentityConfig",
        "AuthConfig",
    ] {
        assert!(
            blob.contains(needle),
            "T3 RED : identifiant identité absent de Lazaret/*/src : {needle}"
        );
    }
    assert!(
        blob.contains("session_ttl_minutes"),
        "T3 RED : [identity] session_ttl_minutes absent"
    );
    let config = std::fs::read_to_string(workspace_root().join("Lazaret/configuration/src/lib.rs"))
        .expect("configuration lisible");
    assert!(
        config.contains("pub struct IdentityConfig"),
        "T3 RED : IdentityConfig absent"
    );
    assert!(
        config.contains("pub auth: AuthConfig"),
        "T3 RED : AuthConfig / [auth.jwt] retiré"
    );
    let identity_struct = identity_config_struct_body(&config);
    assert!(
        !identity_struct.contains("hs256_secret"),
        "T3 RED : IdentityConfig réutilise hs256_secret IAM"
    );
}

#[test]
fn t3_session_proof_is_not_sufficient_authorization() {
    let blob = concat_src(&lazaret_prod_files());
    assert!(
        blob.contains("LiveManifestoCheckRequired"),
        "T3 RED : LiveManifestoCheckRequired absent"
    );
    assert!(
        blob.contains("AUTHORIZATION_FROM_SESSION"),
        "T3 RED : AUTHORIZATION_FROM_SESSION absent"
    );
    assert!(
        blob.contains("fn authorization_from_session"),
        "T3 RED : authorization_from_session absent"
    );
    assert!(
        !blob.contains("AllowBySession"),
        "T3 RED : session traitée comme autorisation suffisante"
    );
}

fn identity_config_struct_body(config: &str) -> String {
    let start = config
        .find("pub struct IdentityConfig")
        .expect("IdentityConfig");
    let from = &config[start..];
    let open = from.find('{').expect("accolade IdentityConfig");
    let mut depth = 0_i32;
    let mut end = 0;
    for (i, ch) in from.char_indices() {
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
    from[..end].to_string()
}
