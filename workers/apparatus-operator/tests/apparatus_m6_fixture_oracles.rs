//! PH-2/3/4/5/8/9/10/16. No runtime, global env mutation, or mocked ensure.
//! Oracle: approved 20261018-m6-proof-hardening contract, not implementation output.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "fixtures/kind/ensure.rs"]
mod ensure;
use ensure::{ensure_with, ExecOutcome, FixturePort};

#[derive(Clone)]
struct Step {
    program: &'static str,
    args: Vec<&'static str>,
    stdout: &'static str,
}

fn step(program: &'static str, args: &[&'static str], stdout: &'static str) -> Step {
    Step {
        program,
        args: args.to_vec(),
        stdout,
    }
}

fn nominal_plan() -> Vec<Step> {
    let mut steps = vec![
        step("docker", &["info"], ""),
        step("kind", &["version"], "kind v0.29.0"),
        step("kind", &["get", "clusters"], "aiforall-local\n"),
        step(
            "kind",
            &[
                "export",
                "kubeconfig",
                "--name",
                "aiforall-local",
                "--kubeconfig",
                "<config>",
            ],
            "",
        ),
    ];
    let operations = api_cni_preconditions()
        .into_iter()
        .chain(cni_readiness())
        .chain(operator_apply_plan());
    for operation in operations {
        steps.push(step("kubectl", &["version", "--client"], "client only"));
        steps.push(operation);
    }
    steps
}

fn api_cni_preconditions() -> Vec<Step> {
    vec![
        step("kubectl", &["get", "--raw=/readyz"], "ok"),
        step(
            "kubectl",
            &[
                "get",
                "ds",
                "kindnet",
                "-n",
                "kube-system",
                "-o",
                "name",
                "--ignore-not-found=true",
            ],
            "",
        ),
        step(
            "kubectl",
            &[
                "get",
                "ds",
                "calico-node",
                "-n",
                "kube-system",
                "-o",
                "name",
                "--ignore-not-found=true",
            ],
            "daemonset.apps/calico-node",
        ),
    ]
}

fn cni_readiness() -> Vec<Step> {
    vec![
        step(
            "kubectl",
            &[
                "rollout",
                "status",
                "ds/calico-node",
                "-n",
                "kube-system",
                "--timeout=600s",
            ],
            "",
        ),
        step(
            "kubectl",
            &[
                "rollout",
                "status",
                "deploy/calico-kube-controllers",
                "-n",
                "kube-system",
                "--timeout=300s",
            ],
            "",
        ),
        step(
            "kubectl",
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
            "",
        ),
        step(
            "kubectl",
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
            "",
        ),
        step(
            "kubectl",
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
            "",
        ),
        step(
            "kubectl",
            &[
                "wait",
                "--for=condition=Ready",
                "node",
                "--all",
                "--timeout=180s",
            ],
            "",
        ),
        step(
            "kubectl",
            &[
                "get",
                "ds",
                "kindnet",
                "-n",
                "kube-system",
                "-o",
                "name",
                "--ignore-not-found=true",
            ],
            "",
        ),
    ]
}

fn operator_apply_plan() -> [Step; 2] {
    [
        step(
            "kubectl",
            &[
                "apply",
                "--server-side",
                "--field-manager=apparatus-it-fixtures",
                "-f",
                "<manifests>",
            ],
            "",
        ),
        step(
            "kubectl",
            &[
                "wait",
                "--for=condition=Established",
                "crd/admissionrecords.apparatus.aiforall.dev",
                "--timeout=90s",
            ],
            "",
        ),
    ]
}

#[derive(Clone, Debug)]
struct Invocation {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    result: Result<bool, String>,
}

#[derive(Clone, Copy)]
enum Failure {
    Exit,
    Spawn,
}

struct Script {
    plan: Vec<Step>,
    trace: Vec<Invocation>,
    fault: Option<(usize, Failure)>,
    now: u64,
    sleeps: Vec<Duration>,
    ready_timeout: bool,
    objects: BTreeMap<String, String>,
    conflict: bool,
    file_reads: Vec<std::path::PathBuf>,
}

impl Script {
    fn nominal() -> Self {
        Self {
            plan: nominal_plan(),
            trace: Vec::new(),
            fault: None,
            now: 0,
            sleeps: Vec::new(),
            ready_timeout: false,
            objects: BTreeMap::from([
                (
                    "namespace/third-party.manager".to_owned(),
                    "platform-j2".to_owned(),
                ),
                (
                    "namespace/third-party.label".to_owned(),
                    "preserve-me".to_owned(),
                ),
            ]),
            conflict: false,
            file_reads: Vec::new(),
        }
    }

    fn position(&self, args: &[&str]) -> usize {
        self.plan
            .iter()
            .position(|s| s.args == args)
            .expect("scenario command exists")
    }

