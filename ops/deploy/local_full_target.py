"""S-11 single PS/Python authority: alias is never API identity. No ambient config."""
import argparse
import base64
import contextlib
import hashlib
import http.client
import ipaddress
import json
import os
import re
import signal
import socket
import stat
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import quote, urlsplit

import yaml

ALLOWED = {"aiforall-local": "kind-aiforall-local", "aiforall-local-full": "kind-aiforall-local-full"}
NODE_IMAGE = "kindest/node:v1.32.2@sha256:142f543559cc55d64e1ab9341df08e5ced84bd2e893736da8f51320f26f5950b"
METADATA = '{"id":{{json .Id}},"running":{{json .State.Running}},"image":{{json .Config.Image}},"cluster":{{json (index .Config.Labels "io.x-k8s.kind.cluster")}},"role":{{json (index .Config.Labels "io.x-k8s.kind.role")}},"ports":{{json .NetworkSettings.Ports}}}'


class AnchorError(RuntimeError):
    pass


@dataclass(frozen=True)
class KubectlOperation:
    argv: tuple
    remote_argv: tuple = ()


@dataclass(frozen=True)
class OwnedPodLogOperation:
    pod: str
    container: str
    tail: int
    limit_bytes: int
    argv: tuple


def parse_owned_pod_logs(argv):
    """Closed diagnostic grammar, separate from generic kubectl authority."""
    if (not isinstance(argv, (list, tuple)) or len(argv) < 2
            or any(not isinstance(x, str) or "\x00" in x for x in argv)
            or argv[0] != "logs" or not _log_name(argv[1])):
        raise AnchorError("invalid owned Pod logs before any probe")
    flags = {"-n": "namespace", "--namespace": "namespace", "-c": "container",
             "--container": "container", "--tail": "tail", "--limit-bytes": "limit_bytes",
             "--previous": "previous"}
    values = {}
    index = 2
    while index < len(argv):
        flag, equal, value = argv[index].partition("=")
        key = flags.get(flag)
        if key is None or key in values or (equal and not flag.startswith("--")):
            raise AnchorError("unsupported owned Pod logs before any probe")
        if key == "previous":
            value = value if equal else "true"
            if value not in {"true", "false"}:
                raise AnchorError("invalid previous boolean before any probe")
        elif not equal:
            index += 1
            if index == len(argv):
                raise AnchorError("missing log bound before any probe")
            value = argv[index]
        values[key] = value
        index += 1
    if (values.get("namespace") != "aiforall-local-full"
            or not _log_name(values.get("container"), container=True)
            or not re.fullmatch(r"[0-9]{1,3}", values.get("tail", ""))
            or not 1 <= int(values["tail"]) <= 200
            or not re.fullmatch(r"[0-9]{1,5}", values.get("limit_bytes", ""))
            or not 1 <= int(values["limit_bytes"]) <= 32768):
        raise AnchorError("exact full Pod/container and log bounds required before any probe")
    args = ("logs", argv[1], "-n", "aiforall-local-full", "-c", values["container"],
            "--tail=" + str(int(values["tail"])), "--limit-bytes=" + str(int(values["limit_bytes"])))
    if "previous" in values:
        args += ("--previous=" + values["previous"],)
    return OwnedPodLogOperation(argv[1], values["container"], int(values["tail"]),
                                int(values["limit_bytes"]), args)


def _log_name(value, container=False):
    return (isinstance(value, str) and 1 <= len(value) <= (63 if container else 253)
            and all(re.fullmatch(r"[a-z0-9](?:[-a-z0-9]{0,61}[a-z0-9])?", part)
                    for part in value.split(".")) and (not container or "." not in value))


def parse_kubectl_operation(argv):
    """Positive local grammar; never rely on kubectl's effective override order."""
    if not argv or any(not isinstance(x, str) or "\x00" in x for x in argv):
        raise AnchorError("invalid kubectl operation")
    args = list(argv)
    remote = ()
    if "--" in args:
        cut = args.index("--")
        remote, args = tuple(args[cut + 1:]), args[:cut]
    verbs = {"get", "create", "apply", "patch", "scale", "wait", "rollout", "set", "cordon", "uncordon", "exec"}
    common = {"-n": "name", "--namespace": "name"}
    options = {
        "get": {"-o": "output", "-l": "selector", "--ignore-not-found": "bool"},
        "create": {"-f": "file", "-o": "output"},
        "apply": {"-f": "file", "-o": "output", "--server-side": "bool", "--field-manager": "name"},
        "patch": {"--type": "patch", "-p": "json", "--patch-file": "file"},
        "scale": {"--replicas": "uint"},
        "wait": {"--for": "condition", "--timeout": "duration", "-l": "selector", "--all": "bool"},
        "rollout": {"--timeout": "duration"}, "set": {}, "cordon": {}, "uncordon": {},
        "exec": {"-i": "bool", "-c": "name", "--container": "name"},
    }
    verb = None
    positionals = []
    index = 0
    while index < len(args):
        arg = args[index]
        if arg.startswith("-"):
            flag, equal, value = arg.partition("=")
            # Short flags must be standalone, never attached/clustered/equal.
            if not flag.startswith("--") and (len(flag) != 2 or equal):
                raise AnchorError("compact/clustered shorthand forbidden before any probe")
            kind = {**common, **(options.get(verb, {}) if verb else {})}.get(flag)
            if kind is None:
                raise AnchorError("unsupported local option; no target/auth overrides")
            if kind == "bool":
                if equal and value not in {"true", "false"}:
                    raise AnchorError("invalid boolean option")
            else:
                if not equal:
                    index += 1
                    if index == len(args):
                        raise AnchorError("missing option value")
                    value = args[index]
                if not value or (value.startswith("-") and not (kind == "file" and value == "-")):
                    raise AnchorError("invalid option value")
                valid = {
                    "name": lambda v: bool(re.fullmatch(r"[a-z0-9][a-z0-9.-]{0,252}", v)),
                    "uint": lambda v: bool(re.fullmatch(r"[0-9]{1,6}", v)),
                    "duration": lambda v: bool(re.fullmatch(r"[0-9]{1,6}s", v)),
                    "patch": lambda v: v in {"merge", "strategic", "json"},
                    "condition": lambda v: v == "delete" or bool(re.fullmatch(r"condition=[A-Za-z]+", v)),
                    "output": lambda v: v in {"name", "json", "yaml"} or (v.startswith("jsonpath=") and len(v) < 4096),
                    "selector": lambda v: bool(re.fullmatch(r"[a-zA-Z0-9./_=-]+", v)),
                    "file": lambda v: len(v) < 32768 and not any(c in v for c in "\r\n"),
                    "json": lambda v: len(v) < 1048576 and isinstance(json.loads(v), (dict, list)),
                }[kind]
                try:
                    accepted = valid(value)
                except (ValueError, TypeError):
                    accepted = False
                if not accepted:
                    raise AnchorError("unsupported option value")
        elif verb is None:
            if arg not in verbs:
                raise AnchorError("unsupported kubectl verb")
            verb = arg
        else:
            if not re.fullmatch(r"[a-zA-Z0-9./,_=:-]+", arg):
                raise AnchorError("unsupported local positional value")
            positionals.append(arg)
        index += 1
    if verb is None or not positionals and verb not in {"create", "apply"}:
        raise AnchorError("missing operation target")
    if verb == "exec":
        if not remote or len(positionals) != 1:
            raise AnchorError("exec requires one target and separate remote suffix")
    elif remote:
        raise AnchorError("remote suffix only allowed for exec")
    if verb == "rollout" and (len(positionals) != 2 or positionals[0] != "status"):
        raise AnchorError("only rollout status supported")
    if verb == "set" and (len(positionals) < 3 or positionals[0] != "env"):
        raise AnchorError("only set env supported")
    if any(part.split("/")[0] in {"se", "secret", "secrets"} for p in positionals for part in p.split(",")) and verb == "get":
        raise AnchorError("live Secret reads forbidden")
    return KubectlOperation(tuple(args), remote)


