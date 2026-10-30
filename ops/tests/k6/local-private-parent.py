"""Companion of the fixed Windows controller; pipes contain metadata only.

No runtime starts on import. Preflight never enters a site session or main.
Installation/evidence paths are private inputs, not authority for current facts.
The private profile has exactly the keys checked by LinuxFacts: existing nominal
refusal/opaque ID/time and reviewed hashes, separate contemporaneous evidence
files, source-file hashes (22 frozen files plus both parent programs), HOME/UID,
tool directory and a fresh output directory. Hash references must come from the
reviewed campaign, never be regenerated from mismatching files at launch. No
profile field can select another platform or skip the native observations.
"""
import argparse
import contextlib
from datetime import datetime, timezone
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import select
import stat
import sys
import time
import uuid

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "lib"))
import private_site as site_channel
from safety import CODES, confidential, digest, name_chain, private_path, private_read, strict_json

SPEC = importlib.util.spec_from_file_location("private_parent_runner", HERE / "run-linux.py")
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)
LIMIT = 4096
KINDS = frozenset("INIT HELLO AUTHORIZE WRITE ALLOW DENY READY POLL WAIT ACK CLOSED PREFLIGHT ABORT".split())
FIELDS = frozenset("v kind seq session browser run deadline home mode cleanup diag".split())
DIAGNOSTICS = frozenset("NONE OBSERVERS_ONLY NATIVE_NON_SUCCESS INPUT_INVALID CLEANUP_FAILED RETENTION_UNPROVEN".split())
FROZEN = (
    ".github/workflows/k6-functional-smoke.yml", "ops/tests/k6/IMPLEMENTATION.md", "ops/tests/k6/README.md",
    "ops/tests/k6/config/campaign.example.json", "ops/tests/k6/config/tools.lock.json", "ops/tests/k6/lib/safety.py",
    "ops/tests/k6/lib/private_site.py", "ops/tests/k6/lib/summary.js", "ops/tests/k6/provision-linux.py", "ops/tests/k6/run-linux.py",
    "ops/tests/k6/scenarios/smoke/iam-health.js", "ops/tests/k6/scenarios/smoke/browser-prerequisite.js",
    "ops/tests/k6/tests/test_boundaries.py", "ops/tests/k6/tests/test_runner.py", "ops/tests/k6/tests/test_provisioning.py",
    "ops/tests/k6/tests/test_browser_observation.py", "ops/tests/k6/tests/test_direct_chromium.py",
    "ops/tests/k6/tests/test_direct_followup.py", "ops/tests/k6/tests/test_direct_stop.py",
    "ops/tests/k6/tests/test_direct_text.py", "ops/tests/k6/tests/test_linux_comparison.py", "ops/tests/k6/tests/test_private_site.py")


class Refused(Exception):
    def __init__(self, diag="INPUT_INVALID"):
        self.diag = "RETENTION_UNPROVEN" if diag == "RETENTION_UNPROVEN" else "INPUT_INVALID"
        super().__init__("PARENT_REFUSED")  # never an OS message, path or packet


def require(value):
    if not value:
        raise Refused()


