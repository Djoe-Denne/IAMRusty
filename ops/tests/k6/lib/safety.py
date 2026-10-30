"""Small Linux child supervisor. No Docker, shell, forced kill or raw logging."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import signal
import stat
import subprocess
import time
import uuid

LIMIT = 64 * 1024
ARTIFACT_LIMIT = 32 * 1024
CODES = frozenset("INPUT_INVALID PATH_INVALID PRIVACY_FAILED SUMMARY_INVALID TOOL_UNPINNED "
                  "TOOL_VERSION TOOL_FAILED OUTPUT_LIMIT CHILD_TIMEOUT INTERRUPTED CLEANUP_FAILED "
                  "SANDBOX_UNPROVEN TARGET_UNAVAILABLE TARGET_BINDING_UNQUALIFIED PLATFORM_UNQUALIFIED "
                  "INTERNAL_ERROR SELFTEST_FAILED PREREQUISITE_UNAVAILABLE".split())
BROWSER_DIAGNOSTICS = frozenset("NO_USABLE_SANDBOX_REPORTED NAMESPACE_PERMISSION_DENIED_REPORTED "
    "SUID_HELPER_CONFIGURATION_REJECTED SUID_HELPER_MISSING_REPORTED ROOT_STARTUP_REFUSAL_REPORTED "
    "BROWSER_STARTUP_ENDED_REPORTED MULTIPLE_NATIVE_CAUSES_REPORTED K6_ZERO_EXIT_NO_SANDBOX_WITNESS "
    "NATIVE_CAUSE_UNCLASSIFIED NATIVE_DIAGNOSTIC_WITHHELD".split())
CASE_COUNTERS = ("expected", "started", "completed", "iterations", "requests")
NAMESPACE_READS = frozenset("READABLE DENIED DISAPPEARED OTHER NOT_OBSERVED".split())
CHILD_STOP_PATHS = frozenset("NOT_OBSERVED NATIVE_OBSERVED_BEFORE_CLEANUP GUARD_REJECTED TIMEOUT INTERRUPTED OUTPUT_LIMIT ERROR".split())
# Linux x86 UAPI v6.8 arch/x86/include/uapi/asm/signal.h, not host/Windows enums.
# Canonical names only; aliases and realtime signals are not projected.
LINUX_SIGNALS = {1: "SIGHUP", 2: "SIGINT", 3: "SIGQUIT", 4: "SIGILL", 5: "SIGTRAP", 6: "SIGABRT",
    7: "SIGBUS", 8: "SIGFPE", 9: "SIGKILL", 10: "SIGUSR1", 11: "SIGSEGV", 12: "SIGUSR2", 13: "SIGPIPE",
    14: "SIGALRM", 15: "SIGTERM", 16: "SIGSTKFLT", 17: "SIGCHLD", 18: "SIGCONT", 19: "SIGSTOP",
    20: "SIGTSTP", 21: "SIGTTIN", 22: "SIGTTOU", 23: "SIGURG", 24: "SIGXCPU", 25: "SIGXFSZ",
    26: "SIGVTALRM", 27: "SIGPROF", 28: "SIGWINCH", 29: "SIGIO", 30: "SIGPWR", 31: "SIGSYS"}
LINUX_SIGNAL_LABELS = frozenset(LINUX_SIGNALS.values()) | {"ABSENT", "NO_SIGNAL", "UNKNOWN_SIGNAL"}


class Rejected(Exception):
    def __init__(self, code, inconclusive=False):
        if code not in CODES:
            raise ValueError("closed diagnostic required")
        self.code, self.inconclusive = code, inconclusive
        self.cleanup = "NOT_STARTED"
        self.browser_diagnostic = None
        self.browser_observation = None
        super().__init__(code)


def native_exit_category(code):
    if code is None:
        return "ABSENT"
    if type(code) is not int:
        raise Rejected("INPUT_INVALID")
    return "ZERO" if code == 0 else "SIGNAL" if code < 0 else "NONZERO"


def linux_signal_category(code):
    if code is None:
        return "ABSENT"
    if type(code) is not int:
        raise Rejected("INPUT_INVALID")
    return "NO_SIGNAL" if code >= 0 else LINUX_SIGNALS.get(-code, "UNKNOWN_SIGNAL")


def closed_browser_observation(value):
    """Validate and copy a bounded projection, including exception/fixture input."""
    fields = {"native_exit", "summary", "counters", "renderer_observed", "namespace_reads"}
    if type(value) is not dict or set(value) != fields:
        raise Rejected("INPUT_INVALID")
    for key, labels in (("native_exit", {"ABSENT", "ZERO", "NONZERO", "SIGNAL"}),
                        ("summary", {"ABSENT", "INVALID", "VALID"})):
        if type(value[key]) is not str or value[key] not in labels:
            raise Rejected("INPUT_INVALID")
    counters, namespaces = value["counters"], value["namespace_reads"]
    if (type(value["renderer_observed"]) is not bool or type(counters) is not dict
            or set(counters) != set(CASE_COUNTERS) or type(namespaces) is not dict
            or set(namespaces) != {"user", "pid"}):
        raise Rejected("INPUT_INVALID")
    for label in counters.values():
        if type(label) is not str or label not in {"ABSENT", "ZERO", "ONE", "OTHER"}:
            raise Rejected("INPUT_INVALID")
        if value["summary"] != "VALID" and label != "ABSENT":
            raise Rejected("INPUT_INVALID")
    for labels in namespaces.values():
        if (type(labels) is not list or not 1 <= len(labels) <= len(NAMESPACE_READS)
                or any(type(label) is not str or label not in NAMESPACE_READS for label in labels)
                or len(set(labels)) != len(labels)
                or ("NOT_OBSERVED" in labels and len(labels) != 1)
                or (not value["renderer_observed"] and labels != ["NOT_OBSERVED"])):
            raise Rejected("INPUT_INVALID")
    return {"native_exit": value["native_exit"], "summary": value["summary"],
            "counters": {key: counters[key] for key in CASE_COUNTERS},
            "renderer_observed": value["renderer_observed"],
            "namespace_reads": {key: sorted(namespaces[key]) for key in ("user", "pid")}}


def strict_json(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate")
            result[key] = value
        return result

    def constant(_):
        raise ValueError("nonfinite")

    try:
        return json.loads(raw.decode("utf-8"), object_pairs_hook=pairs, parse_constant=constant)
    except (ValueError, UnicodeError, RecursionError):
        raise Rejected("INPUT_INVALID") from None


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            value.update(block)
    return value.hexdigest()


def confidential(raw):
    # Deliberately conservative. Never print rejected bytes or a regex match.
    if len(raw) > LIMIT:
        raise Rejected("OUTPUT_LIMIT")
    try:
        text = raw.decode("utf-8")
    except UnicodeError:
        raise Rejected("PRIVACY_FAILED") from None
    patterns = [r"CONFIDENTIAL_SENTINEL", r"(?:https?|wss?)://", r"(?i)bearer\s+\S+",
                r"(?i)(?:authorization|cookie|password|credential|code|state|verifier|token)[\"']?\s*[=:]\s*[^\s,}]+",
                r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
                r"-----BEGIN [A-Z ]*PRIVATE KEY-----"]
    if any(re.search(pattern, text) for pattern in patterns):
        raise Rejected("PRIVACY_FAILED")


def browser_diagnostic(stdout, stderr, native_exit):
    """Lexical message category only, never sandbox proof or AppArmor attribution.

    Chromium 156.0.8078.73 messages and k6 v2.3.0 startup errors. Return constants
    only: never a match, path, URL, native suffix or suggested workaround.
    """
    if any(type(raw) is not bytes or len(raw) > LIMIT for raw in (stdout, stderr)):
        return "NATIVE_DIAGNOSTIC_WITHHELD"
    raw = stdout + b"\n" + stderr
    try:
        raw.decode("utf-8", errors="strict")
    except UnicodeError:
        return "NATIVE_DIAGNOSTIC_WITHHELD"
    if b"CONFIDENTIAL_SENTINEL" in raw:
        return "NATIVE_DIAGNOSTIC_WITHHELD"
    markers = {
        b"No usable sandbox!": "NO_USABLE_SANDBOX_REPORTED",
        b"The SUID sandbox helper binary was found, but is not configured correctly.": "SUID_HELPER_CONFIGURATION_REJECTED",
        b"The SUID sandbox helper binary is missing:": "SUID_HELPER_MISSING_REPORTED",
        b"Running as root without --no-sandbox is not supported.": "ROOT_STARTUP_REFUSAL_REPORTED",
    }
    causes = {label for marker, label in markers.items() if marker in raw}
    if re.search(rb"Failed to move to new namespace:[^\r\n]{0,256}but failed: errno = (?:Operation not permitted|Permission denied)\b", raw):
        causes.add("NAMESPACE_PERMISSION_DENIED_REPORTED")
    if len(causes) > 1:
        return "MULTIPLE_NATIVE_CAUSES_REPORTED"
    if causes:
        return causes.pop()
    if (b"browser process ended unexpectedly" in raw
            or b"browser process shutdown unexpectedly before establishing a connection:" in raw):
        return "BROWSER_STARTUP_ENDED_REPORTED"
    if type(native_exit) is int and native_exit == 0:
        return "K6_ZERO_EXIT_NO_SANDBOX_WITNESS"
    return "NATIVE_CAUSE_UNCLASSIFIED"


def outside_git(path):
    if any((parent / ".git").exists() for parent in (path, *path.parents)):
        raise Rejected("PATH_INVALID")


def name_chain(path):
    """Protect path-entry integrity separately from leaf confidentiality.

    Trust only root and the effective UID. System 0755 ancestors are fine;
    writable ancestors require sticky protection AND trusted ownership of every
    child entry (checked at the next component). No assumption of private /tmp.
    Directory timestamps are excluded: unrelated entries may change legitimately.
    """
    path = Path(path)
    trusted = {0, os.getuid()}
    identities = []
    try:
        if not path.is_absolute() or path != path.resolve(strict=True):
            raise Rejected("PATH_INVALID")
        for component in (*reversed(path.parents), path):
            item = component.stat(follow_symlinks=False)
            directory = stat.S_ISDIR(item.st_mode)
            if (item.st_uid not in trusted or stat.S_ISLNK(item.st_mode)
                    or (component != path and not directory)
                    or (directory and item.st_mode & 0o022 and not item.st_mode & stat.S_ISVTX)):
                raise Rejected("PATH_INVALID")
            identity = (str(component), item.st_dev, item.st_ino, item.st_uid, stat.S_IMODE(item.st_mode))
            if not directory:
                identity += (item.st_size, item.st_mtime_ns, item.st_ctime_ns)
            identities.append(identity)
    except (OSError, RuntimeError):
        raise Rejected("PATH_INVALID") from None
    return tuple(identities)


def private_path(value, directory=False, readonly=True):
    path = Path(value)
    if not path.is_absolute() or path != path.resolve(strict=True):
        raise Rejected("PATH_INVALID")
    outside_git(path)
    item = path.stat()
    if (item.st_uid != os.getuid() or item.st_mode & 0o077
            or not (stat.S_ISDIR(item.st_mode) if directory else stat.S_ISREG(item.st_mode))
            or (readonly and not directory and item.st_mode & 0o222)):
        raise Rejected("PATH_INVALID")
    if not directory:
        # A file also requires its immediate containing directory to be private.
        private_path(path.parent, directory=True)
    name_chain(path)
    return path


def private_read(path, limit=ARTIFACT_LIMIT):
    private_path(path)
    with Path(path).open("rb") as stream:
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise Rejected("OUTPUT_LIMIT")
    return raw


def new_output(value):
    path = Path(value)
    if not path.is_absolute() or path != path.resolve() or path.exists():
        raise Rejected("PATH_INVALID")
    private_path(path.parent, directory=True)
    parent_identity = name_chain(path.parent)
    outside_git(path)
    if name_chain(private_path(path.parent, directory=True)) != parent_identity:
        raise Rejected("PATH_INVALID")
    path.mkdir(mode=0o700)
    for name in ("home", "tmp", "private"):
        (path / name).mkdir(mode=0o700)
    (path / "private" / "empty.json").write_text("{}", encoding="utf-8")
    (path / "private" / "empty.json").chmod(0o400)
    return path


def child_environment(output, ambient=None):
    # ambient is intentionally unused: no K6/proxy/loader/cloud/target inheritance.
    return {"PATH": "/usr/bin:/bin", "HOME": str(output / "home"),
            "XDG_CONFIG_HOME": str(output / "home"), "TMPDIR": str(output / "tmp"),
            "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8", "TZ": "UTC",
            "K6_NO_USAGE_REPORT": "true", "K6_BROWSER_HEADLESS": "true",
            "K6_BROWSER_ARGS": "", "K6_BROWSER_DEBUG": "false",
            "K6_BROWSER_TIMEOUT": "10s"}


def proc(pid):
    """Private process identity; never export cmdline or arbitrary /proc content."""
    try:
        root = Path("/proc") / str(pid)
        raw = (root / "stat").read_text()
        fields = raw[raw.rfind(")") + 2:].split()
        status = dict(line.split(":", 1) for line in (root / "status").read_text().splitlines() if ":" in line)
        result = {"pid": pid, "start": int(fields[19]), "ppid": int(fields[1]),
                "pgrp": int(fields[2]), "session": int(fields[3]), "zombie": fields[0] == "Z",
                "uid": int(status["Uid"].split()[0]), "seccomp": status.get("Seccomp", "").strip(),
                "filters": int(status.get("Seccomp_filters", "0")),
                "no_new_privs": status.get("NoNewPrivs", "").strip(),
                "args": [], "userns": None, "pidns": None,
                "namespace_reads": {"user": "NOT_OBSERVED", "pid": "NOT_OBSERVED"}}
        # Zombies and nondumpable renderers may deny optional files: retain their
        # base identity for cleanup, but never count missing namespace data as proof.
        reading = None
        try:
            with (root / "cmdline").open("rb") as stream:
                raw_args = stream.read(32769)
            result["args"] = raw_args.split(b"\0") if len(raw_args) <= 32768 else []
            # Keep the existing read order/short-circuit, including foreign
            # processes visited by inventory. Do not add reads after a denial.
            for name in ("user", "pid"):
                reading = name
                result[name + "ns"] = os.readlink(root / "ns" / name)
                result["namespace_reads"][name] = "READABLE"
        except OSError as error:
            if reading is not None:
                result["namespace_reads"][reading] = ("DENIED" if isinstance(error, PermissionError)
                    else "DISAPPEARED" if isinstance(error, FileNotFoundError) else "OTHER")
        return result
    except (OSError, ValueError, KeyError):
        return None


def same(identity):
    current = proc(identity["pid"])
    if current is None and (Path("/proc") / str(identity["pid"])).exists():
        raise Rejected("CLEANUP_FAILED")
    return current if current and all(current[k] == identity[k] for k in ("pid", "start", "uid")) else None


def sandbox_witness(identity, parent):
    args = identity["args"]
    forbidden = (b"--no-sandbox", b"--disable-setuid-sandbox", b"--disable-seccomp-filter-sandbox",
                 b"--disable-namespace-sandbox", b"--ignore-certificate-errors", b"--single-process")
    if any(arg.split(b"=", 1)[0] in forbidden for arg in args):
        raise Rejected("SANDBOX_UNPROVEN")
    return (b"--type=renderer" in args and identity["uid"] == os.getuid()
            and isinstance(identity["userns"], str) and isinstance(identity["pidns"], str)
            and identity["seccomp"] == "2" and identity["userns"] != parent["userns"]
            and identity["pidns"] != parent["pidns"] and identity["no_new_privs"] == "1"
            and identity["filters"] > parent["filters"])


class Child:
    def __init__(self):
        self.cleanup = False
        self.sandbox = False
        self.sandbox_proof = []
        self.owned = {}
        self.stdout = b""
        self.stderr = b""
        self.output_overflow = False  # sticky stdout/stderr fact, not the final error priority
        self.code = None
        self.stop_path = "NOT_OBSERVED"  # event fact, never the final failure priority
        self.before = set()
        self.recorded = set()
        self.renderer_observed = False
        self.namespace_reads = {"user": set(), "pid": set()}
        self.observe_browser = False

    def observe_renderer(self, current):
        # Consume only already-read observations after ownership/identity binding.
        owned = self.owned.get(current["pid"])
        if (not owned or any(current[k] != owned[k] for k in ("pid", "start", "uid"))
                or current["uid"] != os.getuid() or b"--type=renderer" not in current["args"]):
            return
        self.renderer_observed = True
        for name in ("user", "pid"):
            label = current.get("namespace_reads", {}).get(name, "NOT_OBSERVED")
            if type(label) is not str or label not in NAMESPACE_READS:
                raise Rejected("INPUT_INVALID")
            self.namespace_reads[name].add(label)
            if len(self.namespace_reads[name]) > 1:
                self.namespace_reads[name].discard("NOT_OBSERVED")

    def record(self, output):
        path = output / "private" / "process-identities.jsonl"
        for pid, value in self.owned.items():
            identity = (pid, value["start"], value["uid"])
            if identity not in self.recorded:
                raw = (json.dumps({k: value[k] for k in ("pid", "uid", "start")}) + "\n").encode()
                if path.exists() and path.stat().st_size + len(raw) > ARTIFACT_LIMIT:
                    raise Rejected("OUTPUT_LIMIT")
                with path.open("ab") as stream:
                    stream.write(raw)
                path.chmod(0o600)
                self.recorded.add(identity)

    def inventory(self, pid):
        entries = list(Path("/proc").iterdir())
        if len(entries) > 32768:
            raise Rejected("PREREQUISITE_UNAVAILABLE", True)
        values = [p for entry in entries if entry.name.isdigit() and (p := proc(int(entry.name)))
                  and (p["pid"], p["start"], p["uid"]) not in self.before]
        parents = {pid, *self.owned}
        for _ in range(16):
            fresh = [p for p in values if p["ppid"] in parents or p["session"] == pid]
            before = len(parents)
            for value in fresh:
                if value["uid"] == os.getuid() and value["pid"] != os.getpid():
                    self.owned.setdefault(value["pid"], value)
                    parents.add(value["pid"])
            if len(parents) == before:
                break
        # Subreaper receives orphans even after a double fork/reparenting.
        for value in values:
            if value["ppid"] == os.getpid() and value["pid"] != pid:
                self.owned.setdefault(value["pid"], value)
        if self.observe_browser:
            for value in values:
                self.observe_renderer(value)

    def execute(self, argv, *, output, env, timeout=30, browser=False, interrupt_after=None):
        self.stop_path = "NOT_OBSERVED"
        self.observe_browser = browser
        parent = proc(os.getpid())
        if not parent:
            self.stop_path = "GUARD_REJECTED"
            raise Rejected("PLATFORM_UNQUALIFIED", True)
        libc = ctypes.CDLL(None, use_errno=True)
        if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
            self.stop_path = "GUARD_REJECTED"
            raise Rejected("PLATFORM_UNQUALIFIED", True)
        previous = {}
        interrupted = [False]

        def stop(_sig, _frame):
            interrupted[0] = True

        for sig in (signal.SIGTERM, signal.SIGINT):
            previous[sig] = signal.signal(sig, stop)
        child = None
        failure = None
        started = time.monotonic()
        streams = [bytearray(), bytearray()]
        selector = selectors.DefaultSelector()
        try:
            self.before = {(p["pid"], p["start"], p["uid"]) for entry in Path("/proc").iterdir()
                           if entry.name.isdigit() and (p := proc(int(entry.name)))}
            child = subprocess.Popen(argv, cwd=output, env=env, stdin=subprocess.DEVNULL,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
            for index, pipe in enumerate((child.stdout, child.stderr)):
                os.set_blocking(pipe.fileno(), False)
                selector.register(pipe, selectors.EVENT_READ, index)
            identity = proc(child.pid)
            if identity:
                self.owned[child.pid] = identity
                self.record(output)
            elif child.poll() is None:
                raise Rejected("CLEANUP_FAILED")
            while True:
                self.inventory(child.pid)
                self.record(output)
                if browser:
                    for value in self.owned.values():
                        if current := same(value):
                            self.observe_renderer(current)
                            if sandbox_witness(current, parent):
                                self.sandbox = True
                                if len(self.sandbox_proof) < 4 and not any(p["pid"] == current["pid"] for p in self.sandbox_proof):
                                    self.sandbox_proof.append({"pid": current["pid"], "uid": current["uid"],
                                        "start_ticks": current["start"], "seccomp_mode": 2,
                                        "filters": current["filters"], "additional_filter": True,
                                        "user_namespace_isolated": True, "pid_namespace_isolated": True,
                                        "no_new_privileges": True, "forbidden_flags_absent": True})
                elapsed = time.monotonic() - started
                if interrupted[0] or (interrupt_after is not None and elapsed >= interrupt_after):
                    raise Rejected("INTERRUPTED")
                if elapsed >= timeout:
                    raise Rejected("CHILD_TIMEOUT")
                for key, _ in selector.select(0.02):
                    raw = os.read(key.fileobj.fileno(), 8192)
                    if not raw:
                        selector.unregister(key.fileobj)
                    else:
                        streams[key.data].extend(raw)
                        if len(streams[key.data]) > LIMIT:
                            self.output_overflow = True
                            raise Rejected("OUTPUT_LIMIT")
                if child.poll() is not None and not selector.get_map():
                    self.code = child.returncode
                    self.stop_path = "NATIVE_OBSERVED_BEFORE_CLEANUP"
                    break
        except BaseException as error:
            self.stop_path = ({"INTERRUPTED": "INTERRUPTED", "CHILD_TIMEOUT": "TIMEOUT", "OUTPUT_LIMIT": "OUTPUT_LIMIT"}.get(
                error.code, "GUARD_REJECTED") if isinstance(error, Rejected) else
                "INTERRUPTED" if isinstance(error, KeyboardInterrupt) else "ERROR")
            failure = error
        finally:
            if child:
                try:
                    self.inventory(child.pid)
                    try:
                        self.record(output)
                    except (OSError, Rejected):
                        # A full private identity journal must not prevent TERM.
                        pass
                    # PID + UID + creation time, never process names or killpg.
                    for identity in self.owned.values():
                        if (current := same(identity)) and not current["zombie"]:
                            try:
                                os.kill(identity["pid"], signal.SIGTERM)
                            except ProcessLookupError:
                                pass
                    deadline = time.monotonic() + 5
                    while time.monotonic() < deadline:
                        known = set(self.owned)
                        self.inventory(child.pid)
                        try:
                            self.record(output)
                        except (OSError, Rejected):
                            pass
                        for pid in set(self.owned) - known:
                            if (current := same(self.owned[pid])) and not current["zombie"]:
                                try:
                                    os.kill(pid, signal.SIGTERM)
                                except ProcessLookupError:
                                    pass
                        child.poll()
                        alive = []
                        for identity in self.owned.values():
                            current = same(identity)
                            if current and current["zombie"]:
                                try:
                                    os.waitpid(identity["pid"], os.WNOHANG)
                                except ChildProcessError:
                                    pass
                            elif current:
                                alive.append(current)
                        if not alive:
                            self.cleanup = True
                            break
                        time.sleep(0.02)
                    private = output / "private" / ("children-" + uuid.uuid4().hex + ".json")
                    raw = json.dumps([{k: value[k] for k in ("pid", "uid", "start")} for value in self.owned.values()]).encode()
                    if len(raw) <= ARTIFACT_LIMIT:
                        private.write_bytes(raw)
                        private.chmod(0o400)
                except (OSError, Rejected):
                    self.cleanup = False
            for pipe in (child.stdout, child.stderr) if child else ():
                pipe.close()
            if child and self.code is None:
                # Reuse the returncode already observed by cleanup's poll(), even
                # on interruption. This category does not change the failure.
                self.code = child.returncode
            selector.close()
            for sig, handler in previous.items():
                signal.signal(sig, handler)
            self.stdout, self.stderr = bytes(streams[0][:LIMIT]), bytes(streams[1][:LIMIT])
        # No stream is ever published, including on timeout/native failures.
        try:
            confidential(self.stdout)
            confidential(self.stderr)
        except Rejected as error:
            error.cleanup = "PASS" if self.cleanup else "FAIL"
            if browser and not self.sandbox:
                error.browser_diagnostic = browser_diagnostic(self.stdout, self.stderr, self.code)
            raise
        if not self.cleanup:
            error = Rejected("CLEANUP_FAILED")
            error.cleanup = "FAIL"
            raise error
        if failure:
            if isinstance(failure, Rejected):
                failure.cleanup = "PASS"
            raise failure
        if browser and not self.sandbox:
            error = Rejected("SANDBOX_UNPROVEN", True)
            error.cleanup = "PASS"
            error.browser_diagnostic = browser_diagnostic(self.stdout, self.stderr, self.code)
            raise error
        return self.code