    fn clean_trace(&self) {
        for invocation in &self.trace {
            assert!(
                !invocation.args.iter().any(|a| matches!(
                    a.as_str(),
                    "create" | "delete" | "install" | "--force" | "--force-conflicts"
                )),
                "PH-4/5: forbidden corrective argv"
            );
            assert!(
                !invocation
                    .args
                    .iter()
                    .any(|a| a.contains("calico.yaml") || a.contains("FELIX_")),
                "PH-4: no CNI installation"
            );
        }
    }
}

impl FixturePort for Script {
    fn run(
        &mut self,
        program: &str,
        args: &[&str],
        env: &[(String, String)],
    ) -> Result<ExecOutcome, String> {
        let index = self.trace.len();
        let step = if self.ready_timeout && index >= 4 {
            if args == ["version", "--client"] {
                step("kubectl", &["version", "--client"], "")
            } else {
                step("kubectl", &["get", "--raw=/readyz"], "")
            }
        } else {
            self.plan
                .get(index)
                .expect("no unplanned command/remediation")
                .clone()
        };
        // Only dedicated generated paths vary. Every argv token is otherwise exact.
        let actual: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        let mut expected: Vec<String> = step.args.iter().map(|s| (*s).to_owned()).collect();
        if let Some(last) = expected.last_mut() {
            if last == "<config>" {
                last.clone_from(actual.last().expect("export path"));
            } else if last == "<manifests>" {
                *last = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("k8s")
                    .to_string_lossy()
                    .into_owned();
            }
        }
        assert_eq!(program, step.program, "PH-4: expected command family");
        assert_eq!(actual, expected, "PH-4/8: exact outgoing argv");
        let fault = self
            .fault
            .filter(|(at, _)| *at == index)
            .map(|(_, mode)| mode);
        let result = if self.ready_timeout && args == ["get", "--raw=/readyz"] {
            Ok(false)
        } else if args.first() == Some(&"apply") && self.conflict {
            // A forced retry would actually corrupt the independent object state.
            if args.contains(&"--force-conflicts") {
                self.objects.insert(
                    "namespace/third-party.manager".to_owned(),
                    "fixture".to_owned(),
                );
            }
            Ok(false)
        } else {
            match fault {
                Some(Failure::Spawn) => Err("private diagnostic must not escape".to_owned()),
                Some(Failure::Exit) => Ok(false),
                None => Ok(true),
            }
        };
        self.trace.push(Invocation {
            program: program.to_owned(),
            args: actual,
            env: env.to_vec(),
            result: result.clone(),
        });
        result.map(|success| ExecOutcome {
            success,
            stdout: step.stdout.as_bytes().to_vec(),
            stderr: if success {
                Vec::new()
            } else {
                b"Conflict Forbidden not found private diagnostic".to_vec()
            },
        })
    }

    fn sleep(&mut self, duration: Duration) {
        self.sleeps.push(duration);
        self.now += u64::try_from(duration.as_millis()).expect("bounded test duration");
    }
    fn monotonic_ms(&mut self) -> u64 {
        self.now
    }
    fn is_file(&mut self, path: &std::path::Path) -> bool {
        self.file_reads.push(path.to_owned());
        // Host fallback is explicitly absent in these scenarios. Explicit
        // overrides use actual test-owned files, never a user/global executable.
        path != std::path::Path::new(ensure::WINDOWS_KIND) && path.is_file()
    }
}

#[test]
fn ph2_ph3_refused_targets_enter_real_ensure_and_make_zero_calls() {
    // Mutation: putting Docker/probe ahead of the guard is caught by trace.is_empty.
    for (raw, category) in [
        ("", "cible vide"),
        (" \t\r\n", "cible vide"),
        ("kind-apparatus-p4-it", "cible protégée"),
        ("apparatus-p4-it", "cible protégée"),
        ("rancher-desktop", "cible protégée"),
        ("../aiforall-local", "cible invalide"),
        ("C:\\secret", "cible invalide"),
        ("aiforall/local", "cible invalide"),
        ("Aiforall-local", "cible invalide"),
        ("aiforall local", "cible invalide"),
        ("-aiforall-local", "cible invalide"),
        ("kind-aiforall-local", "cible non autorisée"),
        ("aiforall-local-full", "cible non autorisée"),
        ("other-valid-name", "cible non autorisée"),
        ("bearer-secret\r\nconfig", "cible invalide"),
    ] {
        let mut script = Script::nominal();
        let error = ensure_with(Some(raw.as_ref()), None, &mut script)
            .expect_err("PH-2: rejected before any command");
        assert_eq!(error, category, "PH-2: closed safe category");
        assert!(
            script.trace.is_empty(),
            "PH-2: rejected input must not invoke Docker/probes"
        );
        assert!(
            script.file_reads.is_empty(),
            "PH-2: no binary filesystem reads before guard"
        );
    }
}

