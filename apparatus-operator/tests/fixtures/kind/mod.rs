//! Cluster Kind `apparatus-p4-it` (IT T10). Fail-loud si Docker/Kind absents.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Nom du cluster Kind singleton.
pub const CLUSTER_NAME: &str = "apparatus-p4-it";
/// Variable d'environnement pour un binaire Kind hors PATH.
pub const KIND_BIN_ENV: &str = "APPARATUS_KIND_BIN";
const WINDOWS_KIND: &str = r"C:\Users\djden\bin\kind.exe";
const IMAGE_TAG: &str = "apparatus-p4-platform-plugin:t10";
/// Calico piné (Kind standard `manifests/calico.yaml`, pas `latest`).
const CALICO_VERSION: &str = "v3.29.7";
const CALICO_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/projectcalico/calico/v3.29.7/manifests/calico.yaml";

/// Cluster Kind prêt : kubeconfig exporté, manifests operator appliqués.
pub struct KindCluster {
    kind_bin: PathBuf,
    /// Kubeconfig Kind (fichier dédié, ne remplace pas `~/.kube/config`).
    pub kubeconfig: PathBuf,
}

impl KindCluster {
    /// Réutilise le cluster s'il existe, sinon le crée. Fail-loud Docker/Kind.
    /// Installe Calico piné et attend Ready **avant** d'appliquer les `NetworkPolicy`.
    /// Recrée le cluster si kindnet est détecté (interdit pour T11).
    ///
    /// # Errors
    ///
    /// Docker ou Kind injoignable, création Kind en échec, kubectl absent,
    /// Calico non Ready, ou apply des manifests operator en échec.
    pub fn ensure() -> Result<Self, Box<dyn std::error::Error>> {
        super::assert_docker_running().map_err(|err| {
            let boxed: Box<dyn std::error::Error> =
                format!("Docker/Kind must be running: {err}").into();
            boxed
        })?;
        let kind_bin = resolve_kind_bin()?;
        if cluster_exists(&kind_bin)? {
            let kubeconfig = export_kubeconfig(&kind_bin)?;
            let probe = Self {
                kind_bin: kind_bin.clone(),
                kubeconfig,
            };
            let reusable = probe.wait_api_server().is_ok() && !probe.has_kindnet().unwrap_or(true);
            if !reusable {
                probe.delete_cluster()?;
                create_cluster(&kind_bin)?;
            }
        } else {
            create_cluster(&kind_bin)?;
        }
        let kubeconfig = export_kubeconfig(&kind_bin)?;
        let cluster = Self {
            kind_bin,
            kubeconfig,
        };
        cluster.wait_api_server()?;
        cluster.install_calico_and_wait()?;
        cluster.apply_operator_manifests()?;
        Ok(cluster)
    }

    /// Applique `apparatus-operator/k8s/*.yaml` et attend la CRD.
    ///
    /// # Errors
    ///
    /// `kubectl apply` ou `kubectl wait` CRD en échec.
    pub fn apply_operator_manifests(&self) -> Result<(), Box<dyn std::error::Error>> {
        let k8s = Path::new(env!("CARGO_MANIFEST_DIR")).join("k8s");
        let k8s_s = k8s.to_string_lossy().into_owned();
        let apply = self.kubectl(&["apply", "-f", k8s_s.as_str()])?;
        if !apply.status.success() {
            return Err(cmd_err(
                "kubectl apply operator manifests",
                &apply,
                "Docker/Kind must be running",
            ));
        }
        let wait = self.kubectl(&[
            "wait",
            "--for=condition=Established",
            "crd/admissionrecords.apparatus.aiforall.dev",
            "--timeout=90s",
        ])?;
        if !wait.status.success() {
            return Err(cmd_err(
                "kubectl wait CRD AdmissionRecord",
                &wait,
                "Docker/Kind must be running",
            ));
        }
        Ok(())
    }

    fn delete_cluster(&self) -> Result<(), Box<dyn std::error::Error>> {
        let out = Command::new(&self.kind_bin)
            .args(["delete", "cluster", "--name", CLUSTER_NAME])
            .output()
            .map_err(|err| format!("Docker/Kind must be running (kind delete: {err})"))?;
        if !out.status.success() {
            return Err(cmd_err(
                "kind delete cluster",
                &out,
                "Docker/Kind must be running",
            ));
        }
        Ok(())
    }

    fn daemonset_exists(&self, name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let out = self.kubectl(&["get", "ds", name, "-n", "kube-system", "-o", "name"])?;
        if out.status.success() {
            return Ok(!String::from_utf8_lossy(&out.stdout).trim().is_empty());
        }
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
        .to_ascii_lowercase();
        if text.contains("notfound") || text.contains("not found") {
            return Ok(false);
        }
        Err(cmd_err(
            &format!("kubectl get ds {name}"),
            &out,
            "Docker/Kind must be running",
        ))
    }

    fn has_kindnet(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.daemonset_exists("kindnet")
    }

