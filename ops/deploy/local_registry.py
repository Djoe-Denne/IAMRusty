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


def run(args, timeout=60):
    from private_diagnostics import capture_failure
    try:
        result = subprocess.run(args, capture_output=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        capture_failure(args, error.output, error.stderr, timeout=True)
        raise
    if result.returncode:
        capture_failure(args, result.stdout, result.stderr, result.returncode)
        raise ValueError("registry operation failed (exit " + str(result.returncode) + "); native output withheld")
    return result.stdout.decode("utf-8")


def record(kind, identity, owner, event="created", safe="no"):
    now = datetime.now(timezone.utc)
    directory = ROOT / ".tmp"
    directory.mkdir(exist_ok=True)
    with (directory / ("docker-resources-" + now.strftime("%Y%m%d") + ".log")).open("a", encoding="utf-8") as stream:
        stream.write(json.dumps({"timestamp": now.isoformat(), "event": event, "kind": kind,
            "id": identity, "owner": owner, "safe_to_delete": safe, "source": "local_registry"}) + "\n")
        stream.flush()
        os.fsync(stream.fileno())


def config():
    return {"distSpecVersion": "1.1.1", "storage": {"rootDirectory": "/data"},
        "http": {"address": "0.0.0.0", "port": "5000", "tls": {
            "cert": "/runtime/server.crt", "key": "/runtime/server.key", "cacert": "/runtime/ca.crt"},
            "accessControl": {"repositories": {"apps/**": {"anonymousPolicy": [], "defaultPolicy": [],
                "policies": [{"users": ["registry-publisher"], "actions": ["read", "create", "update"]},
                             {"users": ["registry-reader"], "actions": ["read"]}]}}}},
        "log": {"level": "error"}}


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
                          "context": context, "helpers": [], "port": None}
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
        item = self.inspect("container", identity)
        if item["Id"] != identity or item["Config"].get("Labels", {}).get("aiforall.registry.owner") != self.state["owner"]:
            raise ValueError("foreign registry container refused")
        return item

    def create(self, arguments, role):
        identity = self.docker(["container", "create", "--label", "aiforall.registry.owner=" + self.state["owner"],
                                "--label", "aiforall.registry.role=" + role, *arguments]).strip()
        if not re.fullmatch(r"[a-f0-9]{64}", identity):
            record("container", "UNKNOWN", self.state["owner"], safe="ask")
            raise ValueError("Docker did not return a container ID")
        if role != "registry":
            self.state["helpers"].append(identity)
        else:
            self.state["container"] = identity
        self.save()
        record("container", identity, self.state["owner"])
        return identity

    def stop_container(self, identity):
        item = self.owned(identity)
        if item["State"]["Running"]:
            self.docker(["container", "stop", "--time", "15", identity], 30)
        if self.owned(identity)["State"]["Running"]:
            raise ValueError("owned container did not stop")
        record("container", identity, self.state["owner"], event="stopped")

    def helper(self, args):
        identity = self.create(["--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges", *args], "helper")
        try:
            self.docker(["container", "start", "--attach", identity], 1800)
            item = self.owned(identity)
            if item["State"]["Running"] or item["State"]["ExitCode"] != 0:
                raise ValueError("registry helper failed; output withheld")
            return identity
        finally:
            self.stop_container(identity)

    def image(self, key):
        pin = self.lock[key]
        if not re.fullmatch(r"[a-z0-9./_-]+@sha256:[a-f0-9]{64}", pin["reference"]):
            raise ValueError("immutable transport image pin required")
        existing = subprocess.run(["docker", "--context", self.state["context"], "image", "inspect", pin["reference"]], capture_output=True)
        if existing.returncode:
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

    def certificates(self):
        directory = pki.checked_path(self.state["cert_directory"])
        pki.private_directory(directory)
        files = {"ca": directory / "mesh/ca.crt", **{name: directory / "extensions/registry" / (name + ".crt")
                 for name in pki.registry_profiles()}}
        actual = {name: digest(path.read_bytes()) for name, path in files.items()}
        if actual != self.state["certificates"]:
            raise ValueError("registry certificates changed; explicit rotation required")
        return directory

    def check(self):
        item = self.owned(self.state["container"])
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
            if value.get("Labels", {}).get("aiforall.registry.owner") != self.state["owner"]:
                raise ValueError("foreign registry volume refused")
        network = self.inspect("network", self.state["network"])
        if network.get("Labels", {}).get("aiforall.registry.owner") != self.state["owner"]:
            raise ValueError("foreign registry network")
        known = {self.state["container"], *self.state["helpers"]}
        if any(identity not in known for identity in network.get("Containers", {})):
            raise ValueError("unexpected container on registry network")
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
            if self.state.get("network"):
                raise ValueError("incomplete setup retained; inspect its IDs before retrying with a new state directory")
            directory = pki.checked_path(cert_directory)
            tool = pki.OpenSSL(pki.openssl_path(None), directory)
            pki.validate_registry(tool, directory)
            self.state.update(cert_directory=str(directory), port=port)
            files = {"ca": directory / "mesh/ca.crt", **{name: directory / "extensions/registry" / (name + ".crt")
                     for name in pki.registry_profiles()}}
            self.state["certificates"] = {name: digest(path.read_bytes()) for name, path in files.items()}
            zot, client = self.image("zot"), self.image("skopeo")
            network = self.docker(["network", "create", "--label", "aiforall.registry.owner=" + self.state["owner"],
                                   "aiforall-images-" + self.state["owner"]]).strip()
            record("network", network, self.state["owner"])
            self.state["network"] = network
            self.save()
            for role in ("data", "runtime"):
                name = self.docker(["volume", "create", "--label", "aiforall.registry.owner=" + self.state["owner"],
                                    "aiforall-images-" + self.state["owner"] + "-" + role]).strip()
                record("volume", name, self.state["owner"])
                self.state[role] = name
                self.save()
            path = self.directory / "registry.json"
            path.write_text(json.dumps(config()), encoding="utf-8")
            path.chmod(0o600)
            mounts = []
            for source, target in [(path, "config.json"), (directory / "mesh/ca.crt", "ca.crt"),
                                   (directory / "extensions/registry/aiforall-apps-registry.crt", "server.crt"),
                                   (directory / "extensions/registry/aiforall-apps-registry.key", "server.key")]:
                mounts.extend(["--mount", "type=bind,src=" + str(source) + ",dst=/inputs/" + target + ",readonly"])
            script = "set -eu; test -z \"$(ls -A /runtime)\"; test -z \"$(ls -A /data)\"; cp /inputs/* /runtime/; chmod 700 /runtime /data; chmod 400 /runtime/*; chown -R 10001:10001 /runtime /data"
            self.helper(["--user", "0:0", "--cap-add", "CHOWN", "--cap-add", "DAC_OVERRIDE", "--network", "none",
                         "--mount", "type=volume,src=" + self.state["data"] + ",dst=/data",
                         "--mount", "type=volume,src=" + self.state["runtime"] + ",dst=/runtime", *mounts,
                         "--entrypoint", "/bin/sh", client, "-c", script])
            self.create(["--user", "10001:10001", "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
                         "--tmpfs", "/tmp:rw,nosuid,nodev,size=32m", "--network", network, "--network-alias", AUTHORITY.split(":")[0],
                         "--publish", "127.0.0.1:" + str(port) + ":5000", "--mount", "type=volume,src=" + self.state["data"] + ",dst=/data",
                         "--mount", "type=volume,src=" + self.state["runtime"] + ",dst=/runtime,readonly", zot,
                         "serve", "/runtime/config.json"], "registry")
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
        self.check()
        directory = self.certificates()
        with tempfile.TemporaryDirectory(prefix="transfer-", dir=self.directory) as temporary:
            work = Path(temporary)
            image = work / "image.tar"
            self.docker(["image", "save", "--output", str(image), local["Id"]], 1800)
            image.chmod(0o600)
            archive(image)
            (work / "tmp").mkdir(mode=0o700)
            policy = work / "policy.json"
            policy.write_text(json.dumps({"default": [{"type": "reject"}], "transports": {"docker-archive": {"": [{"type": "insecureAcceptAnything"}]}}}))
            policy.chmod(0o600)
            mounts = ["--mount", "type=bind,src=" + str(work) + ",dst=/work"]
            for source, filename in [(directory / "mesh/ca.crt", "ca.crt"),
                                     (directory / "extensions/registry/registry-publisher.crt", "client.cert"),
                                     (directory / "extensions/registry/registry-publisher.key", "client.key")]:
                mounts.extend(["--mount", "type=bind,src=" + str(source) + ",dst=/certs/" + filename + ",readonly"])
            user = str(os.getuid()) + ":" + str(os.getgid()) if os.name != "nt" else "10001:10001"
            self.helper(["--user", user, "--network", self.state["network"], *mounts, "--env", "TMPDIR=/work/tmp",
                "--entrypoint", "/bin/sh", self.image("skopeo"), "-c", 'umask 077; exec /usr/bin/skopeo "$@"', "skopeo",
                "--policy", "/work/policy.json", "copy", "--format", "oci", "--dest-tls-verify=true", "--dest-cert-dir", "/certs",
                "--digestfile", "/work/digest", "docker-archive:/work/image.tar", "docker://" + AUTHORITY + "/" + repo + ":candidate-" + uuid.uuid4().hex])
            result = (work / "digest").read_text().strip()
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
