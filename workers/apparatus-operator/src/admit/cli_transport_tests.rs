//! Gate du transport ORAS/Cosign : argv, stdin, config privée, deadline.
//! Aucun binaire Docker, registry ou image n'est lancé : le fallback `docker`
//! est un mock en processus, les enfants réels sont des outils du système.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::cli_transport::{
    execute, oras_password_frame, remove_private_dir, run_cli, with_registry_config, CliSpec,
    ConfigGuard, RunRequest,
};
use super::{
    cli_failed, oras_push_and_oci_digest, redact, sign_and_verify, signature_tag_exists,
    AdmitError, AdmitTarget, RegistryAuth, COSIGN_BIN_ENV, COSIGN_IMAGE, ORAS_BIN_ENV, ORAS_IMAGE,
};

const PARENT_ENV: &[&str] = &[
    "DOCKER_CONFIG",
    "VAULT_TOKEN",
    "REGISTRY_PASSWORD",
    "ORAS_PASSWORD",
    "COSIGN_REGISTRY_PASSWORD",
    "APPARATUS_ORAS_BIN",
    "APPARATUS_COSIGN_BIN",
    "APPARATUS_REGISTRY_PASSWORD",
];

thread_local! {
    static HOST: RefCell<HashMap<String, Option<PathBuf>>> = RefCell::new(HashMap::new());
    static DEADLINE: Cell<Option<Duration>> = Cell::new(None);
    static LIMIT: Cell<Option<usize>> = Cell::new(None);
    static MOCK: RefCell<MockState> = RefCell::new(MockState::inactive());
}

struct MockState {
    active: bool,
    fail_stop: bool,
    fail_tool: bool,
    bad_cid: bool,
    deadline_on_run: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    specs: Vec<CliSpec>,
    config_dirs: Vec<PathBuf>,
    configs: Vec<String>,
}

impl MockState {
    fn inactive() -> Self {
        Self {
            active: false,
            fail_stop: false,
            fail_tool: false,
            bad_cid: false,
            deadline_on_run: false,
            stdout: Vec::new(),
            stderr: Vec::new(),
            specs: Vec::new(),
            config_dirs: Vec::new(),
            configs: Vec::new(),
        }
    }
}

pub(crate) fn host_override(bin_env: &str) -> Option<Option<PathBuf>> {
    HOST.with(|host| host.borrow().get(bin_env).cloned())
}

pub(crate) fn deadline_override() -> Option<Duration> {
    DEADLINE.with(Cell::get)
}

pub(crate) fn limit_override() -> Option<usize> {
    LIMIT.with(Cell::get)
}

pub(crate) fn intercept(spec: &CliSpec) -> Option<Result<Output, AdmitError>> {
    MOCK.with(|slot| {
        let mut mock = slot.borrow_mut();
        if !mock.active {
            return None;
        }
        mock.specs.push(spec.clone());
        capture_config(&mut mock, spec);
        Some(scripted(spec, &mock))
    })
}

fn capture_config(mock: &mut MockState, spec: &CliSpec) {
    for (key, value) in &spec.set_env {
        if key == "DOCKER_CONFIG" {
            read_config(mock, Path::new(value));
        }
    }
    for arg in &spec.args {
        if let Some(host) = arg.strip_suffix(":/auth:ro") {
            read_config(mock, Path::new(host));
        }
    }
}

fn read_config(mock: &mut MockState, dir: &Path) {
    let path = dir.join("config.json");
    if let Ok(text) = fs::read_to_string(&path) {
        mock.config_dirs.push(dir.to_path_buf());
        mock.configs.push(text);
    }
}

fn scripted(spec: &CliSpec, state: &MockState) -> Result<Output, AdmitError> {
    let head = spec.args.first().map(String::as_str);
    if head == Some("run") {
        write_cid(spec, state);
        if state.deadline_on_run {
            return Err(AdmitError::new("cli deadline exceeded"));
        }
    }
    if head == Some("stop") && state.fail_stop {
        return Ok(fake_output(1, b"", b""));
    }
    if head == Some("inspect") {
        return Ok(fake_output(0, b"exited\n", b""));
    }
    if state.fail_tool && !matches!(head, Some("stop" | "inspect" | "rm" | "run")) {
        return Ok(fake_output(9, &state.stdout, &state.stderr));
    }
    Ok(fake_output(0, &state.stdout, &state.stderr))
}

fn write_cid(spec: &CliSpec, state: &MockState) {
    let Some(pos) = spec.args.iter().position(|arg| arg == "--cidfile") else {
        return;
    };
    let Some(path) = spec.args.get(pos + 1) else {
        return;
    };
    let body = if state.bad_cid {
        "named-container\n".to_owned()
    } else {
        format!("{}\n", container_id())
    };
    let _ = fs::write(path, body);
}

struct Guard;

impl Guard {
    fn install() -> Self {
        HOST.with(|host| {
            let mut map = host.borrow_mut();
            map.clear();
            map.insert(ORAS_BIN_ENV.to_owned(), None);
            map.insert(COSIGN_BIN_ENV.to_owned(), None);
        });
        DEADLINE.with(|cell| cell.set(None));
        LIMIT.with(|cell| cell.set(None));
        MOCK.with(|slot| *slot.borrow_mut() = MockState::inactive());
        Self
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        HOST.with(|host| host.borrow_mut().clear());
        DEADLINE.with(|cell| cell.set(None));
        LIMIT.with(|cell| cell.set(None));
        MOCK.with(|slot| *slot.borrow_mut() = MockState::inactive());
    }
}

