#!/usr/bin/env python3
"""Mission1 launcher. The sole runtime/CI entry point; never provisions tools."""
import argparse
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import subprocess
import sys
import time
import uuid

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
sys.path.insert(0, str(HERE / "lib"))
from safety import (ARTIFACT_LIMIT, BROWSER_DIAGNOSTICS, CASE_COUNTERS, CHILD_STOP_PATHS, LINUX_SIGNAL_LABELS, Child, Rejected, child_environment,
                    browser_diagnostic, closed_browser_observation, confidential, digest, name_chain, native_exit_category,
                    linux_signal_category, new_output, private_path, private_read, strict_json)
import private_site as site_channel

CHECKS = {"HTTP": ("iam-status", "iam-body"), "Chromium": ("browser-evaluation", "browser-blank")}
CASES = {"HTTP": "iam-health", "Chromium": "browser-prerequisite"}
SCENARIOS = {phase: HERE / "scenarios" / "smoke" / (case + ".js") for phase, case in CASES.items()}

# Exact names from grafana/k6 v2.3.0 errext/exitcodes/codes.go. Never used for Chrome.
K6_EXIT_CODES = {97: "CLOUD_TEST_RUN_FAILED", 98: "CLOUD_FAILED_TO_GET_PROGRESS", 99: "THRESHOLDS_HAVE_FAILED",
    100: "SETUP_TIMEOUT", 101: "TEARDOWN_TIMEOUT", 102: "GENERIC_TIMEOUT", 103: "SCRIPT_STOPPED_FROM_REST_API",
    104: "INVALID_CONFIG", 105: "EXTERNAL_ABORT", 106: "CANNOT_START_REST_API", 107: "SCRIPT_EXCEPTION",
    108: "SCRIPT_ABORTED", 109: "GO_PANIC", 110: "MARKED_AS_FAILED"}
K6_EXIT_CATEGORIES = frozenset(K6_EXIT_CODES.values()) | {"ABSENT", "ZERO", "SIGNAL", "UNKNOWN"}


def k6_exit_category(code):
    if code is None:
        return "ABSENT"
    if type(code) is not int:
        raise Rejected("INPUT_INVALID")
    return "SIGNAL" if code < 0 else "ZERO" if code == 0 else K6_EXIT_CODES.get(code, "UNKNOWN")


def config_schema(data):
    fields = {"version", "profile", "cluster", "context", "task_id", "target_id", "origin", "lease", "observations"}
    if (not isinstance(data, dict) or set(data) != fields or type(data["version"]) is not int
            or data["version"] != 1 or data["profile"] != "local-full"
            or data["cluster"] != "aiforall-local-full" or data["context"] != "kind-aiforall-local-full"
            or data["origin"] != "http://127.0.0.1:18080"):
        raise Rejected("INPUT_INVALID")
    for key in ("task_id", "target_id"):
        if not isinstance(data[key], str) or not re.fullmatch(r"[a-f0-9]{32}", data[key]):
            raise Rejected("INPUT_INVALID")
    for key in ("lease", "observations"):
        if not isinstance(data[key], str) or not PurePosixPath(data[key]).is_absolute():
            raise Rejected("INPUT_INVALID")
    return data


def configuration(path):
    data = config_schema(strict_json(private_read(path)))
    # Offline only: no import/call of Docker, kubectl or the live target authority.
    lease = strict_json(private_read(data["lease"]))
    private_read(data["observations"])
    if (not isinstance(lease, dict) or lease.get("version") != 2
            or any(lease.get(k) != data[k] for k in ("task_id", "cluster", "context"))):
        raise Rejected("INPUT_INVALID")
    return data


def live_target(config):
    """Preserve the existing authority; never manufacture/adopt a POSIX lease.

    The contract explicitly leaves live origin binding unqualified. A successful
    cluster check is NOT an authorization to send HTTP to a declared loopback.
    No flag or campaign field can lift this gate. Integration needs the parent's
    actual existing guarded observation path (contract lines 103-125).
    """
    try:
        spec = importlib.util.spec_from_file_location("k6_target_authority", REPO / "ops/deploy/local_full_target.py")
        authority = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = authority
        spec.loader.exec_module(authority)
        anchor = authority.LocalClusterTargetAnchor(config["lease"], expected_cluster="aiforall-local-full")
        lease = anchor.check()  # real check; no injected runner, root or privacy
        if lease["task_id"] != config["task_id"]:
            raise Rejected("TARGET_UNAVAILABLE", True)
    except Rejected:
        raise
    except Exception:
        raise Rejected("TARGET_UNAVAILABLE", True) from None
    raise Rejected("TARGET_BINDING_UNQUALIFIED", True)


def verify_summary(data, run_id, phase):
    fields = {"version", "run_id", "phase", "cases", "expected", "started", "completed", "iterations", "requests", "checks"}
    if (not isinstance(data, dict) or set(data) != fields or type(data["version"]) is not int
            or data["version"] != 1 or data["run_id"] != run_id or data["phase"] != phase
            or data["cases"] != [CASES[phase]]):
        raise Rejected("SUMMARY_INVALID")
    for key in ("expected", "started", "completed", "iterations", "requests"):
        expected = 0 if key == "requests" and phase == "Chromium" else 1
        if type(data[key]) is not int or data[key] != expected:
            raise Rejected("SUMMARY_INVALID")
    if not isinstance(data["checks"], dict) or set(data["checks"]) != set(CHECKS[phase]):
        raise Rejected("SUMMARY_INVALID")
    for check in data["checks"].values():
        if (not isinstance(check, dict) or set(check) != {"passes", "fails"}
                or type(check["passes"]) is not int or check["passes"] != 1
                or type(check["fails"]) is not int or check["fails"] != 0):
            raise Rejected("SUMMARY_INVALID")


