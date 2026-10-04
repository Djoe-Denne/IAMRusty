"""S-11 single PS/Python authority: alias is never API identity. No ambient config."""
import argparse
import base64
import hashlib
import ipaddress
import json
import os
import re
import socket
import stat
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

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
    result = subprocess.run(command, capture_output=True, check=False, **kwargs)
    if result.returncode:
        raise AnchorError("target operation failed; output redacted")
    return result.stdout


def approved_root():
    return Path.home() / "AppData/Local/Temp/opencode" if os.name == "nt" else Path.home() / ".cache/opencode"


def private_file(path):
    """Read-only file, owned/private directory; no raw ACL/credentials emitted."""
    if os.name != "nt":
        return (path.stat().st_uid == os.getuid() and path.parent.stat().st_uid == os.getuid()
                and not path.stat().st_mode & 0o222 and not path.stat().st_mode & 0o077
                and not path.parent.stat().st_mode & 0o077)
    script = "$p=$args[0];$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value;$ok=$true;foreach($f in @($p,(Split-Path -Parent $p))){$a=Get-Acl -LiteralPath $f;if($a.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $sid){$ok=$false};foreach($r in $a.Access){if($r.AccessControlType -eq 'Allow' -and $r.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value -notin @($sid,'S-1-5-18')){$ok=$false}}};if(-not (Get-Item -LiteralPath $p).IsReadOnly){$ok=$false};if($ok){'yes'}else{'no'}"
    return run(["powershell.exe", "-NoProfile", "-Command", script, str(path)], text=True, timeout=30).strip() == "yes"


def secure(path, directory=False):
    if os.name != "nt":
        path.chmod(0o700 if directory else 0o400)
        return
    # No inherited broad ACE, no Administrators/Everyone grants. SYSTEM + task
    # user only; generated file is read-only and no longer writable by task tools.
    script = "$p=$args[0];$dir=$args[1] -eq 'dir';$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User;$a=if($dir){New-Object Security.AccessControl.DirectorySecurity}else{New-Object Security.AccessControl.FileSecurity};$a.SetOwner($sid);$a.SetAccessRuleProtection($true,$false);$rights=if($dir){'FullControl'}else{'ReadAndExecute'};$a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,$rights,'Allow'));$a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.SecurityIdentifier]::new('S-1-5-18'),'FullControl','Allow'));Set-Acl -LiteralPath $p -AclObject $a;if(-not $dir){(Get-Item -LiteralPath $p).IsReadOnly=$true}"
    run(["powershell.exe", "-NoProfile", "-Command", script, str(path), "dir" if directory else "file"], text=True, timeout=30)


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

    def kind_load(self, image):
        if not re.fullmatch(r"[a-z0-9./:_-]+(?::sha256-[a-f0-9]{64}|@sha256:[a-f0-9]{64})", image):
            raise AnchorError("immutable image reference required")
        self._uid_check()
        if any(not n["running"] for n in self.local_check()):
            raise AnchorError("image import requires all leased nodes running")
        self.runner(["kind", "load", "docker-image", image, "--name", self.data["cluster"]], text=True, timeout=180, env=self.environment())


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
    parser.add_argument("action", choices=["check", "kubectl", "kind-load", "bootstrap"])
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
