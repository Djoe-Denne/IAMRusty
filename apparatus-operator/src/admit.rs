//! Signataire + registry P4 (feature `admit`).
//!
//! Pousse une enveloppe ORAS **non-CRI** et la signe avec Cosign + `OpenBao`
//! Transit. ORAS et Cosign passent par `std::process::Command` (binaire hôte
//! ou `docker run`). Le HTTP Transit (reqwest) vit **uniquement** ici — pas
//! dans le worker de build. Ce module ne compile ni n'exécute le code auteur.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::digest::ReleaseDigest;

/// Variable d'environnement pour un binaire Cosign hôte (sinon image Docker).
pub const COSIGN_BIN_ENV: &str = "APPARATUS_COSIGN_BIN";
/// Variable d'environnement pour un binaire ORAS hôte (sinon image Docker).
pub const ORAS_BIN_ENV: &str = "APPARATUS_ORAS_BIN";
/// Image Cosign pinée (helpers Windows sans binaire dans le PATH).
pub const COSIGN_IMAGE: &str = "gcr.io/projectsigstore/cosign:v2.4.3";
/// Image ORAS pinée.
pub const ORAS_IMAGE: &str = "ghcr.io/oras-project/oras:v1.2.2";
/// Type d'artifact ORAS : pas un index/manifeste CRI comme identité catalogue.
pub const ENVELOPE_ARTIFACT_TYPE: &str = "application/vnd.aiforall.apparatus.envelope.v1+json";

/// Identité HTTP pour pousser vers zot (seul le signer est autorisé).
#[derive(Debug, Clone)]
pub struct RegistryAuth {
    /// Utilisateur htpasswd.
    pub username: String,
    /// Mot de passe htpasswd (tests uniquement).
    pub password: String,
}

/// Cibles registry + Transit vues de l'hôte et du réseau Docker.
#[derive(Debug, Clone)]
pub struct AdmitTarget {
    /// `127.0.0.1:{port}` pour un binaire hôte.
    pub registry_host: String,
    /// `{container}:5000` pour un helper sur le réseau de test.
    pub registry_network: String,
    /// Réseau Docker des fixtures zot/`OpenBao`.
    pub docker_network: String,
    /// `http://127.0.0.1:{port}` pour reqwest (processus hôte).
    pub vault_addr_host: String,
    /// `http://{container}:8200` pour Cosign en conteneur.
    pub vault_addr_network: String,
    /// Token `OpenBao` (Transit).
    pub vault_token: String,
    /// Nom de clé Transit (`ecdsa-p256`).
    pub transit_key: String,
    /// Chemin de dépôt (`namespace/name`).
    pub repository: String,
    /// Tag d'enveloppe (pas l'identité catalogue).
    pub tag: String,
    /// Identité autorisée à pousser.
    pub auth: RegistryAuth,
}

/// Document d'enveloppe : digest 0002 + pin CRI.
#[derive(Debug, Clone)]
pub struct Envelope {
    /// Identité catalogue ([`ReleaseDigest`] ADR-0002).
    pub descriptor_digest: ReleaseDigest,
    /// Image runtime pinée `name@sha256:…` (pas `latest`).
    pub cri_image: String,
}

/// Résultat d'un push + signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedEnvelope {
    /// Identité catalogue = digest descripteur 0002.
    pub catalog_digest: ReleaseDigest,
    /// Digest OCI de l'enveloppe (≠ [`Self::catalog_digest`]).
    pub envelope_oci_digest: String,
    /// Référence registry (`host/repo@sha256:…`).
    pub reference: String,
    /// Pin CRI recopié.
    pub cri_image: String,
    /// Type d'artifact ORAS.
    pub artifact_type: String,
    /// `true` uniquement si `cosign verify` a réussi (pas un tag `.sig`).
    pub signature_verified: bool,
}

/// Erreur d'admission / signature (messages sans secret).
#[derive(Debug)]
pub struct AdmitError {
    message: String,
}

impl AdmitError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AdmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AdmitError {}

#[derive(Debug, Serialize, Deserialize)]
struct EnvelopeDocument {
    descriptor_digest: String,
    cri_image: String,
}