def browser_summary_observation(path, run_id):
    """Current-run shape/correlation only, NOT the successful-summary oracle.

    Missing/null case counters remain absent; never coerce them to zero. Even a
    VALID diagnostic summary cannot lift native/sandbox/privacy/cleanup failure.
    """
    absent = {key: "ABSENT" for key in CASE_COUNTERS}
    try:
        if path.is_symlink():
            return "INVALID", absent
        try:
            item = path.stat()
        except FileNotFoundError:
            return "ABSENT", absent
        if not path.is_file() or item.st_size > ARTIFACT_LIMIT:
            return "INVALID", absent
        path.chmod(0o400)
        raw = private_read(path)
        if len(raw) > ARTIFACT_LIMIT:
            return "INVALID", absent
        confidential(raw)
        data = strict_json(raw)
        required = {"version", "run_id", "phase", "cases", "checks"}
        if (type(data) is not dict or not required <= set(data) <= required | set(CASE_COUNTERS)
                or type(data["version"]) is not int or data["version"] != 1
                or data["run_id"] != run_id or data["phase"] != "Chromium"
                or data["cases"] != [CASES["Chromium"]]
                or any(data.get(key) is not None and (type(data[key]) is not int or data[key] < 0)
                       for key in CASE_COUNTERS)
                or type(data["checks"]) is not dict or not set(data["checks"]) <= set(CHECKS["Chromium"])):
            return "INVALID", absent
        for check in data["checks"].values():
            if (type(check) is not dict or set(check) != {"passes", "fails"}
                    or any(type(count) is not int or count < 0 for count in check.values())):
                return "INVALID", absent
        return "VALID", {key: "ABSENT" if data.get(key) is None else "ZERO" if data[key] == 0
                         else "ONE" if data[key] == 1 else "OTHER" for key in CASE_COUNTERS}
    except Rejected as error:
        if error.code == "PRIVACY_FAILED":
            raise
        return "INVALID", absent
    except OSError:
        return "INVALID", absent


def verify_false_check(data, run_id, exit_code):
    # A broken endpoint must not masquerade as the deliberate failing check.
    if (exit_code == 0 or not isinstance(data, dict) or type(data.get("negative_baseline")) is not int
            or data["negative_baseline"] != 1 or data.get("checks", {}).get("iam-body") != {"passes": 0, "fails": 1}
            or any(type(value) is not int for value in data["checks"]["iam-body"].values())):
        raise Rejected("SELFTEST_FAILED")
    normalized = {key: value for key, value in data.items() if key != "negative_baseline"}
    normalized["checks"] = {**data["checks"], "iam-body": {"passes": 1, "fails": 0}}
    verify_summary(normalized, run_id, "HTTP")


def linux_platform(*, comparison=False):
    if sys.platform != "linux" or platform.machine() not in {"x86_64", "amd64"} or os.getuid() == 0:
        raise Rejected("PLATFORM_UNQUALIFIED", True)
    try:
        release = dict(line.split("=", 1) for line in Path("/etc/os-release").read_text().splitlines() if "=" in line)
        expected = "22.04" if comparison is True else "24.04"
        if release.get("ID", "").strip('"') != "ubuntu" or release.get("VERSION_ID", "").strip('"') != expected:
            raise Rejected("PLATFORM_UNQUALIFIED", True)
    except OSError:
        raise Rejected("PLATFORM_UNQUALIFIED", True) from None


class PinnedTools(dict):
    """Internal paths plus identities bound before hashing, not a PATH fallback."""
    def __init__(self):
        super().__init__()
        self.identities = {}

    def remember(self, name, archive, binary):
        self[name] = binary
        self.identities[name] = tuple((path, name_chain(path)) for path in (archive, binary))

    def verify(self, names):
        for name in names:
            if name not in self or name not in self.identities:
                raise Rejected("TOOL_UNPINNED", True)
            if self[name] != self.identities[name][-1][0]:
                raise Rejected("TOOL_UNPINNED", True)
            for path, expected in self.identities[name]:
                try:
                    current = name_chain(private_path(path))
                except (Rejected, OSError):
                    raise Rejected("TOOL_UNPINNED", True) from None
                if current != expected:
                    raise Rejected("TOOL_UNPINNED", True)


def verify_tools(tools, names):
    if not isinstance(tools, PinnedTools):
        raise Rejected("TOOL_UNPINNED", True)
    tools.verify(names)


def pinned_tools(browser=False):
    lock = strict_json((HERE / "config/tools.lock.json").read_bytes())
    value = os.environ.get("K6_TOOLS_DIR")
    if not value:
        raise Rejected("TOOL_UNPINNED", True)
    root = private_path(value, directory=True)
    tools = PinnedTools()
    for name in ("k6", "chromium") if browser else ("k6",):
        entry = lock[name]
        archive = root / ("k6.tar.gz" if name == "k6" else "chromium.zip")
        binary = root / entry["executable"]
        # Verify archived distribution AND actual ELF, before any --version.
        for path in (archive, binary):
            if path != path.resolve(strict=True) or not path.is_file():
                raise Rejected("TOOL_UNPINNED", True)
            private_path(path)
        tools.remember(name, archive, binary)
        with binary.open("rb") as stream:
            elf = stream.read(4)
        if (digest(archive) != entry["archive_sha256"] or digest(binary) != entry["executable_sha256"] or elf != b"\x7fELF"):
            raise Rejected("TOOL_UNPINNED", True)
        verify_tools(tools, (name,))
    if browser and os.environ.get("K6_BROWSER_EXECUTABLE_PATH") != str(tools["chromium"]):
        raise Rejected("TOOL_UNPINNED", True)
    return tools, lock


def version_checks(tools, lock, output):
    versions = {}
    for name, binary in tools.items():
        verify_tools(tools, (name,))
        child = Child()
        args = [str(binary), "version" if name == "k6" else "--version"]
        if child.execute(args, output=output, env=child_environment(output), timeout=10) != 0:
            raise Rejected("TOOL_VERSION")
        text = child.stdout.decode("utf-8").strip()
        version = lock[name]["version"]
        pattern = r"k6 v" + re.escape(version) + r"(?:\s|$)" if name == "k6" else r"Google Chrome for Testing " + re.escape(version) + r"$"
        if not re.match(pattern, text):
            raise Rejected("TOOL_VERSION")
        versions[name] = version
    return versions


def k6_argv(binary, output, verb):
    return [str(binary), "--quiet", "--no-color", "--address", "", "--log-format", "raw",
            "--log-output", "stderr", "--config", str(output / "private/empty.json"),
            verb, "--include-system-env-vars=false", "--new-machine-readable-summary=false"]


