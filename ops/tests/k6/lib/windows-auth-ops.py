"""One-shot Windows auth parent operations; no server, secret GET or free command.

Reuses the existing lease-v2 CLI and Admin reader. stdout is closed/redacted;
metadata stays in the parent-owned private directory, never in the public report.
"""
import argparse
import base64
import hashlib
import http.client
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[4]
TARGET = ROOT / "ops/deploy/local_full_target.py"
NS = "aiforall-local-full"


def require(condition):
    if not condition:
        raise ValueError("AUTH_OPS_REFUSED")


def read_json(path):
    require(path.stat().st_size <= 1024 * 1024)
    return json.loads(path.read_text(encoding="utf-8-sig"), object_pairs_hook=unique)


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result)
        result[key] = value
    return result


def write_json(path, value):
    # Names are fixed by this parent adapter, inside the freshly owned run.
    path.write_text(json.dumps(value, separators=(",", ":")), encoding="utf-8")


class Operations:
    def __init__(self, config, directory):
        self.config, self.directory = read_json(config), directory
        require(hashlib.sha256(TARGET.read_bytes()).hexdigest() == self.config["source_sha256"]["target"])
        for name, tool in self.config["tools"].items():
            require(hashlib.sha256(Path(tool["path"]).read_bytes()).hexdigest() == tool["sha256"])
            if name in ("docker", "kubectl"):
                require(Path(shutil.which(name) or "").resolve() == Path(tool["path"]).resolve())
        sys.path.insert(0, str(TARGET.parent))
        from local_full_target import LocalClusterTargetAnchor, private_file
        self.private_file = private_file
        require(private_file(config))
        self.lease = Path(self.config["lease"])
        self.anchor = LocalClusterTargetAnchor(self.lease, expected_cluster="aiforall-local-full")
        self.data = self.anchor.data
        self.cli("check")

    def execute(self, arguments):
        # Inherit the existing native parent's console group and Job. Its bounded
        # CTRL_BREAK/join owns cancellation; do not add subprocess.run's kill-on-
        # timeout/KeyboardInterrupt behavior to this adapter.
        process = subprocess.Popen(arguments, stdin=subprocess.DEVNULL,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        output, _ = process.communicate()
        return process.returncode, output

    def cli(self, action, arguments=()):
        require(hashlib.sha256(TARGET.read_bytes()).hexdigest() == self.config["source_sha256"]["target"])
        code, output = self.execute([sys.executable, "-B", str(TARGET), action, "--lease", str(self.lease),
                                    "--cluster", "aiforall-local-full", "--", *arguments])
        require(code == 0)
        return output.decode("utf-8", errors="strict")

    def get(self, kind, name):
        require(kind in ("deployment", "replicaset", "configmap"))
        return json.loads(self.cli("kubectl", ["get", kind, name, "-n", NS, "-o", "json"]), object_pairs_hook=unique)

    def owned(self, kind, name, uid=None):
        rows = [row for row in self.data.get("ownedResources", []) if row.get("kind") == kind
                and row.get("name") == name and row.get("namespace") == NS
                and row.get("context") == "kind-aiforall-local-full" and row.get("cluster") == NS]
        require(len(rows) == 1 and rows[0].get("uid") and (uid is None or rows[0]["uid"] == uid))
        return rows[0]["uid"]

    def pod(self, name):
        deployment = self.get("deployment", name)
        dep_uid = self.owned("Deployment", name, deployment["metadata"]["uid"])
        items = json.loads(self.cli("kubectl", ["get", "pods", "-n", NS, "-l",
                           "app.kubernetes.io/name=" + name, "-o", "json"]))["items"]
        pods = [pod for pod in items if not pod["metadata"].get("deletionTimestamp")]
        require(len(pods) == 1)
        pod = pods[0]
        require(any(row.get("type") == "Ready" and row.get("status") == "True"
                    for row in pod["status"].get("conditions", [])))
        owners = pod["metadata"].get("ownerReferences", [])
        require(len(owners) == 1 and owners[0]["kind"] == "ReplicaSet")
        replica = self.get("replicaset", owners[0]["name"])
        parents = replica["metadata"].get("ownerReferences", [])
        require(replica["metadata"]["uid"] == owners[0]["uid"] and len(parents) == 1
                and parents[0]["kind"] == "Deployment" and parents[0]["uid"] == dep_uid)
        return pod, deployment, [owners[0], parents[0]]

    def configuration(self, pod, container_name):
        containers = [row for row in pod["spec"]["containers"] if row["name"] == container_name]
        require(len(containers) == 1)
        container = containers[0]
        maps = []
        for volume in pod["spec"].get("volumes", []):
            if "secret" in volume:
                self.owned("Secret", volume["secret"]["secretName"])
            if "configMap" in volume:
                name = volume["configMap"]["name"]
                value = self.get("configmap", name)
                self.owned("ConfigMap", name, value["metadata"]["uid"])
                maps.append(dict(name=name, uid=value["metadata"]["uid"],
                                 resource_version=value["metadata"]["resourceVersion"], data=value.get("data", {})))
        return container, maps

    def binding(self):
        envoy, _, owners = self.pod("envoy-mesh")
        container, maps = self.configuration(envoy, "envoy")
        statuses = [row for row in envoy["status"]["containerStatuses"] if row["name"] == "envoy"]
        require(len(statuses) == 1)
        status = statuses[0]
        configs = [(row, key, value) for row in maps for key, value in row["data"].items()
                   if hashlib.sha256(value.encode()).hexdigest() == self.config["envoy_config_sha256"]]
        require(len(configs) == 1)
        config_map, config_key, _ = configs[0]
        import yaml
        from local_full_target import UniqueLoader
        loaded = yaml.load(configs[0][2], Loader=UniqueLoader)
        require(isinstance(loaded, dict) and not loaded.get("dynamic_resources"))
        # Bind the matched ConfigMap/key to Envoy's actual config-path mount.
        args = container.get("args", [])
        option = "--config-path" if "--config-path" in args else "-c"
        require(option in args)
        config_path = args[args.index(option) + 1]
        volumes = {row["name"]: row for row in envoy["spec"].get("volumes", [])}
        require(any(volumes.get(mount["name"], {}).get("configMap", {}).get("name") == config_map["name"]
                    and (mount.get("subPath") == config_key and mount["mountPath"] == config_path
                         or not mount.get("subPath") and mount["mountPath"].rstrip("/") + "/" + config_key == config_path)
                    for mount in container.get("volumeMounts", [])))
        provider, provider_dep, _ = self.pod("oauth-idp-simulator")
        _, provider_maps = self.configuration(provider, "provider")
        script_hash = hashlib.sha256((ROOT / "ops/tests/support/oauth-idp-simulator.py").read_bytes().replace(b"\r\n", b"\n")).hexdigest()
        require(any(hashlib.sha256(value.replace("\r\n", "\n").encode()).hexdigest() == script_hash
                    for row in provider_maps for value in row["data"].values()))
        iam, iam_dep, _ = self.pod("iam")
        iam_container, _ = self.configuration(iam, "iam")
        iam_status = next(row for row in iam["status"]["containerStatuses"] if row["name"] == "iam")
        provider_status = next(row for row in provider["status"]["containerStatuses"] if row["name"] == "provider")
        iam_config = {key: iam_container.get(key) for key in ("command", "args", "env", "volumeMounts")}
        iam_config["volumes"] = iam["spec"].get("volumes", [])
        return dict(task_id=self.data["task_id"], provider_service_uid=self.owned("Service", "oauth-idp-simulator"),
                    attestation=dict(pod=envoy["metadata"]["name"], namespace_uid=self.data["namespaceUids"][NS],
                    pod_uid=envoy["metadata"]["uid"], container="envoy",
                    owner_chain=[{key: owner[key] for key in ("kind", "name", "uid")} for owner in owners],
                    incarnation=dict(container_id=status["containerID"], restart_count=status["restartCount"], image_id=status["imageID"])),
                    envoy_config=dict(uid=config_map["uid"], resource_version=config_map["resource_version"],
                                      sha256=self.config["envoy_config_sha256"]),
                    provider_pod_uid=provider["metadata"]["uid"], provider_deployment_uid=provider_dep["metadata"]["uid"],
                    provider_incarnation=dict(container_id=provider_status["containerID"], restart_count=provider_status["restartCount"], image_id=provider_status["imageID"]),
                    iam_deployment_uid=iam_dep["metadata"]["uid"],
                    iam_image_id=iam_status["imageID"],
                    iam_config_sha256=hashlib.sha256(json.dumps(iam_config, sort_keys=True).encode()).hexdigest(),
                    mesh_secret_uid=self.owned("Secret", "platform-mesh-certs"))

    def stable_binding(self):
        initial = read_json(self.directory / "auth-binding.json")
        require(self.binding() == initial)
        return initial

    def openssl(self, arguments, success=True):
        tool = self.config["tools"]["openssl"]
        require(hashlib.sha256(Path(tool["path"]).read_bytes()).hexdigest() == tool["sha256"])
        code, _ = self.execute([tool["path"], *arguments])
        require((code == 0) == success)

    def admit(self):
        mesh = Path(self.config["cert_directory"]) / "mesh"
        self.openssl(["verify", "-CAfile", str(mesh / "ca.crt"), str(mesh / "iam-service.crt"), str(mesh / "hive-service.crt")])
        for name, success in (("iam-service.crt", True), ("hive-service.crt", False)):
            self.openssl(["verify", "-CAfile", str(mesh / "ca.crt"), "-verify_hostname",
                          "iam.aiforall-local-full.svc.cluster.local", str(mesh / name)], success)
        write_json(self.directory / "auth-binding.json", self.binding())
        iam, _, _ = self.pod("iam")
        write_json(self.directory / "auth-iam-before.json", {"pod_uid": iam["metadata"]["uid"]})
        material = {kind: {suffix: base64.b64encode((mesh / (name + "." + suffix)).read_bytes()).decode()
                           for suffix in ("crt", "key")}
                    for kind, name in (("good", "iam-service"), ("bad", "hive-service"))}
        path = self.directory / "auth-cert-material.json"
        with path.open("x", encoding="utf-8"):
            pass
        try:
            path.chmod(0o400)
            require(self.private_file(path))
        finally:
            path.chmod(0o600)
        write_json(path, material)  # Frozen original bytes, private parent-only; retired by owned cleanup.

    def snapshot(self, phase):
        binding = self.stable_binding()
        snapshot = self.anchor.envoy_admin_snapshot(**binding["attestation"])
        self.stable_binding()
        write_json(self.directory / ("auth-snapshot-" + phase + ".json"), snapshot)

    def provider(self, phase):
        self.stable_binding()
        connection = http.client.HTTPConnection("127.0.0.1", 18081, timeout=5)
        try:
            connection.request("GET", "/observations", headers={"Connection": "close"})
            response = connection.getresponse()
            require(response.status == 200)
            raw = response.read(4097)
            require(len(raw) <= 4096)
            value = json.loads(raw, object_pairs_hook=unique)
        finally:
            connection.close()
        require(set(value) == {"run_id", "authorize", "token", "profile", "pkce_validated"}
                and value["run_id"] == self.config["run_id"]
                and all(type(value[key]) is int and value[key] >= 0 for key in value if key != "run_id"))
        self.stable_binding()
        write_json(self.directory / ("auth-provider-" + phase + ".json"), value)

    def certificate(self, restore):
        if restore:
            # Repair follows a joined operation under the parent's lock. A failed
            # rollout must not make Pod readiness a prerequisite of the repair.
            binding = read_json(self.directory / "auth-binding.json")
            self.owned("Secret", "platform-mesh-certs", binding["mesh_secret_uid"])
            deployment = self.get("deployment", "iam")
            self.owned("Deployment", "iam", deployment["metadata"]["uid"])
            require(deployment["metadata"]["uid"] == binding["iam_deployment_uid"])
            prior_pod_uid = None  # Not a live observation; never infer one from a journal.
        else:
            binding = self.stable_binding()
            prior_pod, _, _ = self.pod("iam")
            prior_pod_uid = prior_pod["metadata"]["uid"]
        material = read_json(self.directory / "auth-cert-material.json")
        good, bad = material["good"], material["bad"]
        expected, replacement = (bad, good) if restore else (good, bad)
        patch = [{"op": "test", "path": "/metadata/uid", "value": binding["mesh_secret_uid"]}]
        patch += [{"op": "test", "path": "/data/iam-service." + key, "value": expected[key]} for key in expected]
        patch += [{"op": "replace", "path": "/data/iam-service." + key, "value": replacement[key]} for key in replacement]
        descriptor, filename = tempfile.mkstemp(prefix="private-patch-", suffix=".json", dir=self.directory)
        os.close(descriptor)
        path = Path(filename)
        phase = "restored" if restore else "bad"
        acknowledgements = dict(patch=False, rollout=False, before_pod_uid=prior_pod_uid, pods=[])
        try:
            path.chmod(0o400)
            require(self.private_file(path))  # Empty file checked before credential write.
            path.chmod(0o600)
            path.write_text(json.dumps(patch), encoding="utf-8")
            path.chmod(0o400)
            require(self.private_file(path))
            self.cli("kubectl", ["patch", "secret/platform-mesh-certs", "-n", NS, "--type=json", "--patch-file", str(path)])
            acknowledgements["patch"] = True
            write_json(self.directory / ("auth-ack-" + phase + ".json"), acknowledgements)
        finally:
            path.chmod(0o600)
            path.unlink()
        deployment = self.get("deployment", "iam")
        require(deployment["metadata"]["uid"] == binding["iam_deployment_uid"])
        key = "aiforall.dev/san-proof-phase"
        annotations = deployment["spec"]["template"]["metadata"].get("annotations")
        change = [{"op": "test", "path": "/metadata/uid", "value": binding["iam_deployment_uid"]}]
        value = self.config["run_id"] + "-" + phase
        if annotations is None:
            change.append({"op": "add", "path": "/spec/template/metadata/annotations", "value": {key: value}})
        else:
            change.append({"op": "add", "path": "/spec/template/metadata/annotations/" + key.replace("/", "~1"), "value": value})
        self.cli("kubectl", ["patch", "deployment/iam", "-n", NS, "--type=json", "-p", json.dumps(change)])
        self.cli("kubectl", ["rollout", "status", "deployment/iam", "-n", NS, "--timeout=240s"])
        acknowledgements["rollout"] = True  # Actual successful rollout command, not a metadata read.
        write_json(self.directory / ("auth-ack-" + phase + ".json"), acknowledgements)
        pod, _, _ = self.pod("iam")
        if prior_pod_uid is not None:
            require(pod["metadata"]["uid"] != prior_pod_uid)
        acknowledgements["pods"] = [pod["metadata"]["uid"]]
        write_json(self.directory / ("auth-ack-" + phase + ".json"), acknowledgements)
        self.stable_binding()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("admit", "snapshot", "provider", "bad", "restore", "forward"))
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--phase", choices=("good", "bad", "restored", "oauth-before", "oauth-after"), default="good")
    args = parser.parse_args()
    # The existing native parent signals CTRL_BREAK, not console-wide CTRL_C.
    # Make it unwind Python finally blocks and the unchanged CLI's owned forward.
    if os.name == "nt":
        signal.signal(signal.SIGBREAK, signal.default_int_handler)
    try:
        operation = Operations(args.config, args.directory)
        if args.action == "forward":
            from local_full_target import main as target_main
            sys.argv = [str(TARGET), "oauth-forward", "--lease", str(operation.lease), "--cluster", NS]
            return target_main()
        elif args.action == "admit":
            operation.admit()
        elif args.action == "snapshot":
            operation.snapshot(args.phase)
        elif args.action == "provider":
            operation.provider(args.phase)
        else:
            operation.certificate(args.action == "restore")
        print('{"ok":true}')
        return 0
    except (Exception, KeyboardInterrupt):
        print(json.dumps(dict(ok=False, reason="AUTH_OPS_REFUSED", action=args.action, phase=args.phase)))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