    fn has_calico_node(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.daemonset_exists("calico-node")
    }

    fn wait_api_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let deadline = Instant::now() + Duration::from_secs(180);
        let mut last: Option<Output> = None;
        loop {
            match self.kubectl(&["get", "--raw=/readyz"]) {
                Ok(out) if out.status.success() => return Ok(()),
                Ok(out) => last = Some(out),
                Err(err) => {
                    if Instant::now() > deadline {
                        return Err(err);
                    }
                }
            }
            if Instant::now() > deadline {
                return Err(last.map_or_else(
                    || "Docker/Kind must be running: API Kind injoignable".into(),
                    |out| {
                        cmd_err(
                            "kubectl get --raw=/readyz",
                            &out,
                            "Docker/Kind must be running",
                        )
                    },
                ));
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    fn wait_kubectl(&self, args: &[&str], op: &str) -> Result<(), Box<dyn std::error::Error>> {
        let out = self.kubectl(args)?;
        if !out.status.success() {
            return Err(cmd_err(op, &out, "Docker/Kind must be running"));
        }
        Ok(())
    }

    fn set_calico_kind_env(&self) -> Result<(), Box<dyn std::error::Error>> {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            if self.has_calico_node()? {
                let out = self.kubectl(&[
                    "-n",
                    "kube-system",
                    "set",
                    "env",
                    "daemonset/calico-node",
                    "FELIX_IGNORELOOSERPF=true",
                ])?;
                if out.status.success() {
                    return Ok(());
                }
                if Instant::now() > deadline {
                    return Err(cmd_err(
                        "kubectl set env calico-node FELIX_IGNORELOOSERPF",
                        &out,
                        "Docker/Kind must be running",
                    ));
                }
            } else if Instant::now() > deadline {
                return Err(format!(
                    "Docker/Kind must be running: calico-node DS absent after apply {CALICO_VERSION}"
                )
                .into());
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    /// Installe Calico piné et attend Ready **avant** `k8s/04-networkpolicy.yaml`.
    fn install_calico_and_wait(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.has_kindnet()? {
            return Err(
                "Docker/Kind must be running: kindnet is forbidden for T11 NetworkPolicy proof"
                    .into(),
            );
        }
        if !self.has_calico_node()? {
            eprintln!("apparatus-p4-it: apply Calico {CALICO_VERSION} ({CALICO_MANIFEST_URL})");
            let apply = self.kubectl(&["apply", "-f", CALICO_MANIFEST_URL])?;
            if !apply.status.success() {
                return Err(cmd_err(
                    "kubectl apply Calico",
                    &apply,
                    "Docker/Kind must be running",
                ));
            }
            self.set_calico_kind_env()?;
        }
        self.wait_cni_ready()
    }

    fn wait_cni_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("apparatus-p4-it: wait Calico + CoreDNS Ready ({CALICO_VERSION}, timeout 600s)");
        self.wait_kubectl(
            &[
                "rollout",
                "status",
                "ds/calico-node",
                "-n",
                "kube-system",
                "--timeout=600s",
            ],
            "calico-node rollout",
        )?;
        self.wait_kubectl(
            &[
                "rollout",
                "status",
                "deploy/calico-kube-controllers",
                "-n",
                "kube-system",
                "--timeout=300s",
            ],
            "calico-kube-controllers rollout",
        )?;
        self.wait_kubectl(
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
            "calico-node Ready",
        )?;
        self.wait_kubectl(
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
            "calico-kube-controllers Ready",
        )?;
        self.wait_kubectl(
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
            "CoreDNS Ready",
        )?;
        self.wait_kubectl(
            &[
                "wait",
                "--for=condition=Ready",
                "node",
                "--all",
                "--timeout=180s",
            ],
            "nodes Ready",
        )?;
        if self.has_kindnet()? {
            return Err(
                "Docker/Kind must be running: kindnet is forbidden for T11 NetworkPolicy proof"
                    .into(),
            );
        }
        Ok(())
    }

    /// Construit l'image in-repo, la charge dans Kind, retourne le pin CRI `@sha256`.
    ///
    /// # Errors
    ///
    /// `docker build`, `kind load`, ou inspection digest en échec.
    pub fn build_and_load_platform_plugin(&self) -> Result<String, Box<dyn std::error::Error>> {
        let context = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/platform-plugin");
        let build = Command::new("docker")
            .args(["build", "-t", IMAGE_TAG])
            .arg(&context)
            .output()
            .map_err(|err| format!("Docker/Kind must be running (docker build: {err})"))?;
        if !build.status.success() {
            return Err(cmd_err(
                "docker build platform-plugin",
                &build,
                "Docker/Kind must be running",
            ));
        }
        let load = Command::new(&self.kind_bin)
            .args(["load", "docker-image", IMAGE_TAG, "--name", CLUSTER_NAME])
            .output()
            .map_err(|err| format!("Docker/Kind must be running (kind load: {err})"))?;
        if !load.status.success() {
            return Err(cmd_err(
                "kind load docker-image",
                &load,
                "Docker/Kind must be running",
            ));
        }
        cri_pin_from_node(CLUSTER_NAME)
    }

    /// Preuve HTTP depuis un Pod Kind vers `url` (zot `/v2/` via `host.docker.internal`).
    ///
    /// # Errors
    ///
    /// DNS / connexion refusée, ou kubectl run/logs en échec.
    pub fn probe_http_from_cluster(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.kubectl(&["delete", "pod", "t10-zot-probe", "--ignore-not-found=true"]);
        let run = self.kubectl(&[
            "run",
            "t10-zot-probe",
            "--restart=Never",
            "--image",
            IMAGE_TAG,
            "--image-pull-policy=Never",
            "--command",
            "--",
            "wget",
            "-T",
            "10",
            "-O",
            "-",
            url,
        ])?;
        if !run.status.success() {
            return Err(cmd_err(
                "kubectl run zot probe",
                &run,
                "Docker/Kind must be running",
            ));
        }
        wait_probe_done(self)?;
        let logs = self.kubectl(&["logs", "t10-zot-probe"])?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&logs.stdout),
            String::from_utf8_lossy(&logs.stderr)
        );
        let describe = self.kubectl(&["describe", "pod", "t10-zot-probe"])?;
        let describe_txt = format!(
            "{}{}",
            String::from_utf8_lossy(&describe.stdout),
            String::from_utf8_lossy(&describe.stderr)
        );
        let blob = format!("{combined}\n{describe_txt}");
        for needle in [
            "Connection refused",
            "connection refused",
            "bad address",
            "Name or service not known",
            "Temporary failure in name resolution",
            "network is unreachable",
        ] {
            if blob.contains(needle) {
                return Err(format!(
                    "Docker/Kind must be running: Kind cannot reach {url}: {needle}"
                )
                .into());
            }
        }
        let _ = self.kubectl(&["delete", "pod", "t10-zot-probe", "--ignore-not-found=true"]);
        Ok(())
    }

    /// `kubectl` borné au kubeconfig Kind.
    ///
    /// # Errors
    ///
    /// Binaire kubectl injoignable.
    pub fn kubectl(&self, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
        let kubectl = resolve_kubectl()?;
        Command::new(&kubectl)
            .args(args)
            .env("KUBECONFIG", &self.kubeconfig)
            .output()
            .map_err(|err| format!("Docker/Kind must be running (kubectl: {err})").into())
    }

    /// Lance `kubectl` en arrière-plan (port-forward).
    ///
    /// # Errors
    ///
    /// Binaire kubectl injoignable.
    pub fn spawn_kubectl(
        &self,
        args: &[&str],
    ) -> Result<std::process::Child, Box<dyn std::error::Error>> {
        let kubectl = resolve_kubectl()?;
        Command::new(&kubectl)
            .args(args)
            .env("KUBECONFIG", &self.kubeconfig)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| format!("Docker/Kind must be running (kubectl spawn: {err})").into())
    }