def inspect_scenario(tools, output, phase):
    verify_tools(tools, ("k6",))
    child = Child()
    if child.execute(k6_argv(tools["k6"], output, "inspect") + [str(SCENARIOS[phase])],
                     output=output, env=child_environment(output), timeout=10) != 0:
        raise Rejected("TOOL_FAILED")
    # Inspect parses options but never runs an iteration or a target guard.
    inspected = strict_json(child.stdout)
    scenarios = inspected.get("scenarios", inspected.get("options", {}).get("scenarios", {}))
    selected = scenarios.get(CASES[phase], {})
    if (set(scenarios) != {CASES[phase]} or selected.get("executor") != "per-vu-iterations"
            or selected.get("vus") != 1 or selected.get("iterations") != 1
            or not selected.get("maxDuration")):
        raise Rejected("INPUT_INVALID")


HOST_LABELS = {
    "unprivileged_userns_clone": {"ZERO", "ONE"},
    "apparmor_restrict_unprivileged_userns": {"ZERO", "ONE"},
    "max_user_namespaces": {"ZERO", "ONE", "POSITIVE"},
    "apparmor_enabled": {"ENABLED", "DISABLED"},
    "parent_seccomp": {"ZERO", "STRICT", "FILTER"},
    "parent_no_new_privs": {"ZERO", "ONE"}}


def closed_host_metadata(value):
    if type(value) is not dict or set(value) != set(HOST_LABELS):
        raise Rejected("INPUT_INVALID")
    for key, label in value.items():
        if type(label) is not str or label not in HOST_LABELS[key] | {"ABSENT", "DENIED", "UNREADABLE"}:
            raise Rejected("INPUT_INVALID")
    return dict(value)


def host_metadata():
    """Optional configured-policy observations, never causal attribution.

    Only five named files; no command, scan, AppArmor profile or log access.
    Raw bytes and arbitrary numeric values never enter the result surface.
    """
    def read(path, limit):
        try:
            with Path(path).open("rb") as stream:
                raw = stream.read(limit + 1)
            if len(raw) > limit:
                return "UNREADABLE", None
            return None, raw.decode("ascii").strip()
        except FileNotFoundError:
            return "ABSENT", None
        except PermissionError:
            return "DENIED", None
        except (OSError, UnicodeError):
            return "UNREADABLE", None

    result = {}
    for key, path in (
            ("unprivileged_userns_clone", "/proc/sys/kernel/unprivileged_userns_clone"),
            ("apparmor_restrict_unprivileged_userns", "/proc/sys/kernel/apparmor_restrict_unprivileged_userns"),
            ("max_user_namespaces", "/proc/sys/user/max_user_namespaces"),
            ("apparmor_enabled", "/sys/module/apparmor/parameters/enabled")):
        state, text = read(path, 64)
        label = {"0": "ZERO", "1": "ONE"}.get(text)
        if key == "apparmor_enabled":
            label = {"Y": "ENABLED", "N": "DISABLED"}.get(text)
        elif key == "max_user_namespaces" and text and re.fullmatch(r"[1-9][0-9]{0,9}", text):
            label = "ONE" if text == "1" else "POSITIVE"
        result[key] = state or label or "UNREADABLE"
    state, text = read("/proc/self/status", 16 * 1024)
    for key, field, labels in (("parent_seccomp", "Seccomp", {"0": "ZERO", "1": "STRICT", "2": "FILTER"}),
                               ("parent_no_new_privs", "NoNewPrivs", {"0": "ZERO", "1": "ONE"})):
        values = [line.split(":", 1)[1].strip() for line in (text or "").splitlines()
                  if line.startswith(field + ":")]
        result[key] = state or (labels.get(values[0], "UNREADABLE") if len(values) == 1
                                else "ABSENT" if not values else "UNREADABLE")
    return closed_host_metadata(result)


def closed_direct_observation(value):
    fields = {"version", "source", "native_exit", "native_diagnostic", "termination"}
    if (type(value) is not dict or set(value) != fields or type(value["version"]) is not int
            or value["version"] != 1 or value["source"] != "CHROMIUM_DIRECT"):
        raise Rejected("INPUT_INVALID")
    for key, labels in (("native_exit", {"ABSENT", "ZERO", "NONZERO", "SIGNAL"}),
                        ("native_diagnostic", BROWSER_DIAGNOSTICS - {"K6_ZERO_EXIT_NO_SANDBOX_WITNESS"}),
                        ("termination", {"EXITED", "NOT_OBSERVED", "CHILD_TIMEOUT", "INTERRUPTED"})):
        if type(value[key]) is not str or value[key] not in labels:
            raise Rejected("INPUT_INVALID")
    return dict(value)


def closed_direct_stop(value):
    # Separate/versioned: the existing k6 receipt and direct observation stay exact.
    if (type(value) is not dict or set(value) != {"version", "path", "signal"}
            or type(value["version"]) is not int or value["version"] != 1
            or type(value["path"]) is not str or value["path"] not in CHILD_STOP_PATHS
            or type(value["signal"]) is not str or value["signal"] not in LINUX_SIGNAL_LABELS):
        raise Rejected("INPUT_INVALID")
    return dict(value)


DIRECT_TEXT_FAMILIES = frozenset({"CHROMIUM_FATAL_REPORTED", "ASSERTION_FAILURE_REPORTED", "OTHER_TEXT", "NO_TEXT", "MULTIPLE"})
# Chromium156 logging.cc:899-970: numeric log items, then FATAL:file:line] body.
# Full LF-terminated lines only; arbitrary log prefixes/other layouts stay OTHER.
DIRECT_FATAL_FORMAT = re.compile(
    r"\[(?:[0-9]+:){0,2}(?:[0-9]{4}/[0-9]{6}\.[0-9]{6}:)?"
    r"(?:[0-9]+:)?FATAL:[-A-Za-z0-9_./+]+:[0-9]+\] [^\r\n]+\n")
# glibc2.35 assert/assert.c:57-61,101: optional program/function, file:line.
DIRECT_ASSERT_HEADER = re.compile(r"(?:[-A-Za-z0-9_.+]+: )?[-A-Za-z0-9_./+]+:[0-9]+: (?:[^\r\n]+: )?")


def closed_direct_text(value):
    if (type(value) is not dict or set(value) != {"version", "stdout", "stderr", "family"}
            or type(value["version"]) is not int or value["version"] != 1
            or any(type(value[key]) is not str or value[key] not in {"EMPTY", "PRESENT"} for key in ("stdout", "stderr"))
            or type(value["family"]) is not str or value["family"] not in DIRECT_TEXT_FAMILIES
            or (value["family"] == "NO_TEXT") != (value["stdout"] == value["stderr"] == "EMPTY")):
        raise Rejected("INPUT_INVALID")
    return dict(value)


