"""Owned local Zot + OCI publication. No Kind, build, login or global trust changes."""
import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import http.client
import importlib.util
import json
import os
from pathlib import Path
import re
import socket
import ssl
import subprocess
import tarfile
import tempfile
import time
import uuid

from image_catalog import FRESHNESS_POLICY, TARGETS
from image_sources import ROOT, capture, identity, require_unchanged, target_identity
from local_full_image_provenance import BOUNDARIES, verify_labels

AUTHORITY = "aiforall-apps-registry:5000"
LOCK = ROOT / "ops/deploy/registry/transport.lock.json"
DECLARATIONS = LOCK.parent
SPEC = importlib.util.spec_from_file_location("registry_pki", ROOT / "ops/scripts/prepare-local-full-certs.py")
pki = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(pki)


def digest(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def refresh_inputs(snapshot):
    # Conservative guard: only ordinary Rust code may vary during image-only
    # refresh. Recipes, assets, build scripts, manifests and migrations may not.
    return "sha256:" + identity([entry for entry in snapshot["files"]
        if not entry["path"].endswith(".rs") or entry["path"].endswith("/build.rs")
        or any(part in {"migration", "migrations"} for part in Path(entry["path"]).parts)])


def run(args, timeout=60, env=None):
    from private_diagnostics import capture_failure
    try:
        result = subprocess.run(args, capture_output=True, timeout=timeout, env=env)
    except subprocess.TimeoutExpired as error:
        capture_failure(args, error.output, error.stderr, timeout=True)
        raise
    if result.returncode:
        capture_failure(args, result.stdout, result.stderr, result.returncode)
        raise ValueError("registry operation failed (exit " + str(result.returncode) + "); native output withheld")
    return result.stdout.decode("utf-8")


def record(kind, identity, owner, event="created", safe="no", intended=None):
    now = datetime.now(timezone.utc)
    directory = ROOT / ".tmp"
    directory.mkdir(exist_ok=True)
    with (directory / ("docker-resources-" + now.strftime("%Y%m%d") + ".log")).open("a", encoding="utf-8") as stream:
        stream.write(json.dumps({"timestamp": now.isoformat(), "event": event, "kind": kind,
            "id": identity, "owner": owner, "safe_to_delete": safe, "source": "local_registry", "intended_name": intended}) + "\n")
        stream.flush()
        os.fsync(stream.fileno())


def fingerprint(item):
    """Immutable inspect preimage; exclude runtime addresses, PID and counters."""
    keys = ("Config", "HostConfig", "Mounts", "Image") if "Config" in item else (
        "Id", "Name", "Created", "CreatedAt", "Labels", "Driver", "Options", "IPAM", "Internal", "Scope")
    return digest(json.dumps({key: item.get(key) for key in keys}, sort_keys=True).encode())


def bind_source(path):
    path = path.replace("\\", "/")
    if re.match(r"^[A-Za-z]:/", path):
        return "/" + path[0].lower() + path[2:]
    for prefix in ("/run/desktop/mnt/host/", "/host_mnt/"):
        if path.startswith(prefix):
            return "/" + path.removeprefix(prefix)
    return path


def artifact_hashes(archive, target):
    """Read only expected regular binaries, applying layer whiteouts; never extract."""
    outputs = TARGETS[target]["outputs"]
    paths = {}
    for package, binary in outputs.items():
        path = ("usr/local/bin/apparatus-controller" if target == "controller" else
                "app/worker" if target == "sentinel" else
                "app/migration" if package.endswith("-migration") else "app/" + binary)
        paths[path] = (package, binary)
    found = {}
    with tarfile.open(archive) as outer:
        manifests = json.load(outer.extractfile("manifest.json"))
        if len(manifests) != 1:
            raise ValueError("single-image Docker archive required")
        for layer in manifests[0]["Layers"]:
            added = {}
            with tarfile.open(fileobj=outer.extractfile(layer), mode="r|*") as inner:
                for member in inner:
                    name = member.name.removeprefix("./").lstrip("/")
                    parent, _, leaf = name.rpartition("/")
                    if leaf.startswith(".wh."):
                        removed = parent + "/" + leaf[4:]
                        for old in list(found):
                            if (leaf == ".wh..wh..opq" and old.startswith(parent + "/")) or old == removed or old.startswith(removed + "/"):
                                del found[old]
                    if name in paths:
                        if not member.isfile():
                            raise ValueError("exported application binary must be a regular file")
                        with inner.extractfile(member) as stream:
                            added[name] = "sha256:" + hashlib.file_digest(stream, "sha256").hexdigest()
            found.update(added)
    if set(found) != set(paths):
        raise ValueError("application binary missing from exported filesystem")
    return [{"package": paths[path][0], "binary": paths[path][1], "sha256": found[path]} for path in sorted(paths)]


class Registry:
    def __init__(self, directory, create=False):
        self.directory = pki.checked_path(directory)
        self.lock = json.loads(LOCK.read_text(encoding="utf-8"))
        if create and not self.directory.exists():
            pki.private_directory(self.directory, create=True)
            context = run(["docker", "context", "show"]).strip()
            self.state = {"schema": "aiforall.local-registry/v1", "owner": uuid.uuid4().hex,
                          "context": context, "helpers": [], "port": None, "compose": {"receipts": {}}}
            engine, platform = self.docker(["info", "--format", "{{.ID}}|{{.OSType}}"], 20).strip().split("|")
            if platform != "linux":
                raise ValueError("Linux Docker engine required")
            self.state["engine"] = engine
            self.save()
        else:
            pki.private_directory(self.directory)
            self.state = json.loads((self.directory / "state.json").read_text(encoding="utf-8"))
            if self.state.get("schema") != "aiforall.local-registry/v1" or not re.fullmatch(r"[a-f0-9]{32}", self.state.get("owner", "")):
                raise ValueError("unsupported registry ownership state")
            if self.docker(["info", "--format", "{{.ID}}"], 20).strip() != self.state["engine"]:
                raise ValueError("registry Docker engine changed")

    def docker(self, args, timeout=60):
        return run(["docker", "--context", self.state["context"], *args], timeout)

    def save(self):
        path = self.directory / "state.new"
        with path.open("w", encoding="utf-8") as stream:
            json.dump(self.state, stream)
            stream.flush()
            os.fsync(stream.fileno())
        path.chmod(0o600)
        os.replace(path, self.directory / "state.json")

    @contextmanager
    def operation(self):
        marker = self.directory / "operation.lock"
        marker.open("x").close()
        try:
            yield
        finally:
            marker.unlink()

    def inspect(self, kind, identity):
        values = json.loads(self.docker([kind, "inspect", identity]))
        if len(values) != 1:
            raise ValueError("ambiguous Docker identity")
        return values[0]

    def owned(self, identity):
        if identity not in [self.state.get("container"), *self.state.get("helpers", [])]:
            raise ValueError("foreign registry container refused")
        return self.receipt("container", identity, self.inspect("container", identity))

    def receipt(self, kind, identity, item):
        transport = self.state.get("transport", {})
        labels = item["Config"].get("Labels", {}) if kind == "container" else item.get("Labels", {})
        if item.get("Id", item.get("Name")) != identity or labels.get("aiforall.registry.owner") != self.state["owner"]:
            raise ValueError("foreign registry identity refused")
        if labels.get("aiforall.registry.role") == "publisher" and identity not in transport.get("helpers", []):
            raise ValueError("transport creation ACK lost")
        ledger = transport if identity in transport.get("helpers", []) else self.state.get("compose")
        if ledger is not None and ledger.get("receipts", {}).get(kind + ":" + identity) != fingerprint(item):
            raise ValueError("Compose resource drift or lost creation ACK")
        return item

    def declarations(self, *ledgers):
        sources = {p.name: digest(p.read_bytes()) for p in DECLARATIONS.iterdir() if p.suffix in {".json", ".yaml", ".sh"}}
        for ledger in filter(None, ledgers):
            if ledger.get("pending"):
                raise ValueError("incomplete registry operation; no automatic retry/adoption")
            if (ledger["receipts"] and not ledger.get("sources")) or ledger.get("sources", sources) != sources:
                raise ValueError("registry declarations changed; explicit reconfiguration required")
            ledger["sources"] = sources

    def compose(self, filename="compose.yaml", **inputs):
        ledger = self.state["compose"] if filename == "compose.yaml" else self.state.setdefault("transport", {"receipts": {}, "helpers": []})
        self.declarations(ledger)
        env = {k: v for k, v in os.environ.items() if not k.startswith(("COMPOSE_", "REGISTRY_"))}
        project = "aiforall-images-" + self.state["owner"] + inputs.pop("suffix", "")
        env.update(REGISTRY_PROJECT=project, REGISTRY_OWNER=self.state["owner"],
                   REGISTRY_CERTS=self.state["cert_directory"], REGISTRY_PORT=str(self.state["port"]), **inputs)
        command = ["docker", "--context", self.state["context"], "compose", "--env-file", os.devnull,
                   "-p", project, "-f", str(DECLARATIONS / filename), "--profile", "transport"]
        model = json.loads(run([*command, "config", "--format", "json"], env=env))
        return command, env, model

    def service_check(self, item, service, model):
        config, host = item["Config"], item["HostConfig"]
        image = self.inspect("image", service["image"])
        expected = {v["target"]: (v["type"], bind_source(v["source"]) if v["type"] == "bind" else model["volumes"][v["source"]]["name"],
                    not v.get("read_only", False)) for v in service.get("volumes", [])}
        mounted = {m["Destination"]: (m["Type"], m.get("Name") if m["Type"] == "volume" else bind_source(m["Source"]), m["RW"]) for m in item["Mounts"]}
        network = service.get("network_mode") or next(model["networks"][name]["name"] for name in service["networks"])
        if (service["image"] not in image.get("RepoDigests", []) or item["Image"] != image["Id"]
                or config["User"] != service.get("user", "") or not host["ReadonlyRootfs"]
                or host.get("Privileged", False) or host.get("NetworkMode") != network
                or set(host.get("CapDrop") or []) != set(service.get("cap_drop", []))
                or set(host.get("CapAdd") or []) != set(service.get("cap_add", []))
                or not any(s in {"no-new-privileges", "no-new-privileges:true"} for s in host.get("SecurityOpt", []))
                or mounted != expected or ("command" in service and config.get("Cmd") != service["command"])
                or ("entrypoint" in service and config.get("Entrypoint") != service["entrypoint"])):
            raise ValueError("created Compose service differs from declaration")

    def compose_create(self, filename="compose.yaml", **inputs):
        """Create-only transaction: absent preimage + daemon creation events + private ACK."""
        command, env, model = self.compose(filename, **inputs)
        ledger = self.state["compose"] if filename == "compose.yaml" else self.state["transport"]
        if filename != "compose.yaml" and (set(model["services"]) != {"publisher"} or model.get("volumes")
                or set(model["networks"]) != {"registry"} or not model["networks"]["registry"].get("external")
                or model["networks"]["registry"]["name"] != self.inspect("network", self.state["network"])["Name"]):
            raise ValueError("transport must only use the verified external network")
        wanted = {("container", v["container_name"]): (key, v) for key, v in model["services"].items()}
        wanted.update({(kind, v["name"]): (key, v) for kind in ("volume", "network")
                       for key, v in model.get(kind + "s", {}).items() if not v.get("external")})
        for kind in ("container", "network", "volume"):
            rows = self.docker([kind, "ls", *(["-a"] if kind == "container" else []), "--format", "{{json .}}"])
            if any((kind, v.get("Names", v.get("Name"))) in wanted or
                   "com.docker.compose.project=" + env["REGISTRY_PROJECT"] in v.get("Labels", "").split(",")
                   for v in map(json.loads, rows.splitlines())):
                raise ValueError("Compose name/project occupied; adoption/recreation refused")
        pins = {self.lock[k]["reference"]: k for k in ("zot", "skopeo")}
        for reference in {service["image"] for service in model["services"].values()}:
            if reference not in pins:
                raise ValueError("Compose transport pin differs from lock")
            self.image(pins[reference])
        since = self.docker(["info", "--format", "{{.SystemTime}}"], 20).strip()
        ledger["pending"] = True
        self.save()
        for kind, name in wanted:
            record(kind, "UNKNOWN", self.state["owner"], safe="ask", intended=name)
        try:
            run([*command, "create", "--no-recreate", "--no-build", "--pull", "never"], 120, env=env)
        finally:
            until = self.docker(["info", "--format", "{{.SystemTime}}"], 20).strip()
            events = [json.loads(line) for line in self.docker(["events", "--since", since, "--until", until,
                "--filter", "event=create", "--format", "{{json .}}"], 30).splitlines()]
            observed = []
            for kind, name in wanted:
                matches = [e for e in events if e.get("Type") == kind and e.get("Action") == "create"
                    and (e["Actor"].get("Attributes", {}).get("name") == name or (kind == "volume" and e["Actor"]["ID"] == name))]
                for event in matches:
                    record(kind, event["Actor"]["ID"], self.state["owner"], event="observed", safe="ask", intended=name)
                if len(matches) != 1:
                    continue  # Opaque/partial creation stays UNKNOWN/ask, never adopted on retry.
                observed.append((kind, name, matches[0]["Actor"]["ID"]))
            for kind, name, identity in observed:
                key, declaration = wanted[kind, name]
                item = self.inspect(kind, identity)
                labels = item["Config"].get("Labels", {}) if kind == "container" else item.get("Labels", {})
                if labels.get("aiforall.registry.owner") != self.state["owner"] or labels.get("com.docker.compose.project") != env["REGISTRY_PROJECT"]:
                    raise ValueError("Compose creation identity mismatch")
                if item["Name"].lstrip("/") != name or (kind != "volume" and (not re.fullmatch(r"[a-f0-9]{64}", identity) or item["Id"] != identity)):
                    raise ValueError("invalid Compose creation ID")
                ledger["receipts"][kind + ":" + identity] = fingerprint(item)
                if kind == "container":
                    if item["State"]["Status"] != "created":
                        raise ValueError("Compose container started before creation ACK")
                    self.service_check(item, declaration, model)
                    if labels["com.docker.compose.service"] != key:
                        raise ValueError("Compose service identity mismatch")
                    if key == "registry":
                        self.state["container"] = identity
                    else:
                        self.state["helpers"].append(identity)
                        if filename != "compose.yaml":
                            ledger["helpers"].append(identity)
                else:
                    if (item["Driver"] != declaration.get("driver", "local" if kind == "volume" else "bridge")
                            or (item.get("Options") or {}) != declaration.get("driver_opts", {})
                            or item.get("Internal", False) != declaration.get("internal", False)
                            or any(cid not in [self.state.get("container"), *self.state["helpers"]] for cid in item.get("Containers", {}))):
                        raise ValueError("created Compose storage/network drift/foreign member")
                    self.state["network" if kind == "network" else key] = identity
                self.save()
                record(kind, identity, self.state["owner"], event="resolved", intended=name)
        if len(observed) != len(wanted):
            raise ValueError("Compose creation incomplete; resources retained, retry refused")
        ledger["pending"] = filename == "compose.yaml"  # Fresh storage remains pending until permissions succeed.
        self.save()

    def stop_container(self, identity):
        item = self.owned(identity)
        if item["State"]["Running"]:
            self.docker(["container", "stop", "--time", "15", identity], 30)
        if self.owned(identity)["State"]["Running"]:
            raise ValueError("owned container did not stop")
        record("container", identity, self.state["owner"], event="stopped")

    def helper(self, filename="compose.yaml", preimage=None, **inputs):
        previous = set(self.state["helpers"])
        try:
            self.compose_create(filename, **inputs)
            if preimage is not None:
                self.unchanged(preimage)
            for identity in set(self.state["helpers"]) - previous:
                if self.owned(identity)["State"].get("Status") != "created":
                    raise ValueError("helper already executed; replay refused")
                self.docker(["container", "start", "--attach", identity], 1800)
                item = self.owned(identity)
                if item["State"]["Running"] or item["State"]["ExitCode"] != 0:
                    raise ValueError("registry helper failed; output withheld")
        finally:
            for identity in set(self.state["helpers"]) - previous:
                try:
                    self.stop_container(identity)
                except (ValueError, OSError, subprocess.TimeoutExpired):
                    self.state["transport" if filename != "compose.yaml" else "compose"]["pending"] = True
                    self.save()
                    record("container", identity, self.state["owner"], event="stop-failed", safe="ask")
                    raise
            if preimage is not None and not self.state["transport"].get("pending"):
                self.unchanged(preimage)

    def unchanged(self, before):
        self.check()
        if self.preimage != before:
            self.state["transport"]["pending"] = True
            self.save()
            raise ValueError("registry/network/volumes changed during transport")

    def image(self, key):
        pin = self.lock[key]
        if not re.fullmatch(r"[a-z0-9./_-]+@sha256:[a-f0-9]{64}", pin["reference"]):
            raise ValueError("immutable transport image pin required")
        existing = subprocess.run(["docker", "--context", self.state["context"], "image", "inspect", pin["reference"]], capture_output=True)
        if existing.returncode:
            record("image", "UNKNOWN", self.state["owner"], safe="ask")
            self.docker(["image", "pull", "--platform", "linux/amd64", pin["reference"]], 600)
            value = self.inspect("image", pin["reference"])
            record("image", value["Id"], self.state["owner"])
        else:
            value = json.loads(existing.stdout)[0]
        # containerd-backed Docker identifies images by manifest/index, while
        # classic Docker uses the config digest. Bind to RepoDigests, not an
        # assumed meaning of .Id.
        if pin["reference"] not in value.get("RepoDigests", []) or value["Os"] != "linux" or value["Architecture"] != "amd64":
            raise ValueError("transport image pin/platform mismatch")
        self.state.setdefault("images", {})[key] = value["Id"]
        self.save()
        return pin["reference"]

    def certificates(self, initialize=False):
        directory = pki.checked_path(self.state["cert_directory"])
        pki.private_directory(directory)
        if initialize:
            pki.validate_registry(pki.OpenSSL(pki.openssl_path(None), directory), directory)
        files = {"ca": directory / "mesh/ca.crt", **{name: directory / "extensions/registry" / (name + ".crt")
                 for name in pki.registry_profiles()}}
        actual = {name: digest(path.read_bytes()) for name, path in files.items()}
        if initialize:
            self.state["certificates"] = actual
        if actual != self.state["certificates"]:
            raise ValueError("registry certificates changed; explicit rotation required")
        return directory

    def check(self):
        self.declarations(self.state.get("compose"), self.state.get("transport"))
        self.certificates()
        item = self.owned(self.state["container"])
        self.preimage = {("container", item["Id"]): fingerprint(item)}
        pinned = self.inspect("image", self.lock["zot"]["reference"])
        expected = {self.state["data"], self.state["runtime"]}
        mounted = {m["Name"] for m in item["Mounts"] if m["Type"] == "volume"}
        bindings = item["HostConfig"].get("PortBindings", {}).get("5000/tcp")
        if (self.lock["zot"]["reference"] not in pinned.get("RepoDigests", [])
                or item["Image"] != pinned["Id"] or item["Config"]["User"] != "10001:10001"
                or mounted != expected or bindings != [{"HostIp": "127.0.0.1", "HostPort": str(self.state["port"])}]):
            raise ValueError("registry container configuration changed")
        for volume in expected:
            value = self.inspect("volume", volume)
            self.preimage["volume", volume] = fingerprint(value)
            self.receipt("volume", volume, value)
        network = self.inspect("network", self.state["network"])
        self.preimage["network", self.state["network"]] = fingerprint(network)
        self.receipt("network", self.state["network"], network)
        known = {self.state["container"], *self.state["helpers"]}
        if any(identity not in known for identity in network.get("Containers", {})):
            raise ValueError("unexpected container on registry network")
        if item["State"]["Running"] and self.state["container"] not in network.get("Containers", {}):
            raise ValueError("registry detached from its network")
        if any(self.owned(identity)["State"]["Running"] for identity in self.state["helpers"]):
            raise ValueError("another registry helper is active")
        return item

    def request(self, path, role="registry-reader", method="GET", body=None, headers=None):
        directory = self.certificates()
        context = ssl.create_default_context(cafile=str(directory / "mesh/ca.crt"))
        bundle = directory / "extensions/registry"
        context.load_cert_chain(str(bundle / (role + ".crt")), str(bundle / (role + ".key")))
        connection = http.client.HTTPSConnection("127.0.0.1", self.state["port"], context=context, timeout=15)
        try:
            connection.request(method, path, body=body, headers=headers or {})
            response = connection.getresponse()
            data = response.read(4 * 1024 * 1024 + 1)
            if len(data) > 4 * 1024 * 1024:
                raise ValueError("registry metadata response too large")
            return response.status, data, response.getheader("Docker-Content-Digest")
        finally:
            connection.close()

    def up(self, cert_directory, port):
        if self.state.get("container"):
            if port != self.state["port"] or (cert_directory is not None and str(pki.checked_path(cert_directory)) != self.state["cert_directory"]):
                raise ValueError("registry bootstrap inputs changed; explicit rotation/reconfiguration required")
            item = self.check()
            if not item["State"]["Running"]:
                self.docker(["container", "start", item["Id"]])
                record("container", item["Id"], self.state["owner"], event="started")
        else:
            if self.state.get("network") or "compose" not in self.state:
                raise ValueError("incomplete setup retained; inspect its IDs before retrying with a new state directory")
            directory = pki.checked_path(cert_directory)
            self.state.update(cert_directory=str(directory), port=port)
            self.certificates(initialize=True)
            with socket.socket() as probe:
                if os.name == "nt":
                    probe.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
                probe.bind(("127.0.0.1", port))
            for identity in self.docker(["container", "ls", "-aq", "--no-trunc"]).splitlines():
                occupied = self.inspect("container", identity)["HostConfig"].get("PortBindings") or {}
                if any(binding["HostPort"] == str(port) for values in occupied.values() for binding in (values or [])):
                    raise ValueError("registry port reserved by an existing container")
            self.helper()
            self.state["compose"]["pending"] = False
            self.save()
            self.check()
            self.docker(["container", "start", self.state["container"]])
        deadline = time.monotonic() + 30
        while True:
            self.check()
            try:
                if self.request("/v2/")[0] == 200:
                    return
            except (OSError, http.client.HTTPException):
                pass
            if time.monotonic() >= deadline:
                raise ValueError("registry mTLS readiness failed")
            time.sleep(0.25)

    def remote(self, repo, manifest_digest, local=None, include_config=False):
        status, body, header = self.request("/v2/" + repo + "/manifests/" + manifest_digest,
            headers={"Accept": "application/vnd.oci.image.manifest.v1+json"})
        if status != 200 or digest(body) != manifest_digest or header != manifest_digest:
            raise ValueError("published OCI manifest identity mismatch")
        manifest = json.loads(body)
        if manifest.get("mediaType") != "application/vnd.oci.image.manifest.v1+json":
            raise ValueError("single-platform OCI manifest required")
        config_digest = manifest["config"]["digest"]
        status, body, _ = self.request("/v2/" + repo + "/blobs/" + config_digest)
        if status != 200 or digest(body) != config_digest:
            raise ValueError("published OCI config identity mismatch")
        value = json.loads(body)
        if value.get("os") != "linux" or value.get("architecture") != "amd64":
            raise ValueError("published image platform/filesystem differs from the exported image")
        if local is not None:
            if value["rootfs"]["diff_ids"] != local["RootFS"]["Layers"]:
                raise ValueError("published filesystem differs from exported image")
            for key in ("Labels", "Env", "Cmd", "Entrypoint", "User", "WorkingDir"):
                if value["config"].get(key) != local["Config"].get(key):
                    raise ValueError("published image configuration differs: " + key)
        metadata = {"oci_ref": AUTHORITY + "/" + repo + "@" + manifest_digest,
                    "manifest_digest": manifest_digest, "config_digest": config_digest}
        return (metadata, value) if include_config else metadata

    def verify_mapping(self, images):
        self.check()
        snapshot = capture()
        for source, reference in images.items():
            if source == "aiforall-zot:gold":
                # Preserve P4's existing upstream version, not the app registry.
                recipe = (ROOT / "ops/deploy/p4/Dockerfile.zot").read_text(encoding="utf-8")
                commands = [line.strip() for line in recipe.splitlines() if line.strip() and not line.startswith("#")]
                if commands != ["FROM ghcr.io/project-zot/zot:v2.1.8"] or reference != self.lock["p4_zot"]["reference"]:
                    raise ValueError("P4 Zot recipe/pin changed; explicit delivery update required")
                continue
            if source not in BOUNDARIES:
                raise ValueError("unknown application source in OCI mapping")
            name = BOUNDARIES[source]
            prefix = AUTHORITY + "/apps/" + name + "@"
            if not reference.startswith(prefix) or not re.fullmatch(r"sha256:[a-f0-9]{64}", reference[len(prefix):]):
                raise ValueError("published application repository/digest mismatch")
            _, config = self.remote("apps/" + name, reference[len(prefix):], include_config=True)
            verify_labels(source, config["config"].get("Labels") or {}, snapshot)
        require_unchanged(snapshot)

    def copy(self, local, repo, archive):
        if not re.fullmatch(r"apps/[a-z0-9][a-z0-9._-]*", repo):
            raise ValueError("invalid publication repository")
        self.check()
        before = self.preimage
        with tempfile.TemporaryDirectory(prefix="transfer-", dir=self.directory) as temporary:
            work = Path(temporary) / "private"
            pki.private_directory(work, create=True)
            # Python 3.13 mkdir(0700) overrides Windows ACLs; children must inherit this private DACL.
            for path in (work / "inputs", work / "outputs", work / "outputs/tmp"):
                path.mkdir(mode=0o777 if os.name == "nt" else 0o700)
            image = work / "inputs/image.tar"
            self.docker(["image", "save", "--output", str(image), local["Id"]], 1800)
            image.chmod(0o600)
            archive(image)
            destination = work / "inputs/destination"
            destination.write_text("docker://" + AUTHORITY + "/" + repo + ":candidate-" + uuid.uuid4().hex + "\n")
            destination.chmod(0o600)
            pki.private_directory(work)
            self.helper("publish.compose.yaml", preimage=before, suffix="-" + uuid.uuid4().hex,
                REGISTRY_INPUTS=str(work / "inputs"), REGISTRY_OUTPUTS=str(work / "outputs"),
                REGISTRY_NETWORK=self.inspect("network", self.state["network"])["Name"],
                REGISTRY_USER=str(os.getuid()) + ":" + str(os.getgid()) if os.name != "nt" else "10001:10001")
            pki.private_directory(work)
            result = (work / "outputs/digest").read_text().strip()
            if not re.fullmatch(r"sha256:[a-f0-9]{64}", result):
                raise ValueError("OCI transport returned no manifest digest")
            return self.remote(repo, result, local)

    def publish(self, build_file, output):
        if output.exists():
            raise ValueError("refuse release manifest overwrite")
        self.check()
        build = json.loads(build_file.read_text(encoding="utf-8"))
        if build.get("schema") != "aiforall.image-build/v1" or build.get("policy") != "local" or build.get("freshness_policy") != FRESHNESS_POLICY:
            raise ValueError("verified local build manifest required")
        if not build.get("targets"):
            raise ValueError("build manifest has no application images")
        snapshot = capture()
        images = {}
        for name, item in build["targets"].items():
            if name not in TARGETS or item["source_input"] != target_identity(snapshot, name):
                raise ValueError("build source is stale")
            local = self.inspect("image", item["image_id"])
            if local["Id"] != item["image_id"]:
                raise ValueError("build image ID mismatch")
            verify_labels(TARGETS[name]["source"], local["Config"].get("Labels") or {}, snapshot)
            if local["Config"]["Labels"].get("aiforall.dev.cargo-profile") != build["profile"]:
                raise ValueError("build receipt profile disagrees with image labels")
            artifacts = []
            remote = self.copy(local, "apps/" + name, lambda path: artifacts.extend(artifact_hashes(path, name)))
            images[name] = {**remote, "source": TARGETS[name]["source"], "inputs_id": "sha256:" + item["source_input"],
                "sdk_inputs_id": "sha256:" + identity([entry for entry in snapshot["files"] if entry["path"].startswith("rustycog/")]),
                "build_revision": local["Config"]["Labels"]["org.opencontainers.image.revision"],
                "sdk_revision": local["Config"]["Labels"]["aiforall.dev.sdk-revision"], "artifacts": artifacts}
            require_unchanged(snapshot)
        require_unchanged(snapshot)
        lock_hash = digest(LOCK.read_bytes())
        receipt = {"schema": "aiforall.image-release/v1", "freshness_policy": FRESHNESS_POLICY,
            "refresh_inputs_id": refresh_inputs(snapshot),
            "source": {"mode": "working-tree", "snapshot_id": "sha256:" + snapshot["sha256"], "root_revision": snapshot["revision"],
                       "sdk_revision": snapshot["sdk_revision"], "cargo_lock_sha256": next(x["sha256"] for x in snapshot["files"] if x["path"] == "Cargo.lock")},
            "profile": build["profile"], "cache_mode": "warm", "platform": "linux/amd64", "toolchain_id": lock_hash,
            "qualification": "local-distribution-only; cloud build pins/export remain gated", "images": images}
        with output.open("x", encoding="utf-8") as stream:
            json.dump(receipt, stream, indent=2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["registry-up", "status", "registry-stop", "publish", "connect-kind", "pull"])
    parser.add_argument("--state-dir", required=True, type=Path)
    parser.add_argument("--cert-directory", type=Path)
    parser.add_argument("--port", type=int, default=15443)
    parser.add_argument("--build-manifest", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--lease", type=Path)
    parser.add_argument("--image")
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535:
        parser.error("registry port must be 1024..65535")
    if args.action == "publish" and (not args.build_manifest or not args.output):
        parser.error("publish requires --build-manifest and --output")
    if args.action == "registry-up" and not args.cert_directory and not args.state_dir.exists():
        parser.error("initial registry-up requires --cert-directory")
    if args.action in {"connect-kind", "pull"} and (not args.lease or (args.action == "pull" and not args.image)):
        parser.error("Kind operations require --lease; pull also requires --image")
    previous = os.umask(0o077)
    try:
        registry = Registry(args.state_dir, create=args.action == "registry-up")
        with registry.operation():
            if args.action == "registry-up":
                registry.up(args.cert_directory, args.port)
                print("owned registry ready with verified mTLS; no Kind/build/deployment performed")
            elif args.action == "status":
                registry.check()
                if registry.request("/v2/")[0] != 200:
                    raise ValueError("registry is not ready")
                print("owned registry mTLS ready")
            elif args.action == "publish":
                registry.publish(args.build_manifest, args.output)
                print("OCI manifest/config digests reread and verified; no deployment performed")
            elif args.action in {"connect-kind", "pull"}:
                from local_full_target import LocalClusterTargetAnchor
                target = LocalClusterTargetAnchor(args.lease)
                result = target.registry_connect(registry) if args.action == "connect-kind" else target.registry_pull(registry, args.image)
                print(json.dumps(result))
            else:
                for identity in [*registry.state["helpers"], *([registry.state["container"]] if registry.state.get("container") else [])]:
                    registry.stop_container(identity)
                print("owned containers stopped; images, data, certificates and networks retained")
    finally:
        os.umask(previous)


if __name__ == "__main__":
    from private_diagnostics import operation
    with operation("registry"):
        try:
            main()
        except ValueError as error:
            raise SystemExit(str(error)) from None
        except (OSError, KeyError, TypeError, subprocess.TimeoutExpired, http.client.HTTPException):
            raise SystemExit("Registry operation refused/failed; private state and resources retained for inspection") from None