fn force_path(bin_env: &str, path: &str) {
    HOST.with(|host| {
        host.borrow_mut()
            .insert(bin_env.to_owned(), Some(PathBuf::from(path)));
    });
}

fn install_mock(stdout: &[u8]) {
    MOCK.with(|slot| {
        let mut mock = slot.borrow_mut();
        *mock = MockState::inactive();
        mock.active = true;
        mock.stdout = stdout.to_vec();
    });
}

fn update_mock(edit: impl FnOnce(&mut MockState)) {
    MOCK.with(|slot| edit(&mut slot.borrow_mut()));
}

fn take_specs() -> Vec<CliSpec> {
    MOCK.with(|slot| std::mem::take(&mut slot.borrow_mut().specs))
}

fn take_configs() -> Vec<(PathBuf, String)> {
    MOCK.with(|slot| {
        let mut mock = slot.borrow_mut();
        let dirs = std::mem::take(&mut mock.config_dirs);
        let texts = std::mem::take(&mut mock.configs);
        dirs.into_iter().zip(texts).collect()
    })
}

fn sha_hex() -> String {
    "ab".repeat(32)
}

fn digest_line() -> Vec<u8> {
    format!("sha256:{}\n", sha_hex()).into_bytes()
}

fn container_id() -> String {
    "cd".repeat(32)
}

fn target(password: &str) -> AdmitTarget {
    AdmitTarget {
        registry_host: "127.0.0.1:5000".to_owned(),
        registry_network: "zot:5000".to_owned(),
        docker_network: "apparatus-net".to_owned(),
        vault_addr_host: "http://127.0.0.1:8200".to_owned(),
        vault_addr_network: "http://openbao:8200".to_owned(),
        vault_token: "vault-token-value".to_owned(),
        transit_key: "apparatus-key".to_owned(),
        repository: "apparatus/envelope".to_owned(),
        tag: "t10".to_owned(),
        auth: RegistryAuth {
            username: "signer".to_owned(),
            password: password.to_owned(),
        },
    }
}

fn env_snapshot() -> Vec<Option<std::ffi::OsString>> {
    PARENT_ENV.iter().map(|key| std::env::var_os(key)).collect()
}

fn assert_parent_unchanged(before: &[Option<std::ffi::OsString>]) {
    assert_eq!(before, env_snapshot().as_slice());
}

fn assert_no_secret(args: &[String], secrets: &[&str]) {
    for arg in args {
        assert_ne!(arg, "--password");
        assert_ne!(arg, "--registry-password");
        for secret in secrets {
            if secret.is_empty() {
                continue;
            }
            assert!(!arg.contains(secret), "argv exposed a secret");
        }
    }
}

fn assert_docker_run(spec: &CliSpec, secrets: &[&str]) {
    assert_eq!(spec.program, PathBuf::from("docker"));
    assert_eq!(spec.args.first().map(String::as_str), Some("run"));
    assert!(spec.args.iter().any(|arg| arg == "-i"));
    assert!(spec.args.iter().any(|arg| arg == "--network"));
    assert!(spec.args.iter().any(|arg| arg == "apparatus-net"));
    assert!(spec.args.iter().any(|arg| arg == "--cidfile"));
    assert!(!spec.args.iter().any(|arg| {
        arg == "--rm" || arg == "--name" || arg == "-t" || arg == "prune" || arg == "login"
    }));
    assert_no_secret(&spec.args, secrets);
    assert!(!spec
        .args
        .iter()
        .any(|arg| arg.contains("vault-token-value") || arg.contains("http://openbao:8200")));
}

fn runs_cleaned_once(specs: &[CliSpec]) {
    let runs = specs
        .iter()
        .filter(|spec| spec.args.first().map(String::as_str) == Some("run"))
        .count();
    let removes: Vec<&CliSpec> = specs
        .iter()
        .filter(|spec| spec.args.first().map(String::as_str) == Some("rm"))
        .collect();
    assert_eq!(removes.len(), runs);
    let id = container_id();
    for spec in removes {
        assert_eq!(spec.args, vec!["rm".to_owned(), id.clone()]);
    }
    for spec in specs
        .iter()
        .filter(|spec| spec.args.first().map(String::as_str) == Some("stop"))
    {
        assert_eq!(spec.args, vec!["stop".to_owned(), id.clone()]);
    }
}

fn assert_auth_json(text: &str, password: &str) {
    assert!(!text.contains("credsStore"));
    assert!(!text.contains("credHelpers"));
    assert!(!text.contains("COSIGN_REGISTRY_PASSWORD"));
    let value: serde_json::Value = serde_json::from_str(text).expect("config json");
    let auths = value
        .get("auths")
        .and_then(serde_json::Value::as_object)
        .expect("auths");
    assert_eq!(auths.len(), 2);
    for host in ["127.0.0.1:5000", "zot:5000"] {
        let auth = auths.get(host).expect(host);
        assert_eq!(
            auth.get("username").and_then(|v| v.as_str()),
            Some("signer")
        );
        assert_eq!(
            auth.get("password").and_then(|v| v.as_str()),
            Some(password)
        );
    }
}