def direct_text_observation(stdout, stderr):
    # Check BOTH entire captured buffers before any format recognition. Never
    # concatenate streams, capture groups, or return any native text/site/data.
    for raw in (stdout, stderr):
        if type(raw) is not bytes:
            raise Rejected("INPUT_INVALID")
        confidential(raw)
    families = set()
    suffix = "' failed.\n"
    for raw in (stdout, stderr):
        for line in raw.decode("utf-8").splitlines(keepends=True):
            if DIRECT_FATAL_FORMAT.fullmatch(line):
                families.add("CHROMIUM_FATAL_REPORTED")
            header, marker, tail = line.partition("Assertion `")
            if marker and tail.endswith(suffix) and tail[:-len(suffix)] and DIRECT_ASSERT_HEADER.fullmatch(header):
                families.add("ASSERTION_FAILURE_REPORTED")
    family = ("MULTIPLE" if len(families) > 1 else next(iter(families)) if families else
              "OTHER_TEXT" if stdout or stderr else "NO_TEXT")
    return closed_direct_text({"version": 1, "stdout": "PRESENT" if stdout else "EMPTY",
                               "stderr": "PRESENT" if stderr else "EMPTY", "family": family})


def _private_site_candidate(child, *, cleanup, diagnostic):
    # This tuple never enters a diagnostic/publisher/callback. The source is data,
    # not a local path to open, and is only passed to the owned private writer.
    if (cleanup is not True or child.cleanup is not True or child.output_overflow is not False
            or child.stop_path != "NATIVE_OBSERVED_BEFORE_CLEANUP" or type(child.code) is not int
            or diagnostic not in {"SANDBOX_UNPROVEN", "TOOL_FAILED", "PREREQUISITE_UNAVAILABLE"}
            or type(child.owned) is not dict or not child.owned or type(child.recorded) is not set):
        return None
    uid = os.getuid()
    for pid, identity in child.owned.items():
        if (type(identity) is not dict or type(pid) is not int or pid <= 0 or identity.get("pid") != pid
                or type(identity.get("uid")) is not int or identity["uid"] != uid
                or type(identity.get("start")) is not int or identity["start"] <= 0
                or (pid, identity["start"], uid) not in child.recorded):
            return None
    try:
        if direct_text_observation(child.stdout, child.stderr)["family"] != "CHROMIUM_FATAL_REPORTED":
            return None
        candidates = [line for raw in (child.stdout, child.stderr)
                      for line in raw.decode("utf-8").splitlines(keepends=True) if DIRECT_FATAL_FORMAT.fullmatch(line)]
        if len(candidates) != 1:
            return None
        header = candidates[0].split("] ", 1)[0]
        site = header.rpartition("FATAL:")[2]
        relative, _separator, number = site.rpartition(":")
        if not re.fullmatch(r"[1-9][0-9]{0,6}", number):
            return None
        return site_channel.source_pair(relative, int(number))
    except Rejected:
        return None


def direct_chromium(output, tools, *, private_site=None):
    """One explicit parent diagnostic; even exit0 + witness NEVER qualifies.

    Child retains its normal sandbox/privacy/cleanup gates, including on failure.
    --dump-dom is only a bounded exit mechanism for about:blank, not an oracle.
    """
    profile = output / "private/chromium-profile"
    try:
        profile.mkdir(mode=0o700)  # exclusive, never reuse a previous browser profile
    except FileExistsError:
        raise Rejected("PATH_INVALID") from None
    verify_tools(tools, ("k6", "chromium"))
    child = Child()
    argv = [str(tools["chromium"]), "--headless", "--dump-dom", "--timeout=5000",
            "--disable-background-networking", "--disable-component-update", "--no-first-run",
            "--no-default-browser-check", "--disable-sync", "--metrics-recording-only",
            "--user-data-dir=" + str(profile), "about:blank"]
    env = child_environment(output)
    env["K6_BROWSER_EXECUTABLE_PATH"] = str(tools["chromium"])
    failure = Rejected("PREREQUISITE_UNAVAILABLE", True)
    try:
        code = child.execute(argv, output=output, env=env, timeout=15, browser=True)
        if code != 0:
            failure = Rejected("TOOL_FAILED")
    except BaseException as error:
        failure = error if isinstance(error, Rejected) else Rejected(
            "INTERRUPTED" if isinstance(error, KeyboardInterrupt) else "INTERNAL_ERROR")
    termination = (failure.code if failure.code in {"CHILD_TIMEOUT", "INTERRUPTED"}
                   else "NOT_OBSERVED" if child.code is None else "EXITED")
    # A reaped handle without any contemporaneous PID/UID/start observation is
    # not identity-based cleanup evidence, even for a very early native exit.
    cleanup = (child.cleanup is True and type(child.owned) is dict and bool(child.owned)
               and failure.code != "CLEANUP_FAILED")
    if not cleanup:
        failure = Rejected("CLEANUP_FAILED")
    observation = None
    try:
        # Cleanup may supersede OUTPUT_LIMIT after Child truncates its buffers.
        # A captured prefix is not complete privacy evidence and is not classified.
        observation = None if child.output_overflow else closed_direct_observation({
            "version": 1, "source": "CHROMIUM_DIRECT", "native_exit": native_exit_category(child.code),
            # No k6 exit table or k6 zero-exit label for the native Chromium process.
            "native_diagnostic": browser_diagnostic(child.stdout, child.stderr, None),
            "termination": termination})
    except Rejected:
        failure = Rejected("INTERNAL_ERROR")
    try:
        confidential(child.stdout)
        confidential(child.stderr)
    except Rejected as error:
        failure = error
        # As for K6, a closed known lexical label may survive a privacy refusal
        # (e.g. a documentation URL in a FATAL). Never retain arbitrary text or
        # categories from sentinel/invalid/over-limit streams. Privacy stays FAIL.
        if (observation is None or observation["native_diagnostic"] in {
                "NATIVE_CAUSE_UNCLASSIFIED", "NATIVE_DIAGNOSTIC_WITHHELD"}):
            observation = None
    if failure.code == "OUTPUT_LIMIT":
        observation = None
    if observation is not None:
        failure.direct_observation = observation
    # Observability only: malformed/withheld facets must not replace any existing
    # privacy, overflow, cleanup, guard or native outcome.
    if child.output_overflow is False and failure.code not in {"PRIVACY_FAILED", "OUTPUT_LIMIT"}:
        try:
            stop = closed_direct_stop({"version": 1, "path": getattr(child, "stop_path", None),
                                       "signal": linux_signal_category(child.code)})
            if stop["path"] != "OUTPUT_LIMIT":
                failure.direct_stop = stop
        except Rejected:
            pass
    if (child.output_overflow is False and getattr(child, "stop_path", None) != "OUTPUT_LIMIT"
            and failure.code not in {"PRIVACY_FAILED", "OUTPUT_LIMIT"}):
        try:
            failure.direct_text = direct_text_observation(child.stdout, child.stderr)
        except Rejected:
            pass  # withheld observation must not change any existing outcome
    failure.cleanup = "PASS" if cleanup else "FAIL"
    if private_site is not None:
        try:
            candidate = _private_site_candidate(child, cleanup=cleanup, diagnostic=failure.code)
            if candidate is not None:
                private_site.record(*candidate)
        except BaseException as error:
            private_site.status = "INTERRUPTED" if isinstance(error, KeyboardInterrupt) else "WRITE_FAILED"
            try:
                if not private_site._remove():
                    private_site.status = "RETENTION_UNPROVEN"
            except BaseException:
                private_site.status = "RETENTION_UNPROVEN"
    raise failure


