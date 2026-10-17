//! Transport ORAS/Cosign : argv sans secret, stdin ORAS, config Docker privée.
//!
//! À l'échéance les lectures encore bloquées sont abandonnées (`JoinHandle`
//! droppé) et seul l'enfant direct est tué. Un héritier de pipe ne prolonge
//! pas l'attente : `tokio` sur Windows joindrait ces lectures dans
//! `spawn_blocking` jusqu'au drop du runtime. `unsafe_code` est interdit,
//! donc pas de poll natif. Aucun `std::env::set_var` / `remove_var`.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::{docker_host_path, host_tool, AdmitError, AdmitTarget};

const CLI_DEADLINE: Duration = Duration::from_secs(120);
const CLI_OUTPUT_LIMIT: usize = 1024 * 1024;
const POLL_SLICE: Duration = Duration::from_millis(15);
const READ_GRACE_AFTER_KILL: Duration = Duration::from_secs(1);

const SCRUB_ENV: &[&str] = &[
    "VAULT_TOKEN",
    "DOCKER_CONFIG",
    "REGISTRY_PASSWORD",
    "ORAS_PASSWORD",
    "COSIGN_REGISTRY_PASSWORD",
];

static UNIQUE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub(crate) struct CliSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub set_env: Vec<(String, String)>,
    pub remove_env: Vec<String>,
    pub stdin: Option<Vec<u8>>,
    pub workdir: Option<PathBuf>,
    pub deadline: Duration,
    pub output_limit: usize,
}

pub(crate) struct RunRequest<'a> {
    pub bin_env: &'a str,
    pub image: &'a str,
    pub network: &'a str,
    pub extra_env: &'a [(&'a str, &'a str)],
    pub args: &'a [String],
    pub workdir: Option<&'a Path>,
    pub stdin: Option<&'a [u8]>,
    pub secrets: &'a [&'a str],
}

pub(crate) fn oras_password_frame(password: &str) -> Vec<u8> {
    let mut framed = Vec::with_capacity(password.len() + 2);
    framed.extend_from_slice(password.as_bytes());
    framed.extend_from_slice(b"\r\n");
    framed
}

pub(crate) fn run_cli(request: RunRequest<'_>) -> Result<Output, AdmitError> {
    reject_secret_argv(request.args, request.secrets)?;
    let deadline = cli_deadline();
    let output_limit = output_limit();
    if let Some(bin) = host_tool(request.bin_env) {
        let spec = CliSpec {
            program: bin,
            args: request.args.to_vec(),
            set_env: owned_env(request.extra_env),
            remove_env: scrub_keys(),
            stdin: request.stdin.map(|bytes| bytes.to_vec()),
            workdir: request.workdir.map(Path::to_path_buf),
            deadline,
            output_limit,
        };
        return execute(spec);
    }
    run_docker(request, deadline, output_limit)
}

fn run_docker(
    request: RunRequest<'_>,
    deadline: Duration,
    output_limit: usize,
) -> Result<Output, AdmitError> {
    let cid_path = unique_path("apparatus-admit-cid");
    let mut args = vec![
        "run".to_owned(),
        "-i".to_owned(),
        "--network".to_owned(),
        request.network.to_owned(),
        "--cidfile".to_owned(),
        path_string(&cid_path)?,
    ];
    let mut set_env = Vec::new();
    for (key, value) in request.extra_env {
        if *key == "DOCKER_CONFIG" {
            let host = docker_host_path(Path::new(value))?;
            args.push("-v".to_owned());
            args.push(format!("{host}:/auth:ro"));
            args.push("-e".to_owned());
            args.push("DOCKER_CONFIG=/auth".to_owned());
        } else {
            set_env.push(((*key).to_owned(), (*value).to_owned()));
            args.push("-e".to_owned());
            args.push((*key).to_owned());
        }
    }
    if let Some(dir) = request.workdir {
        let host = docker_host_path(dir)?;
        args.push("-v".to_owned());
        args.push(format!("{host}:/work"));
        args.push("-w".to_owned());
        args.push("/work".to_owned());
    }
    args.push(request.image.to_owned());
    args.extend(request.args.iter().cloned());
    reject_secret_argv(&args, request.secrets)?;
    let spec = CliSpec {
        program: PathBuf::from("docker"),
        args,
        set_env,
        remove_env: scrub_keys(),
        stdin: request.stdin.map(|bytes| bytes.to_vec()),
        workdir: None,
        deadline,
        output_limit,
    };
    let ran = execute(spec);
    let id = read_container_id(&cid_path);
    let _ = fs::remove_file(&cid_path);
    let missing_id = matches!(&ran, Ok(output) if output.status.success()) && id.is_none();
    let mut release = ContainerRelease {
        id,
        attempted: false,
    };
    let cleanup = release.finish();
    let ran = if missing_id {
        Err(AdmitError::new("container cleanup failed"))
    } else {
        ran
    };
    combine(ran, cleanup)
}