#[test]
fn ph2_non_unicode_present_is_not_an_absent_default() {
    #[cfg(windows)]
    let raw = {
        use std::os::windows::ffi::OsStringExt;
        OsString::from_wide(&[0xD800])
    };
    #[cfg(unix)]
    let raw = {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![0xff])
    };
    let mut script = Script::nominal();
    assert_eq!(
        ensure_with(Some(raw.as_os_str()), None, &mut script).expect_err("PH-2 invalid Unicode"),
        "cible invalide"
    );
    assert!(
        script.trace.is_empty(),
        "PH-2 no fallback for present invalid env"
    );
}

#[test]
fn ph3_ph8_nominal_default_and_trim_use_dedicated_config_and_client_probes() {
    // Mutation: bare kubectl version, user kubeconfig, reordered CNI/apply or trim
    // removal fails exact boundary argv and the exported config assertions.
    let mut configs = Vec::new();
    for target in [None, Some(OsString::from("  aiforall-local \t"))] {
        let mut script = Script::nominal();
        let cluster =
            ensure_with(target.as_deref(), None, &mut script).expect("PH-3 nominal complete");
        assert_eq!(
            script.trace.len(),
            script.plan.len(),
            "all preconditions and SSA/CRD completed"
        );
        assert!(script.sleeps.is_empty());
        let config = cluster.kubeconfig.path.to_string_lossy().into_owned();
        assert!(cluster
            .kubeconfig
            .path
            .parent()
            .expect("dedicated parent")
            .is_dir());
        assert_eq!(
            script.trace[3].args.last(),
            Some(&config),
            "export destination is dedicated"
        );
        for invocation in script.trace.iter().filter(|i| i.program == "kubectl") {
            assert_eq!(
                invocation.env,
                vec![("KUBECONFIG".to_owned(), config.clone())],
                "every kubectl including probes is isolated"
            );
            assert!(invocation.result.as_ref().is_ok_and(|success| *success));
        }
        assert_eq!(script.trace[1].args, ["version"]);
        script.clean_trace();
        configs.push(cluster.kubeconfig.path.clone());
        drop(cluster);
        assert!(
            !std::path::Path::new(&config)
                .parent()
                .expect("parent")
                .exists(),
            "owned temp directory cleaned"
        );
    }
    assert_ne!(
        configs[0], configs[1],
        "no concurrent test/global profile collision"
    );
}

#[test]
fn ph16_authorized_cluster_absent_stops_after_list_without_mutation() {
    let mut script = Script::nominal();
    script.plan[2].stdout = "aiforall-local-full\nunrelated\n";
    assert_eq!(
        ensure_with(None, None, &mut script).expect_err("authorized cluster absent"),
        "authorized Kind cluster absent: runtime operation required"
    );
    assert_eq!(
        script.trace.len(),
        3,
        "no export, kubectl or cluster creation"
    );
    script.clean_trace();
}

#[test]
fn ph4_ph10_incompatible_absent_and_api_error_cni_are_distinct() {
    for (args, reply, failure, expected) in [
        (
            vec![
                "get",
                "ds",
                "kindnet",
                "-n",
                "kube-system",
                "-o",
                "name",
                "--ignore-not-found=true",
            ],
            "daemonset.apps/kindnet",
            None,
            "kindnet forbidden",
        ),
        (
            vec![
                "get",
                "ds",
                "calico-node",
                "-n",
                "kube-system",
                "-o",
                "name",
                "--ignore-not-found=true",
            ],
            "",
            None,
            "calico-node absent: runtime operation required",
        ),
        (
            vec![
                "get",
                "ds",
                "calico-node",
                "-n",
                "kube-system",
                "-o",
                "name",
                "--ignore-not-found=true",
            ],
            "",
            Some(Failure::Exit),
            "CNI daemonset read: command failed",
        ),
    ] {
        let mut script = Script::nominal();
        let at = script.position(&args);
        script.plan[at].stdout = reply;
        script.fault = failure.map(|mode| (at, mode));
        assert_eq!(
            ensure_with(None, None, &mut script).expect_err("CNI rejection"),
            expected
        );
        assert_eq!(
            script.trace.len(),
            at + 1,
            "no CNI install, rollout or SSA after failure"
        );
        script.clean_trace();
    }
}