def scenario_run(phase, config, output, tools, *, negative=None):
    # Internal arguments are only supplied by selftest; no CLI/env override.
    run_id = uuid.uuid4().hex
    summary = output / "private/summary.json"
    if summary.exists():
        raise Rejected("SUMMARY_INVALID")
    if phase == "HTTP":
        live_target(config)
    verify_tools(tools, ("k6", "chromium") if phase == "Chromium" else ("k6",))
    child = Child()
    argv = k6_argv(tools["k6"], output, "run")
    for key, value in {"K6_RUN_ID": run_id, "K6_SUMMARY_PATH": str(summary),
                       **({"K6_ORIGIN": config["origin"]} if phase == "HTTP" else {}),
                       **({"K6_INTERNAL_NEGATIVE": negative} if negative else {})}.items():
        argv += ["--env", key + "=" + value]
    argv.append(str(SCENARIOS[phase]))
    env = child_environment(output)
    if phase == "Chromium":
        env["K6_BROWSER_EXECUTABLE_PATH"] = str(tools["chromium"])
    failure = None
    try:
        code = child.execute(argv, output=output, env=env, timeout=30, browser=phase == "Chromium",
                             interrupt_after=0.2 if negative == "interrupt" else None)
        if code != 0 and negative not in {"false-check", "absent-case"}:
            raise Rejected("TOOL_FAILED")
        # k6 creates this in our fresh 0700 directory; seal only this owned file.
        if summary.is_symlink() or not summary.is_file() or summary.stat().st_size > ARTIFACT_LIMIT:
            raise Rejected("SUMMARY_INVALID")
        summary.chmod(0o400)
        raw = private_read(summary)
        confidential(raw)
        parsed = strict_json(raw)
        if negative == "absent-case":
            if (parsed.get("run_id") != run_id or parsed.get("expected") != 1
                    or parsed.get("cases") != ["iam-health"] or parsed.get("iterations") != 1
                    or parsed.get("started") not in {None, 0} or parsed.get("completed") not in {None, 0}
                    or parsed.get("requests") not in {None, 0} or parsed.get("checks") != {}):
                raise Rejected("SELFTEST_FAILED")
            raise Rejected("SUMMARY_INVALID")
        if negative == "false-check":
            verify_false_check(parsed, run_id, code)
            raise Rejected("TOOL_FAILED")
        verify_summary(parsed, run_id, phase)
    except BaseException as error:
        failure = error
    finally:
        if phase == "HTTP":
            # Required even for native failure or interruption; never cached.
            try:
                live_target(config)
            except Rejected as error:
                error.cleanup = "PASS" if child.cleanup else "FAIL"
                raise
    if failure:
        if phase == "Chromium":
            if not isinstance(failure, Rejected):
                failure = Rejected("INTERNAL_ERROR")
            try:
                state, counters = browser_summary_observation(summary, run_id)
                if failure.code == "SANDBOX_UNPROVEN" and state == "VALID" and not child.output_overflow:
                    # Recheck complete captured streams before admitting the private
                    # parent signal; a known refusal is not a privacy exemption.
                    confidential(child.stdout)
                    confidential(child.stderr)
                    failure.browser_confidentiality = "PASS"
                failure.browser_observation = closed_browser_observation({
                    "native_exit": native_exit_category(child.code), "summary": state, "counters": counters,
                    "renderer_observed": child.renderer_observed,
                    "namespace_reads": {key: sorted(child.namespace_reads[key]) or ["NOT_OBSERVED"]
                                        for key in ("user", "pid")}})
            except Rejected as error:
                if error.code == "PRIVACY_FAILED":
                    error.browser_diagnostic = failure.browser_diagnostic
                    failure = error
                # Bad diagnostic input is withheld, never an alternate PASS.
            except (AttributeError, KeyError, TypeError, ValueError):
                # A malformed supervisor fixture must not replace the original
                # failure/cleanup verdict or escape through the result surface.
                pass
        if isinstance(failure, Rejected):
            failure.cleanup = "PASS" if child.cleanup else "FAIL"
            if phase == "Chromium":
                try:
                    failure.k6_exit = k6_exit_category(child.code)
                except Rejected:
                    pass  # malformed native result is withheld, never projected
        raise failure
    return {"run_id": run_id, "cleanup": "PASS", "sandbox": "PASS" if phase == "Chromium" else "NOT_APPLICABLE",
            **({"sandbox_proof": child.sandbox_proof, "k6_exit": k6_exit_category(code)} if phase == "Chromium" else {})}


def publish(output, result, provenance=None):
    values = {"result.json": result}
    if provenance is not None:
        values["provenance.json"] = provenance
    encoded = {}
    # Validate EVERY byte before writing ANY publishable artifact.
    for name, value in values.items():
        raw = json.dumps(value, sort_keys=True, allow_nan=False, separators=(",", ":")).encode("utf-8")
        if len(raw) > ARTIFACT_LIMIT:
            raise Rejected("OUTPUT_LIMIT")
        confidential(raw)
        encoded[name] = raw
    if sum(map(len, encoded.values())) > 256 * 1024:
        raise Rejected("OUTPUT_LIMIT")
    for name, raw in encoded.items():
        path = output / name
        with path.open("xb") as stream:
            stream.write(raw)
        path.chmod(0o400)