fn fake_output(code: i32, stdout: &[u8], stderr: &[u8]) -> Output {
    Output {
        status: exit_status(code),
        stdout: stdout.to_vec(),
        stderr: stderr.to_vec(),
    }
}

fn exit_status(code: i32) -> ExitStatus {
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code)
    }
}

struct TempWork(PathBuf);

impl TempWork {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let path = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
        fs::create_dir_all(&path).expect("temp workdir");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempWork {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct DirCleanup(PathBuf);

impl Drop for DirCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn real_spec(program: PathBuf, args: Vec<String>) -> CliSpec {
    CliSpec {
        program,
        args,
        set_env: Vec::new(),
        remove_env: Vec::new(),
        stdin: None,
        workdir: None,
        deadline: Duration::from_secs(15),
        output_limit: 1024 * 1024,
    }
}

fn system_exe(name: &str) -> PathBuf {
    let root =
        std::env::var_os("SystemRoot").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    root.join("System32").join(name)
}

fn assert_bounded(started: Instant, deadline: Duration) {
    let elapsed = started.elapsed();
    assert!(
        elapsed < deadline + Duration::from_secs(3),
        "returned in {elapsed:?}"
    );
    assert!(
        elapsed >= Duration::from_millis(1500),
        "returned too fast ({elapsed:?})"
    );
}

#[cfg(windows)]
struct PingSweep {
    before: Vec<u32>,
}

#[cfg(windows)]
impl PingSweep {
    fn arm() -> Self {
        Self {
            before: ping_pids(),
        }
    }
}

#[cfg(windows)]
impl Drop for PingSweep {
    fn drop(&mut self) {
        for pid in ping_pids() {
            if self.before.contains(&pid) {
                continue;
            }
            let _ = Command::new(system_exe("taskkill.exe"))
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
        }
    }
}

#[cfg(windows)]
fn ping_pids() -> Vec<u32> {
    let Ok(output) = Command::new(system_exe("tasklist.exe"))
        .args(["/FI", "IMAGENAME eq PING.EXE", "/FO", "CSV", "/NH"])
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split(',');
            let _name = parts.next()?;
            let pid = parts.next()?.trim().trim_matches('"');
            pid.parse().ok()
        })
        .collect()
}

#[test]
fn oras_password_frames_keep_cr_lf_and_empty() {
    let cases = [
        ("s3cret", "s3cret\r\n"),
        ("ends-cr\r", "ends-cr\r\r\n"),
        ("ends-lf\n", "ends-lf\n\r\n"),
        ("ends-crlf\r\n", "ends-crlf\r\n\r\n"),
        ("", "\r\n"),
    ];
    for (password, expected) in cases {
        assert_eq!(oras_password_frame(password), expected.as_bytes());
    }
    let long = "x".repeat(4096);
    let framed = oras_password_frame(&long);
    assert_eq!(&framed[..long.len()], long.as_bytes());
    assert_eq!(&framed[long.len()..], b"\r\n");
}

#[test]
fn oras_sites_omit_password_and_frame_stdin() {
    let _guard = Guard::install();
    force_path(ORAS_BIN_ENV, "oras-host");
    DEADLINE.with(|cell| cell.set(Some(Duration::from_secs(7))));
    LIMIT.with(|cell| cell.set(Some(32)));
    install_mock(&digest_line());
    let work = TempWork::new("apparatus-m4-oras");
    let passwords = ["s3cret", "ends-cr\r", "ends-lf\n", "ends-crlf\r\n", ""];
    for password in passwords {
        update_mock(|mock| mock.specs.clear());
        let target = target(password);
        let tagged = format!(
            "{}/{}:{}",
            target.registry_host, target.repository, target.tag
        );
        let digest = oras_push_and_oci_digest(&target, work.path(), &tagged).expect("push");
        assert_eq!(digest, format!("sha256:{}", sha_hex()));
        let digest_ref = format!(
            "{}/{}@sha256:{}",
            target.registry_host,
            target.repository,
            sha_hex()
        );
        assert!(signature_tag_exists(&target, &digest_ref).expect("tag"));
        let specs = take_specs();
        assert_eq!(specs.len(), 3);
        let frame = oras_password_frame(password);
        for spec in &specs {
            assert_eq!(spec.program, PathBuf::from("oras-host"));
            assert!(spec.args.iter().any(|arg| arg == "--password-stdin"));
            assert!(spec.args.iter().any(|arg| arg == "--username"));
            assert!(spec.args.iter().any(|arg| arg == "signer"));
            assert_no_secret(&spec.args, &[password, target.vault_token.as_str()]);
            assert_eq!(spec.stdin.as_deref(), Some(frame.as_slice()));
            assert!(spec.remove_env.iter().any(|key| key == "VAULT_TOKEN"));
            assert!(!spec.set_env.iter().any(|(key, _)| key == "VAULT_TOKEN"));
            assert_eq!(spec.deadline, Duration::from_secs(7));
            assert_eq!(spec.output_limit, 32);
        }
        assert!(specs[0].args.iter().any(|arg| arg == &tagged));
        assert!(specs[2].args.iter().any(|arg| arg.contains(".sig")));
    }
}

