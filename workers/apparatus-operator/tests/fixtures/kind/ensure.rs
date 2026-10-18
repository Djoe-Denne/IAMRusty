//! Single test-side ensure orchestration (PH-2/3/4/5/8/9/10/16).
//! Scripted ports prove client decisions, never Kubernetes server semantics.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const DEFAULT_CLUSTER_NAME: &str = "aiforall-local";
pub const WINDOWS_KIND: &str = r"C:\Users\djden\bin\kind.exe";
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct ExecOutcome {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub trait FixturePort {
    fn run(
        &mut self,
        program: &str,
        args: &[&str],
        env: &[(String, String)],
    ) -> Result<ExecOutcome, String>;
    fn sleep(&mut self, duration: Duration);
    fn monotonic_ms(&mut self) -> u64;
    /// Explicit filesystem boundary for Kind overrides and the host fallback.
    fn is_file(&mut self, path: &Path) -> bool {
        path.is_file()
    }
}

pub struct RealPort {
    epoch: Instant,
}

impl RealPort {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl FixturePort for RealPort {
    fn run(
        &mut self,
        program: &str,
        args: &[&str],
        env: &[(String, String)],
    ) -> Result<ExecOutcome, String> {
        let output = Command::new(program)
            .args(args)
            .envs(env.iter().map(|(key, value)| (key, value)))
            .output()
            .map_err(|_| "fixture command spawn failed".to_owned())?;
        Ok(ExecOutcome {
            success: output.status.success(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }

    fn monotonic_ms(&mut self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// No raw input appears in a rejection, including non-Unicode environment values.
pub fn target_name(raw: Option<&std::ffi::OsStr>) -> Result<&str, String> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_CLUSTER_NAME);
    };
    let name = raw.to_str().ok_or("cible invalide")?.trim();
    if name.is_empty() {
        return Err("cible vide".to_owned());
    }
    if matches!(
        name,
        "kind-apparatus-p4-it" | "apparatus-p4-it" | "rancher-desktop"
    ) {
        return Err("cible protégée".to_owned());
    }
    if name.len() > 63
        || !name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("cible invalide".to_owned());
    }
    if name != DEFAULT_CLUSTER_NAME {
        return Err("cible non autorisée".to_owned());
    }
    Ok(name)
}

/// Owns only a unique temporary kubeconfig directory, not a user's profile.
#[derive(Debug)]
pub struct KubeconfigFile {
    directory: PathBuf,
    pub path: PathBuf,
}

impl KubeconfigFile {
    pub fn new() -> Result<Self, String> {
        loop {
            let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let directory = std::env::temp_dir()
                .join(format!("apparatus-kubeconfig-{}-{id}", std::process::id()));
            match std::fs::create_dir(&directory) {
                Ok(()) => {
                    return Ok(Self {
                        path: directory.join("config"),
                        directory,
                    })
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err("dedicated kubeconfig creation failed".to_owned()),
            }
        }
    }
}

impl Drop for KubeconfigFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[derive(Debug)]
pub struct PreparedCluster {
    pub kind_bin: PathBuf,
    pub kubeconfig: KubeconfigFile,
}

fn checked(
    port: &mut impl FixturePort,
    program: &str,
    args: &[&str],
    env: &[(String, String)],
    operation: &str,
) -> Result<ExecOutcome, String> {
    let output = port
        .run(program, args, env)
        .map_err(|_| format!("{operation}: spawn failed"))?;
    if !output.success {
        return Err(format!("{operation}: command failed"));
    }
    Ok(output)
}

pub fn probe_kubectl(port: &mut impl FixturePort, kubeconfig: &Path) -> Result<(), String> {
    checked(
        port,
        "kubectl",
        &["version", "--client"],
        &kube_env(kubeconfig),
        "kubectl client probe",
    )?;
    Ok(())
}

fn kube_env(kubeconfig: &Path) -> Vec<(String, String)> {
    vec![(
        "KUBECONFIG".to_owned(),
        kubeconfig.to_string_lossy().into_owned(),
    )]
}

fn kubectl(
    port: &mut impl FixturePort,
    kubeconfig: &Path,
    args: &[&str],
) -> Result<ExecOutcome, String> {
    probe_kubectl(port, kubeconfig)?;
    port.run("kubectl", args, &kube_env(kubeconfig))
        .map_err(|_| "kubectl: spawn failed".to_owned())
}

fn kube_checked(
    port: &mut impl FixturePort,
    kubeconfig: &Path,
    args: &[&str],
    operation: &str,
) -> Result<ExecOutcome, String> {
    let output = kubectl(port, kubeconfig, args)?;
    if !output.success {
        return Err(format!("{operation}: command failed"));
    }
    Ok(output)
}

fn resolve_kind(
    port: &mut impl FixturePort,
    override_bin: Option<OsString>,
) -> Result<PathBuf, String> {
    if let Some(raw) = override_bin {
        let path = PathBuf::from(raw);
        if !port.is_file(&path) || path.to_str().is_none() {
            return Err("Kind override is not a usable file".to_owned());
        }
        return Ok(path);
    }
    match port.run("kind", &["version"], &[]) {
        Ok(out) if out.success => Ok(PathBuf::from("kind")),
        result => {
            // Real adapter retains the existing host-specific filesystem fallback;
            // scripted ports control this read, never depend on a developer's file.
            let fallback = PathBuf::from(WINDOWS_KIND);
            if port.is_file(&fallback) {
                return Ok(fallback);
            }
            Err(if result.is_err() {
                "kind probe: spawn failed"
            } else {
                "kind probe: command failed"
            }
            .to_owned())
        }
    }
}

fn wait_api(port: &mut impl FixturePort, kubeconfig: &Path) -> Result<(), String> {
    let deadline = port.monotonic_ms().saturating_add(180_000);
    loop {
        let output = kubectl(port, kubeconfig, &["get", "--raw=/readyz"])?;
        if output.success {
            return Ok(());
        }
        if port.monotonic_ms() > deadline {
            return Err("Kind API readiness timeout".to_owned());
        }
        port.sleep(Duration::from_secs(2));
    }
}

fn daemonset_exists(
    port: &mut impl FixturePort,
    kubeconfig: &Path,
    name: &str,
) -> Result<bool, String> {
    // Structured absence, never reinterpret an RBAC/API error by substring.
    let out = kube_checked(
        port,
        kubeconfig,
        &[
            "get",
            "ds",
            name,
            "-n",
            "kube-system",
            "-o",
            "name",
            "--ignore-not-found=true",
        ],
        "CNI daemonset read",
    )?;
    Ok(!String::from_utf8_lossy(&out.stdout).trim().is_empty())
}

fn cni_ready(port: &mut impl FixturePort, kubeconfig: &Path) -> Result<(), String> {
    if daemonset_exists(port, kubeconfig, "kindnet")? {
        return Err("kindnet forbidden".to_owned());
    }
    if !daemonset_exists(port, kubeconfig, "calico-node")? {
        return Err("calico-node absent: runtime operation required".to_owned());
    }
    let waits: &[&[&str]] = &[
        &[
            "rollout",
            "status",
            "ds/calico-node",
            "-n",
            "kube-system",
            "--timeout=600s",
        ],
        &[
            "rollout",
            "status",
            "deploy/calico-kube-controllers",
            "-n",
            "kube-system",
            "--timeout=300s",
        ],
        &[
            "wait",
            "--for=condition=Ready",
            "pod",
            "-l",
            "k8s-app=calico-node",
            "-n",
            "kube-system",
            "--timeout=600s",
        ],
        &[
            "wait",
            "--for=condition=Ready",
            "pod",
            "-l",
            "k8s-app=calico-kube-controllers",
            "-n",
            "kube-system",
            "--timeout=300s",
        ],
        &[
            "wait",
            "--for=condition=Ready",
            "pod",
            "-l",
            "k8s-app=kube-dns",
            "-n",
            "kube-system",
            "--timeout=300s",
        ],
        &[
            "wait",
            "--for=condition=Ready",
            "node",
            "--all",
            "--timeout=180s",
        ],
    ];
    for args in waits {
        kube_checked(port, kubeconfig, args, "CNI readiness")?;
    }
    if daemonset_exists(port, kubeconfig, "kindnet")? {
        return Err("kindnet forbidden".to_owned());
    }
    Ok(())
}

pub fn apply_operator_manifests_with(
    port: &mut impl FixturePort,
    kubeconfig: &Path,
) -> Result<(), String> {
    let manifests = Path::new(env!("CARGO_MANIFEST_DIR")).join("k8s");
    let path = manifests.to_string_lossy();
    kube_checked(
        port,
        kubeconfig,
        &[
            "apply",
            "--server-side",
            "--field-manager=apparatus-it-fixtures",
            "-f",
            &path,
        ],
        "operator SSA apply",
    )?;
    kube_checked(
        port,
        kubeconfig,
        &[
            "wait",
            "--for=condition=Established",
            "crd/admissionrecords.apparatus.aiforall.dev",
            "--timeout=90s",
        ],
        "AdmissionRecord CRD readiness",
    )?;
    Ok(())
}

/// The real entry and scripted oracle call this exact body; no alternate ensure.
pub fn ensure_with(
    raw_target: Option<&std::ffi::OsStr>,
    override_bin: Option<OsString>,
    port: &mut impl FixturePort,
) -> Result<PreparedCluster, String> {
    let name = target_name(raw_target)?;
    checked(port, "docker", &["info"], &[], "Docker readiness")?;
    let kind_bin = resolve_kind(port, override_bin)?;
    let program = kind_bin.to_str().ok_or("Kind path invalid")?;
    let clusters = checked(port, program, &["get", "clusters"], &[], "Kind clusters")?;
    if !String::from_utf8_lossy(&clusters.stdout)
        .lines()
        .any(|line| line.trim() == name)
    {
        return Err("authorized Kind cluster absent: runtime operation required".to_owned());
    }
    let kubeconfig = KubeconfigFile::new()?;
    let path = kubeconfig.path.to_string_lossy();
    checked(
        port,
        program,
        &[
            "export",
            "kubeconfig",
            "--name",
            name,
            "--kubeconfig",
            &path,
        ],
        &kube_env(&kubeconfig.path),
        "Kind kubeconfig export",
    )?;
    wait_api(port, &kubeconfig.path)?;
    cni_ready(port, &kubeconfig.path)?;
    apply_operator_manifests_with(port, &kubeconfig.path)?;
    Ok(PreparedCluster {
        kind_bin,
        kubeconfig,
    })
}