def observed_environment():
    observed = {}
    for name, value in {"kernel": platform.release(), "architecture": platform.machine(),
                         "runner_image": os.environ.get("ImageOS", "not-provided"),
                         "runner_image_version": os.environ.get("ImageVersion", "not-provided")}.items():
        observed[name] = value if re.fullmatch(r"[A-Za-z0-9._+-]{1,160}", value) else "not-projectable"
    return observed


def comparison_metadata(tools, versions):
    """Closed facts from the already admitted host/pins; never a platform PASS."""
    lock = strict_json((HERE / "config/tools.lock.json").read_bytes())
    names = ("k6", "chromium")
    if (type(versions) is not dict or set(versions) != set(names)
            or any(type(versions[name]) is not str or versions[name] != lock[name]["version"] for name in names)):
        raise Rejected("TOOL_VERSION")
    hashes = {name: digest(tools[name]) for name in names}
    if any(type(value) is not str or not re.fullmatch(r"[a-f0-9]{64}", value) for value in hashes.values()):
        raise Rejected("TOOL_UNPINNED", True)
    uid = os.getuid()
    if type(uid) is not int or not 0 < uid < 2**32:
        raise Rejected("PLATFORM_UNQUALIFIED", True)
    value = {"observed": {**observed_environment(), "uid": uid},
             "versions": {name: versions[name] for name in names}, "tools": hashes}
    confidential(json.dumps(value, sort_keys=True).encode())
    return value


def admissible_browser_refusal(value, metadata, host):
    """A fresh private parent signal, not runtime authority or sandbox proof."""
    fixed = {"version": 1, "HTTP": "INCONCLUSIVE", "Chromium": "INCONCLUSIVE", "cleanup": "PASS",
             "confidentiality": "PASS", "cleanup_scope": "LAUNCHER_CHILDREN_ONLY", "parent_runtime_cleanup": "INCONCLUSIVE",
             "browser_source": "K6", "tools": "PASS", "diagnostic": "SANDBOX_UNPROVEN"}
    fields = set(fixed) | {"run_id", "started_unix_ms", "finished_unix_ms", "comparison", "browser_host",
                           "browser_native_diagnostic", "browser_observation", "k6_exit"}
    try:
        if (type(value) is not dict or set(value) != fields or type(value["version"]) is not int
                or any(value[key] != expected for key, expected in fixed.items())
                or type(value["run_id"]) is not str or not re.fullmatch(r"[a-f0-9]{32}", value["run_id"])):
            return False
        start, finish, now = value["started_unix_ms"], value["finished_unix_ms"], int(time.time() * 1000)
        if (type(start) is not int or type(finish) is not int or not 0 <= finish - start <= 120000
                or not 0 <= now - finish <= 120000):
            return False
        expected = {"platform": "ubuntu-22.04-linux-amd64-diagnostic-only",
            "qualification": "NOT_EQUIVALENT_TO_UBUNTU_24_04", "reference_run": "40973768642",
            "browser_launch": "ATTEMPTED", "case": "NOT_PROVEN", "sandbox_witness": "NOT_PROVEN", **metadata}
        if value["comparison"] != expected or closed_host_metadata(value["browser_host"]) != host:
            return False
        if (value["browser_native_diagnostic"] not in BROWSER_DIAGNOSTICS
                or value["k6_exit"] not in K6_EXIT_CATEGORIES):
            return False
        observation = closed_browser_observation(value["browser_observation"])
        native = {"ABSENT": "ABSENT", "ZERO": "ZERO", "SIGNAL": "SIGNAL"}.get(value["k6_exit"], "NONZERO")
        counters = observation["counters"]
        return (native != "ABSENT" and observation["native_exit"] == native and observation["summary"] == "VALID"
                and counters == {"expected": "ONE", "started": counters["started"], "completed": "ZERO",
                                 "iterations": "ONE", "requests": "ZERO"} and counters["started"] in {"ZERO", "ONE"})
    except (Rejected, KeyError, TypeError, ValueError):
        return False


def admit_browser_refusal(path, output_path, tools, lock):
    # private_read supplies the existing path/owner/mode/size guards. Same private
    # campaign parent and fresh matching metadata; never an old runtime lease.
    receipt = Path(path)
    value = strict_json(private_read(receipt))
    metadata = comparison_metadata(tools, {name: lock[name]["version"] for name in ("k6", "chromium")})
    if (receipt != receipt.resolve(strict=True) or receipt.name != "comparison-result.json"
            or receipt.parent.name != "private" or Path(output_path).parent.resolve(strict=True) != receipt.parent.parent.parent
            or not admissible_browser_refusal(value, metadata, closed_host_metadata(host_metadata()))):
        raise Rejected("INPUT_INVALID")
    # Exclusive, private, never removed/replaced: even a failed followup cannot retry.
    try:
        marker = receipt.parent / "direct-attempted.json"
        with marker.open("xb") as stream:
            stream.write(b"{}")
        marker.chmod(0o400)
    except OSError:
        raise Rejected("INPUT_INVALID") from None
    return value["run_id"]


def provenance(tools, lock, versions, config_path=None):
    # Only stable labels and hashes. No URLs, full environment, cmdlines or paths.
    verify_tools(tools, tuple(tools))
    sources = [REPO / "docs/contracts/k6-functional-linux.md", HERE / "run-linux.py",
               HERE / "lib/safety.py", HERE / "lib/summary.js", *SCENARIOS.values(), HERE / "config/tools.lock.json"]
    head = subprocess.run(["/usr/bin/git", "-C", str(REPO), "rev-parse", "HEAD"],
                          capture_output=True, timeout=5, check=True,
                          env={"PATH": "/usr/bin:/bin", "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null"}).stdout.strip().decode("ascii")
    if not re.fullmatch(r"[a-f0-9]{40}", head):
        raise Rejected("INPUT_INVALID")
    return {"version": 1, "head": head, "platform": "ubuntu-24.04-linux-amd64", "observed": observed_environment(), "versions": versions,
            "tools": {name: digest(path) for name, path in tools.items()},
            "sources": {str(path.relative_to(REPO)): digest(path) for path in sources},
            "config_sha256": digest(Path(config_path)) if config_path else None,
            "chromium_scope": "PREREQUISITE_ONLY", "http_scope": "INCONCLUSIVE"}