    /// `kind load` d'une image locale puis pin CRI `{pin_name}@sha256:…` sur le nœud.
    ///
    /// # Errors
    ///
    /// `kind load` ou inspection CRI en échec.
    pub fn load_and_pin_image(
        &self,
        tag: &str,
        pin_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let load = Command::new(&self.kind_bin)
            .args(["load", "docker-image", tag, "--name", CLUSTER_NAME])
            .output()
            .map_err(|err| format!("Docker/Kind must be running (kind load: {err})"))?;
        if !load.status.success() {
            return Err(cmd_err(
                "kind load docker-image",
                &load,
                "Docker/Kind must be running",
            ));
        }
        cri_pin_from_loaded_tag(CLUSTER_NAME, tag, pin_name)
    }

    /// Mirror HTTP `host.docker.internal:{port}` dans containerd du nœud Kind.
    ///
    /// # Errors
    ///
    /// Écriture `certs.d` ou redémarrage containerd en échec.
    pub fn ensure_zot_http_mirror(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let node = format!("{CLUSTER_NAME}-control-plane");
        let hosts_dir = format!("/etc/containerd/certs.d/host.docker.internal:{port}");
        let hosts_file = format!("{hosts_dir}/hosts.toml");
        let contents = format!(
            "server = \"http://host.docker.internal:{port}\"\n\n\
             [host.\"http://host.docker.internal:{port}\"]\n\
             \tcapabilities = [\"pull\", \"resolve\"]\n\
             \tskip_verify = true\n"
        );
        let mkdir = node_exec(&node, &["mkdir", "-p", &hosts_dir])?;
        if !mkdir.status.success() {
            return Err(cmd_err(
                "mkdir containerd certs.d zot mirror",
                &mkdir,
                "Docker/Kind must be running",
            ));
        }
        node_write_file(&node, &hosts_file, &contents)?;
        ensure_containerd_registry_config_path(&node)?;
        let restart = node_exec(&node, &["systemctl", "restart", "containerd"])?;
        if !restart.status.success() {
            return Err(cmd_err(
                "systemctl restart containerd",
                &restart,
                "Docker/Kind must be running",
            ));
        }
        wait_crictl_ready(&node)?;
        let wait = self.kubectl(&[
            "wait",
            "--for=condition=Ready",
            "node",
            "--all",
            "--timeout=90s",
        ])?;
        if !wait.status.success() {
            return Err(cmd_err(
                "kubectl wait node Ready after containerd restart",
                &wait,
                "Docker/Kind must be running",
            ));
        }
        Ok(())
    }