def run(command, **kwargs):
    # Go CLIs emit UTF-8 on Windows too. Capture bytes so a decoder exception
    # cannot be swallowed in subprocess's Windows pipe-reader thread.
    encoding = kwargs.pop("encoding", None)
    errors = kwargs.pop("errors", None)
    text = kwargs.pop("text", False)
    universal = kwargs.pop("universal_newlines", False)
    textual = bool(text or universal or encoding or errors)
    if errors not in {None, "strict"}:
        raise AnchorError("non-strict target text decoding forbidden")
    encoding = encoding or "utf-8"
    if textual and isinstance(kwargs.get("input"), str):
        try:
            value = kwargs["input"]
            if os.linesep != "\n":
                value = value.replace("\n", os.linesep)
            kwargs["input"] = value.encode(encoding, errors="strict")
        except (UnicodeError, LookupError):
            raise AnchorError("invalid target input encoding; output redacted") from None
    result = subprocess.run(command, capture_output=True, check=False, **kwargs)
    if result.returncode:
        raise AnchorError("target operation failed; output redacted")
    if textual:
        try:
            output = result.stdout.decode(encoding, errors="strict")
            result.stderr.decode(encoding, errors="strict")
        except (UnicodeError, LookupError):
            raise AnchorError("invalid target text encoding; output redacted") from None
        return output.replace("\r\n", "\n").replace("\r", "\n")
    return result.stdout


def approved_root():
    return Path.home() / "AppData/Local/Temp/opencode" if os.name == "nt" else Path.home() / ".cache/opencode"


def _windows_path_script(script, path, directory=False):
    # -Command does not bind trailing process arguments to $args in a script
    # string. Keep the path out of PowerShell syntax as well as CLI arguments.
    data = base64.b64encode(str(path).encode("utf-8")).decode("ascii")
    prefix = ("$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue';"
              "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);"
              "$p=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('" + data + "'));"
              "$dir=" + ("$true;" if directory else "$false;"))
    encoded = base64.b64encode((prefix + script).encode("utf-16-le")).decode("ascii")
    return run(["powershell.exe", "-NoProfile", "-EncodedCommand", encoded], text=True, encoding="utf-8", timeout=30)


def private_file(path):
    """Read-only file, owned/private directory; no raw ACL/credentials emitted."""
    if os.name != "nt":
        return (path.stat().st_uid == os.getuid() and path.parent.stat().st_uid == os.getuid()
                and not path.stat().st_mode & 0o222 and not path.stat().st_mode & 0o077
                and not path.parent.stat().st_mode & 0o077)
    script = "$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value;$ok=$true;foreach($f in @($p,(Split-Path -Parent $p))){$a=Get-Acl -LiteralPath $f;if($a.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $sid){$ok=$false};foreach($r in $a.Access){if($r.AccessControlType -eq 'Allow' -and $r.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value -notin @($sid,'S-1-5-18')){$ok=$false}}};if(-not (Get-Item -LiteralPath $p).IsReadOnly){$ok=$false};if($ok){'yes'}else{'no'}"
    return _windows_path_script(script, path).strip() == "yes"


def secure(path, directory=False):
    if os.name != "nt":
        path.chmod(0o700 if directory else 0o400)
        return
    # No inherited broad ACE, no Administrators/Everyone grants. SYSTEM + task
    # user only; generated file is read-only and no longer writable by task tools.
    # Mark the new owned file read-only before withdrawing WriteAttributes.
    # Persist only modified owner/access sections through .NET: Set-Acl can
    # attempt an unrelated SACL write on Python's protected Windows 0o700 DACL.
    script = "$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User;$a=if($dir){New-Object Security.AccessControl.DirectorySecurity}else{New-Object Security.AccessControl.FileSecurity};$a.SetOwner($sid);$a.SetAccessRuleProtection($true,$false);$rights=if($dir){'FullControl'}else{'ReadAndExecute'};$a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,$rights,'Allow'));$a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.SecurityIdentifier]::new('S-1-5-18'),'FullControl','Allow'));$item=Get-Item -LiteralPath $p;if(-not $dir){$item.IsReadOnly=$true};$item.SetAccessControl($a)"
    _windows_path_script(script, path, directory)


class UniqueLoader(yaml.SafeLoader):
    pass


def unique_mapping(loader, node):
    result = {}
    for key, value in node.value:
        key = loader.construct_object(key)
        if key in result:
            raise AnchorError("ambiguous kubeconfig")
        result[key] = loader.construct_object(value)
    return result


UniqueLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, unique_mapping)


