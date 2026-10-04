"""Parent-run local-full entrypoint; offline render is the only worker-safe action."""
import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONTEXT = "kind-aiforall-local-full"
CLUSTER = "aiforall-local-full"
NAMESPACES = ["aiforall-local-full", "aiforall-local-apparatus", "aiforall-local-plugins"]
RUST_APPS = ["iam", "hive", "telegraph", "manifesto"]


def load_module(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


runtime = load_module("storage_runtime", "local-full-storage.py")
renderer = load_module("local_full_renderer", "render-local-full.py")


def k(*args, stdin=None, timeout=360):
    return runtime.kubectl(*args, stdin=stdin, timeout=timeout)


def apply(items):
    if items:
        # Server-side apply avoids client-side last-applied annotations containing
        # local key material. Output is object names, never configuration data.
        k("apply", "--server-side", "--field-manager=local-full", "-f", "-", "-o", "name",
          stdin=json.dumps({"apiVersion": "v1", "kind": "List", "items": items}))


def prepare_namespaces(args, data):
    for name in NAMESPACES:
        uid = k("get", "namespace", name, "--ignore-not-found", "-o", "jsonpath={.metadata.uid}")
        if uid:
            if data.get("namespaceUids", {}).get(name) != uid:
                raise ValueError("namespace exists with unknown ownership: " + name)
            profile = k("get", "namespace", name, "-o", "jsonpath={.metadata.labels.aiforall\\.dev/profile}")
            if profile != "local-full":
                raise ValueError("namespace belongs to another profile: " + name)
        else:
            runtime.create({"apiVersion": "v1", "kind": "Namespace", "metadata": {"name": name, "labels": {"aiforall.dev/profile": "local-full"}}}, args, data)


def preflight_cluster_objects(objects, args, data):
    for obj in objects:
        if obj["kind"] not in {"CustomResourceDefinition", "ClusterRole", "ClusterRoleBinding"}:
            continue
        name = obj["metadata"]["name"]
        uid = k("get", obj["kind"], name, "--ignore-not-found", "-o", "jsonpath={.metadata.uid}")
        key = obj["kind"] + "/" + name
        if uid and data.get("clusterResourceUids", {}).get(key) != uid:
            raise ValueError("shared cluster object requires explicit parent UID lease: " + key)
        if uid and obj["kind"] == "CustomResourceDefinition":
            existing = json.loads(k("get", obj["kind"], name, "-o", "json"))
            expected = obj["spec"]
            actual = existing["spec"]
            if actual["names"]["kind"] != expected["names"]["kind"] or actual["scope"] != expected["scope"] or actual["versions"] != expected["versions"]:
                raise ValueError("existing AdmissionRecord schema differs; preserve it and escalate")


def record_resources(objects, args, data):
    for obj in objects:
        if obj["kind"] == "Secret":
            # Track local input ownership without GET of a live Secret.
            data.setdefault("localSecretInputs", {})[obj["metadata"]["namespace"] + "/" + obj["metadata"]["name"]] = True
            continue
        flags = ["get", obj["kind"], obj["metadata"]["name"], "-o", "jsonpath={.metadata.uid}"]
        if obj["metadata"].get("namespace"):
            flags += ["-n", obj["metadata"]["namespace"]]
        uid = k(*flags)
        metadata = obj["metadata"]
        entry = {"kind": obj["kind"], "name": metadata["name"], "namespace": metadata.get("namespace"), "uid": uid, "context": CONTEXT, "cluster": CLUSTER}
        prior = data.setdefault("ownedResources", [])
        if entry not in prior:
            prior.append(entry)
        if obj["kind"] in {"CustomResourceDefinition", "ClusterRole", "ClusterRoleBinding"}:
            data.setdefault("clusterResourceUids", {})[obj["kind"] + "/" + metadata["name"]] = uid
    runtime.save_lease(args.lease, data)


def cert_inputs(path):
    if not path or not path.is_dir():
        raise ValueError("explicit prepared local-full certificate directory required")
    mesh = path / "mesh"
    workload = path / "workload"
    files = {"ca.crt": (mesh / "ca.crt").read_bytes()}
    for name in ["iam-service", "hive-service", "telegraph-service", "manifesto-service", "envoy-mesh", "ext-authz", "mesh-client"]:
        for suffix in ["crt", "key"]:
            file = name + "." + suffix
            files[file] = (mesh / file).read_bytes()
    identity_files = {name: (workload / name).read_bytes() for name in ["ca.crt", "ca.key", "server.crt", "server.key"]}
    if files["ca.crt"] == identity_files["ca.crt"]:
        raise ValueError("transport and workload CA must be distinct")
    digest = hashlib.sha256(b"".join(files[key] for key in sorted(files)) + b"".join(identity_files[key] for key in sorted(identity_files))).hexdigest()
    return files, identity_files, digest


def secret(name, values):
    import base64
    return {"apiVersion": "v1", "kind": "Secret", "metadata": {"name": name, "namespace": "aiforall-local-full", "labels": {"aiforall.dev/profile": "local-full"}},
            "type": "Opaque", "data": {key: base64.b64encode(value).decode() for key, value in values.items()}}


def verify_images(images):
    runtime.target().check()
    for image in set(images.values()):
        match = re.fullmatch(r"[a-z0-9./:_-]+:sha256-([a-f0-9]{64})", image)
        if not match:
            raise ValueError("application image must use a full image-ID content tag")
        result = subprocess.run(["docker", "image", "inspect", "--format", "{{.Id}}", image], capture_output=True, text=True, timeout=30)
        if result.returncode or result.stdout.strip() != "sha256:" + match.group(1):
            raise ValueError("local immutable image identity mismatch")
        runtime.target().kind_load(image)


def configure_api_policy():
    address = k("get", "nodes", "-l", "node-role.kubernetes.io/control-plane", "-o", "jsonpath={.items[0].status.addresses[?(@.type=='InternalIP')].address}")
    import ipaddress
    address = str(ipaddress.IPv4Address(address))
    for namespace in ["aiforall-local-full", "aiforall-local-apparatus"]:
        patch = {"spec": {"egress": [{"to": [{"ipBlock": {"cidr": "10.96.0.1/32"}}, {"ipBlock": {"cidr": address + "/32"}}], "ports": [{"protocol": "TCP", "port": 443}, {"protocol": "TCP", "port": 6443}]}]}}
        # kube API egress is scoped to the actual control-plane address, not a wildcard.
        k("patch", "networkpolicy/allow-api", "-n", namespace, "--type=merge", "-p", json.dumps(patch))


def deploy_locked(args):
    images = json.loads(args.image_manifest.read_text())
    mesh, identity, cert_hash = cert_inputs(args.cert_directory)
    objects = renderer.render(uuid.uuid4().hex[:16], images, cert_hash)
    capability_sources = {}
    if args.isolated_plugins:
        required = {
            "services/Lazaret/configuration/src/lib.rs": ["pub namespace: String", "fn validate"],
            "services/Lazaret/application/src/invoke.rs": ["NamespacedDigestDnsPluginLocator", "try_new"],
            "services/Lazaret/setup/src/app.rs": ["NamespacedDigestDnsPluginLocator::try_new", "config.plugin_hop.namespace.clone()", "config.plugin_hop.validate()"],
            "crates/apparatus-reference-kv/src/bin/reference-kv-http.rs": ["LAZARET_CA_CERT_PATH", "add_root_certificate"],
        }
        for file, markers in required.items():
            source = (ROOT / file).read_bytes()
            text = source.decode("utf-8")
            if any(marker not in text for marker in markers):
                raise ValueError("isolated plugin capability source is missing: " + file)
            capability_sources[file] = hashlib.sha256(source).hexdigest()
        if not args.namespace_wiring_confirmed:
            raise ValueError("detected capability source hashes still require parent confirmation against built artifacts/final tests")
        for obj in objects:
            if obj["kind"] == "Deployment" and obj["metadata"]["name"] == "lazaret":
                obj["spec"]["template"]["spec"]["containers"][0]["env"] += [
                    {"name": "LAZARET_PLUGIN_HOP__USE_DNS_FORMULA", "value": "true"},
                    {"name": "LAZARET_PLUGIN_HOP__NAMESPACE", "value": "aiforall-local-plugins"}]
            if obj["kind"] == "Deployment" and obj["metadata"]["name"] == "apparatus-operator":
                obj["spec"]["template"]["spec"]["containers"][0]["env"] += [
                    {"name": "APPARATUS_PLUGIN_CA_SECRET", "value": "lazaret-workload-ca"},
                    {"name": "APPARATUS_PLUGIN_CA_SHA256", "value": hashlib.sha256(identity["ca.crt"]).hexdigest()}]
    for obj in objects:
        if obj["kind"] == "Secret" and obj["metadata"]["name"] == "lazaret-workload-ca":
            # Separate plugin-namespace object; never mirror ca.key/server.key.
            obj["data"] = secret("unused", {"ca.crt": identity["ca.crt"]})["data"]
    data = json.loads(args.lease.read_text(encoding="utf-8-sig"))
    if data.get("context") != CONTEXT or data.get("cluster") != CLUSTER:
        raise ValueError("parent local runtime lease required")
    bootstrap_command = ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(ROOT / "ops/deploy/bootstrap-local-full.ps1"), "-LeasePath", str(args.lease.resolve())]
    if args.after_final_it:
        bootstrap_command.append("-AfterFinalIT")
    bootstrap = subprocess.run(bootstrap_command, capture_output=True, timeout=1200)
    if bootstrap.returncode:
        raise RuntimeError("local Kind bootstrap failed; parent must inspect recorded ownership/topology conflict")
    data = runtime.lease(args.lease)
    if capability_sources:
        data["isolatedPluginSourceSha256"] = capability_sources
        runtime.save_lease(args.lease, data)
    preflight_cluster_objects(objects, args, data)
    verify_images(images)
    prepare_namespaces(args, data)
    # No concurrent bootstrap or migration runs in this exclusive namespace.
    for namespace in NAMESPACES:
        active = k("get", "jobs", "-n", namespace, "-o", "jsonpath={.items[*].status.active}")
        if any(value not in {"0", ""} for value in active.split()):
            raise ValueError("active job remains in local-full inventory; no implicit cleanup")
    groups = {"foundation": [], "dependencies": [], "bootstrap": [], "applications": []}
    original_replicas = {}
    for obj in objects:
        if obj["kind"] == "Deployment":
            name = obj["metadata"]["name"]
            key = (obj["metadata"]["namespace"], name)
            old = k("get", "deployment", name, "-n", key[0], "--ignore-not-found", "-o", "jsonpath={.spec.replicas}")
            if old:
                uid = k("get", "deployment", name, "-n", key[0], "-o", "jsonpath={.metadata.uid}")
                if not any(entry.get("kind") == "Deployment" and entry.get("namespace") == key[0]
                           and entry.get("name") == name and entry.get("uid") == uid
                           for entry in data.get("ownedResources", [])):
                    raise ValueError("existing deployment UID not explicitly owned: " + name)
                original_replicas[key] = int(old)
            if name in renderer.DEPENDENCIES:
                groups["dependencies"].append(obj)
            else:
                obj["spec"]["replicas"] = 1
                groups["applications"].append(obj)
        elif obj["kind"] == "Job":
            groups["bootstrap"].append(obj)
        else:
            groups["foundation"].append(obj)
    groups["foundation"] += [secret("platform-mesh-certs", mesh), secret("lazaret-identity-ca", identity)]
    try:
        if original_replicas and not args.confirm_quiesce:
            raise ValueError("existing owned deployment: --confirm-quiesce required before serial migration/rollout")
        for obj in groups["applications"]:
            namespace, name = obj["metadata"]["namespace"], obj["metadata"]["name"]
            if (namespace, name) in original_replicas:
                k("scale", "deployment/" + name, "-n", namespace, "--replicas=0")
                k("wait", "--for=delete", "pod", "-n", namespace,
                  "-l", "app.kubernetes.io/name=" + name, "--timeout=180s")
        apply(groups["foundation"])
        record_resources(groups["foundation"], args, data)
        configure_api_policy()
        apply(groups["dependencies"])
        record_resources(groups["dependencies"], args, data)
        for obj in groups["dependencies"]:
            k("rollout", "status", "deployment/" + obj["metadata"]["name"], "-n", "aiforall-local-full", "--timeout=300s")
        apply(groups["bootstrap"])
        record_resources(groups["bootstrap"], args, data)
        for obj in groups["bootstrap"]:
            k("wait", "--for=condition=complete", "job/" + obj["metadata"]["name"], "-n", "aiforall-local-full", "--timeout=240s")
        apply(groups["applications"])
        record_resources(groups["applications"], args, data)
        for obj in groups["applications"]:
            k("rollout", "status", "deployment/" + obj["metadata"]["name"], "-n", obj["metadata"]["namespace"], "--timeout=300s")
        for name in RUST_APPS:
            k("scale", "deployment/" + name, "-n", "aiforall-local-full", "--replicas=2")
            k("rollout", "status", "deployment/" + name, "-n", "aiforall-local-full", "--timeout=300s")
        print("local-full source profile applied and rollout waits passed; parent must run final behavioral proofs")
    except Exception:
        # Reversible rollback to pre-existing replica counts only, no resource
        # deletion, reset, Secret fetch or unknown workload adoption.
        for (namespace, name), replicas in original_replicas.items():
            try:
                k("scale", "deployment/" + name, "-n", namespace, "--replicas=" + str(replicas))
            except Exception:
                print("WARNING: replica rollback blocked; parent lease/inventory retained")
        raise