    /// `crictl inspecti` de la ref registre doit échouer (image absente).
    ///
    /// # Errors
    ///
    /// Image déjà présente (faux vert) ou `crictl` injoignable.
    pub fn assert_registry_cri_absent(
        &self,
        cri_image: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let node = format!("{CLUSTER_NAME}-control-plane");
        let out = node_exec(&node, &["crictl", "inspecti", cri_image])?;
        if out.status.success() {
            return Err(format!(
                "faux vert: crictl inspecti a trouvé {cri_image} sur {node} avant schedule"
            )
            .into());
        }
        Ok(())
    }
}

/// Construit l'image plugin avec un LABEL unique, la pousse vers zot HTTP, pin `@sha256`.
///
/// # Errors
///
/// `docker build`, skopeo/crane absents, push zot, ou digest non 64 hex.
pub fn push_platform_plugin_to_zot(host_port: u16) -> Result<String, Box<dyn std::error::Error>> {
    let nonce = nonce_hex();
    let local_tag = format!("apparatus-p4-platform-plugin:zot-{nonce}");
    let context = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/platform-plugin");
    let build = Command::new("docker")
        .args([
            "build",
            "--label",
            &format!("apparatus.p4.nonce={nonce}"),
            "-t",
            &local_tag,
        ])
        .arg(&context)
        .output()
        .map_err(|err| format!("Docker/Kind must be running (docker build: {err})"))?;
    if !build.status.success() {
        return Err(cmd_err(
            "docker build platform-plugin zot nonce",
            &build,
            "Docker/Kind must be running",
        ));
    }

    let tar = std::env::temp_dir().join(format!("apparatus-p4-zot-{nonce}.tar"));
    let save = Command::new("docker")
        .args(["save", "-o"])
        .arg(&tar)
        .arg(&local_tag)
        .output()
        .map_err(|err| format!("Docker/Kind must be running (docker save: {err})"))?;
    if !save.status.success() {
        return Err(cmd_err(
            "docker save platform-plugin zot",
            &save,
            "Docker/Kind must be running",
        ));
    }

    let dest_tag = format!("apparatus/platform-plugin:{nonce}");
    let tool = detect_copy_tool()?;
    copy_archive_to_zot(&tool, &tar, host_port, &dest_tag)?;
    let _ = std::fs::remove_file(&tar);

    let digest = inspect_zot_digest(&tool, host_port, &dest_tag)?;
    let hex = sha256_hex(&digest)
        .ok_or_else(|| format!("digest zot n'est pas 64 hex minuscules: {digest}"))?;
    if hex
        .bytes()
        .any(|b| !(b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    {
        return Err(format!("digest zot n'est pas 64 hex minuscules: {hex}").into());
    }
    Ok(format!(
        "host.docker.internal:{host_port}/apparatus/platform-plugin@sha256:{hex}"
    ))
}

const SKOPEO_IMAGE: &str = "quay.io/skopeo/stable:v1.17.0";
const CRANE_IMAGE: &str = "gcr.io/go-containerregistry/crane:v0.20.3";
const ZOT_PUSH_USER: &str = "signer";
const ZOT_PUSH_PASSWORD: &str = "apparatus-signer-push";

enum CopyTool {
    HostSkopeo,
    HostCrane,
    DockerSkopeo,
    DockerCrane,
}

fn detect_copy_tool() -> Result<CopyTool, Box<dyn std::error::Error>> {
    if host_cli_ok("skopeo", "--version") {
        return Ok(CopyTool::HostSkopeo);
    }
    if host_cli_ok("crane", "version") || host_cli_ok("crane", "--version") {
        return Ok(CopyTool::HostCrane);
    }
    if docker_image_usable(SKOPEO_IMAGE) {
        return Ok(CopyTool::DockerSkopeo);
    }
    if docker_image_usable(CRANE_IMAGE) {
        return Ok(CopyTool::DockerCrane);
    }
    Err(format!(
        "Docker/Kind must be running: ni skopeo/crane sur PATH, ni images {SKOPEO_IMAGE} / {CRANE_IMAGE}"
    )
    .into())
}

fn host_cli_ok(name: &str, arg: &str) -> bool {
    Command::new(name)
        .arg(arg)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn docker_image_usable(image: &str) -> bool {
    let inspect = Command::new("docker")
        .args(["image", "inspect", image])
        .output();
    if matches!(inspect, Ok(ref out) if out.status.success()) {
        return true;
    }
    let pull = Command::new("docker").args(["pull", image]).output();
    matches!(pull, Ok(out) if out.status.success())
}

fn copy_archive_to_zot(
    tool: &CopyTool,
    tar: &Path,
    host_port: u16,
    dest_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let creds = format!("{ZOT_PUSH_USER}:{ZOT_PUSH_PASSWORD}");
    match tool {
        CopyTool::HostSkopeo => {
            let archive = format!("docker-archive:{}", docker_host_path(tar)?);
            let dest = format!("docker://127.0.0.1:{host_port}/{dest_tag}");
            let out = Command::new("skopeo")
                .args([
                    "copy",
                    "--dest-tls-verify=false",
                    "--dest-creds",
                    &creds,
                    &archive,
                    &dest,
                ])
                .output()
                .map_err(|err| format!("skopeo copy: {err}"))?;
            if !out.status.success() {
                return Err(cmd_err(
                    "skopeo copy zot",
                    &out,
                    "Docker/Kind must be running",
                ));
            }
            Ok(())
        }
        CopyTool::HostCrane => {
            let registry = format!("127.0.0.1:{host_port}");
            let login = Command::new("crane")
                .args([
                    "auth",
                    "login",
                    "--insecure",
                    "-u",
                    ZOT_PUSH_USER,
                    "-p",
                    ZOT_PUSH_PASSWORD,
                    &registry,
                ])
                .output()
                .map_err(|err| format!("crane auth login: {err}"))?;
            if !login.status.success() {
                return Err(cmd_err(
                    "crane auth login zot",
                    &login,
                    "Docker/Kind must be running",
                ));
            }
            let dest = format!("{registry}/{dest_tag}");
            let out = Command::new("crane")
                .args(["push", "--insecure"])
                .arg(tar)
                .arg(&dest)
                .output()
                .map_err(|err| format!("crane push: {err}"))?;
            if !out.status.success() {
                return Err(cmd_err(
                    "crane push zot",
                    &out,
                    "Docker/Kind must be running",
                ));
            }
            Ok(())
        }
        CopyTool::DockerSkopeo => {
            let host_tar = docker_host_path(tar)?;
            let dest = format!("docker://host.docker.internal:{host_port}/{dest_tag}");
            let out = Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "-v",
                    &format!("{host_tar}:/img.tar:ro"),
                    SKOPEO_IMAGE,
                    "copy",
                    "--dest-tls-verify=false",
                    "--dest-creds",
                    &creds,
                    "docker-archive:/img.tar",
                    &dest,
                ])
                .output()
                .map_err(|err| format!("docker run skopeo: {err}"))?;
            if !out.status.success() {
                return Err(cmd_err(
                    "docker run skopeo copy zot",
                    &out,
                    "Docker/Kind must be running",
                ));
            }
            Ok(())
        }
        CopyTool::DockerCrane => {
            let host_tar = docker_host_path(tar)?;
            let cfg = std::env::temp_dir().join(format!("apparatus-p4-crane-cfg-{}", nonce_hex()));
            std::fs::create_dir_all(&cfg)
                .map_err(|err| format!("crane docker config dir: {err}"))?;
            let host_cfg = docker_host_path(&cfg)?;
            let registry = format!("host.docker.internal:{host_port}");
            let login = Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "-e",
                    "DOCKER_CONFIG=/cfg",
                    "-v",
                    &format!("{host_cfg}:/cfg"),
                    CRANE_IMAGE,
                    "auth",
                    "login",
                    "--insecure",
                    "-u",
                    ZOT_PUSH_USER,
                    "-p",
                    ZOT_PUSH_PASSWORD,
                    &registry,
                ])
                .output()
                .map_err(|err| format!("docker run crane auth login: {err}"))?;
            if !login.status.success() {
                return Err(cmd_err(
                    "docker run crane auth login zot",
                    &login,
                    "Docker/Kind must be running",
                ));
            }
            let dest = format!("{registry}/{dest_tag}");
            let out = Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "-e",
                    "DOCKER_CONFIG=/cfg",
                    "-v",
                    &format!("{host_cfg}:/cfg"),
                    "-v",
                    &format!("{host_tar}:/img.tar:ro"),
                    CRANE_IMAGE,
                    "push",
                    "--insecure",
                    "/img.tar",
                    &dest,
                ])
                .output()
                .map_err(|err| format!("docker run crane: {err}"))?;
            let _ = std::fs::remove_dir_all(&cfg);
            if !out.status.success() {
                return Err(cmd_err(
                    "docker run crane push zot",
                    &out,
                    "Docker/Kind must be running",
                ));
            }
            Ok(())
        }
    }
}