#[test]
fn password_flags_never_spawn() {
    let _guard = Guard::install();
    force_path(ORAS_BIN_ENV, "oras-host");
    force_path(COSIGN_BIN_ENV, "cosign-host");
    install_mock(&digest_line());
    let target = target("s3cret");
    let oras_args = vec![
        "push".to_owned(),
        "--password".to_owned(),
        "s3cret".to_owned(),
    ];
    let err = run_cli(RunRequest {
        bin_env: ORAS_BIN_ENV,
        image: ORAS_IMAGE,
        network: &target.docker_network,
        extra_env: &[],
        args: &oras_args,
        workdir: None,
        stdin: Some(b"s3cret\r\n"),
        secrets: &["s3cret"],
    })
    .expect_err("password flag");
    assert_eq!(err.to_string(), "cannot start helper");
    assert!(take_specs().is_empty());

    let cosign_args = vec!["sign".to_owned(), "--registry-password".to_owned()];
    let err = run_cli(RunRequest {
        bin_env: COSIGN_BIN_ENV,
        image: COSIGN_IMAGE,
        network: &target.docker_network,
        extra_env: &[],
        args: &cosign_args,
        workdir: None,
        stdin: None,
        secrets: &["s3cret", "vault-token-value"],
    })
    .expect_err("registry password flag");
    assert_eq!(err.to_string(), "cannot start helper");
    assert!(!err.to_string().contains("s3cret"));
    assert!(take_specs().is_empty());
}

#[test]
fn cosign_host_writes_private_config_without_touching_parent_env() {
    let _guard = Guard::install();
    force_path(COSIGN_BIN_ENV, "cosign-host");
    install_mock(b"");
    let password = "ends-crlf\r\n";
    let target = target(password);
    let before = env_snapshot();
    let digest_ref = format!(
        "{}/{}@sha256:{}",
        target.registry_host,
        target.repository,
        sha_hex()
    );
    assert!(sign_and_verify(&target, &digest_ref).expect("sign"));
    assert_parent_unchanged(&before);
    let specs = take_specs();
    assert_eq!(specs.len(), 2);
    for spec in &specs {
        assert_eq!(spec.program, PathBuf::from("cosign-host"));
        assert!(spec.stdin.is_none());
        assert_no_secret(&spec.args, &[password, "vault-token-value"]);
        assert!(spec.args.iter().any(|arg| arg == "--registry-username"));
        assert!(spec.args.iter().any(|arg| arg == "signer"));
        assert_eq!(
            spec.set_env
                .iter()
                .find(|(key, _)| key == "VAULT_ADDR")
                .map(|(_, value)| value.as_str()),
            Some("http://127.0.0.1:8200")
        );
        assert_eq!(
            spec.set_env
                .iter()
                .find(|(key, _)| key == "VAULT_TOKEN")
                .map(|(_, value)| value.as_str()),
            Some("vault-token-value")
        );
        assert_eq!(
            spec.set_env
                .iter()
                .find(|(key, _)| key == "TRANSIT_SECRET_ENGINE_PATH")
                .map(|(_, value)| value.as_str()),
            Some("transit")
        );
        assert_eq!(
            spec.set_env
                .iter()
                .find(|(key, _)| key == "COSIGN_YES")
                .map(|(_, value)| value.as_str()),
            Some("1")
        );
        assert!(spec.set_env.iter().any(|(key, _)| key == "DOCKER_CONFIG"));
        assert!(!spec
            .set_env
            .iter()
            .any(|(key, _)| key == "COSIGN_REGISTRY_PASSWORD"));
        for key in [
            "VAULT_TOKEN",
            "DOCKER_CONFIG",
            "REGISTRY_PASSWORD",
            "ORAS_PASSWORD",
            "COSIGN_REGISTRY_PASSWORD",
        ] {
            assert!(spec.remove_env.iter().any(|name| name == key));
        }
    }
    assert!(specs[0].args.iter().any(|arg| arg == "sign"));
    assert!(specs[1].args.iter().any(|arg| arg == "verify"));
    let configs = take_configs();
    assert_eq!(configs.len(), 2);
    assert_ne!(configs[0].0, configs[1].0);
    for (dir, text) in &configs {
        assert_auth_json(text, password);
        assert!(!dir.exists());
    }
}

#[test]
fn cosign_failure_omits_output_and_removes_config() {
    let _guard = Guard::install();
    force_path(COSIGN_BIN_ENV, "cosign-host");
    let mut stdout = b"ADVERSARIAL-MARKER s3cret vault-token-value".to_vec();
    stdout.push(0xff);
    install_mock(&stdout);
    update_mock(|mock| {
        mock.fail_tool = true;
        mock.stderr = b"ADVERSARIAL-MARKER".to_vec();
    });
    let target = target("s3cret");
    let err = sign_and_verify(&target, "127.0.0.1:5000/apparatus/envelope@sha256:abcd")
        .expect_err("sign failed");
    let text = err.to_string();
    assert!(text.contains("cosign sign failed"));
    assert!(text.contains("Some(9)"));
    assert!(!text.contains("ADVERSARIAL-MARKER"));
    assert!(!text.contains("s3cret"));
    assert!(!text.contains("vault-token-value"));
    assert!(!text.contains('\u{FFFD}'));
    let configs = take_configs();
    assert_eq!(configs.len(), 1);
    assert!(!configs[0].0.exists());
}

