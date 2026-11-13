"""Owned disposable Kind runtime for the hosted E2E job, not the local lab."""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import signal
import socket
import subprocess
import sys
import threading
import time

from binaries import ROOT, Refused, require, sha, strict_json

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from safety import private_path, proc, same

NS = "aiforall-local-full"
OWNER = "k6-ci-e2e"


def hosted(environment=None):
    env = os.environ if environment is None else environment
    require(sys.platform == "linux" and os.getuid() != 0 and env.get("GITHUB_ACTIONS") == "true"
            and env.get("GITHUB_JOB") == "e2e" and env.get("GITHUB_REF") == "refs/heads/master", "HOSTED_E2E_ONLY")
    require(re.fullmatch(r"[1-9][0-9]*", env.get("GITHUB_RUN_ID", ""))
            and re.fullmatch(r"[1-9][0-9]*", env.get("GITHUB_RUN_ATTEMPT", "")), "E2E_RUN_ID")


def write_json(path, value):
    path = Path(path)
    temporary = path.with_suffix(path.suffix + ".pending")
    with temporary.open("w", encoding="utf-8") as stream:
        json.dump(value, stream, sort_keys=True)
        stream.flush()
        os.fsync(stream.fileno())
    temporary.chmod(0o600)
    temporary.replace(path)


def owned_nodes(records, state):
    owned = set()
    for row in records:
        if row.get("kind") != "container" or row.get("event") != "created":
            continue
        identity = row.get("id", "")
        require(row.get("owner") == OWNER and row.get("job_run_id", row.get("run_id")) == state["job_run_id"]
                and row.get("run_attempt") == state["job_attempt"] and row.get("campaign") == state["run_id"]
                and row.get("source") == "docker-engine-create-event" and re.fullmatch(r"[a-f0-9]{64}", identity)
                and identity not in state["before_containers"]
                and row.get("name") == state["cluster"] + "-control-plane", "E2E_NODE_RECEIPT_CONFLICT")
        require(identity not in owned, "E2E_DUPLICATE_NODE_RECEIPT")
        owned.add(identity)
    for row in records:
        if row.get("kind") == "container" and row.get("id") in owned:
            require(row.get("safe_to_delete") != "ask" and row.get("owner") == OWNER
                    and row.get("run_id") == state["job_run_id"] and row.get("run_attempt") == state["job_attempt"]
                    and row.get("campaign") == state["run_id"], "E2E_NODE_RECEIPT_CONFLICT")
    require(len(owned) <= 1, "E2E_NODE_RECEIPT_CONFLICT")
    return owned