/// Pousse l'enveloppe non-CRI et la signe (Cosign + Transit).
///
/// Le signer ne lance pas de compilateur et n'exécute pas de `build.rs` auteur.
///
/// # Errors
///
/// Transit injoignable, pin CRI invalide, échec ORAS/Cosign, ou digest OCI
/// identique au digest catalogue (ne devrait pas arriver).
pub async fn push_and_sign_envelope(
    target: &AdmitTarget,
    envelope: &Envelope,
) -> Result<SignedEnvelope, AdmitError> {
    validate_cri_pin(&envelope.cri_image)?;
    inspect_transit_key(
        &target.vault_addr_host,
        &target.vault_token,
        &target.transit_key,
    )
    .await?;

    let document = EnvelopeDocument {
        descriptor_digest: envelope.descriptor_digest.as_str().to_owned(),
        cri_image: envelope.cri_image.clone(),
    };
    let json = serde_json::to_string(&document)
        .map_err(|err| AdmitError::new(format!("serialize envelope: {err}")))?;

    let workdir = make_workdir()?;
    let envelope_path = workdir.join("envelope.json");
    fs::write(&envelope_path, json.as_bytes())
        .map_err(|err| AdmitError::new(format!("write envelope.json: {err}")))?;

    let oras_host = host_tool(ORAS_BIN_ENV);
    let registry = if oras_host.is_some() {
        target.registry_host.as_str()
    } else {
        target.registry_network.as_str()
    };
    let tagged = format!("{registry}/{}:{}", target.repository, target.tag);

    let push_args = vec![
        "push".to_owned(),
        "--plain-http".to_owned(),
        "--username".to_owned(),
        target.auth.username.clone(),
        "--password".to_owned(),
        target.auth.password.clone(),
        tagged.clone(),
        "--artifact-type".to_owned(),
        ENVELOPE_ARTIFACT_TYPE.to_owned(),
        format!("envelope.json:{ENVELOPE_ARTIFACT_TYPE}"),
    ];
    let push = run_cli(
        ORAS_BIN_ENV,
        ORAS_IMAGE,
        &target.docker_network,
        &[],
        &push_args,
        Some(workdir.as_path()),
    )?;
    if !push.status.success() {
        return Err(cli_failed("oras push", &push, &target.vault_token));
    }

    let desc_args = vec![
        "manifest".to_owned(),
        "fetch".to_owned(),
        "--plain-http".to_owned(),
        "--descriptor".to_owned(),
        tagged,
    ];
    let desc = run_cli(
        ORAS_BIN_ENV,
        ORAS_IMAGE,
        &target.docker_network,
        &[],
        &desc_args,
        None,
    )?;
    if !desc.status.success() {
        return Err(cli_failed(
            "oras manifest fetch",
            &desc,
            &target.vault_token,
        ));
    }
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&push.stdout),
        String::from_utf8_lossy(&desc.stdout)
    );
    let oci_digest = parse_oci_digest(&combined)?;
    if oci_digest == envelope.descriptor_digest.as_str() {
        return Err(AdmitError::new(
            "OCI envelope digest unexpectedly equals catalog descriptor_digest",
        ));
    }

    let digest_ref = format!("{registry}/{}@{oci_digest}", target.repository);
    let signature_verified = sign_and_verify(target, &digest_ref)?;
    let _ = fs::remove_dir_all(&workdir);

    Ok(SignedEnvelope {
        catalog_digest: envelope.descriptor_digest.clone(),
        envelope_oci_digest: oci_digest,
        reference: digest_ref,
        cri_image: envelope.cri_image.clone(),
        artifact_type: ENVELOPE_ARTIFACT_TYPE.to_owned(),
        signature_verified,
    })
}