#[test]
fn cosign_docker_run_mounts_config_and_keeps_secrets_out_of_argv() {
    let _guard = Guard::install();
    install_mock(b"");
    let before = env_snapshot();
    let target = target("s3cret");
    let digest_ref = format!(
        "{}/{}@{}",
        target.registry_network,
        target.repository,
        format!("sha256:{}", sha_hex())
    );
    assert!(sign_and_verify(&target, &digest_ref).expect("docker sign"));
    assert_parent_unchanged(&before);
    let specs = take_specs();
    let runs: Vec<&CliSpec> = specs
        .iter()
        .filter(|spec| spec.args.first().map(String::as_str) == Some("run"))
        .collect();
    assert_eq!(runs.len(), 2);
    for spec in &runs {
        assert_docker_run(spec, &["s3cret", "vault-token-value"]);
        assert!(spec.args.iter().any(|arg| arg == COSIGN_IMAGE));
        assert!(spec.args.iter().any(|arg| arg == "-e"));
        assert!(spec.args.iter().any(|arg| arg == "VAULT_TOKEN"));
        assert!(spec.args.iter().any(|arg| arg == "DOCKER_CONFIG=/auth"));
        assert!(spec.args.iter().any(|arg| arg.ends_with(":/auth:ro")));
        assert!(spec.stdin.is_none());
        assert!(!spec.set_env.iter().any(|(key, _)| key == "DOCKER_CONFIG"));
        assert_eq!(
            spec.set_env
                .iter()
                .find(|(key, _)| key == "VAULT_TOKEN")
                .map(|(_, value)| value.as_str()),
            Some("vault-token-value")
        );
        assert_eq!(
            spec.set_env
                .iter()
                .find(|(key, _)| key == "VAULT_ADDR")
                .map(|(_, value)| value.as_str()),
            Some("http://openbao:8200")
        );
    }
    runs_cleaned_once(&specs);
    let configs = take_configs();
    assert_eq!(configs.len(), 2);
    for (dir, text) in &configs {
        assert_auth_json(text, "s3cret");
        assert!(!dir.exists());
    }
}

#[test]
fn oras_docker_run_uses_stdin_and_exact_container_id() {
    let _guard = Guard::install();
    install_mock(&digest_line());
    let work = TempWork::new("apparatus-m4-docker-oras");
    let target = target("s3cret");
    let tagged = format!(
        "{}/{}:{}",
        target.registry_host, target.repository, target.tag
    );
    let digest = oras_push_and_oci_digest(&target, work.path(), &tagged).expect("docker push");
    assert_eq!(digest, format!("sha256:{}", sha_hex()));
    let specs = take_specs();
    let runs: Vec<&CliSpec> = specs
        .iter()
        .filter(|spec| spec.args.first().map(String::as_str) == Some("run"))
        .collect();
    assert_eq!(runs.len(), 2);
    for spec in &runs {
        assert_docker_run(spec, &["s3cret", "vault-token-value"]);
        assert!(spec.args.iter().any(|arg| arg == ORAS_IMAGE));
        assert!(spec.args.iter().any(|arg| arg == "--password-stdin"));
        assert_eq!(
            spec.stdin.as_deref(),
            Some(oras_password_frame("s3cret").as_slice())
        );
        assert!(!spec.set_env.iter().any(|(key, _)| key == "VAULT_TOKEN"));
        assert!(spec.remove_env.iter().any(|key| key == "VAULT_TOKEN"));
    }
    assert!(runs[0].args.iter().any(|arg| arg == "-w"));
    assert!(!runs[1].args.iter().any(|arg| arg == "-w"));
    runs_cleaned_once(&specs);
}

#[test]
fn docker_stop_failure_skips_rm() {
    let _guard = Guard::install();
    install_mock(&digest_line());
    update_mock(|mock| mock.fail_stop = true);
    let work = TempWork::new("apparatus-m4-stop");
    let target = target("s3cret");
    let err = oras_push_and_oci_digest(
        &target,
        work.path(),
        "127.0.0.1:5000/apparatus/envelope:t10",
    )
    .expect_err("stop");
    let text = err.to_string();
    assert!(text.contains("container cleanup failed"));
    assert!(!text.contains("s3cret"));
    let specs = take_specs();
    assert!(specs
        .iter()
        .any(|spec| spec.args.first().map(String::as_str) == Some("stop")));
    assert!(!specs
        .iter()
        .any(|spec| spec.args.first().map(String::as_str) == Some("rm")));
    assert!(!specs
        .iter()
        .any(|spec| spec.args.first().map(String::as_str) == Some("inspect")));
}