def date(value):
    require(type(value) is str and re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z", value))
    try:
        return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError:
        raise Refused() from None


def frame_check(value):
    require(type(value) is dict and set(value) == FIELDS)
    require(type(value["v"]) is int and value["v"] == 1)
    require(type(value["seq"]) is int and 0 <= value["seq"] <= 8192)
    for key in ("session", "browser", "run"):
        require(type(value[key]) is str and (key == "run" and value[key] == "" or re.fullmatch(r"[a-f0-9]{32}", value[key])))
    require(type(value["kind"]) is str and value["kind"] in KINDS)
    require(value["mode"] in {"preflight", "capture"} and type(value["mode"]) is str)
    require(type(value["home"]) is list and len(value["home"]) == 3
            and all(type(n) is int and 0 <= n <= 2**63 - 1 for n in value["home"]))
    require(value["cleanup"] in {"NOT_STARTED", "PASS", "FAIL"} and type(value["cleanup"]) is str)
    require(type(value["diag"]) is str and value["diag"] in DIAGNOSTICS)
    date(value["deadline"])
    return value


def encode_frame(value):
    try:
        raw = json.dumps(frame_check(value), separators=(",", ":"), ensure_ascii=True, allow_nan=False).encode("ascii")
        require(len(raw) + 1 <= LIMIT)
        confidential(raw)
        return raw
    except BaseException:
        raise Refused() from None


def decode_frame(raw):
    try:
        require(type(raw) is bytes and 0 < len(raw) < LIMIT and b"\n" not in raw and b"\r" not in raw)
        confidential(raw)
        return frame_check(strict_json(raw))
    except BaseException:
        raise Refused() from None


class Channel:
    def __init__(self, incoming, outgoing):
        self.incoming, self.outgoing, self.count = incoming, outgoing, 0

    def receive(self, timeout):
        end, raw = time.monotonic() + max(0, timeout), bytearray()
        while True:
            remaining = end - time.monotonic()
            require(remaining > 0)
            require(select.select([self.incoming], [], [], remaining)[0])
            block = os.read(self.incoming, LIMIT + 1 - len(raw))
            require(block)
            raw.extend(block)
            require(len(raw) <= LIMIT)
            if b"\n" in raw:
                require(raw[-1:] == b"\n" and raw.count(b"\n") == 1)
                return decode_frame(bytes(raw[:-1]))

    def send(self, value, timeout=5):
        self.count += 1
        require(self.count <= 8192)
        raw, end = encode_frame(value) + b"\n", time.monotonic() + max(0, timeout)
        while raw:
            remaining = end - time.monotonic()
            require(remaining > 0 and select.select([], [self.outgoing], [], remaining)[1])
            try:
                done = os.write(self.outgoing, raw)
            except BlockingIOError:
                continue
            require(done > 0)
            raw = raw[done:]


class Peer:
    def __init__(self, transport, init, home):
        self.transport, self.init, self.home = transport, frame_check(init), home
        require(init["kind"] == "INIT" and init["seq"] == 0 and init["run"] == "" and init["home"] == [0, 0, 0])
        require(0 < (date(init["deadline"]) - datetime.now(timezone.utc)).total_seconds() <= 3600)
        self.seq, self.run, self.phase = 0, "", 0

    def message(self, kind, cleanup="NOT_STARTED", diag="NONE"):
        return {**self.init, "kind": kind, "seq": self.seq, "run": self.run, "home": self.home,
                "cleanup": cleanup, "diag": diag}

    def ask(self, kind, run=None, timeout=30):
        if kind in {"AUTHORIZE", "WRITE"}:
            require(kind == ("AUTHORIZE", "WRITE")[self.phase] if self.phase < 2 else False)
            require(self.run in {"", run})
            self.run, self.phase = run, self.phase + 1
        else:
            require(kind in {"READY", "POLL"} and self.phase == 2)
        self.seq += 1
        request = self.message(kind, "PASS" if kind in {"READY", "POLL"} else "NOT_STARTED")
        started = time.monotonic()
        self.transport.send(request, timeout=min(timeout, 5))
        response = self.transport.receive(max(0, timeout - (time.monotonic() - started)))
        allowed = {"ALLOW"} if kind in {"AUTHORIZE", "WRITE"} else {"WAIT", "ACK"}
        require(response["kind"] in allowed and {**response, "kind": kind} == request)
        require(time.monotonic() - started <= timeout)
        return response["kind"]

    def finish(self, kind, cleanup, diag):
        self.seq += 1
        self.transport.send(self.message(kind, cleanup, diag))


def ext4_mount(fdinfo, mounts, device):
    ids = [line.split(":", 1)[1].strip() for line in fdinfo.splitlines() if line.startswith("mnt_id:")]
    if len(ids) != 1:
        return False
    matched = [line.split() for line in mounts.splitlines() if line.split() and line.split()[0] == ids[0]]
    if len(matched) != 1:
        return False
    fields = matched[0]
    try:
        split = fields.index("-")
        return fields[split + 1] == "ext4" and fields[2] == f"{os.major(device)}:{os.minor(device)}"
    except (ValueError, IndexError):
        return False


def bounded_read(path, maximum=65536):
    path = Path(path)
    require(path.name != "site.json" and "k6-causal" not in path.parts)
    with path.open("rb") as stream:
        raw = stream.read(maximum + 1)
    require(len(raw) <= maximum)
    return raw


class LinuxFacts:
    """Real observations; pinned identities are checked again, not a cached True."""
    def __init__(self, profile):
        self.profile, self.identities, self.fd = profile, {}, None
        try:
            runner.linux_platform()
            site_channel.native_primitives()
            import pwd  # native account database, never an environment-only HOME
            require(set(profile) == {"v", "home", "uid", "tools_dir", "refusal", "refusal_sha256", "browser_run_id",
                                     "started_unix_ms", "evidence", "source_files", "output"} and profile["v"] == 1)
            require(type(profile["uid"]) is int and os.getuid() == os.geteuid() == profile["uid"] != 0)
            home = private_path(profile["home"], directory=True)
            account = pwd.getpwuid(os.getuid())
            require(account.pw_name == "aiforall" and account.pw_dir == str(home)
                    and home == Path.home() and stat.S_IMODE(home.stat().st_mode) == 0o700)
            self.account = (account.pw_name, account.pw_dir)
            self.home_chain = name_chain(home)
            self.fd = os.open(home, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
            item = os.fstat(self.fd)
            self.home_identity = [item.st_dev, item.st_ino, item.st_uid]
            self._mount()
            sources = profile["source_files"]
            require(type(sources) is dict and 10 <= len(sources) <= 40)
            required = set(FROZEN) | {str(path.relative_to(runner.REPO)) for path in (
                Path(__file__), HERE / "local-private-parent.ps1")}
            require(required <= set(sources))
            for relative, sha in sources.items():
                require(type(relative) is str and re.fullmatch(r"[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*", relative)
                        and all(p not in {".", ".."} for p in relative.split("/")))
                self._bind(runner.REPO / relative, sha)
            self._bind(profile["refusal"], profile["refusal_sha256"], private=True)
            self.refusal = strict_json(bounded_read(profile["refusal"]))
            require(site_channel.nominal_refusal(self.refusal, profile["browser_run_id"]))
            require(type(profile["started_unix_ms"]) is int and self.refusal["started_unix_ms"] == profile["started_unix_ms"])
            require(type(profile["evidence"]) is list and 1 <= len(profile["evidence"]) <= 8)
            for evidence in profile["evidence"]:
                require(type(evidence) is dict and set(evidence) == {"path", "sha256"})
                self._bind(evidence["path"], evidence["sha256"], private=True)
            os.environ["K6_TOOLS_DIR"] = str(private_path(profile["tools_dir"], directory=True))
            lock = strict_json(bounded_read(HERE / "config/tools.lock.json"))
            os.environ["K6_BROWSER_EXECUTABLE_PATH"] = str(Path(profile["tools_dir"]) / lock["chromium"]["executable"])
            self.tools, _ = runner.pinned_tools(browser=True)
            self.verify()
        except BaseException:
            self.close()
            raise Refused() from None

    def _bind(self, path, sha, private=False):
        path = Path(path)
        require(path.is_absolute() and path.name != "site.json" and "k6-causal" not in path.parts)
        require(type(sha) is str and re.fullmatch(r"[a-f0-9]{64}", sha))
        if private:
            private_path(path)
        before = name_chain(path)
        require(path.is_file() and digest(path) == sha and name_chain(path) == before)
        self.identities[path] = before

    def _mount(self):
        require(ext4_mount(bounded_read(f"/proc/self/fdinfo/{self.fd}", 4096).decode("ascii"),
                           bounded_read("/proc/self/mountinfo").decode("ascii"), self.home_identity[0]))

    def verify(self):
        import pwd
        item = os.fstat(self.fd)
        require([item.st_dev, item.st_ino, item.st_uid] == self.home_identity)
        require(os.getuid() == os.geteuid() == self.profile["uid"])
        account = pwd.getpwuid(os.getuid())
        require((account.pw_name, account.pw_dir) == self.account)
        require(name_chain(self.profile["home"]) == self.home_chain)
        self._mount()
        for path, identity in self.identities.items():
            require(name_chain(path) == identity)
        runner.verify_tools(self.tools, ("k6", "chromium"))

    def close(self):
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None


class Admission:
    def __init__(self, peer, facts, init):
        self.peer, self.facts, self.init, self.phase = peer, facts, init, 0

    def __call__(self, evidence):
        try:
            started = time.monotonic()
            require(self.phase < 2 and evidence["phase"] == ("AUTHORIZE", "WRITE")[self.phase])
            require(evidence["browser_run_id"] == self.init["browser"] and evidence["delete_by_utc"] == self.init["deadline"])
            require(list(evidence["home_identity"]) == self.facts.home_identity)
            self.facts.verify()
            self.peer.ask(evidence["phase"], evidence["run_id"], timeout=max(0, 30 - (time.monotonic() - started)))
            self.facts.verify()
            require(time.monotonic() - started <= 30)
            self.phase += 1
            return True
        except BaseException:
            return False


class BoundedText(io.StringIO):
    def write(self, value):
        require(len((self.getvalue() + value).encode("utf-8")) <= runner.ARTIFACT_LIMIT)
        return super().write(value)


def result_check(text, code):
    try:
        require(type(code) is int and code in {1, 2})
        raw = text.encode("utf-8")
        confidential(raw)
        value = strict_json(raw)
        required = {"version", "HTTP", "Chromium", "cleanup", "confidentiality", "run_id", "started_unix_ms",
                    "cleanup_scope", "parent_runtime_cleanup", "diagnostic"}
        require(type(value) is dict and required <= set(value) <= required | {
            "browser_source", "browser_host", "direct_observation", "direct_stop", "direct_text"})
        require(type(value["version"]) is int and value["version"] == 1
                and value["HTTP"] == value["Chromium"] == value["parent_runtime_cleanup"] == "INCONCLUSIVE"
                and value["cleanup_scope"] == "LAUNCHER_CHILDREN_ONLY"
                and value["cleanup"] in {"PASS", "FAIL", "NOT_STARTED"}
                and value["confidentiality"] in {"PASS", "FAIL", "NOT_QUALIFIED"}
                and re.fullmatch(r"[a-f0-9]{32}", value["run_id"])
                and type(value["started_unix_ms"]) is int and value["started_unix_ms"] > 0)
        require(value["diagnostic"] in CODES)
        for key, check in (("direct_observation", runner.closed_direct_observation), ("direct_stop", runner.closed_direct_stop),
                           ("direct_text", runner.closed_direct_text), ("browser_host", runner.closed_host_metadata)):
            if key in value:
                check(value[key])
        if "browser_source" in value:
            require(value["browser_source"] == "CHROMIUM_DIRECT")
        return value
    except BaseException:
        raise Refused() from None


def preflight(peer, facts):
    run = uuid.uuid4().hex
    for phase in ("AUTHORIZE", "WRITE"):
        started = time.monotonic()
        facts.verify()
        peer.ask(phase, run, timeout=max(0, 30 - (time.monotonic() - started)))
        facts.verify()
        require(time.monotonic() - started <= 30)
    peer.finish("PREFLIGHT", "NOT_STARTED", "OBSERVERS_ONLY")
    return 0


def await_read(session, peer, facts):
    kind = "READY"
    while not session._expired():
        facts.verify()
        remaining = min((session.deadline - datetime.now(timezone.utc)).total_seconds(),
                        session._expires_monotonic - time.monotonic())
        try:
            answer = peer.ask(kind, timeout=max(0, min(30, remaining)))
        except BaseException:
            if session._expired():
                break
            raise Refused() from None
        if answer == "ACK":
            require(session.retire(read_confirmed=True) is True and session.status == "REMOVED")
            return
        time.sleep(min(1, max(0, remaining)))
        kind = "POLL"
    require(session.retire(read_confirmed=False) is True and session.status == "REMOVED")


def capture(peer, facts):
    callback = Admission(peer, facts, peer.init)
    result, code = None, 2
    session = site_channel.PrivateSiteSession(facts.refusal, peer.init["browser"], parent_check=callback,
                                            delete_by_utc=peer.init["deadline"])
    try:
        with session:
            with contextlib.redirect_stdout(BoundedText()) as output, contextlib.redirect_stderr(BoundedText()) as errors:
                code = runner.main(["diagnose-chromium", "--private-site", "--output", facts.profile["output"]], private_site=session)
            require(errors.getvalue() == "")
            result = result_check(output.getvalue(), code)
            require(peer.run in {"", result["run_id"]})
            if session.status == "READY":
                require(result["cleanup"] == "PASS" and result["confidentiality"] == "PASS")
                await_read(session, peer, facts)
        if session.status == "RETENTION_UNPROVEN":
            raise Refused("RETENTION_UNPROVEN")
    except BaseException:
        # __exit__ can replace the initial retirement error with CLEANUP_FAILED.
        # Preserve the module's final closed state, never its exception payload.
        if session.status == "RETENTION_UNPROVEN":
            raise Refused("RETENTION_UNPROVEN") from None
        raise
    peer.finish("CLOSED", result["cleanup"], "NATIVE_NON_SUCCESS")
    return code


def main(argv=None):
    facts, peer = None, None
    try:
        require(not site_channel.ci_present())
        parser = argparse.ArgumentParser(add_help=False)
        parser.add_argument("--mode", choices=("preflight", "capture"), required=True)
        parser.add_argument("--profile", required=True)
        with contextlib.redirect_stderr(BoundedText()):
            args = parser.parse_args(argv)
        require(not sys.stdin.isatty() and not sys.stdout.isatty())
        os.set_blocking(sys.stdin.fileno(), False)
        os.set_blocking(sys.stdout.fileno(), False)
        wire = Channel(sys.stdin.fileno(), sys.stdout.fileno())
        init = wire.receive(30)
        require(init["mode"] == args.mode)
        path = Path(args.profile)
        require(path.name != "site.json" and "k6-causal" not in path.parts)
        profile = strict_json(private_read(path, 65536))
        require(profile["browser_run_id"] == init["browser"])
        facts = LinuxFacts(profile)
        peer = Peer(wire, init, facts.home_identity)
        wire.send(peer.message("HELLO"))
        return preflight(peer, facts) if args.mode == "preflight" else capture(peer, facts)
    except BaseException as error:
        if peer is not None:
            try:
                peer.finish("ABORT", "FAIL", error.diag if isinstance(error, Refused) else "INPUT_INVALID")
            except BaseException:
                pass
        return 2
    finally:
        if facts is not None:
            try:
                facts.close()
            except BaseException:
                return 2


if __name__ == "__main__":
    raise SystemExit(main())