def negative_main(name, config_path, output_path):
    """Private selftest driver, deliberately NOT a run CLI/environment override."""
    output = None
    try:
        linux_platform()
        if name not in {"false-check", "absent-case", "missing-summary", "interrupt", "missing-input", "refused-target"}:
            raise Rejected("INPUT_INVALID")
        config = configuration(config_path)
        output = new_output(output_path)
        live_target(config)
        tools, lock = pinned_tools()
        version_checks(tools, lock, output)
        scenario_run("HTTP", config, output, tools, negative=name)
        return 0
    except Rejected as error:
        # A closed result from the actual negative launcher, with nonzero exit.
        print(json.dumps({"version": 1, "diagnostic": error.code, "non_success": True, "cleanup": error.cleanup}))
        return 2 if error.inconclusive else 1
    except BaseException:
        print('{"version":1,"diagnostic":"INTERNAL_ERROR","non_success":true}')
        return 1


def selftest(config, output, tools, config_path):
    # Fails closed before any child if no live binding. Never substitute a mock.
    live_target(config)
    expected = {"false-check": "TOOL_FAILED", "absent-case": "SUMMARY_INVALID",
                "missing-summary": "SUMMARY_INVALID", "interrupt": "INTERRUPTED"}
    outcomes = {}
    expected.update({"missing-input": "INPUT_INVALID", "refused-target": "INPUT_INVALID"})
    # A fixed child driver calls the same launcher; no shell, mock server or
    # user-selectable script. Each negative has a real observable nonzero exit.
    driver = ("import importlib.util,sys;from pathlib import Path;"
              "p=Path(sys.argv[1]);s=importlib.util.spec_from_file_location('negative_runner',p);"
              "m=importlib.util.module_from_spec(s);s.loader.exec_module(m);"
              "sys.exit(m.negative_main(sys.argv[2],sys.argv[3],sys.argv[4]))")
    for name, code in expected.items():
        child_config = str(config_path)
        if name in {"missing-input", "refused-target"}:
            value = {} if name == "missing-input" else {**config, "origin": "http://192.0.2.1"}
            path = output / "private" / (name + ".json")
            path.write_text(json.dumps(value), encoding="utf-8")
            path.chmod(0o400)
            child_config = str(path)
        supervisor_output = new_output(str(output / "private" / (name + "-supervisor")))
        env = child_environment(supervisor_output)
        # This Python driver is also a target guard parent, not the k6 child.
        # Keep the existing POSIX authority rooted at the same HOME as its parent.
        # Its own k6 subprocess still gets a fresh private HOME.
        env["HOME"] = str(Path.home())
        env["K6_TOOLS_DIR"] = os.environ["K6_TOOLS_DIR"]
        child = Child()
        exit_code = child.execute([sys.executable, "-B", "-c", driver, str(HERE / "run-linux.py"),
                                   name, child_config, str(output / "private" / (name + "-launch"))],
                                  output=supervisor_output, env=env, timeout=45)
        projection = strict_json(child.stdout)
        outcomes[name] = (exit_code != 0 and child.cleanup and projection == {
            "version": 1, "diagnostic": code, "non_success": True,
            "cleanup": "NOT_STARTED" if name in {"missing-input", "refused-target"} else "PASS"})
    if set(outcomes) != set(expected) or not all(outcomes.values()):
        raise Rejected("SELFTEST_FAILED")
    return outcomes


class ClosedParser(argparse.ArgumentParser):
    def error(self, message):
        # argparse ordinarily echoes arbitrary rejected input to stderr.
        raise Rejected("INPUT_INVALID")


def arguments(argv):
    parser = ClosedParser(description="Mission1 Linux prerequisites; HTTP gate remains fail-closed.")
    commands = parser.add_subparsers(dest="command", required=True, parser_class=ClosedParser)
    for name in ("validate", "run", "selftest", "browser", "tools", "diagnose-chromium"):
        command = commands.add_parser(name)
        if name in {"validate", "run", "selftest"}:
            command.add_argument("--config", required=True)
        if name != "validate":
            command.add_argument("--output", required=True)
        if name in {"browser", "diagnose-chromium"}:
            command.add_argument("--compare-ubuntu-2204", action="store_true",
                                 help="Temporary nonqualifying comparison only; never publishes and always exits nonzero.")
        if name == "diagnose-chromium":
            command.add_argument("--browser-result", help="Private current comparison refusal, required only for the22 followup.")
            command.add_argument("--private-site", action="store_true",
                                 help="Local24 only; requires a live trusted parent session and private read appointment.")
    return parser.parse_args(argv)