#[test]
fn docker_bad_id_skips_rm() {
    let _guard = Guard::install();
    install_mock(&digest_line());
    update_mock(|mock| mock.bad_cid = true);
    let work = TempWork::new("apparatus-m4-bad-id");
    let target = target("s3cret");
    let err = oras_push_and_oci_digest(
        &target,
        work.path(),
        "127.0.0.1:5000/apparatus/envelope:t10",
    )
    .expect_err("bad id");
    assert!(err.to_string().contains("container cleanup failed"));
    assert!(!err.to_string().contains("named-container"));
    assert!(!err.to_string().contains("s3cret"));
    let specs = take_specs();
    assert!(specs
        .iter()
        .any(|spec| spec.args.first().map(String::as_str) == Some("run")));
    assert!(!specs.iter().any(|spec| {
        let head = spec.args.first().map(String::as_str);
        head == Some("stop") || head == Some("rm") || head == Some("inspect")
    }));
}

#[test]
fn docker_deadline_still_removes_exact_id() {
    let _guard = Guard::install();
    install_mock(b"");
    update_mock(|mock| mock.deadline_on_run = true);
    let work = TempWork::new("apparatus-m4-deadline-docker");
    let target = target("s3cret");
    let err = oras_push_and_oci_digest(
        &target,
        work.path(),
        "127.0.0.1:5000/apparatus/envelope:t10",
    )
    .expect_err("deadline");
    assert!(err.to_string().contains("cli deadline exceeded"));
    assert!(!err.to_string().contains("s3cret"));
    let specs = take_specs();
    runs_cleaned_once(&specs);
}

#[test]
fn container_drop_cleans_once_and_invalid_id_does_not_spawn() {
    let _guard = Guard::install();
    install_mock(b"");
    let id = container_id();
    super::cli_transport::drop_container_release(&id);
    let specs = take_specs();
    assert_eq!(specs.len(), 3);
    assert_eq!(specs[0].args, vec!["stop".to_owned(), id.clone()]);
    assert_eq!(
        specs[1].args,
        vec![
            "inspect".to_owned(),
            "--format".to_owned(),
            "{{.State.Status}}".to_owned(),
            id.clone(),
        ]
    );
    assert_eq!(specs[2].args, vec!["rm".to_owned(), id]);
    super::cli_transport::drop_container_release("named-container");
    assert!(take_specs().is_empty());
}

#[test]
fn cli_failed_and_redact_hide_secrets() {
    let status = exit_status(9);
    let err = cli_failed("oras push", &status);
    assert_eq!(err.to_string(), "oras push failed status=Some(9)");
    assert_eq!(redact("token-and-more", "token"), "***-and-more");
    assert_eq!(redact("same", ""), "same");
}

#[test]
fn non_utf8_oras_stdout_still_parses_digest() {
    let _guard = Guard::install();
    force_path(ORAS_BIN_ENV, "oras-host");
    let mut stdout = vec![0xff, 0xfe];
    stdout.extend(digest_line());
    install_mock(&stdout);
    let work = TempWork::new("apparatus-m4-utf8");
    let target = target("s3cret");
    let digest = oras_push_and_oci_digest(
        &target,
        work.path(),
        "127.0.0.1:5000/apparatus/envelope:t10",
    )
    .expect("digest");
    assert_eq!(digest, format!("sha256:{}", sha_hex()));
}

#[test]
fn config_removed_on_success_error_and_unwind() {
    let _guard = Guard::install();
    let password = "s3cret-cleanup";
    let target = target(password);
    let guard = ConfigGuard::create(&target).expect("config");
    let path = guard.path().to_path_buf();
    assert!(path.join("config.json").is_file());
    guard.finish().expect("finish");
    assert!(!path.exists());
    remove_private_dir(&path).expect("second remove");

    let recorded = RefCell::new(None);
    let err = with_registry_config(&target, |dir| -> Result<(), AdmitError> {
        *recorded.borrow_mut() = Some(PathBuf::from(dir));
        Err(AdmitError::new("tool failed"))
    })
    .expect_err("body");
    assert_eq!(err.to_string(), "tool failed");
    assert!(!err.to_string().contains(password));
    let path = recorded.borrow().clone().expect("recorded");
    assert!(!path.exists());

    let recorded = RefCell::new(None);
    let caught = catch_unwind(AssertUnwindSafe(|| {
        let guard = ConfigGuard::create(&target).expect("config");
        *recorded.borrow_mut() = Some(guard.path().to_path_buf());
        panic!("config unwind");
    }));
    assert!(caught.is_err());
    let path = recorded.borrow().clone().expect("unwind path");
    assert!(!path.exists());
}

#[test]
fn remove_failure_is_returned_once() {
    let file = TempWork::new("apparatus-m4-file");
    let path = file.path().join("not-a-directory");
    fs::write(&path, b"x").expect("file");
    let err = remove_private_dir(&path).expect_err("not a directory");
    assert_eq!(err.to_string(), "private config cleanup failed");
    assert!(path.exists());
    remove_private_dir(&path).expect_err("still not a directory");
}

#[cfg(windows)]
#[test]
fn locked_config_cleanup_fails_closed() {
    let _guard = Guard::install();
    let target = target("s3cret-locked");
    let guard = ConfigGuard::create(&target).expect("config");
    let path = guard.path().to_path_buf();
    let _cleanup = DirCleanup(path.clone());
    let held = lock_shared_read(&path.join("config.json"));
    let err = guard.finish().expect_err("locked");
    assert_eq!(err.to_string(), "private config cleanup failed");
    assert!(!err.to_string().contains("s3cret-locked"));
    assert!(path.exists());
    drop(held);
}