/// GET Transit `keys/{name}` depuis le process hôte (reqwest, pas le CLI).
///
/// # Errors
///
/// HTTP non-succès, JSON inattendu, ou type de clé autre que ECDSA P-256.
pub async fn inspect_transit_key(
    vault_addr: &str,
    token: &str,
    key: &str,
) -> Result<(), AdmitError> {
    let url = format!("{}/v1/transit/keys/{key}", vault_addr.trim_end_matches('/'));
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|err| AdmitError::new(format!("reqwest client: {err}")))?
        .get(&url)
        .header("X-Vault-Token", token)
        .send()
        .await
        .map_err(|err| AdmitError::new(format!("Transit GET {url}: {err}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| AdmitError::new(format!("Transit body: {err}")))?;
    if !status.is_success() {
        return Err(AdmitError::new(format!(
            "Transit GET {url} failed: {status} {}",
            redact(&body, token)
        )));
    }
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|err| AdmitError::new(format!("Transit JSON: {err}")))?;
    let key_type = value
        .get("data")
        .and_then(|data| data.get("type"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if key_type != "ecdsa-p256" {
        return Err(AdmitError::new(format!(
            "Transit key type must be ecdsa-p256, got {key_type}"
        )));
    }
    Ok(())
}

fn validate_cri_pin(cri_image: &str) -> Result<(), AdmitError> {
    let Some((_, digest)) = cri_image.split_once("@sha256:") else {
        return Err(AdmitError::new(
            "cri_image must be pinned as name@sha256:<64 hex>",
        ));
    };
    let valid_digest = digest.len() == 64
        && digest
            .as_bytes()
            .iter()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if !valid_digest {
        return Err(AdmitError::new(
            "cri_image must be pinned as name@sha256:<64 hex>",
        ));
    }
    if cri_image.ends_with(":latest") || cri_image.contains(":latest@") {
        return Err(AdmitError::new("cri_image must not use tag latest"));
    }
    Ok(())
}

fn sign_and_verify(target: &AdmitTarget, digest_ref: &str) -> Result<bool, AdmitError> {
    let host_cosign = host_tool(COSIGN_BIN_ENV).is_some();
    let vault_addr = if host_cosign {
        target.vault_addr_host.as_str()
    } else {
        target.vault_addr_network.as_str()
    };
    let key_uri = format!("hashivault://{}", target.transit_key);
    let env_vars = [
        ("VAULT_ADDR", vault_addr),
        ("VAULT_TOKEN", target.vault_token.as_str()),
        ("TRANSIT_SECRET_ENGINE_PATH", "transit"),
        ("COSIGN_YES", "1"),
    ];

    let sign_args = vec![
        "sign".to_owned(),
        "--yes".to_owned(),
        "--tlog-upload=false".to_owned(),
        "--allow-insecure-registry".to_owned(),
        "--allow-http-registry".to_owned(),
        "--registry-username".to_owned(),
        target.auth.username.clone(),
        "--registry-password".to_owned(),
        target.auth.password.clone(),
        "--key".to_owned(),
        key_uri,
        digest_ref.to_owned(),
    ];
    let signed = run_cli(
        COSIGN_BIN_ENV,
        COSIGN_IMAGE,
        &target.docker_network,
        &env_vars,
        &sign_args,
        None,
    )?;
    if !signed.status.success() {
        return Err(cli_failed("cosign sign", &signed, &target.vault_token));
    }

    verify_cosign_signature(target, digest_ref)?;
    Ok(true)
}

/// `Ok` uniquement si `cosign verify` réussit. Un tag `.sig` n'est pas une preuve.
///
/// # Errors
///
/// `cosign verify` non-succès (y compris tag `.sig` fetchable mais crypto KO).
pub fn verify_cosign_signature(target: &AdmitTarget, digest_ref: &str) -> Result<(), AdmitError> {
    let host_cosign = host_tool(COSIGN_BIN_ENV).is_some();
    let vault_addr = if host_cosign {
        target.vault_addr_host.as_str()
    } else {
        target.vault_addr_network.as_str()
    };
    let key_uri = format!("hashivault://{}", target.transit_key);
    let env_vars = [
        ("VAULT_ADDR", vault_addr),
        ("VAULT_TOKEN", target.vault_token.as_str()),
        ("TRANSIT_SECRET_ENGINE_PATH", "transit"),
        ("COSIGN_YES", "1"),
    ];
    let verify_args = vec![
        "verify".to_owned(),
        "--insecure-ignore-tlog".to_owned(),
        "--allow-insecure-registry".to_owned(),
        "--allow-http-registry".to_owned(),
        "--key".to_owned(),
        key_uri,
        digest_ref.to_owned(),
    ];
    let verified = run_cli(
        COSIGN_BIN_ENV,
        COSIGN_IMAGE,
        &target.docker_network,
        &env_vars,
        &verify_args,
        None,
    )?;
    if verified.status.success() {
        return Ok(());
    }
    Err(cli_failed(
        "cosign verify (Vault-compat Transit)",
        &verified,
        &target.vault_token,
    ))
}

/// Existence du tag Cosign `:sha256-{hex}.sig` (précondition de test, pas un oracle crypto).
///
/// # Errors
///
/// ORAS injoignable.
pub fn signature_tag_exists(target: &AdmitTarget, digest_ref: &str) -> Result<bool, AdmitError> {
    let Some((_, digest)) = digest_ref.rsplit_once('@') else {
        return Ok(false);
    };
    let hex = digest.trim_start_matches("sha256:");
    let host_oras = host_tool(ORAS_BIN_ENV).is_some();
    let registry = if host_oras {
        target.registry_host.as_str()
    } else {
        target.registry_network.as_str()
    };
    let sig_ref = format!("{registry}/{}:sha256-{hex}.sig", target.repository);
    let args = vec![
        "manifest".to_owned(),
        "fetch".to_owned(),
        "--plain-http".to_owned(),
        sig_ref,
    ];
    let output = run_cli(
        ORAS_BIN_ENV,
        ORAS_IMAGE,
        &target.docker_network,
        &[],
        &args,
        None,
    )?;
    Ok(output.status.success())
}

fn run_cli(
    bin_env: &str,
    image: &str,
    network: &str,
    extra_env: &[(&str, &str)],
    args: &[String],
    workdir: Option<&Path>,
) -> Result<Output, AdmitError> {
    if let Some(bin) = host_tool(bin_env) {
        let mut cmd = Command::new(&bin);
        cmd.args(args);
        for (key, value) in extra_env {
            cmd.env(key, value);
        }
        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }
        return cmd.output().map_err(|err| {
            AdmitError::new(format!(
                "cannot execute {} ({}): {err}",
                bin.display(),
                bin_env
            ))
        });
    }

    let mut cmd = Command::new("docker");
    cmd.args(["run", "--rm", "--network", network]);
    for (key, value) in extra_env {
        cmd.args(["-e", &format!("{key}={value}")]);
    }
    if let Some(dir) = workdir {
        let host = docker_host_path(dir)?;
        cmd.args(["-v", &format!("{host}:/work"), "-w", "/work"]);
    }
    cmd.arg(image);
    cmd.args(args);
    cmd.output().map_err(|err| {
        AdmitError::new(format!(
            "Docker cannot start helper {image}: {err}. Docker daemon must be running; image pull errors must not be ignored."
        ))
    })
}

fn host_tool(bin_env: &str) -> Option<PathBuf> {
    std::env::var_os(bin_env).map(PathBuf::from)
}

fn docker_host_path(path: &Path) -> Result<String, AdmitError> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| AdmitError::new(format!("canonicalize {}: {err}", path.display())))?;
    let mut rendered = canonical.to_string_lossy().into_owned();
    if let Some(stripped) = rendered.strip_prefix(r"\\?\") {
        rendered = stripped.to_owned();
    }
    Ok(rendered.replace('\\', "/"))
}