def deploy(args):
    # A persistent exclusive-operation marker prevents concurrent deploys from
    # sharing a parent ledger. A killed process leaves a stale marker: STOP and
    # ask the parent to inspect ownership; never reclaim it by PID/name alone.
    with runtime.operation_lock(args.lease, "deploy"):
        deploy_locked(args)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["render", "deploy", "inventory"])
    parser.add_argument("--lease", type=Path)
    parser.add_argument("--image-manifest", type=Path)
    parser.add_argument("--cert-directory", type=Path)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--isolated-plugins", action="store_true")
    parser.add_argument("--namespace-wiring-confirmed", action="store_true")
    parser.add_argument("--confirm-quiesce", action="store_true")
    parser.add_argument("--after-final-it", action="store_true")
    args = parser.parse_args()
    if args.action == "render":
        # No runtime inspection, startup or API contact. Source secrets remain
        # in render artifacts; report only resource kind/name metadata.
        images = json.loads(args.image_manifest.read_text()) if args.image_manifest else None
        objects = renderer.render("staticvalidation", images)
        if args.output_dir:
            args.output_dir.mkdir(parents=True, exist_ok=True)
            file = args.output_dir / "local-full.json"
            with file.open("x", encoding="utf-8") as target:
                json.dump({"apiVersion": "v1", "kind": "List", "items": objects}, target)
        print(json.dumps({"resource_count": len(objects), "immutable_app_images_supplied": images is not None,
                          "objects": [o["kind"] + "/" + o["metadata"]["name"] for o in objects]}))
    elif args.action == "inventory":
        if not args.lease:
            raise ValueError("inventory requires an explicit parent lease")
        data = runtime.lease(args.lease)
        host = data["api_host"]
        endpoint = "https://" + ("[" + host + "]" if ":" in host else host) + ":" + str(data["api_port"])
        print(json.dumps({"context": CONTEXT, "cluster": CLUSTER, "api_endpoint": endpoint,
                          "http_endpoint": "http://127.0.0.1:18080", "leased_node_ids": data["nodeContainerIds"]}))
        for namespace in NAMESPACES:
            uid = k("get", "namespace", namespace, "-o", "jsonpath={.metadata.uid}")
            if data.get("namespaceUids", {}).get(namespace) != uid:
                raise ValueError("inventory namespace is not owned")
            print(k("get", "deployment,pod,pvc,job,networkpolicy", "-n", namespace, "-o", "name"))
    else:
        if not args.lease or not args.image_manifest or not args.cert_directory:
            raise ValueError("deploy requires --lease, --image-manifest and --cert-directory; no build or key generation is implicit")
        deploy(args)


if __name__ == "__main__":
    main()