fn reject_secret_argv(args: &[String], secrets: &[&str]) -> Result<(), AdmitError> {
    for secret in secrets {
        if secret.is_empty() {
            continue;
        }
        if args.iter().any(|arg| argument_exposes_secret(arg, secret)) {
            return Err(AdmitError::new("cannot start helper"));
        }
    }
    if args
        .iter()
        .any(|arg| arg == "--password" || arg == "--registry-password")
    {
        return Err(AdmitError::new("cannot start helper"));
    }
    Ok(())
}

fn argument_exposes_secret(arg: &str, secret: &str) -> bool {
    arg == secret || arg.split('=').skip(1).any(|value| value.contains(secret))
}

fn owned_env(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn scrub_keys() -> Vec<String> {
    SCRUB_ENV.iter().map(|key| (*key).to_owned()).collect()
}

pub(crate) fn execute(spec: CliSpec) -> Result<Output, AdmitError> {
    #[cfg(test)]
    if let Some(output) = super::cli_transport_tests::intercept(&spec) {
        return output;
    }
    supervise(spec)
}

fn supervise(spec: CliSpec) -> Result<Output, AdmitError> {
    let deadline_at = Instant::now() + spec.deadline;
    let mut child = spawn_child(&spec)?;
    if !write_stdin(&mut child, spec.stdin.as_deref()) {
        reap_direct(&mut child);
        return Err(AdmitError::new("cannot start helper"));
    }
    drop(child.stdin.take());

    let limit = spec.output_limit;
    let stdout = match spawn_reader(child.stdout.take(), limit) {
        Ok(reader) => reader,
        Err(()) => {
            reap_direct(&mut child);
            return Err(AdmitError::new("cannot start helper"));
        }
    };
    let stderr = match spawn_reader(child.stderr.take(), limit) {
        Ok(reader) => reader,
        Err(()) => {
            reap_direct(&mut child);
            drop(stdout);
            return Err(AdmitError::new("cannot start helper"));
        }
    };

    let status = match wait_direct(&mut child, deadline_at) {
        Ok(status) => status,
        Err(err) => {
            reap_direct(&mut child);
            drop(stdout);
            drop(stderr);
            return Err(err);
        }
    };
    let reader_deadline = if status.is_none() {
        reap_direct(&mut child);
        deadline_at + READ_GRACE_AFTER_KILL
    } else {
        deadline_at
    };
    let stdout = stdout.wait_until(reader_deadline);
    let stderr = stderr.wait_until(reader_deadline);
    assemble(status, stdout, stderr)
}

fn spawn_child(spec: &CliSpec) -> Result<Child, AdmitError> {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    for key in &spec.remove_env {
        command.env_remove(key);
    }
    for (key, value) in &spec.set_env {
        command.env(key, value);
    }
    if let Some(dir) = &spec.workdir {
        command.current_dir(dir);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
        .spawn()
        .map_err(|_| AdmitError::new("cannot start helper"))
}

fn write_stdin(child: &mut Child, payload: Option<&[u8]>) -> bool {
    let Some(payload) = payload else {
        return true;
    };
    match child.stdin.as_mut() {
        Some(stdin) => stdin.write_all(payload).is_ok(),
        None => false,
    }
}

fn wait_direct(child: &mut Child, deadline_at: Instant) -> Result<Option<ExitStatus>, AdmitError> {
    loop {
        if Instant::now() >= deadline_at {
            return Ok(None);
        }
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => thread::sleep(POLL_SLICE),
            Err(_) => return Err(AdmitError::new("cannot start helper")),
        }
    }
}

fn reap_direct(child: &mut Child) {
    match child.try_wait() {
        Ok(Some(_)) => {}
        _ => {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

struct PipeDrain {
    bytes: Vec<u8>,
    overflow: bool,
    eof: bool,
}

struct Reader {
    rx: mpsc::Receiver<PipeDrain>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Reader {
    fn wait_until(mut self, deadline_at: Instant) -> Option<PipeDrain> {
        let wait = deadline_at.saturating_duration_since(Instant::now());
        match self.rx.recv_timeout(wait) {
            Ok(drain) => {
                if let Some(handle) = self.handle.take() {
                    let _ = handle.join();
                }
                Some(drain)
            }
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => None,
        }
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        // Lecture encore bloquée : on l'abandonne. Le thread se termine quand
        // le pipe se ferme (héritier tué par le test, ou EOF). La prod ne
        // chasse pas cet héritier.
        drop(self.handle.take());
    }
}

fn spawn_reader<R>(pipe: Option<R>, limit: usize) -> Result<Reader, ()>
where
    R: Read + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let handle = thread::Builder::new()
        .name("admit-cli-read".to_owned())
        .spawn(move || {
            let drain = match pipe {
                Some(pipe) => read_capped(pipe, limit),
                None => PipeDrain {
                    bytes: Vec::new(),
                    overflow: false,
                    eof: true,
                },
            };
            let _ = tx.send(drain);
        })
        .map_err(|_| ())?;
    Ok(Reader {
        rx,
        handle: Some(handle),
    })
}

fn read_capped(mut pipe: impl Read, limit: usize) -> PipeDrain {
    let mut bytes = Vec::new();
    let mut overflow = false;
    let mut buf = [0_u8; 8192];
    loop {
        match pipe.read(&mut buf) {
            Ok(0) => {
                return PipeDrain {
                    bytes,
                    overflow,
                    eof: true,
                };
            }
            Ok(read) => {
                let chunk = &buf[..read];
                if bytes.len() >= limit {
                    overflow = true;
                } else {
                    let room = limit - bytes.len();
                    if chunk.len() > room {
                        bytes.extend_from_slice(&chunk[..room]);
                        overflow = true;
                    } else {
                        bytes.extend_from_slice(chunk);
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => {
                return PipeDrain {
                    bytes,
                    overflow,
                    eof: false,
                };
            }
        }
    }
}

fn assemble(
    status: Option<ExitStatus>,
    stdout: Option<PipeDrain>,
    stderr: Option<PipeDrain>,
) -> Result<Output, AdmitError> {
    let (Some(status), Some(stdout), Some(stderr)) = (status, stdout, stderr) else {
        return Err(AdmitError::new("cli deadline exceeded"));
    };
    if !stdout.eof || !stderr.eof {
        return Err(AdmitError::new("cannot start helper"));
    }
    if stdout.overflow || stderr.overflow {
        return Err(AdmitError::new("cli output exceeded limit"));
    }
    Ok(Output {
        status,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
    })
}

struct ContainerRelease {
    id: Option<String>,
    attempted: bool,
}

impl ContainerRelease {
    fn finish(&mut self) -> Result<(), AdmitError> {
        if self.attempted {
            return Ok(());
        }
        self.attempted = true;
        match self.id.as_deref() {
            Some(id) => release_container(id),
            None => Ok(()),
        }
    }
}

impl Drop for ContainerRelease {
    fn drop(&mut self) {
        if self.attempted {
            return;
        }
        self.attempted = true;
        if let Some(id) = self.id.as_deref() {
            let _ = release_container(id);
        }
    }
}

fn release_container(id: &str) -> Result<(), AdmitError> {
    if !is_container_id(id) {
        return Err(AdmitError::new("container cleanup failed"));
    }
    let stop = execute(docker_tool_spec(&["stop".to_owned(), id.to_owned()]))?;
    if !stop.status.success() {
        return Err(AdmitError::new("container cleanup failed"));
    }
    let inspect = execute(docker_tool_spec(&[
        "inspect".to_owned(),
        "--format".to_owned(),
        "{{.State.Status}}".to_owned(),
        id.to_owned(),
    ]))?;
    if !inspect.status.success() {
        return Err(AdmitError::new("container cleanup failed"));
    }
    let state = String::from_utf8_lossy(&inspect.stdout);
    let state = state.trim();
    if state != "exited" && state != "dead" {
        return Err(AdmitError::new("container cleanup failed"));
    }
    let removed = execute(docker_tool_spec(&["rm".to_owned(), id.to_owned()]))?;
    if !removed.status.success() {
        return Err(AdmitError::new("container cleanup failed"));
    }
    Ok(())
}

fn docker_tool_spec(args: &[String]) -> CliSpec {
    CliSpec {
        program: PathBuf::from("docker"),
        args: args.to_vec(),
        set_env: Vec::new(),
        remove_env: scrub_keys(),
        stdin: None,
        workdir: None,
        deadline: cli_deadline(),
        output_limit: output_limit(),
    }
}

fn is_container_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn read_container_id(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let id = text.trim();
    if is_container_id(id) {
        Some(id.to_owned())
    } else {
        None
    }
}

pub(crate) struct ConfigGuard {
    path: PathBuf,
    attempted: bool,
}

impl ConfigGuard {
    pub(crate) fn create(target: &AdmitTarget) -> Result<Self, AdmitError> {
        let path = unique_path("apparatus-admit-cfg");
        fs::create_dir(&path).map_err(|_| AdmitError::new("private config permissions failed"))?;
        let mut guard = Self {
            path,
            attempted: false,
        };
        if let Err(err) = write_private_config(&guard.path, target) {
            guard.attempted = true;
            let cleanup = remove_private_dir(&guard.path);
            return match cleanup {
                Ok(()) => Err(err),
                Err(cleanup_err) => Err(AdmitError::new(format!("{err}; {cleanup_err}"))),
            };
        }
        Ok(guard)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn finish(mut self) -> Result<(), AdmitError> {
        self.attempted = true;
        remove_private_dir(&self.path)
    }
}

impl Drop for ConfigGuard {
    fn drop(&mut self) {
        if self.attempted {
            return;
        }
        self.attempted = true;
        let _ = remove_private_dir(&self.path);
    }
}

pub(crate) fn remove_private_dir(path: &Path) -> Result<(), AdmitError> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_dir_all(path).map_err(|_| AdmitError::new("private config cleanup failed"))
}

pub(crate) fn with_registry_config<T>(
    target: &AdmitTarget,
    body: impl FnOnce(&str) -> Result<T, AdmitError>,
) -> Result<T, AdmitError> {
    let guard = ConfigGuard::create(target)?;
    let dir = match path_string(guard.path()) {
        Ok(dir) => dir,
        Err(err) => {
            let cleanup = guard.finish();
            return combine(Err(err), cleanup);
        }
    };
    let result = body(&dir);
    let cleanup = guard.finish();
    combine(result, cleanup)
}

fn write_private_config(dir: &Path, target: &AdmitTarget) -> Result<(), AdmitError> {
    lock_down_dir(dir)?;
    let file_path = dir.join("config.json");
    create_private_file(&file_path)?;
    lock_down_file(&file_path)?;
    let mut auths = BTreeMap::new();
    let auth = AuthConfig {
        username: target.auth.username.clone(),
        password: target.auth.password.clone(),
    };
    auths.insert(target.registry_host.clone(), auth.clone());
    auths.insert(target.registry_network.clone(), auth);
    let document = DockerConfigFile { auths };
    let mut file = OpenOptions::new()
        .write(true)
        .open(&file_path)
        .map_err(|_| AdmitError::new("private config permissions failed"))?;
    serde_json::to_writer(&mut file, &document)
        .map_err(|_| AdmitError::new("private config permissions failed"))?;
    file.sync_all()
        .map_err(|_| AdmitError::new("private config permissions failed"))?;
    Ok(())
}

#[derive(Clone, Serialize)]
struct AuthConfig {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct DockerConfigFile {
    auths: BTreeMap<String, AuthConfig>,
}

#[cfg(unix)]
fn lock_down_dir(dir: &Path) -> Result<(), AdmitError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        .map_err(|_| AdmitError::new("private config permissions failed"))
}

#[cfg(unix)]
fn create_private_file(path: &Path) -> Result<(), AdmitError> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map(|_| ())
        .map_err(|_| AdmitError::new("private config permissions failed"))
}

#[cfg(unix)]
fn lock_down_file(path: &Path) -> Result<(), AdmitError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|_| AdmitError::new("private config permissions failed"))
}

#[cfg(windows)]
fn lock_down_dir(dir: &Path) -> Result<(), AdmitError> {
    let sid = current_user_sid()?;
    icacls(dir, "/inheritance:r", None)?;
    icacls(dir, "/grant:r", Some(format!("*{sid}:(OI)(CI)F")))
}

#[cfg(windows)]
fn create_private_file(path: &Path) -> Result<(), AdmitError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(|_| ())
        .map_err(|_| AdmitError::new("private config permissions failed"))
}

#[cfg(windows)]
fn lock_down_file(path: &Path) -> Result<(), AdmitError> {
    let sid = current_user_sid()?;
    icacls(path, "/inheritance:r", None)?;
    icacls(path, "/grant:r", Some(format!("*{sid}:F")))
}

#[cfg(windows)]
fn current_user_sid() -> Result<String, AdmitError> {
    let output = Command::new("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .map_err(|_| AdmitError::new("private config permissions failed"))?;
    if !output.status.success() {
        return Err(AdmitError::new("private config permissions failed"));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next().unwrap_or("");
    let sid = line
        .split(',')
        .nth(1)
        .unwrap_or("")
        .trim()
        .trim_matches('"');
    if sid.starts_with("S-1-") {
        Ok(sid.to_owned())
    } else {
        Err(AdmitError::new("private config permissions failed"))
    }
}

#[cfg(windows)]
fn icacls(path: &Path, action: &str, grant: Option<String>) -> Result<(), AdmitError> {
    let mut command = Command::new("icacls");
    command.arg(path);
    command.arg(action);
    if let Some(grant) = grant.as_ref() {
        command.arg(grant);
    }
    let output = command
        .output()
        .map_err(|_| AdmitError::new("private config permissions failed"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(AdmitError::new("private config permissions failed"))
    }
}

fn combine<T>(
    result: Result<T, AdmitError>,
    cleanup: Result<(), AdmitError>,
) -> Result<T, AdmitError> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(cleanup_err)) => Err(cleanup_err),
        (Err(action_err), Ok(())) => Err(action_err),
        (Err(action_err), Err(cleanup_err)) => {
            Err(AdmitError::new(format!("{action_err}; {cleanup_err}")))
        }
    }
}

fn unique_path(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let seq = UNIQUE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{nanos}-{seq}"))
}

fn path_string(path: &Path) -> Result<String, AdmitError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| AdmitError::new("private config permissions failed"))
}

fn cli_deadline() -> Duration {
    #[cfg(test)]
    if let Some(deadline) = super::cli_transport_tests::deadline_override() {
        return deadline;
    }
    CLI_DEADLINE
}

fn output_limit() -> usize {
    #[cfg(test)]
    if let Some(limit) = super::cli_transport_tests::limit_override() {
        return limit;
    }
    CLI_OUTPUT_LIMIT
}

#[cfg(test)]
pub(crate) fn forced_host(bin_env: &str) -> Option<Option<PathBuf>> {
    super::cli_transport_tests::host_override(bin_env)
}

#[cfg(test)]
pub(crate) fn drop_container_release(id: &str) {
    let release = ContainerRelease {
        id: Some(id.to_owned()),
        attempted: false,
    };
    drop(release);
}