fn inspect_zot_digest(
    tool: &CopyTool,
    host_port: u16,
    dest_tag: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    match tool {
        CopyTool::HostSkopeo => {
            let src = format!("docker://127.0.0.1:{host_port}/{dest_tag}");
            let out = Command::new("skopeo")
                .args(["inspect", "--tls-verify=false", &src])
                .output()
                .map_err(|err| format!("skopeo inspect: {err}"))?;
            if !out.status.success() {
                return Err(cmd_err(
                    "skopeo inspect zot",
                    &out,
                    "Docker/Kind must be running",
                ));
            }
            digest_from_skopeo_inspect(&String::from_utf8_lossy(&out.stdout))
        }
        CopyTool::DockerSkopeo => {
            let src = format!("docker://host.docker.internal:{host_port}/{dest_tag}");
            let out = Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    SKOPEO_IMAGE,
                    "inspect",
                    "--tls-verify=false",
                    &src,
                ])
                .output()
                .map_err(|err| format!("docker run skopeo inspect: {err}"))?;
            if !out.status.success() {
                return Err(cmd_err(
                    "docker run skopeo inspect zot",
                    &out,
                    "Docker/Kind must be running",
                ));
            }
            digest_from_skopeo_inspect(&String::from_utf8_lossy(&out.stdout))
        }
        CopyTool::HostCrane => {
            let src = format!("127.0.0.1:{host_port}/{dest_tag}");
            let out = Command::new("crane")
                .args(["digest", "--insecure", &src])
                .output()
                .map_err(|err| format!("crane digest: {err}"))?;
            if !out.status.success() {
                return Err(cmd_err(
                    "crane digest zot",
                    &out,
                    "Docker/Kind must be running",
                ));
            }
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
        }
        CopyTool::DockerCrane => {
            let src = format!("host.docker.internal:{host_port}/{dest_tag}");
            let out = Command::new("docker")
                .args(["run", "--rm", CRANE_IMAGE, "digest", "--insecure", &src])
                .output()
                .map_err(|err| format!("docker run crane digest: {err}"))?;
            if !out.status.success() {
                return Err(cmd_err(
                    "docker run crane digest zot",
                    &out,
                    "Docker/Kind must be running",
                ));
            }
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
        }
    }
}