class Runtime:
    def __init__(self, directory, state):
        self.directory = Path(directory)
        self.state = state
        self.monitor = None
        self.monitor_thread = None
        self.monitor_error = False
        self.lock = threading.Lock()

    def save(self):
        write_json(self.directory / "private/state.json", self.state)

    def env(self):
        return {"PATH": "/usr/bin:/bin", "HOME": str(self.directory / "home"),
                "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8", "TZ": "UTC",
                "DOCKER_HOST": "unix:///var/run/docker.sock", "KIND_EXPERIMENTAL_PROVIDER": "docker"}

    def cli(self, tool, arguments, stdin=None, timeout=60, expected=0):
        pin = self.state["tools"][tool]
        require(sha(pin["path"]) == pin["sha256"], "E2E_CLI_CHANGED")
        try:
            result = subprocess.run([pin["path"], *arguments], input=stdin, capture_output=True,
                                    env=self.env(), cwd=self.directory, timeout=timeout)
        except subprocess.TimeoutExpired:
            raise Refused("E2E_CLI_TIMEOUT_" + tool.upper()) from None
        require(len(result.stdout) <= 4 * 1024 ** 2 and len(result.stderr) <= 4 * 1024 ** 2, "E2E_CLI_OUTPUT_LIMIT")
        # Diagnostics stay private; stdout can contain Secrets or OAuth URLs.
        with self.lock:
            sequence = self.state.get("cli_sequence", 0) + 1
            self.state["cli_sequence"] = sequence
        (self.directory / "private" / f"cli-{sequence}.stderr").write_bytes(result.stderr)
        require(result.returncode == expected, "E2E_CLI_FAILED_" + tool.upper())
        return result.stdout

    def docker(self, args, **kwargs):
        return self.cli("docker", args, **kwargs)

    def inspect(self, identity):
        return strict_json(self.docker(["inspect", "--format", "{{json .}}", identity]))

    def container_ids(self, cluster=False):
        args = ["container", "ls", "--all", "--no-trunc", "--format", "{{.ID}}"]
        if cluster:
            args += ["--filter", "label=io.x-k8s.kind.cluster=" + self.state["cluster"]]
        values = set(self.docker(args).decode().splitlines())
        require(all(re.fullmatch(r"[a-f0-9]{64}", value) for value in values), "E2E_CONTAINER_INVENTORY")
        return values

    def record(self, event, kind, identity, safe, **facts):
        row = dict(timestamp=datetime.now(timezone.utc).isoformat(), event=event, kind=kind, id=identity,
                   safe_to_delete=safe, owner=OWNER, run_id=self.state["job_run_id"],
                   run_attempt=self.state["job_attempt"], campaign=self.state["run_id"], **facts)
        registry = ROOT / ".tmp" / ("docker-resources-" + datetime.now(timezone.utc).strftime("%Y%m%d") + ".log")
        registry.parent.mkdir(exist_ok=True)
        with self.lock:
            for path in (registry, self.directory / "private/docker-resources.jsonl"):
                with path.open("a", encoding="utf-8") as stream:
                    stream.write(json.dumps(row, sort_keys=True) + "\n")
                    stream.flush()
                    os.fsync(stream.fileno())

    def receipts(self):
        path = self.directory / "private/docker-resources.jsonl"
        if not path.exists():
            return []
        require(path.stat().st_size < 2 * 1024 ** 2, "E2E_RECEIPT_LIMIT")
        return [strict_json(line) for line in path.read_bytes().splitlines() if line.strip()]

    def image_ids(self):
        return set(self.docker(["image", "ls", "--no-trunc", "--quiet"]).decode().splitlines())

    def observe_images(self):
        values = self.image_ids()
        known = set(self.state["known_images"])
        for identity in sorted(values - known):
            self.record("created", "image", identity, "no", reason="active_disposable_e2e")
        self.state["known_images"] = sorted(values)
        self.save()

    def track_process(self, process, purpose):
        value = proc(process.pid)
        require(value is not None, "E2E_PROCESS_IDENTITY")
        self.state["processes"].append({key: value[key] for key in ("pid", "uid", "start")} | {"purpose": purpose})
        self.save()

    def start_events(self):
        since = self.docker(["info", "--format", "{{.SystemTime}}"]).decode().strip()
        require(re.fullmatch(r"[0-9TZ:.+-]+", since), "E2E_DAEMON_CLOCK")
        self.state["daemon_since"] = since
        self.save()
        self.monitor = subprocess.Popen([self.state["tools"]["docker"]["path"], "events", "--since", since,
                                         "--filter", "type=container", "--format", "{{json .}}"],
                                        env=self.env(), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.track_process(self.monitor, "docker-events")

        def read():
            try:
                for line in self.monitor.stdout:
                    require(len(line) < 65536, "E2E_ENGINE_EVENT_LIMIT")
                    value = strict_json(line)
                    attrs = value.get("Actor", {}).get("Attributes", {})
                    if value.get("Action") == "create" and attrs.get("io.x-k8s.kind.cluster") == self.state["cluster"]:
                        identity = value.get("Actor", {}).get("ID", "")
                        require(re.fullmatch(r"[a-f0-9]{64}", identity)
                                and identity not in self.state["before_containers"]
                                and attrs.get("name") == self.state["cluster"] + "-control-plane"
                                and self.state.get("kind_creation_authorized") is True, "E2E_ENGINE_CREATE_CONFLICT")
                        self.record("created", "container", identity, "no", source="docker-engine-create-event",
                                    daemon_time_nano=value.get("timeNano"), name=attrs["name"])
            except Exception:
                self.monitor_error = True

        self.monitor_thread = threading.Thread(target=read, daemon=True)
        self.monitor_thread.start()

    def create_cluster(self):
        existing = self.cli("kind", ["get", "clusters"]).decode().splitlines()
        require(self.state["cluster"] not in existing, "E2E_CLUSTER_ALREADY_EXISTS")
        self.state["kind_creation_authorized"] = True
        self.save()
        self.start_events()
        try:
            self.cli("kind", ["create", "cluster", "--name", self.state["cluster"], "--config",
                              str(ROOT / "ops/tests/k6/ci/kind.yaml"), "--kubeconfig", self.state["kubeconfig"],
                              "--wait", "150s"], timeout=240)
        finally:
            self.observe_images()
        owned = owned_nodes(self.receipts(), self.state)
        present = self.container_ids(True)
        require(not self.monitor_error and len(owned) == 1 and owned == present, "E2E_KIND_RECEIPT_UNPROVEN")
        self.state["kind_creation_authorized"] = False
        identity = next(iter(owned))
        node = self.inspect(identity)
        require(node["Config"]["Labels"].get("io.x-k8s.kind.cluster") == self.state["cluster"], "E2E_KIND_IDENTITY")
        volumes = []
        for mount in node.get("Mounts", []):
            if mount["Type"] == "volume":
                require(mount["Name"] not in self.state["before_volumes"], "E2E_FOREIGN_NODE_VOLUME")
                volumes.append(mount["Name"])
                self.record("created", "volume", mount["Name"], "no", source="new_node_mount_engine_inspection", node_id=identity)
        self.state.update(node_id=identity, volumes=volumes, kubeconfig_sha256=sha(self.state["kubeconfig"]))
        self.save()
        self.check_context()

    def check_context(self):
        private_path(self.state["kubeconfig"], readonly=False)
        require(sha(self.state["kubeconfig"]) == self.state["kubeconfig_sha256"], "E2E_KUBECONFIG_CHANGED")
        args = ["--kubeconfig", self.state["kubeconfig"], "--context", "kind-" + self.state["cluster"],
                "config", "view", "--minify", "--output", "json"]
        value = strict_json(self.cli("kubectl", args))
        require(value.get("current-context") == "kind-" + self.state["cluster"]
                and len(value["clusters"]) == 1 and re.fullmatch(r"https://127\.0\.0\.1:[1-9][0-9]*",
                                                              value["clusters"][0]["cluster"]["server"]), "E2E_CONTEXT")

    def k(self, args, **kwargs):
        self.check_context()
        return self.cli("kubectl", ["--kubeconfig", self.state["kubeconfig"], "--context", "kind-" + self.state["cluster"],
                                    "--namespace", NS, "--request-timeout=30s", *args], **kwargs)

    def close_processes(self):
        failed = False
        for identity in self.state["processes"]:
            current = same(identity)
            if current and not current["zombie"]:
                os.kill(identity["pid"], signal.SIGTERM)
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            alive = [value for identity in self.state["processes"] if (value := same(identity)) and not value["zombie"]]
            if not alive:
                break
            time.sleep(0.05)
        else:
            failed = True
        if self.monitor:
            self.monitor.wait(timeout=5)
            self.monitor.stdout.close()
        if self.monitor_thread:
            self.monitor_thread.join(timeout=5)
        require(not failed, "E2E_PROCESS_CLEANUP_UNPROVEN")

    def close(self):
        require(self.state["host"] == socket.gethostname()
                and self.state["boot_id"] == Path("/proc/sys/kernel/random/boot_id").read_text().strip(), "E2E_FOREIGN_HOST")
        errors = []
        try:
            self.close_processes()
        except Exception:
            errors.append("PROCESS_CLEANUP_UNPROVEN")
        owned = owned_nodes(self.receipts(), self.state)
        present = self.container_ids()
        candidates = self.container_ids(True)
        if not candidates <= owned:
            errors.append("NODE_OWNERSHIP_UNPROVEN")
        for identity in sorted(owned & present):
            node = self.inspect(identity)
            if (node["Id"] != identity or node["Config"]["Labels"].get("io.x-k8s.kind.cluster") != self.state["cluster"]
                    or node.get("Name") != "/" + self.state["cluster"] + "-control-plane"):
                errors.append("NODE_IDENTITY_CONFLICT")
                continue
            mounted = [mount["Name"] for mount in node.get("Mounts", []) if mount["Type"] == "volume"]
            if any(name in self.state["before_volumes"] for name in mounted):
                errors.append("NODE_VOLUME_OWNERSHIP_UNPROVEN")
                continue
            for name in mounted:
                if name not in self.state.get("volumes", []):
                    self.record("created", "volume", name, "no", source="new_node_mount_engine_inspection", node_id=identity)
            self.state["volumes"] = sorted(set(self.state.get("volumes", [])) | set(mounted))
            self.save()
            if node["State"]["Running"]:
                self.docker(["stop", "--time", "30", identity], timeout=45)
            require(self.inspect(identity)["State"]["Running"] is False, "E2E_NODE_STOP_UNPROVEN")
            self.record("stopped", "container", identity, "no")
        # No broad Docker removal. Kind may retire only the exact stopped nodes
        # received during our exclusive create operation, including cancellation.
        unsafe = {"NODE_OWNERSHIP_UNPROVEN", "NODE_IDENTITY_CONFLICT", "NODE_VOLUME_OWNERSHIP_UNPROVEN"}
        if not (set(errors) & unsafe) and candidates:
            self.cli("kind", ["delete", "cluster", "--name", self.state["cluster"],
                              "--kubeconfig", self.state["kubeconfig"]], timeout=90)
        after = self.container_ids()
        for identity in owned - after:
            self.record("removed", "container", identity, "yes")
        if owned & after or self.container_ids(True):
            errors.append("NODE_CLEANUP_INCOMPLETE")
        volumes = set(self.docker(["volume", "ls", "--format", "{{.Name}}"]).decode().splitlines())
        for name in self.state.get("volumes", []):
            if name in volumes:
                errors.append("NODE_VOLUME_RETAINED")
            else:
                self.record("removed", "volume", name, "yes")
        self.state["cleanup"] = {"status": "FAIL" if errors else "PASS", "errors": errors,
                                 "owned_running": None if errors else 0, "node_ids": sorted(owned),
                                 "volumes": self.state.get("volumes", []), "images": "retained_on_disposable_host"}
        self.save()
        return self.state["cleanup"]