fn make_workdir() -> Result<PathBuf, AdmitError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("apparatus-admit-{nanos}"));
    fs::create_dir_all(&dir).map_err(|err| AdmitError::new(format!("create workdir: {err}")))?;
    Ok(dir)
}

fn parse_oci_digest(text: &str) -> Result<String, AdmitError> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text.trim()) {
        if let Some(digest) = value.get("digest").and_then(serde_json::Value::as_str) {
            if is_sha256_digest(digest) {
                return Ok(digest.to_owned());
            }
        }
    }
    for line in text.lines() {
        let trimmed = line.trim();
        let candidate = trimmed.strip_prefix("Digest:").unwrap_or(trimmed).trim();
        if let Some(idx) = candidate.find("sha256:") {
            let digest = candidate[idx..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(|c: char| !c.is_ascii_hexdigit() && c != ':');
            if is_sha256_digest(digest) {
                return Ok(digest.to_owned());
            }
        }
    }
    Err(AdmitError::new(
        "cannot parse OCI envelope digest from oras output",
    ))
}

fn is_sha256_digest(value: &str) -> bool {
    value.starts_with("sha256:")
        && value.len() == 71
        && value.as_bytes()[7..].iter().all(u8::is_ascii_hexdigit)
}

fn cli_failed(op: &str, output: &Output, token: &str) -> AdmitError {
    let stdout = redact(&String::from_utf8_lossy(&output.stdout), token);
    let stderr = redact(&String::from_utf8_lossy(&output.stderr), token);
    AdmitError::new(format!(
        "{op} failed status={:?} stdout={stdout} stderr={stderr}",
        output.status.code()
    ))
}

fn redact(text: &str, token: &str) -> String {
    if token.is_empty() {
        return text.to_owned();
    }
    text.replace(token, "***")
}