def main(argv=None, *, private_site=None):
    output = None
    phase = None
    mode = None
    comparison = False
    result = {"version": 1, "HTTP": "INCONCLUSIVE", "Chromium": "INCONCLUSIVE",
              "cleanup": "NOT_STARTED", "confidentiality": "PASS", "run_id": uuid.uuid4().hex,
              "started_unix_ms": int(time.time() * 1000), "cleanup_scope": "LAUNCHER_CHILDREN_ONLY",
              "parent_runtime_cleanup": "INCONCLUSIVE"}
    try:
        args = arguments(argv)
        mode = args.command
        phase = {"run": "HTTP", "browser": "Chromium"}.get(mode)
        comparison = mode in {"browser", "diagnose-chromium"} and args.compare_ubuntu_2204
        private_requested = getattr(args, "private_site", False)
        if private_requested or private_site is not None:
            # Inspect unsanitized ambient CI markers before any child/environment
            # scrub, and never accept a manifest/bool as parent admission.
            site_channel.opt_in(private_requested, mode, comparison, private_site)
        if mode == "diagnose-chromium" and (comparison != (args.browser_result is not None) or args.browser_result == ""):
            raise Rejected("INPUT_INVALID")
        if comparison:
            linux_platform(comparison=True)
            result["comparison"] = {"platform": "ubuntu-22.04-linux-amd64-diagnostic-only",
                "qualification": "NOT_EQUIVALENT_TO_UBUNTU_24_04", "reference_run": "40973768642",
                "browser_launch": "NOT_STARTED", "case": "NOT_PROVEN", "sandbox_witness": "NOT_PROVEN"}
        else:
            linux_platform()
        if private_requested:
            private_site.authorize(result["run_id"])
        config = configuration(args.config) if args.command in {"validate", "run", "selftest"} else None
        if args.command == "validate":
            result["diagnostic"] = "OFFLINE_ADMISSIBLE_NOT_AUTHORIZED"
        else:
            output = new_output(args.output)
            if args.command in {"run", "selftest"}:
                live_target(config)
            tools, lock = pinned_tools(browser=args.command in {"browser", "tools", "diagnose-chromium"})
            if comparison and mode == "diagnose-chromium":
                result["comparison"]["parent_browser_run_id"] = admit_browser_refusal(args.browser_result, args.output, tools, lock)
            versions = version_checks(tools, lock, output)
            if mode in {"browser", "diagnose-chromium"}:
                result["browser_source"] = "K6" if mode == "browser" else "CHROMIUM_DIRECT"
                result["browser_host"] = closed_host_metadata(host_metadata())
            if comparison and mode == "browser":
                # Same versions/inspect/identity/environment/scenario/Child as V1.
                # No direct launch, retry, HTTP target, publication or qualification.
                result["comparison"].update(comparison_metadata(tools, versions))
                for inspected_phase in SCENARIOS:
                    inspect_scenario(tools, output, inspected_phase)
                result["tools"] = "PASS"
                result["comparison"]["browser_launch"] = "ATTEMPTED"
                completed = scenario_run("Chromium", None, output, tools)
                result["run_id"] = completed["run_id"]
                result["cleanup"] = completed["cleanup"]
                result["k6_exit"] = completed["k6_exit"]
                result["comparison"]["case"] = "COMPLETE"
                result["comparison"]["sandbox_witness"] = "OBSERVED"
                result["diagnostic"] = "PREREQUISITE_UNAVAILABLE"
                result["finished_unix_ms"] = int(time.time() * 1000)
                raw = json.dumps(result, sort_keys=True, allow_nan=False).encode()
                if len(raw) > ARTIFACT_LIMIT:
                    raise Rejected("OUTPUT_LIMIT")
                confidential(raw)
                print(raw.decode())
                return 2
            if mode == "diagnose-chromium":
                if private_requested:
                    direct_chromium(output, tools, private_site=private_site)
                else:
                    direct_chromium(output, tools)  # always nonzero; no artifacts/phase PASS
                raise Rejected("INTERNAL_ERROR")  # never fall through to HTTP if its contract breaks
            if args.command == "tools":
                for phase in SCENARIOS:
                    inspect_scenario(tools, output, phase)
                result["tools"] = "PASS"
                result["diagnostic"] = "PREREQUISITES_ONLY_HTTP_NOT_RUN"
                result["cleanup"] = "PASS"
            elif args.command == "selftest":
                result["selftest"] = selftest(config, output, tools, args.config)
                result["cleanup"] = "PASS"
            else:
                phase = "Chromium" if args.command == "browser" else "HTTP"
                result.update(scenario_run(phase, config, output, tools))
                result[phase] = "PASS"
            result["finished_unix_ms"] = int(time.time() * 1000)
            publish(output, result, provenance(tools, lock, versions, getattr(args, "config", None)))
        raw = json.dumps(result, sort_keys=True).encode()
        confidential(raw)
        print(raw.decode())
        return 0
    except BaseException as error:
        if isinstance(error, SystemExit) and error.code == 0:
            return 0
        failure = error if isinstance(error, Rejected) else Rejected("INTERNAL_ERROR")
        result["diagnostic"] = failure.code
        result["cleanup"] = failure.cleanup
        native_reason = failure.browser_diagnostic
        if mode == "browser" and type(getattr(failure, "k6_exit", None)) is str and failure.k6_exit in K6_EXIT_CATEGORIES:
            result["k6_exit"] = failure.k6_exit
        if mode == "browser" and type(native_reason) is str and native_reason in BROWSER_DIAGNOSTICS:
            result["browser_native_diagnostic"] = native_reason
        if mode == "browser" and failure.browser_observation is not None:
            try:
                result["browser_observation"] = closed_browser_observation(failure.browser_observation)
            except Rejected:
                pass
        if mode == "diagnose-chromium":
            try:
                result["direct_observation"] = closed_direct_observation(failure.direct_observation)
            except (AttributeError, Rejected):
                pass
            try:
                result["direct_stop"] = closed_direct_stop(getattr(failure, "direct_stop", None))
            except Rejected:
                pass
            try:
                result["direct_text"] = closed_direct_text(getattr(failure, "direct_text", None))
            except Rejected:
                pass
            # The direct path is diagnostic only, including native exit0/witness.
            result["Chromium"] = "INCONCLUSIVE"
        if phase:
            result[phase] = "INCONCLUSIVE" if failure.inconclusive else "FAIL"
        if comparison:
            result["HTTP"] = result["Chromium"] = "INCONCLUSIVE"
        if mode in {"selftest", "tools"}:
            result[mode] = "INCONCLUSIVE" if failure.inconclusive else "FAIL"
        result["confidentiality"] = ("FAIL" if failure.code == "PRIVACY_FAILED" else
            "PASS" if mode == "diagnose-chromium" and "direct_observation" in result else "NOT_QUALIFIED")
        if comparison and mode == "browser":
            result["finished_unix_ms"] = int(time.time() * 1000)
            if failure.code == "SANDBOX_UNPROVEN" and getattr(failure, "browser_confidentiality", None) == "PASS":
                result["confidentiality"] = "PASS"
        # No file publication on a failed confidentiality or incomplete cleanup.
        # Final one-line diagnostic is constructed from closed labels, not errors.
        raw = json.dumps(result, sort_keys=True).encode()
        confidential(raw)
        if comparison and mode == "browser" and len(raw) <= ARTIFACT_LIMIT and "comparison" in result:
            metadata = {key: result["comparison"].get(key) for key in ("observed", "versions", "tools")}
            if admissible_browser_refusal(result, metadata, result.get("browser_host")):
                try:
                    receipt = output / "private/comparison-result.json"
                    with receipt.open("xb") as stream:
                        stream.write(raw)
                    receipt.chmod(0o400)
                except OSError:
                    # No signal means no direct launch. Never publish or retry.
                    pass
        print(raw.decode())
        return 2 if failure.inconclusive else 1


if __name__ == "__main__":
    sys.exit(main())