fn digest_from_skopeo_inspect(json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|err| format!("skopeo inspect JSON: {err}"))?;
    value
        .get("Digest")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "skopeo inspect: champ Digest absent".into())
}

fn docker_host_path(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|err| format!("canonicalize {}: {err}", path.display()))?;
    let mut rendered = canonical.to_string_lossy().into_owned();
    if let Some(stripped) = rendered.strip_prefix(r"\\?\") {
        rendered = stripped.to_owned();
    }
    Ok(rendered.replace('\\', "/"))
}

fn node_write_file(
    node: &str,
    path: &str,
    contents: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("docker")
        .args(["exec", "-i", node, "tee", path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Docker/Kind must be running (docker exec tee: {err})"))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("docker exec tee: stdin indisponible")?;
        stdin
            .write_all(contents.as_bytes())
            .map_err(|err| format!("docker exec tee write: {err}"))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|err| format!("docker exec tee wait: {err}"))?;
    if !out.status.success() {
        return Err(cmd_err(
            "docker exec tee hosts.toml",
            &out,
            "Docker/Kind must be running",
        ));
    }
    Ok(())
}

fn wait_crictl_ready(node: &str) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let out = node_exec(node, &["crictl", "info"])?;
        if out.status.success() {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(cmd_err(
                "crictl info after containerd restart",
                &out,
                "Docker/Kind must be running",
            ));
        }
        std::thread::sleep(Duration::from_millis(400));
    }
}

fn nonce_hex() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}

fn ensure_containerd_registry_config_path(node: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out = node_exec(node, &["cat", "/etc/containerd/config.toml"])?;
    if !out.status.success() {
        return Err(cmd_err(
            "cat containerd config.toml",
            &out,
            "Docker/Kind must be running",
        ));
    }
    let current = String::from_utf8_lossy(&out.stdout);
    if current.contains("config_path") && current.contains("/etc/containerd/certs.d") {
        return Ok(());
    }
    let mut updated = current.into_owned();
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(
        "\n[plugins.\"io.containerd.grpc.v1.cri\".registry]\n\
         \tconfig_path = \"/etc/containerd/certs.d\"\n\n\
         [plugins.\"io.containerd.cri.v1.images\".registry]\n\
         \tconfig_path = \"/etc/containerd/certs.d\"\n",
    );
    node_write_file(node, "/etc/containerd/config.toml", &updated)
}

fn wait_probe_done(cluster: &KindCluster) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        let phase = cluster.kubectl(&[
            "get",
            "pod",
            "t10-zot-probe",
            "-o",
            "jsonpath={.status.phase}",
        ])?;
        let text = String::from_utf8_lossy(&phase.stdout);
        if text.contains("Succeeded") || text.contains("Failed") {
            return Ok(());
        }
        if std::time::Instant::now() > deadline {
            return Err("Docker/Kind must be running: zot probe pod did not finish".into());
        }
        std::thread::sleep(Duration::from_millis(400));
    }
}