#[test]
fn ph4_each_command_spawn_failure_and_nonzero_status_stop_without_remediation() {
    // Script explicitly controls the fallback FS read (absent), so this case
    // cannot accidentally depend on the developer's C:\\Users executable.
    let plan = nominal_plan();
    for (at, command) in plan.iter().enumerate() {
        for mode in [Failure::Spawn, Failure::Exit] {
            if command.args == ["get", "--raw=/readyz"] && matches!(mode, Failure::Exit) {
                continue;
            }
            let mut script = Script::nominal();
            script.fault = Some((at, mode));
            let error = ensure_with(None, None, &mut script)
                .expect_err("failed outgoing command must propagate");
            assert!(
                !error.contains("private diagnostic"),
                "raw tool output must not escape"
            );
            assert_eq!(
                script.trace.len(),
                at + 1,
                "no command after terminal failure"
            );
            script.clean_trace();
        }
    }
}

#[test]
fn ph4_readyz_timeout_is_bounded_by_virtual_monotonic_time() {
    let wall = Instant::now();
    let mut script = Script::nominal();
    script.ready_timeout = true;
    assert_eq!(
        ensure_with(None, None, &mut script).expect_err("API timeout"),
        "Kind API readiness timeout"
    );
    assert!(
        script.now > 180_000 && script.now <= 182_000,
        "real inclusive deadline plus one 2s pause"
    );
    let polls = script
        .trace
        .iter()
        .filter(|i| i.args == ["get", "--raw=/readyz"])
        .count();
    assert_eq!(
        polls,
        script.sleeps.len() + 1,
        "one initial attempt plus each virtual pause"
    );
    assert!(script.sleeps.iter().all(|d| *d == Duration::from_secs(2)));
    assert!(
        wall.elapsed() < Duration::from_secs(5),
        "no 180-second wall sleep"
    );
    assert!(
        !script
            .trace
            .iter()
            .any(|i| i.args.first().is_some_and(|a| a == "apply")),
        "API timeout prevents mutation"
    );
    script.clean_trace();
}

#[test]
fn ph5_ph9_ssa_conflict_is_one_apply_and_preserves_simulated_objects() {
    let mut script = Script::nominal();
    script.conflict = true;
    let before = script.objects.clone();
    let at = script
        .plan
        .iter()
        .position(|s| s.args.first() == Some(&"apply"))
        .expect("SSA scenario");
    assert_eq!(
        ensure_with(None, None, &mut script).expect_err("SSA conflict propagated"),
        "operator SSA apply: command failed"
    );
    assert_eq!(
        script.trace.len(),
        at + 1,
        "no forced second try, deletion or CRD wait"
    );
    assert_eq!(
        script
            .trace
            .iter()
            .filter(|i| i.args.first().is_some_and(|a| a == "apply"))
            .count(),
        1
    );
    assert_eq!(
        script.objects, before,
        "compare independent OBJECT state, not consumed responses"
    );
    script.clean_trace();
}

#[test]
fn ph8_kind_override_file_bypasses_probe_and_missing_override_is_safe_error() {
    struct OverridePort<'a> {
        inner: &'a mut Script,
        binary: &'a str,
    }
    impl FixturePort for OverridePort<'_> {
        fn run(
            &mut self,
            program: &str,
            args: &[&str],
            env: &[(String, String)],
        ) -> Result<ExecOutcome, String> {
            let program = if program == self.binary {
                "kind"
            } else {
                program
            };
            self.inner.run(program, args, env)
        }
        fn sleep(&mut self, duration: Duration) {
            self.inner.sleep(duration);
        }
        fn monotonic_ms(&mut self) -> u64 {
            self.inner.monotonic_ms()
        }
        fn is_file(&mut self, path: &std::path::Path) -> bool {
            self.inner.is_file(path)
        }
    }
    let directory = ensure::KubeconfigFile::new().expect("test-owned directory");
    let binary = directory.path.with_file_name("kind-fixture.exe");
    std::fs::write(&binary, b"test-owned, never executed").expect("fixture file");
    let mut script = Script::nominal();
    script.plan.remove(1);
    // Script assertions use a static program; the owned path is checked separately.
    let missing = directory.path.with_file_name("missing-kind.exe");
    let mut rejected = Script::nominal();
    assert_eq!(
        ensure_with(None, Some(missing.into_os_string()), &mut rejected)
            .expect_err("missing override"),
        "Kind override is not a usable file"
    );
    assert_eq!(
        rejected.trace.len(),
        1,
        "only Docker, never fallback/probe after explicit bad override"
    );
    let mut port = OverridePort {
        inner: &mut script,
        binary: binary.to_str().expect("test path Unicode"),
    };
    let cluster =
        ensure_with(None, Some(binary.clone().into_os_string()), &mut port).expect("file override");
    assert_eq!(cluster.kind_bin, binary);
    assert!(
        !script
            .trace
            .iter()
            .any(|i| i.program == "kind" && i.args == ["version"]),
        "file override has no kind probe"
    );
    script.clean_trace();
}