#[cfg(windows)]
fn lock_shared_read(path: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path)
        .expect("lock config")
}

#[cfg(windows)]
#[test]
fn windows_acl_is_single_owner_ace() {
    let _guard = Guard::install();
    let target = target("s3cret-acl");
    let guard = ConfigGuard::create(&target).expect("config");
    let _cleanup = DirCleanup(guard.path().to_path_buf());
    assert_owner_only(guard.path());
    assert_owner_only(&guard.path().join("config.json"));
    let listed = Command::new(system_exe("icacls.exe"))
        .arg(guard.path())
        .output()
        .expect("icacls");
    let text = String::from_utf8_lossy(&listed.stdout);
    assert!(!text.contains("s3cret-acl"));
}

#[cfg(windows)]
fn assert_owner_only(path: &Path) {
    let output = Command::new(system_exe("icacls.exe"))
        .arg(path)
        .output()
        .expect("icacls");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    let aces: Vec<&str> = text.lines().filter(|line| line.contains(":(")).collect();
    assert_eq!(aces.len(), 1, "aces: {text}");
    let ace = aces[0];
    assert!(!ace.contains("(I)"), "inherited {ace}");
    let account = current_account();
    let sid = current_sid();
    let ace_lower = ace.to_lowercase();
    assert!(
        ace_lower.contains(&account.to_lowercase()) || ace.contains(&sid),
        "ace {ace} account {account} sid {sid}"
    );
}

#[cfg(windows)]
fn current_account() -> String {
    let output = Command::new(system_exe("whoami.exe"))
        .output()
        .expect("whoami");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[cfg(windows)]
fn current_sid() -> String {
    let output = Command::new(system_exe("whoami.exe"))
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .expect("whoami sid");
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .next()
        .unwrap_or("")
        .split(',')
        .nth(1)
        .unwrap_or("")
        .trim()
        .trim_matches('"')
        .to_owned()
}

#[cfg(unix)]
#[test]
fn unix_config_modes_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let _guard = Guard::install();
    let guard = ConfigGuard::create(&target("s3cret-mode")).expect("config");
    let dir_mode = fs::metadata(guard.path())
        .expect("dir meta")
        .permissions()
        .mode()
        & 0o777;
    let file_mode = fs::metadata(guard.path().join("config.json"))
        .expect("file meta")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(dir_mode, 0o700);
    assert_eq!(file_mode, 0o600);
    guard.finish().expect("finish");
}

#[cfg(windows)]
#[test]
fn stdin_is_closed_before_wait() {
    let _guard = Guard::install();
    let mut spec = real_spec(
        system_exe("cmd.exe"),
        vec![
            "/c".to_owned(),
            "findstr".to_owned(),
            "/R".to_owned(),
            ".*".to_owned(),
        ],
    );
    spec.stdin = Some(b"hello-frame\r\n".to_vec());
    spec.deadline = Duration::from_secs(8);
    let started = Instant::now();
    let output = execute(spec).expect("findstr");
    assert!(started.elapsed() < Duration::from_secs(8));
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("hello-frame"));
}

#[cfg(windows)]
#[test]
fn direct_child_deadline_returns_without_hunting_heirs() {
    let _guard = Guard::install();
    let _sweep = PingSweep::arm();
    let mut spec = real_spec(
        system_exe("ping.exe"),
        vec!["-n".to_owned(), "40".to_owned(), "127.0.0.1".to_owned()],
    );
    spec.deadline = Duration::from_secs(2);
    let started = Instant::now();
    let err = execute(spec).expect_err("deadline");
    assert_bounded(started, Duration::from_secs(2));
    assert!(err.to_string().contains("cli deadline exceeded"));
}

#[cfg(windows)]
#[test]
fn pipe_heir_deadline_returns_before_bound() {
    let _sweep = PingSweep::arm();
    let _guard = Guard::install();
    let mut spec = real_spec(
        system_exe("cmd.exe"),
        vec![
            "/c".to_owned(),
            "start".to_owned(),
            "/b".to_owned(),
            system_exe("ping.exe").to_string_lossy().into_owned(),
            "-n".to_owned(),
            "40".to_owned(),
            "127.0.0.1".to_owned(),
        ],
    );
    spec.deadline = Duration::from_secs(2);
    let started = Instant::now();
    let err = execute(spec).expect_err("heir");
    assert_bounded(started, Duration::from_secs(2));
    assert!(err.to_string().contains("cli deadline exceeded"));
}

#[cfg(windows)]
#[test]
fn output_over_limit_is_generic() {
    let _guard = Guard::install();
    let mut spec = real_spec(
        system_exe("cmd.exe"),
        vec![
            "/c".to_owned(),
            "echo".to_owned(),
            "ADVERSARIAL-MARKER-0123456789".to_owned(),
        ],
    );
    spec.output_limit = 4;
    spec.deadline = Duration::from_secs(8);
    let err = execute(spec).expect_err("cap");
    let text = err.to_string();
    assert_eq!(text, "cli output exceeded limit");
    assert!(!text.contains("ADVERSARIAL-MARKER"));
}