def unique_json(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise AnchorError("ambiguous lease fields")
        result[key] = value
    return result


_ENVOY_IMAGE = "envoyproxy/envoy:v1.31.2@sha256:c56804e9fc8d5184c45862ad851173a925ff383151cb94156f86bed047da29bc"
_ENVOY_STATS = tuple("cluster." + cluster + "." + suffix
                     for cluster in ("ext_authz", "iam", "hive", "telegraph", "manifesto")
                     for suffix in ("upstream_cx_total", "upstream_cx_connect_fail",
                                    "upstream_cx_connect_timeout", "upstream_rq_total", "upstream_rq_5xx",
                                    "ssl.handshake", "ssl.fail_verify_san", "ssl.fail_verify_error"))
_ENVOY_STATS_PATH = "/stats?format=json&filter=" + quote(
    "^(?:" + "|".join(re.escape(name) for name in _ENVOY_STATS) + ")$", safe="")


def _admin_remaining(deadline):
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise AnchorError("Admin snapshot deadline exceeded")
    return remaining


def _admin_json(raw):
    def invalid_constant(_):
        raise ValueError("non-JSON number")
    return json.loads(raw.decode("utf-8", errors="strict"), object_pairs_hook=unique_json,
                      parse_constant=invalid_constant)


def _admin_marker_private(path):
    # Unlike the sealed kubeconfig, this exclusive lifetime marker must remain
    # writable/deletable. Windows os.open(0600) alone does not establish its DACL.
    if os.name == "nt":
        _windows_path_script(
            "$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User;"
            "$a=[Security.AccessControl.FileSecurity]::new();"
            "$a.SetOwner($sid);$a.SetAccessRuleProtection($true,$false);"
            "$a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','Allow'));"
            "$a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new("
            "[Security.Principal.SecurityIdentifier]::new('S-1-5-18'),'FullControl','Allow'));"
            "$item=Get-Item -LiteralPath $p;$item.SetAccessControl($a)", path)


def _admin_get(port, stats, tunnel_deadline):
    """Private fixed GET, no urllib proxy/cookie/redirect handlers or ambient auth.

    A socket shutdown timer bounds header parsing and slow/chunked bodies too;
    individual socket timeouts alone would not bound a trickling response.
    """
    deadline = min(tunnel_deadline, time.monotonic() + 5)
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=_admin_remaining(deadline))
    response = timer = None
    try:
        connection.connect()
        _admin_remaining(deadline)
        sock = connection.sock

        def expire():
            try:
                sock.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
        timer = threading.Timer(_admin_remaining(deadline), expire)
        timer.daemon = True
        timer.start()
        connection.request("GET", _ENVOY_STATS_PATH if stats else "/server_info", headers={
            "Accept": "application/json", "Accept-Encoding": "identity", "Connection": "close"})
        response = connection.getresponse()
        _admin_remaining(deadline)
        if response.status != 200:
            raise AnchorError("Admin status refused")  # Never read/echo error or redirect bodies.
        lengths = response.headers.get_all("Content-Length", [])
        transfers = response.headers.get_all("Transfer-Encoding", [])
        if (response.headers.get_all("Content-Encoding", []) not in ([], ["identity"])
                or len(lengths) > 1 or transfers not in ([], ["chunked"])
                or (lengths and transfers)
                or (lengths and not re.fullmatch(r"0|[1-9][0-9]{0,19}", lengths[0]))):
            raise AnchorError("Admin encoding refused")
        limit = 65536 if stats else 131072
        body = bytearray()
        while True:
            sock.settimeout(_admin_remaining(deadline))
            chunk = response.read1(min(4096, limit + 1 - len(body)))
            _admin_remaining(deadline)
            if not chunk:
                if response.length not in (None, 0):
                    raise AnchorError("Admin response truncated")
                break
            body.extend(chunk)
            if len(body) > limit:
                raise AnchorError("Admin response bound exceeded")
        return _admin_json(body)
    finally:
        if timer is not None:
            timer.cancel()
            timer.join()
        try:
            if response is not None:
                response.close()
        finally:
            connection.close()


def _admin_server_projection(value):
    if not isinstance(value, dict) or value.get("state") not in {
            "LIVE", "DRAINING", "PRE_INITIALIZING", "INITIALIZING"}:
        raise AnchorError("Admin server state unavailable")
    version = value.get("version")
    if not isinstance(version, str) or not re.fullmatch(
            r"[a-f0-9]{40}/1\.31\.2/(?:Clean|Modified)/(?:RELEASE|DEBUG)/[A-Za-z0-9_.-]{1,64}", version):
        raise AnchorError("Admin version differs from expected image")
    options = value.get("command_line_options")
    epoch = options.get("restart_epoch") if isinstance(options, dict) else None
    if type(epoch) is not int or not 0 <= epoch <= 2**32 - 1:
        raise AnchorError("Admin restart epoch unavailable")
    result = {"state": value["state"], "version": "1.31.2", "restart_epoch": epoch}
    for name in ("uptime_current_epoch", "uptime_all_epochs"):
        duration = value.get(name)
        match = re.fullmatch(r"(0|[1-9][0-9]{0,11})(?:\.([0-9]{1,9}))?s", duration) if isinstance(duration, str) else None
        if not match or int(match[1]) > 315576000000:
            raise AnchorError("Admin uptime unavailable")
        result[name + "_ns"] = int(match[1]) * 10**9 + int((match[2] or "").ljust(9, "0"))
    if result["uptime_current_epoch_ns"] > result["uptime_all_epochs_ns"]:
        raise AnchorError("Admin uptimes inconsistent")
    return result


def _admin_stats_projection(value):
    if not isinstance(value, dict) or set(value) != {"stats"} or not isinstance(value["stats"], list):
        raise AnchorError("Admin counters unavailable")
    counters = dict.fromkeys(_ENVOY_STATS, "MISSING")
    seen = set()
    for entry in value["stats"]:
        if not isinstance(entry, dict) or set(entry) != {"name", "value"}:
            raise AnchorError("Admin counter fields refused")
        name, number = entry["name"], entry["value"]
        if (not isinstance(name, str) or name not in counters or name in seen
                or type(number) is not int or not 0 <= number <= 2**64 - 1):
            raise AnchorError("Admin counter refused")
        seen.add(name)
        counters[name] = number
    return counters