fn resolve_kind_bin() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(raw) = std::env::var_os(KIND_BIN_ENV) {
        let path = PathBuf::from(raw);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "Docker/Kind must be running: {KIND_BIN_ENV} is not a file ({})",
            path.display()
        )
        .into());
    }
    if bin_ok("kind") {
        return Ok(PathBuf::from("kind"));
    }
    let fallback = PathBuf::from(WINDOWS_KIND);
    if fallback.is_file() {
        return Ok(fallback);
    }
    Err(format!(
        "Docker/Kind must be running: kind not found in PATH, {KIND_BIN_ENV}, or {WINDOWS_KIND}"
    )
    .into())
}

fn resolve_kubectl() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if bin_ok("kubectl") {
        return Ok(PathBuf::from("kubectl"));
    }
    Err("Docker/Kind must be running: kubectl not found in PATH".into())
}

fn bin_ok(name: &str) -> bool {
    Command::new(name)
        .arg("version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn cluster_exists(kind_bin: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let out = Command::new(kind_bin)
        .args(["get", "clusters"])
        .output()
        .map_err(|err| format!("Docker/Kind must be running (kind get clusters: {err})"))?;
    if !out.status.success() {
        return Err(cmd_err(
            "kind get clusters",
            &out,
            "Docker/Kind must be running",
        ));
    }
    let names = String::from_utf8_lossy(&out.stdout);
    Ok(names.lines().any(|line| line.trim() == CLUSTER_NAME))
}

fn create_cluster(kind_bin: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kind/cluster.yaml");
    let out = Command::new(kind_bin)
        .args(["create", "cluster", "--name", CLUSTER_NAME, "--config"])
        .arg(&config)
        .output()
        .map_err(|err| format!("Docker/Kind must be running (kind create: {err})"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(cmd_err(
        "kind create cluster",
        &out,
        "Docker/Kind must be running",
    ))
}

fn export_kubeconfig(kind_bin: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join("apparatus-p4-it.kubeconfig");
    let out = Command::new(kind_bin)
        .args([
            "export",
            "kubeconfig",
            "--name",
            CLUSTER_NAME,
            "--kubeconfig",
        ])
        .arg(&path)
        .output()
        .map_err(|err| format!("Docker/Kind must be running (kind export kubeconfig: {err})"))?;
    if !out.status.success() {
        return Err(cmd_err(
            "kind export kubeconfig",
            &out,
            "Docker/Kind must be running",
        ));
    }
    Ok(path)
}

fn cri_pin_from_node(cluster: &str) -> Result<String, Box<dyn std::error::Error>> {
    let node = format!("{cluster}-control-plane");
    let library_tag = format!("docker.io/library/{IMAGE_TAG}");
    let inspect_json = wait_inspecti(&node, &[&library_tag, IMAGE_TAG])?;
    let cri_digest = digest_from_inspecti_json(&inspect_json)?;
    let (source, ctr_digest) = ctr_tagged_source(&node)?;

    let mut last_err: Option<Box<dyn std::error::Error>> = None;
    for digest in [Some(cri_digest), ctr_digest].into_iter().flatten() {
        match tag_cri_pin(&node, &source, &digest) {
            Ok(pin) => return Ok(pin),
            Err(err) => last_err = Some(err),
        }
    }
    Err(last_err
        .unwrap_or_else(|| format!("Kind node {node} has no CRI digest for {IMAGE_TAG}").into()))
}

fn cri_pin_from_loaded_tag(
    cluster: &str,
    tag: &str,
    pin_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let node = format!("{cluster}-control-plane");
    let library_tag = format!("docker.io/library/{tag}");
    let deadline = Instant::now() + Duration::from_secs(20);
    let json = loop {
        match inspecti(&node, tag).or_else(|_| inspecti(&node, &library_tag)) {
            Ok(body) => break body,
            Err(err) => {
                if Instant::now() > deadline {
                    return Err(format!("Kind node {node} is missing image {tag}: {err}").into());
                }
                std::thread::sleep(Duration::from_millis(400));
            }
        }
    };
    let digest = digest_from_inspecti_json(&json)?;
    let source = ctr_source_containing(&node, tag)?;
    let short_pin = format!("{pin_name}@sha256:{digest}");
    let library_pin = format!("docker.io/library/{short_pin}");
    ctr_tag(&node, &source, &short_pin)?;
    let _ = ctr_tag(&node, &source, &library_pin);
    inspecti(&node, &short_pin)
        .or_else(|_| inspecti(&node, &library_pin))
        .map_err(|err| format!("Kind node {node} did not expose CRI pin {short_pin}: {err}"))?;
    Ok(short_pin)
}

fn ctr_source_containing(node: &str, tag: &str) -> Result<String, Box<dyn std::error::Error>> {
    let out = node_exec(node, &["ctr", "-n", "k8s.io", "images", "ls"])?;
    if !out.status.success() {
        return Err(cmd_err(
            "ctr images ls",
            &out,
            "Docker/Kind must be running",
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut fallback = None;
    for line in stdout.lines() {
        if !line.contains(tag) && !line.contains(tag.split(':').next().unwrap_or(tag)) {
            continue;
        }
        let Some(image_ref) = line.split_whitespace().next() else {
            continue;
        };
        if image_ref.contains("@sha256:") {
            continue;
        }
        if image_ref.contains(tag) || image_ref.ends_with(tag) {
            return Ok(image_ref.to_owned());
        }
        fallback = Some(image_ref.to_owned());
    }
    fallback.ok_or_else(|| format!("Kind node {node} has no tagged image {tag}").into())
}

fn wait_inspecti(node: &str, refs: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut last_err: Option<Box<dyn std::error::Error>> = None;
    loop {
        for image in refs {
            match inspecti(node, image) {
                Ok(json) => return Ok(json),
                Err(err) => last_err = Some(err),
            }
        }
        if Instant::now() > deadline {
            return Err(last_err.unwrap_or_else(|| {
                format!("Kind node {node} is missing image {IMAGE_TAG}").into()
            }));
        }
        std::thread::sleep(Duration::from_millis(400));
    }
}

fn inspecti(node: &str, image: &str) -> Result<String, Box<dyn std::error::Error>> {
    let out = node_exec(node, &["crictl", "inspecti", image])?;
    if !out.status.success() {
        return Err(cmd_err("crictl inspecti", &out, "Kind node missing image"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn digest_from_inspecti_json(json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|err| format!("crictl inspecti JSON: {err}"))?;
    if let Some(hex) = value
        .pointer("/status/id")
        .and_then(serde_json::Value::as_str)
        .and_then(sha256_hex)
    {
        return Ok(hex);
    }
    if let Some(hex) = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .and_then(sha256_hex)
    {
        return Ok(hex);
    }
    if let Some(digests) = value
        .pointer("/status/repoDigests")
        .and_then(serde_json::Value::as_array)
    {
        for entry in digests {
            if let Some(raw) = entry.as_str() {
                if let Some((_, digest)) = raw.split_once("@sha256:") {
                    if let Some(hex) = sha256_hex(digest) {
                        return Ok(hex);
                    }
                }
            }
        }
    }
    Err("Kind node image has no sha256 digest (crictl inspecti)".into())
}

fn sha256_hex(raw: &str) -> Option<String> {
    let hex = raw.strip_prefix("sha256:").unwrap_or(raw);
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(hex.to_ascii_lowercase())
    } else {
        None
    }
}

fn ctr_tagged_source(node: &str) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    let out = node_exec(node, &["ctr", "-n", "k8s.io", "images", "ls"])?;
    if !out.status.success() {
        return Err(cmd_err(
            "ctr images ls",
            &out,
            "Docker/Kind must be running",
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if !line.contains("apparatus-p4-platform-plugin") {
            continue;
        }
        let mut cols = line.split_whitespace();
        let Some(image_ref) = cols.next() else {
            continue;
        };
        if image_ref.contains("@sha256:") {
            continue;
        }
        if !image_ref.contains(":t10") {
            continue;
        }
        let digest = cols
            .find(|col| col.starts_with("sha256:") && !col.contains("<none>"))
            .and_then(sha256_hex);
        return Ok((image_ref.to_owned(), digest));
    }
    Err(format!("Kind node {node} has no tagged image {IMAGE_TAG}").into())
}

fn tag_cri_pin(
    node: &str,
    source: &str,
    digest: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let short_pin = format!("apparatus-p4-platform-plugin@sha256:{digest}");
    let library_pin = format!("docker.io/library/{short_pin}");
    ctr_tag(node, source, &short_pin)?;
    let _ = ctr_tag(node, source, &library_pin);
    inspecti(node, &short_pin)
        .or_else(|_| inspecti(node, &library_pin))
        .map_err(|err| format!("Kind node {node} did not expose CRI pin {short_pin}: {err}"))?;
    Ok(short_pin)
}

fn ctr_tag(node: &str, source: &str, target: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out = node_exec(
        node,
        &["ctr", "-n", "k8s.io", "images", "tag", source, target],
    )?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("already exists") {
        return Ok(());
    }
    Err(cmd_err(
        "ctr images tag",
        &out,
        "Docker/Kind must be running",
    ))
}

fn node_exec(node: &str, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    Command::new("docker")
        .arg("exec")
        .arg(node)
        .args(args)
        .output()
        .map_err(|err| format!("Docker/Kind must be running (docker exec {node}: {err})").into())
}

fn cmd_err(op: &str, output: &Output, prefix: &str) -> Box<dyn std::error::Error> {
    Box::from(format!(
        "{prefix}: {op} failed status={:?} stdout={} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}