#[cfg(windows)]
#[test]
fn missing_helper_error_hides_the_path() {
    let _guard = Guard::install();
    let work = TempWork::new("apparatus-m4-missing");
    let program = work.path().join("missing-s3cret.exe");
    let err = execute(real_spec(program, vec!["s3cret".to_owned()])).expect_err("spawn");
    assert_eq!(err.to_string(), "cannot start helper");
    assert!(!err.to_string().contains("s3cret"));
}

#[cfg(windows)]
#[test]
fn child_env_scrubs_vault_token_unless_explicit() {
    let _guard = Guard::install();
    let parent = std::env::var_os("VAULT_TOKEN");
    let mut spec = real_spec(
        system_exe("cmd.exe"),
        vec![
            "/c".to_owned(),
            "echo".to_owned(),
            "%VAULT_TOKEN%".to_owned(),
        ],
    );
    spec.remove_env = vec!["VAULT_TOKEN".to_owned()];
    let output = execute(spec).expect("scrub");
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(text.trim(), "%VAULT_TOKEN%");
    assert_eq!(std::env::var_os("VAULT_TOKEN"), parent);

    let mut spec = real_spec(
        system_exe("cmd.exe"),
        vec![
            "/c".to_owned(),
            "echo".to_owned(),
            "%VAULT_TOKEN%".to_owned(),
        ],
    );
    spec.remove_env = vec!["VAULT_TOKEN".to_owned()];
    spec.set_env = vec![("VAULT_TOKEN".to_owned(), "vault-token-value".to_owned())];
    let output = execute(spec).expect("set");
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(text.trim(), "vault-token-value");
    assert_eq!(std::env::var_os("VAULT_TOKEN"), parent);
}

#[cfg(unix)]
#[test]
fn stdin_is_closed_before_wait() {
    let _guard = Guard::install();
    let mut spec = real_spec(
        PathBuf::from("/bin/sh"),
        vec!["-c".to_owned(), "cat".to_owned()],
    );
    spec.stdin = Some(b"hello-frame\n".to_vec());
    spec.deadline = Duration::from_secs(8);
    let started = Instant::now();
    let output = execute(spec).expect("cat");
    assert!(started.elapsed() < Duration::from_secs(8));
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("hello-frame"));
}

#[cfg(unix)]
#[test]
fn direct_child_deadline_returns_without_hunting_heirs() {
    let _guard = Guard::install();
    let mut spec = real_spec(
        PathBuf::from("/bin/sh"),
        vec!["-c".to_owned(), "sleep 30".to_owned()],
    );
    spec.deadline = Duration::from_secs(2);
    let started = Instant::now();
    let err = execute(spec).expect_err("deadline");
    assert_bounded(started, Duration::from_secs(2));
    assert!(err.to_string().contains("cli deadline exceeded"));
}

#[cfg(unix)]
#[test]
fn pipe_heir_deadline_returns_before_bound() {
    let _guard = Guard::install();
    let mut spec = real_spec(
        PathBuf::from("/bin/sh"),
        vec!["-c".to_owned(), "sleep 30 &".to_owned()],
    );
    spec.deadline = Duration::from_secs(2);
    let started = Instant::now();
    let err = execute(spec).expect_err("heir");
    assert_bounded(started, Duration::from_secs(2));
    assert!(err.to_string().contains("cli deadline exceeded"));
}

#[cfg(unix)]
#[test]
fn output_over_limit_is_generic() {
    let _guard = Guard::install();
    let mut spec = real_spec(
        PathBuf::from("/bin/sh"),
        vec![
            "-c".to_owned(),
            "printf %s ADVERSARIAL-MARKER-0123456789".to_owned(),
        ],
    );
    spec.output_limit = 4;
    let err = execute(spec).expect_err("cap");
    assert_eq!(err.to_string(), "cli output exceeded limit");
}

#[cfg(unix)]
#[test]
fn missing_helper_error_hides_the_path() {
    let work = TempWork::new("apparatus-m4-missing");
    let program = work.path().join("missing-s3cret");
    let err = execute(real_spec(program, Vec::new())).expect_err("spawn");
    assert_eq!(err.to_string(), "cannot start helper");
    assert!(!err.to_string().contains("s3cret"));
}

#[cfg(unix)]
#[test]
fn child_env_scrubs_vault_token_unless_explicit() {
    let parent = std::env::var_os("VAULT_TOKEN");
    let mut spec = real_spec(
        PathBuf::from("/bin/sh"),
        vec!["-c".to_owned(), "printf %s \"$VAULT_TOKEN\"".to_owned()],
    );
    spec.remove_env = vec!["VAULT_TOKEN".to_owned()];
    let output = execute(spec).expect("scrub");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(std::env::var_os("VAULT_TOKEN"), parent);

    let mut spec = real_spec(
        PathBuf::from("/bin/sh"),
        vec!["-c".to_owned(), "printf %s \"$VAULT_TOKEN\"".to_owned()],
    );
    spec.remove_env = vec!["VAULT_TOKEN".to_owned()];
    spec.set_env = vec![("VAULT_TOKEN".to_owned(), "vault-token-value".to_owned())];
    let output = execute(spec).expect("set");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "vault-token-value");
    assert_eq!(std::env::var_os("VAULT_TOKEN"), parent);
}