class _EnvoyAdminForward:
    """Private sole-child handle; never adopts a PID or returns a tunnel to callers."""
    def __init__(self):
        self.process = None
        self.no_launch = True
        self.deadline = None
        self.timer = None
        self.threads = []
        self.output = [bytearray(), bytearray()]
        self.lock = threading.Lock()
        self.changed = threading.Event()
        self.stop_lock = threading.Lock()
        self.stopping = False
        self.failed = False

    def start(self, command, env):
        self.no_launch = False
        try:
            self.process = subprocess.Popen(command, env=env, stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0,
                creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0)
        except OSError:
            self.no_launch = True  # Popen failed without returning a live handle.
            raise
        self.deadline = time.monotonic() + 30
        self.timer = threading.Timer(30, self.stop)
        self.timer.daemon = True
        self.timer.start()
        for index, pipe in enumerate((self.process.stdout, self.process.stderr)):
            thread = threading.Thread(target=self._drain, args=(index, pipe), daemon=True)
            self.threads.append(thread)
            thread.start()

    def _drain(self, index, pipe):
        try:
            while True:
                chunk = pipe.read(1024)
                if not chunk:
                    break
                with self.lock:
                    if len(self.output[index]) + len(chunk) > 4096:
                        self.failed = True
                        break
                    self.output[index].extend(chunk)
                self.changed.set()
        except Exception:
            self.failed = True
        finally:
            self.changed.set()

    def mapping(self, complete=False, exited=False):
        with self.lock:
            out, err = (bytes(value) for value in self.output)
        if self.failed or err or (not exited and (self.process.poll() is not None or self.stopping)):
            raise AnchorError("Admin forward unavailable")
        lines = out.split(b"\n")
        if lines[-1]:
            if complete:
                raise AnchorError("Admin forward output incomplete")
            return None
        lines.pop()
        if not lines:
            return None
        match = re.fullmatch(rb"Forwarding from 127\.0\.0\.1:([1-9][0-9]{0,4}) -> 9901\r?", lines[0])
        if not match or not 1 <= int(match[1]) <= 65535 or len(lines) > 3:
            raise AnchorError("Admin forward mapping refused")
        connection_line = b"Handling connection for " + match[1]
        if any(line not in (connection_line, connection_line + b"\r") for line in lines[1:]):
            raise AnchorError("Admin forward mapping refused")
        return int(match[1])

    def ready(self):
        deadline = min(self.deadline, time.monotonic() + 5)
        while True:
            self.changed.clear()
            port = self.mapping()
            if port is not None:
                return port
            self.changed.wait(_admin_remaining(deadline))

    def stop(self):
        with self.stop_lock:
            if self.stopping:
                return
            self.stopping = True
            try:
                if self.process is not None and self.process.poll() is None:
                    if os.name == "nt":
                        self.process.send_signal(signal.CTRL_BREAK_EVENT)
                    else:
                        self.process.terminate()
            except Exception:
                self.failed = True
            self.changed.set()

    def close(self):
        deadline = time.monotonic() + 15
        if self.timer is not None:
            self.timer.cancel()
        self.stop()
        try:
            if self.process is not None:
                self.process.wait(timeout=_admin_remaining(deadline))
                if self.process.poll() is None:
                    raise AnchorError("Admin child exit unattested")
                for thread in self.threads:
                    thread.join(timeout=_admin_remaining(deadline))
                    if thread.is_alive():
                        raise AnchorError("Admin pipe cleanup incomplete")
                for pipe in (self.process.stdout, self.process.stderr):
                    pipe.close()
            elif not self.no_launch:
                raise AnchorError("Admin launch outcome indeterminate")
            if self.timer is not None:
                self.timer.join(timeout=_admin_remaining(deadline))
                if self.timer.is_alive():
                    raise AnchorError("Admin timer cleanup incomplete")
        except Exception:
            raise AnchorError("INCONCLUSIVE: Admin cleanup blocked; retain private marker") from None


class LocalClusterTargetAnchor:
    def __init__(self, lease_path, runner=run, root=None, privacy=private_file, expected_cluster=None):
        self.path = Path(lease_path)
        self.runner, self.root, self.privacy = runner, Path(root or approved_root()).resolve(), privacy
        self.data = json.loads(self.path.read_text(encoding="utf-8-sig"), object_pairs_hook=unique_json)
        d = self.data
        if type(d.get("version")) is not int or d["version"] != 2 or d.get("cluster") not in ALLOWED or d.get("context") != ALLOWED[d["cluster"]]:
            raise AnchorError("leasev2 with closed cluster/context pair required; no adoption")
        if expected_cluster and d["cluster"] != expected_cluster:
            raise AnchorError("lease target differs from caller target")
        if not re.fullmatch(r"[a-f0-9]{32}", d.get("task_id", "")):
            raise AnchorError("explicit task identity required")
        self.config = Path(d.get("kubeconfig_path", ""))
        if not self.config.is_absolute() or self.config != self.config.resolve():
            raise AnchorError("absolute canonical kubeconfig path required")
        if self.config != self.root / ("cluster-anchor-" + d["task_id"]) / "kubeconfig.yaml":
            raise AnchorError("kubeconfig outside task-owned approved directory")
        for path in (self.config, self.config.parent):
            if path.exists() and (path.is_symlink() or getattr(path.stat(), "st_file_attributes", 0) & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)):
                raise AnchorError("linked kubeconfig path forbidden")
        if os.environ.get("KUBECONFIG") not in {None, "", str(self.config)} or os.environ.get("KUBERNETES_MASTER"):
            raise AnchorError("ambient target override forbidden")

    def environment(self):
        return {**os.environ, "KUBECONFIG": str(self.config)}

    def metadata(self, container_id):
        if not re.fullmatch(r"[a-f0-9]{64}", container_id):
            raise AnchorError("full exact container ID required")
        result = self.runner(["docker", "inspect", "--format", METADATA, container_id], text=True, timeout=30)
        value = json.loads(result)
        if value.get("id") != container_id:
            raise AnchorError("container ID mismatch")
        return value

    def roster(self):
        raw = self.runner(["docker", "ps", "-aq", "--no-trunc", "--filter", "label=io.x-k8s.kind.cluster=" + self.data["cluster"]], text=True, timeout=30)
        return raw.split()

    def local_check(self, pending=False):
        d = self.data
        if os.environ.get("KUBECONFIG") not in {None, "", str(self.config)} or os.environ.get("KUBERNETES_MASTER"):
            raise AnchorError("ambient target override forbidden")
        if d.get("anchor_state") != ("pending_creation" if pending else "anchored"):
            raise AnchorError("anchor state invalid; pending is creation-only")
        uid = d.get("kube_system_uid")
        if not pending and (not isinstance(uid, str) or not uid or uid.strip() != uid):
            raise AnchorError("missing prior kube-system UID; no adoption")
        ids = d.get("nodeContainerIds", [])
        creation = d.get("creation", {})
        phase = "endpoint-pending" if pending else "anchored"
        if creation.get("phase") != phase or creation.get("task_id") != d["task_id"] or creation.get("created_ids") != ids or not isinstance(creation.get("before_ids"), list) or set(ids) & set(creation["before_ids"]):
            raise AnchorError("exact task-created node delta not attested")
        if pending and d.get("kube_system_uid"):
            raise AnchorError("pending anchor cannot overwrite an existing UID")
        if not ids or len(ids) != len(set(ids)) or set(self.roster()) != set(ids):
            raise AnchorError("missing/foreign/ambiguous leased node roster")
        nodes = [self.metadata(i) for i in ids]
        for n in nodes:
            if n.get("image") != NODE_IMAGE or n.get("cluster") != d["cluster"] or n.get("role") not in {"worker", "control-plane"}:
                raise AnchorError("leased node metadata mismatch")
            if n.get("running") is not True:
                record = d.get("nodeFailures", {}).get(n["id"], {})
                if n["role"] != "worker" or not record.get("prior_running") or record.get("phase") not in {"cordoned", "stopped"} or record.get("context") != d["context"] or record.get("cluster") != d["cluster"]:
                    raise AnchorError("unexpected stopped node; no restart/adoption")
        cp = [n for n in nodes if n["role"] == "control-plane"]
        expected = 3 if d["cluster"] == "aiforall-local-full" else 1
        if len(nodes) != expected or len(cp) != 1 or cp[0]["id"] != d.get("control_plane_container_id"):
            raise AnchorError("control-plane/topology mismatch")
        mappings = (cp[0].get("ports") or {}).get("6443/tcp")
        if not isinstance(mappings, list) or len(mappings) != 1:
            raise AnchorError("missing/ambiguous API publication")
        mapping = mappings[0]
        try:
            ip = ipaddress.ip_address(mapping["HostIp"])
            port = int(mapping["HostPort"])
        except (KeyError, ValueError):
            raise AnchorError("invalid API publication") from None
        if not ip.is_loopback or ip not in {ipaddress.ip_address("127.0.0.1"), ipaddress.ip_address("::1")} or not 1 <= port <= 65535:
            raise AnchorError("literal non-wildcard loopback API required")
        if str(ip) != d.get("api_host") or type(d.get("api_port")) is not int or port != d.get("api_port"):
            raise AnchorError("leased API publication mismatch")
        if not self.config.is_file() or not self.privacy(self.config):
            raise AnchorError("fixed kubeconfig ownership/access mismatch")
        contents = self.config.read_bytes()
        if len(contents) > 1024 * 1024 or hashlib.sha256(contents).hexdigest() != d.get("kubeconfig_sha256"):
            raise AnchorError("fixed kubeconfig hash mismatch")
        try:
            config = yaml.load(contents, Loader=UniqueLoader)
        except yaml.YAMLError:
            raise AnchorError("invalid kubeconfig syntax; output redacted") from None
        try:
            contexts, clusters, users = config["contexts"], config["clusters"], config["users"]
            if len(contexts) != 1 or len(clusters) != 1 or len(users) != 1:
                raise AnchorError("single task-owned kubeconfig target required")
            ctx, cluster, user = contexts[0], clusters[0], users[0]
            if ctx["name"] != d["context"] or ctx["context"] != {"cluster": cluster["name"], "user": user["name"]} or config.get("current-context") != d["context"]:
                raise AnchorError("kubeconfig target binding mismatch")
            endpoint = cluster["cluster"]
            credentials = user["user"]
            if set(endpoint) != {"server", "certificate-authority-data"} or set(credentials) != {"client-certificate-data", "client-key-data"}:
                raise AnchorError("external auth/TLS override forbidden")
            for value in [endpoint["certificate-authority-data"], *credentials.values()]:
                if not base64.b64decode(value, validate=True):
                    raise AnchorError("inline verified TLS material required")
            url = urlsplit(endpoint["server"])
            if url.scheme != "https" or url.username is not None or url.password is not None or url.path or url.query or url.fragment:
                raise AnchorError("invalid exact API URL")
            if ipaddress.ip_address(url.hostname) != ip or url.port != port:
                raise AnchorError("kubeconfig points to another API; zero API calls")
            canonical = "https://" + ("[" + str(ip) + "]" if ip.version == 6 else str(ip)) + ":" + str(port)
            if endpoint["server"] != canonical:
                raise AnchorError("noncanonical/ambiguous API URL")
        except (KeyError, TypeError, ValueError):
            raise AnchorError("invalid fixed kubeconfig; output redacted") from None
        return nodes

    def prefix(self):
        return ["kubectl", "--kubeconfig", str(self.config), "--context", self.data["context"]]

    def _uid_check(self, pending=False):
        # Validate the fixed context and CP endpoint BEFORE even the UID GET.
        self.local_check(pending=pending)
        uid_args = parse_kubectl_operation(["get", "namespace", "kube-system", "-o", "jsonpath={.metadata.uid}"])
        uid = self.runner(self.prefix() + list(uid_args.argv), text=True, timeout=30, env=self.environment()).strip()
        if not uid or (not pending and uid != self.data["kube_system_uid"]):
            raise AnchorError("kube-system UID mismatch; anchor never updated")
        self.local_check(pending=pending)
        return uid

    def check(self):
        self._uid_check()
        return self.data

    def kubectl(self, args, **kwargs):
        op = parse_kubectl_operation(args)  # BEFORE probe/Docker/UID
        self._uid_check()
        suffix = list(op.argv) + (["--"] + list(op.remote_argv) if op.remote_argv else [])
        return self.runner(self.prefix() + suffix, env=self.environment(), **kwargs)

    def diagnostic_logs(self, args, *, namespace_uid, pod_uid, container, owner_chain):
        """Parent-attested diagnostic only; consume via private Context Mode capture.

        owner_chain is the ordered controller chain, ending at a leased workload:
        each entry has kind/name/uid. No log text is returned, even on failure.
        Pre/post identity checks cannot make the Kubernetes log endpoint UID-atomic.
        """
        op = parse_owned_pod_logs(args)  # Invalid grammar/attestation: zero probes.
        kinds = {"ReplicaSet": "replicaset", "Deployment": "deployment",
                 "StatefulSet": "statefulset", "DaemonSet": "daemonset",
                 "Job": "job", "CronJob": "cronjob"}
        valid_uid = lambda value: isinstance(value, str) and bool(re.fullmatch(r"[A-Za-z0-9-]{1,128}", value))
        if (self.data["cluster"] != "aiforall-local-full" or container != op.container
                or not valid_uid(namespace_uid) or not valid_uid(pod_uid)
                or self.data.get("namespaceUids", {}).get("aiforall-local-full") != namespace_uid
                or not isinstance(owner_chain, (list, tuple)) or not 1 <= len(owner_chain) <= 3
                or any(not isinstance(owner, dict) or set(owner) != {"kind", "name", "uid"}
                       or not isinstance(owner["kind"], str) or owner["kind"] not in kinds or not _log_name(owner["name"])
                       or not valid_uid(owner["uid"]) for owner in owner_chain)):
            raise AnchorError("owned Pod log attestation required before any probe")
        owners = [dict(owner) for owner in owner_chain]  # Snapshot parent attestation.
        if (len({owner["uid"] for owner in owners} | {pod_uid}) != len(owners) + 1
                or owners[-1]["kind"] == "ReplicaSet"
                or not any(isinstance(entry, dict) and all(entry.get(key) == value for key, value in {
                    **owners[-1], "namespace": "aiforall-local-full", "context": self.data["context"],
                    "cluster": self.data["cluster"]}.items()) for entry in self.data.get("ownedResources", []))):
            raise AnchorError("owned workload chain required before any probe")
        try:
            namespace = self.kubectl(["get", "namespace", "aiforall-local-full", "-o",
                                      "jsonpath={.metadata.uid}"], text=True, timeout=30).strip()
            if namespace != namespace_uid:
                raise AnchorError("namespace identity changed")
            # Only identity, regular container names and controller references;
            # never fetch a full Pod, env, args, volumes or Secret objects.
            projection = ('jsonpath={.metadata.uid}{"\\n"}{.metadata.name}{"\\n"}'
                          '{.metadata.namespace}{"\\n"}')
            references = ('{range .metadata.ownerReferences[*]}{.kind}{"\\t"}{.name}'
                          '{"\\t"}{.uid}{"\\t"}{.controller}{"\\n"}{end}')
            targets = [{"kind": "Pod", "name": op.pod, "uid": pod_uid}, *owners]
            for index, expected in enumerate(targets):
                fields = projection + ('{.spec.containers[*].name}{"\\n"}' if index == 0 else '') + references
                value = self.kubectl(["get", "pod" if index == 0 else kinds[expected["kind"]],
                                      expected["name"], "-n", "aiforall-local-full", "-o", fields],
                                     text=True, timeout=30).splitlines()
                if value[:3] != [expected["uid"], expected["name"], "aiforall-local-full"]:
                    raise AnchorError("workload identity changed")
                if index == 0 and (len(value) < 4 or op.container not in value[3].split()):
                    raise AnchorError("regular container not owned")
                refs = [line.split("\t") for line in value[4 if index == 0 else 3:]]
                controllers = [ref[:3] for ref in refs if len(ref) == 4 and ref[3] == "true"]
                if index + 1 < len(targets):
                    parent = targets[index + 1]
                    if controllers != [[parent["kind"], parent["name"], parent["uid"]]]:
                        raise AnchorError("controller chain changed")
            self._uid_check()
            captured = self.runner(self.prefix() + list(op.argv), env=self.environment(),
                                   text=True, timeout=30, stdin=subprocess.DEVNULL)
            # No retry/adoption if the Pod disappeared or was replaced during capture.
            after = self.kubectl(["get", "pod", op.pod, "-n", "aiforall-local-full", "-o",
                                  "jsonpath={.metadata.uid}"], text=True, timeout=30).strip()
            if (after != pod_uid or not isinstance(captured, str)
                    or len(captured.encode("utf-8", errors="strict")) > op.limit_bytes
                    or len(captured.splitlines()) > op.tail):
                raise AnchorError("capture discarded")
            # Closed categories only: no match groups, excerpts or supplied strings.
            # This classifies evidence, not a universal regex-based secret scrubber.
            motifs = {"connection-refused": "connection refused", "connection-timeout": "connection timed out",
                      "dns-failure": "no such host", "tls-verification-failed": "certificate verify failed",
                      "permission-denied": "permission denied", "address-in-use": "address already in use",
                      "missing-file": "no such file or directory",
                      "configuration-initialization-failed": "error initializing configuration",
                      "configuration-missing-field": "missing field", "configuration-unknown-field": "unknown field",
                      "configuration-type-mismatch": "invalid type", "process-panicked": "panicked at"}
            folded = captured.casefold()
            result = tuple(label for label, motif in motifs.items() if motif in folded)
            if not result:
                raise AnchorError("unclassified capture discarded")
            return result
        except Exception:
            raise AnchorError("INCONCLUSIVE: owned Pod diagnostic unavailable; output redacted") from None

    def _envoy_admin_identity(self, pod, namespace_uid, pod_uid, owners, incarnation, deadline=None):
        """Identity projections only. Existing kubectl checks anchor BEFORE each UID API.

        The independent child timer closes the tunnel even if a metadata probe
        stalls. These pre/post checks cannot make a name-addressed forward UID-atomic.
        """
        def get(kind, name, projection, namespaced=True):
            timeout = min(5, _admin_remaining(deadline)) if deadline is not None else 5
            args = ["get", kind, name] + (["-n", "aiforall-local-full"] if namespaced else [])
            value = self.kubectl(args + ["-o", projection], text=True, timeout=timeout,
                                 stdin=subprocess.DEVNULL)
            if deadline is not None:
                _admin_remaining(deadline)
            if not isinstance(value, str) or len(value.encode("utf-8")) > 8192:
                raise AnchorError("Admin identity projection refused")
            return value

        if get("namespace", "aiforall-local-full", "jsonpath={.metadata.uid}", False).strip() != namespace_uid:
            raise AnchorError("Admin namespace changed")
        base = ('jsonpath={.metadata.uid}{"\\n"}{.metadata.name}{"\\n"}'
                '{.metadata.namespace}{"\\n"}')
        refs = ('{range .metadata.ownerReferences[*]}R{"\\t"}{.kind}{"\\t"}{.name}'
                '{"\\t"}{.uid}{"\\t"}{.controller}{"\\n"}{end}')
        containers = ('{.spec.containers[*].name}{"\\n"}{.spec.initContainers[*].name}{"\\n"}'
                      '{.spec.ephemeralContainers[*].name}{"\\n"}{.spec.containers[*].image}{"\\n"}'
                      '{.status.initContainerStatuses[*].name}{"\\n"}'
                      '{.status.ephemeralContainerStatuses[*].name}{"\\n"}'
                      '{range .status.containerStatuses[*]}S{"\\t"}{.name}{"\\t"}{.containerID}'
                      '{"\\t"}{.restartCount}{"\\t"}{.imageID}{"\\t"}{.state.running.startedAt}{"\\n"}{end}')
        targets = [("pod", pod, pod_uid), ("replicaset", owners[0]["name"], owners[0]["uid"]),
                   ("deployment", "envoy-mesh", owners[1]["uid"])]
        for index, (kind, name, uid) in enumerate(targets):
            lines = get(kind, name, base + (containers if index == 0 else "") + refs).splitlines()
            if lines[:3] != [uid, name, "aiforall-local-full"]:
                raise AnchorError("Admin workload identity changed")
            if index == 0:
                if lines[3:9] != ["envoy", "", "", _ENVOY_IMAGE, "", ""] or len(lines) < 10:
                    raise AnchorError("Admin Pod network has unexpected containers")
                status = lines[9].split("\t")
                expected = ["S", "envoy", incarnation["container_id"], str(incarnation["restart_count"]),
                            incarnation["image_id"]]
                if (status[:5] != expected or len(status) != 6
                        or not re.fullmatch(r"[0-9]{4}-[0-9T:.+-]{10,40}Z", status[5])):
                    raise AnchorError("Admin container incarnation changed or not running")
                lines = lines[10:]
            else:
                lines = lines[3:]
            expected_refs = [] if index == 2 else ["\t".join([
                "R", owners[index]["kind"], owners[index]["name"], owners[index]["uid"], "true"])]
            if lines != expected_refs:
                raise AnchorError("Admin controller chain changed")

    def envoy_admin_snapshot(self, *, pod, namespace_uid, pod_uid, container, owner_chain, incarnation):
        """Closed full-lab snapshot, never a general HTTP client or public tunnel.

        Caller must ensure no OAuth browser navigation is active. The ephemeral
        loopback TCP tunnel still exposes mutating Admin routes to local processes;
        this is NOT host isolation, CSRF protection or production authority.
        Missing counters remain MISSING/INCONCLUSIVE. Compare deltas only for the
        identical returned identity and restart_epoch, with monotone counters.
        """
        valid_uid = lambda value: isinstance(value, str) and bool(re.fullmatch(r"[A-Za-z0-9-]{1,128}", value))
        if (self.data["cluster"] != "aiforall-local-full"
                or self.data["context"] != "kind-aiforall-local-full" or container != "envoy"
                or not _log_name(pod) or not pod.startswith("envoy-mesh-")
                or not valid_uid(namespace_uid) or not valid_uid(pod_uid)
                or self.data.get("namespaceUids", {}).get("aiforall-local-full") != namespace_uid
                or not isinstance(owner_chain, (list, tuple)) or len(owner_chain) != 2
                or any(not isinstance(owner, dict) or set(owner) != {"kind", "name", "uid"}
                       or not _log_name(owner["name"]) or not valid_uid(owner["uid"]) for owner in owner_chain)
                or owner_chain[0]["kind"] != "ReplicaSet" or not owner_chain[0]["name"].startswith("envoy-mesh-")
                or owner_chain[1]["kind"] != "Deployment" or owner_chain[1]["name"] != "envoy-mesh"
                or not isinstance(incarnation, dict) or set(incarnation) != {"container_id", "restart_count", "image_id"}
                or not isinstance(incarnation["container_id"], str)
                or not re.fullmatch(r"(?:containerd|docker|cri-o)://[a-f0-9]{64}", incarnation["container_id"])
                or type(incarnation["restart_count"]) is not int or not 0 <= incarnation["restart_count"] <= 2**31 - 1
                or not isinstance(incarnation["image_id"], str)
                or not re.fullmatch(r"[A-Za-z0-9._:/@-]{0,400}sha256:[a-f0-9]{64}", incarnation["image_id"])):
            raise AnchorError("Admin attestation required before any probe")
        owners, incarnation = tuple(dict(owner) for owner in owner_chain), dict(incarnation)
        if (len({pod_uid, namespace_uid, *(owner["uid"] for owner in owners)}) != 4
                or not any(isinstance(entry, dict) and all(entry.get(key) == value for key, value in {
                    **owners[1], "namespace": "aiforall-local-full", "context": self.data["context"],
                    "cluster": self.data["cluster"]}.items()) for entry in self.data.get("ownedResources", []))):
            raise AnchorError("Admin owned Deployment chain required before any probe")
        marker = self.config.parent / ".envoy-admin"
        if marker.exists() or marker.is_symlink() or self.path.with_name(self.path.name + ".oauth-forward").exists():
            raise AnchorError("Admin/OAuth marker present; STOP without adoption")
        child = _EnvoyAdminForward()
        try:
            self.local_check()  # Includes private directory/ACL/hash; no UID API before anchoring.
            with os.fdopen(os.open(marker, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w", encoding="utf-8") as ownership:
                try:
                    _admin_marker_private(marker)
                    self._envoy_admin_identity(pod, namespace_uid, pod_uid, owners, incarnation)
                    child.start(self.prefix() + ["port-forward", "-n", "aiforall-local-full", "pod/" + pod,
                                                "--address", "127.0.0.1", ":9901"], self.environment())
                    json.dump({"task_id": self.data["task_id"], "pid": child.process.pid,
                               "pod_uid": pod_uid, "ownership": "live-child-handle-only"}, ownership)
                    ownership.flush()
                    port = child.ready()
                    self._envoy_admin_identity(pod, namespace_uid, pod_uid, owners, incarnation, child.deadline)
                    if child.mapping(complete=True) != port:
                        raise AnchorError("Admin forward mapping changed")
                    server = _admin_server_projection(_admin_get(port, False, child.deadline))
                    self._envoy_admin_identity(pod, namespace_uid, pod_uid, owners, incarnation, child.deadline)
                    if child.mapping(complete=True) != port:
                        raise AnchorError("Admin forward mapping changed")
                    counters = _admin_stats_projection(_admin_get(port, True, child.deadline))
                    self._envoy_admin_identity(pod, namespace_uid, pod_uid, owners, incarnation, child.deadline)
                    if child.mapping(complete=True) != port:
                        raise AnchorError("Admin forward mapping changed")
                    _admin_remaining(child.deadline)
                    result = {**server, "identity": {"namespace_uid": namespace_uid, "pod": pod,
                        "pod_uid": pod_uid, "owner_chain": owners, "container": "envoy", **incarnation},
                        "stats": counters, "status": "INCONCLUSIVE" if "MISSING" in counters.values() else "COMPLETE"}
                finally:
                    child.close()
                    ownership.close()
                    marker.unlink()  # Only our exclusive marker, only after attested cleanup/non-launch.
                child.mapping(complete=True, exited=True)  # Include output drained during shutdown.
        except Exception:
            raise AnchorError("INCONCLUSIVE: Envoy Admin snapshot unavailable; output redacted; inspect marker if present") from None
        return result

    def kind_load(self, image):
        if not re.fullmatch(r"[a-z0-9./:_-]+(?::sha256-[a-f0-9]{64}|@sha256:[a-f0-9]{64})", image):
            raise AnchorError("immutable image reference required")
        self._uid_check()
        if any(not n["running"] for n in self.local_check()):
            raise AnchorError("image import requires all leased nodes running")
        self.runner(["kind", "load", "docker-image", image, "--name", self.data["cluster"]], text=True, timeout=180, env=self.environment())

    @contextlib.contextmanager
    def oauth_forward(self, namespace="aiforall-local-full", service="oauth-idp-simulator",
                      bind="127.0.0.1", mapping="18081:8080", *, launcher=subprocess.Popen):
        """One foreground, owned child only; no generic kubectl forward authority.

        A stale marker is a STOP requiring parent inspection, never PID adoption.
        Cleanup uses the child handle created in this invocation, not a PID lookup.
        """
        if (self.data["cluster"] != "aiforall-local-full" or namespace != "aiforall-local-full"
                or service != "oauth-idp-simulator" or bind != "127.0.0.1" or mapping != "18081:8080"):
            raise AnchorError("unsupported OAuth forward target before any probe")
        marker = self.path.with_name(self.path.name + ".oauth-forward")
        process = None
        # Exclusive lifetime marker also prevents two child creators sharing a lease.
        with marker.open("x", encoding="utf-8") as ownership:
            try:
                self.check()
                namespace_uid = self.kubectl(["get", "namespace", namespace, "-o", "jsonpath={.metadata.uid}"], text=True, timeout=30).strip()
                if not namespace_uid or self.data.get("namespaceUids", {}).get(namespace) != namespace_uid:
                    raise AnchorError("OAuth forward namespace UID not owned")
                value = json.loads(self.kubectl(["get", "service", service, "-n", namespace, "-o", "json"], text=True, timeout=30))
                uid = value.get("metadata", {}).get("uid")
                if not uid or not any(entry.get("kind") == "Service" and entry.get("name") == service
                                      and entry.get("namespace") == namespace and entry.get("uid") == uid
                                      and entry.get("context") == self.data["context"]
                                      and entry.get("cluster") == self.data["cluster"]
                                      for entry in self.data.get("ownedResources", [])):
                    raise AnchorError("OAuth forward Service UID not owned")
                spec = value.get("spec", {})
                ports = spec.get("ports", [])
                if (spec.get("type", "ClusterIP") != "ClusterIP"
                        or spec.get("selector") != {"app.kubernetes.io/name": service}
                        or len(ports) != 1 or ports[0].get("port") != 8080
                        or ports[0].get("targetPort") != 8080 or ports[0].get("protocol", "TCP") != "TCP"):
                    raise AnchorError("OAuth forward Service binding differs")
                with socket.socket() as probe:
                    probe.bind((bind, 18081))  # Occupied => STOP, never kill owner.
                self.check()
                command = self.prefix() + ["port-forward", "-n", namespace, "service/" + service,
                                            "--address", bind, mapping]
                process = launcher(command, env=self.environment(), stdin=subprocess.DEVNULL,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                   creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0)
                json.dump({"task_id": self.data["task_id"], "context": self.data["context"],
                           "service_uid": uid, "namespace_uid": namespace_uid,
                           "pid": process.pid, "bind": bind, "mapping": mapping,
                           "ownership": "live-child-handle-only"}, ownership)
                ownership.flush()
                yield process
            finally:
                if process is not None:
                    if process.poll() is None:
                        if os.name == "nt":
                            process.send_signal(signal.CTRL_BREAK_EVENT)
                        else:
                            process.terminate()  # SIGTERM to sole child handle, no PID adoption.
                    try:
                        process.wait(timeout=15)
                    except subprocess.TimeoutExpired:
                        raise AnchorError("owned OAuth forward stop blocked; parent must inspect marker") from None
                ownership.close()
                marker.unlink()  # Our marker only, after cleanup (or before any child existed).


def persist(path, data):
    temporary = path.with_name(path.name + ".anchor-update")
    with temporary.open("x", encoding="utf-8") as file:
        json.dump(data, file, indent=2)
    temporary.replace(path)


def bootstrap(path, cluster, config, after_final_it=False, runner=run, root=None, privacy=private_file):
    target = LocalClusterTargetAnchor(path, runner, root, privacy, cluster)
    d = target.data
    if d.get("anchor_state") == "anchored":
        target.check()
        return d
    prior_anchor = any(d.get(key) for key in ["kube_system_uid", "control_plane_container_id", "api_host", "api_port", "kubeconfig_sha256"])
    if not after_final_it or d.get("anchor_state") != "pending_creation" or d.get("nodeContainerIds") or prior_anchor or d.get("creation") or target.config.exists():
        raise AnchorError("new pending lease and explicit final-IT completion required; never adopt/export")
    if target.roster():
        raise AnchorError("existing cluster cannot initialize a pending anchor")
    cfg = yaml.safe_load(Path(config).read_text())
    if cfg.get("name") != cluster or len(cfg.get("nodes", [])) != (3 if cluster == "aiforall-local-full" else 1):
        raise AnchorError("creation config target/topology mismatch")
    # Reserve-check host HTTP endpoint without stopping its unknown owner.
    if cluster == "aiforall-local-full":
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 18080))
    before = runner(["docker", "ps", "-aq", "--no-trunc"], text=True, timeout=30).split()
    target.config.parent.mkdir(mode=0o700)  # exclusive directory; no existing inputs
    secure(target.config.parent, directory=True)
    d["creation"] = {"task_id": d["task_id"], "before_ids": before, "phase": "creating"}
    persist(target.path, d)
    failure = None
    try:
        runner(["kind", "create", "cluster", "--name", cluster, "--config", str(config), "--kubeconfig", str(target.config), "--wait", "0"], text=True, timeout=900, env=target.environment())
    except Exception as error:
        failure = error
    ids = target.roster()
    created = [i for i in ids if i not in before]
    d["nodeContainerIds"] = created
    d["creation"]["created_ids"] = created
    d["creation"]["phase"] = "failed-partial" if failure else "endpoint-pending"
    persist(target.path, d)
    if failure or set(ids) != set(created):
        raise AnchorError("creation failed/foreign delta; partial IDs retained, no retry/reset")
    nodes = [target.metadata(i) for i in created]
    d["resourceStates"] = [{"id": n["id"], "context": d["context"], "cluster": cluster, "priorState": "absent", "state": "running" if n["running"] else "stopped", "action": "created"} for n in nodes]
    persist(target.path, d)
    cp = [n for n in nodes if n.get("role") == "control-plane"]
    if len(cp) != 1:
        raise AnchorError("created control-plane ambiguous")
    mappings = (cp[0].get("ports") or {}).get("6443/tcp")
    if not isinstance(mappings, list) or len(mappings) != 1:
        raise AnchorError("created API publication ambiguous")
    d["control_plane_container_id"] = cp[0]["id"]
    d["api_host"], d["api_port"] = mappings[0]["HostIp"], int(mappings[0]["HostPort"])
    secure(target.config)
    d["kubeconfig_sha256"] = hashlib.sha256(target.config.read_bytes()).hexdigest()
    persist(target.path, d)  # candidate/partial metadata, never an adopted UID
    uid = target._uid_check(pending=True)
    d["kube_system_uid"], d["anchor_state"] = uid, "anchored"
    d["creation"]["phase"] = "anchored"
    d["resourceStates"] = [{"id": n["id"], "context": d["context"], "cluster": cluster, "priorState": "absent", "state": "running", "action": "created"} for n in nodes]
    persist(target.path, d)
    return d


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["check", "kubectl", "kind-load", "bootstrap", "oauth-forward"])
    parser.add_argument("--lease", type=Path, required=True)
    parser.add_argument("--cluster", choices=list(ALLOWED), required=True)
    parser.add_argument("--config", type=Path)
    parser.add_argument("--after-final-it", action="store_true")
    args, arguments = parser.parse_known_args()
    try:
        if args.action == "bootstrap":
            bootstrap(args.lease, args.cluster, args.config, args.after_final_it)
        else:
            target = LocalClusterTargetAnchor(args.lease, expected_cluster=args.cluster)
            rest = arguments[1:] if arguments[:1] == ["--"] else arguments
            if args.action == "check":
                target.check()
            elif args.action == "kind-load":
                if len(rest) != 1:
                    raise AnchorError("one immutable image argument required")
                target.kind_load(rest[0])
            elif args.action == "oauth-forward":
                if rest or args.config or args.after_final_it:
                    raise AnchorError("OAuth forward accepts only its fixed target and parent lease")
                with target.oauth_forward() as process:
                    try:
                        if process.wait() != 0:
                            raise AnchorError("owned OAuth forward failed")
                    except KeyboardInterrupt:
                        pass  # Context manager stops only this invocation's child.
            else:
                # PowerShell pipelines are passed through without argument/JSON
                # logging; callers only consume known metadata/object names.
                output = target.kubectl(rest, text=True, input=sys.stdin.read(), timeout=600)
                sys.stdout.write(output)
    except Exception:
        print("STOP: target anchor validation/operation failed (redacted); no adoption/reset", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
