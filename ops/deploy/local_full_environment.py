"""Durable local-full references and reversible power operations; no build/reset."""
import copy
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import tomllib
import uuid
from contextlib import contextmanager
from types import SimpleNamespace

from local_full_target import LocalClusterTargetAnchor, persist, private_file, unique_json
from local_registry import Registry, pki
from local_full_openbao import github_seed_job

CLUSTER = "aiforall-local-full"
SCHEMA = "aiforall.local-environment/v1"
IDENTITY = ("task_id", "cluster", "context", "nodeContainerIds", "control_plane_container_id",
            "kube_system_uid", "api_host", "api_port", "kubeconfig_path", "kubeconfig_sha256")


def input_reference(path):
    try:
        # Check before resolving: resolve() alone would hide a symlink/reparse
        # source. Reuse the existing private-input path/ACL policy.
        path = pki.checked_path(path)
        if not path.is_file() or not private_file(path):
            raise ValueError
        info = path.stat()
    except (OSError, ValueError, TypeError):
        raise ValueError("local input must remain private and read-only") from None
    # No hash/copy of secrets in environment metadata. Changed private sources
    # require an explicit new registration/rotation, never an implicit rotation.
    return {"path": str(path), "size": info.st_size, "mtime_ns": info.st_mtime_ns}


def read_private_input(reference, reader):
    """Read one unchanged private source without persisting a secret fingerprint."""
    if (not isinstance(reference, dict) or set(reference) != {"path", "size", "mtime_ns"}
            or not isinstance(reference["path"], str)
            or type(reference["size"]) is not int or type(reference["mtime_ns"]) is not int
            or input_reference(reference["path"]) != reference):
        raise ValueError("unchanged private input source required")
    try:
        value = reader(Path(reference["path"]))
    except (OSError, ValueError, KeyError, TypeError, UnicodeError):
        raise ValueError("invalid private input source (values redacted)") from None
    if input_reference(reference["path"]) != reference:
        raise ValueError("private input source changed during reading")
    return value


def owned(target, kind, name, namespace=CLUSTER):
    data = target.data
    uid = target.kubectl(["get", "namespace", namespace, "-o", "jsonpath={.metadata.uid}"], text=True, timeout=30).strip()
    if not uid or uid != data.get("namespaceUids", {}).get(namespace):
        raise ValueError("local namespace is not owned by the environment lease")
    result = json.loads(target.kubectl(["get", kind, name, "-n", namespace, "-o", "json"], text=True, timeout=30))
    if not any(entry.get("kind") == kind and entry.get("name") == name and entry.get("namespace") == namespace
            and entry.get("uid") == result["metadata"]["uid"] and entry.get("context") == data["context"]
            and entry.get("cluster") == data["cluster"] for entry in data.get("ownedResources", [])):
        raise ValueError("local resource UID is not owned: " + kind + "/" + name)
    return result


def coffer_epoch(target):
    deployment = owned(target, "Deployment", "openbao")
    replicas = json.loads(target.kubectl(["get", "replicasets", "-n", CLUSTER, "-o", "json"], text=True, timeout=30))["items"]
    owners = {r["metadata"]["uid"] for r in replicas if any(
        o.get("controller") and o.get("kind") == "Deployment" and o.get("uid") == deployment["metadata"]["uid"]
        for o in r["metadata"].get("ownerReferences", []))}
    pods = json.loads(target.kubectl(["get", "pods", "-n", CLUSTER, "-l", "app.kubernetes.io/name=openbao", "-o", "json"],
                                     text=True, timeout=30))["items"]
    pods = [pod for pod in pods if not pod["metadata"].get("deletionTimestamp") and any(
        o.get("controller") and o.get("kind") == "ReplicaSet" and o.get("uid") in owners
        for o in pod["metadata"].get("ownerReferences", []))]
    if len(pods) != 1:
        raise ValueError("expected one owned OpenBao pod")
    statuses = [c for c in pods[0].get("status", {}).get("containerStatuses", []) if c["name"] == "openbao"]
    if len(statuses) != 1 or not statuses[0].get("ready") or not statuses[0].get("containerID"):
        raise ValueError("OpenBao bootstrap is not ready")
    # Public pod/container identities, never tokens/keys or their hashes.
    return hashlib.sha256((pods[0]["metadata"]["uid"] + "|" + statuses[0]["containerID"]).encode()).hexdigest()


def selected_connectors(data):
    """Respect an explicit activation without dropping historical ownership."""
    names = {e.get("name") for e in data.get("ownedResources", [])
             if e.get("kind") == "Deployment" and e.get("namespace") == CLUSTER}
    github, oauth = "github-connect" in names, "oauth-connect" in names
    if data.get("connectorProfile") == "oauth-connect":
        if not oauth:
            raise ValueError("selected OAuthConnect profile is not installed in the original lease")
        return False, True
    if github and oauth:
        raise ValueError("ambiguous connector profiles in environment lease")
    return github, oauth


def recover_coffer(target, state, save, runtime, read_github, make_secret):
    apps = [(CLUSTER, "lazaret")]
    github, oauth = selected_connectors(target.data)
    inputs = None
    simulated_reference = None
    if github or oauth:
        reference = state.get("github_input")
        if not reference or input_reference(reference["path"]) != reference:
            raise ValueError("new coffer epoch needs the unchanged private GitHub input source")
        inputs = read_private_input(reference, read_github)
        connector_name = "oauth-connect" if oauth else "github-connect"
        connector = owned(target, "Deployment", connector_name)
        volumes = connector["spec"]["template"]["spec"]["volumes"]
        config_name = next(v["configMap"]["name"] for v in volumes if v["name"] == ("config" if oauth else "github-config"))
        config = owned(target, "ConfigMap", config_name)
        declaration = tomllib.loads(config["data"]["development.toml"])
        public_github = declaration["providers"][0] if oauth else declaration["github"]
        if public_github["client_id"] != inputs["client_id"]:
            raise ValueError("GitHub input source belongs to a different connector")
        if oauth:
            from local_full_oauthconnect import seed_inputs
            simulated_reference = state.get("simulated_input")
            if not simulated_reference:
                raise ValueError("new coffer epoch requires the private simulated input source")
            simulated = read_private_input(simulated_reference, read_github)
            providers = declaration["providers"]
            simulator_config = owned(target, "ConfigMap", "oauth-simulator-config")
            if ([p["id"] for p in providers] != ["github", "simulated"]
                    or providers[1]["client_id"] != simulated["client_id"]
                    or json.loads(simulator_config["data"]["provider.json"])["client_id"] != simulated["client_id"]):
                raise ValueError("simulated input source belongs to a different connector")
            inputs = seed_inputs(inputs, simulated)
        scripts = next(v["configMap"]["name"] for v in volumes if v["name"] == ("scripts" if oauth else "openbao-scripts"))
        owned(target, "ConfigMap", scripts)
        owned(target, "ConfigMap", "openbao-public-ca")
        apps += [(CLUSTER, "iam"), (CLUSTER, connector_name)]
        if oauth:
            apps.append((CLUSTER, "oauth-idp-simulator"))
    # Preflight every workload before creating a seed or patching a template.
    deployments = [(namespace, name, owned(target, "Deployment", name, namespace)) for namespace, name in apps]
    for _, name, deployment in deployments:
        initializers = deployment["spec"]["template"]["spec"].get("initContainers", [])
        expected = "openbao-agent" if name in {"github-connect", "oauth-connect"} else "openbao-snapshot"
        if not any(container["name"] == expected for container in initializers):
            raise ValueError("existing deployment predates managed OpenBao identities; explicit initial upgrade required: " + name)
    if inputs is not None and state.get("recovery_seeded_epoch") != state["pending_epoch"]:
        if input_reference(reference["path"]) != reference:
            raise ValueError("private GitHub source changed before seed")
        if simulated_reference and input_reference(simulated_reference["path"]) != simulated_reference:
            raise ValueError("private simulated source changed before seed")
        if state.get("recovery_job"):
            job_name = state["recovery_job"]
            owned(target, "Job", job_name)  # Do not silently recreate a failed/expired attempt.
        else:
            run_id = "resume-" + uuid.uuid4().hex[:16]
            if oauth:
                from local_full_oauthconnect import seed_job
                job = seed_job(run_id, scripts)
            else:
                job = github_seed_job(run_id, scripts)
            job["metadata"]["name"] += "-" + run_id
            state["recovery_job"] = job["metadata"]["name"]
            save()  # Crash before/after create is visible; no blind retry/adoption.
            data = runtime.lease(Path(state["lease_path"]))
            created = runtime.create(job, SimpleNamespace(lease=Path(state["lease_path"])), data)
            secret = make_secret(("oauth-connect-seed-inputs-" if oauth else "github-seed-inputs-") + run_id,
                                 {"inputs.json": json.dumps(inputs).encode()})
            secret["metadata"]["ownerReferences"] = [{"apiVersion": "batch/v1", "kind": "Job", "name": job["metadata"]["name"],
                                                       "uid": created["metadata"]["uid"], "controller": True}]
            runtime.create(secret, SimpleNamespace(lease=Path(state["lease_path"])), data)
            job_name = job["metadata"]["name"]
        target.kubectl(["wait", "--for=condition=complete", "job/" + job_name, "-n", CLUSTER, "--timeout=240s"], text=True, timeout=270)
        state["recovery_seeded_epoch"] = state["pending_epoch"]
        save()
    for namespace, name, deployment in deployments:
        if inputs is not None and input_reference(reference["path"]) != reference:
            raise ValueError("private GitHub source changed before rollout")
        if simulated_reference and input_reference(simulated_reference["path"]) != simulated_reference:
            raise ValueError("private simulated source changed before rollout")
        # Re-read after seed: reject a concurrent change, not a stale preflight RV.
        deployment = owned(target, "Deployment", name, namespace)
        metadata = deployment["metadata"]
        template = deployment["spec"]["template"]
        annotations = template.get("metadata", {}).get("annotations")
        path = "/spec/template/metadata/annotations"
        if annotations is None:
            value = {"aiforall.dev/openbao-epoch": state["pending_epoch"]}
        else:
            path += "/aiforall.dev~1openbao-epoch"
            value = state["pending_epoch"]
        patch = [{"op": "test", "path": "/metadata/uid", "value": metadata["uid"]},
                 {"op": "test", "path": "/metadata/resourceVersion", "value": metadata["resourceVersion"]},
                 {"op": "add", "path": path, "value": value}]
        target.kubectl(["patch", "deployment/" + name, "-n", namespace, "--type=json", "-p", json.dumps(patch)], text=True, timeout=30)
        target.kubectl(["rollout", "status", "deployment/" + name, "-n", namespace, "--timeout=300s"], text=True, timeout=330)
    if coffer_epoch(target) != state["pending_epoch"]:
        raise ValueError("OpenBao restarted during recovery; environment not ready")


@contextmanager
def power_cleanup(target, registry, nodes, engine, enabled):
    """On failed up, undo only this call's reversible starts, never a running peer."""
    registry_was_running = registry.check()["State"]["Running"] if registry else False
    try:
        yield
    except BaseException:
        failures = []
        if enabled:
            for node in sorted(nodes, key=lambda n: n["role"] == "control-plane"):
                if not node["running"]:
                    try:
                        target._power_node(node["id"], False, engine)
                    except Exception:
                        failures.append(node["id"])
            if registry and not registry_was_running:
                try:
                    with registry.operation():
                        registry.stop_container(registry.state["container"])
                except Exception:
                    failures.append("registry")
        if failures:
            raise ValueError("local-up failed; graceful cleanup blocked for recorded owned IDs; inspect private state") from None
        raise


def _register(args, directory):
    """One-time explicit binding; no node/API mutation or credential reconstruction."""
    if args.action != "up" or not args.lease:
        raise ValueError("first local-up needs --lease; no cluster is created or adopted")
    target = LocalClusterTargetAnchor(args.lease, expected_cluster=CLUSTER)
    engine = target.runner(["docker", "info", "--format", "{{.ID}}"], text=True, timeout=15).strip()
    target.lifecycle_nodes(engine)
    registry = Registry(args.registry_state) if args.registry_state else None
    if registry and registry.state["engine"] != engine:
        raise ValueError("registry and environment engines differ")
    reference = input_reference(args.github_input_file) if args.github_input_file else None
    simulated_reference = input_reference(args.simulated_input_file) if getattr(args, "simulated_input_file", None) else None
    if not any(e.get("kind") == "Deployment" and e.get("name") == "openbao" and e.get("namespace") == CLUSTER
            for e in target.data.get("ownedResources", [])):
        raise ValueError("initial deployment must be explicitly installed before registering local-up")
    if not reference and any(e.get("kind") == "Deployment" and e.get("name") in {"github-connect", "oauth-connect"}
            and e.get("namespace") == CLUSTER for e in target.data.get("ownedResources", [])):
        raise ValueError("register the GitHub profile with its private --github-input-file for future coffer epochs")
    if not simulated_reference and any(e.get("kind") == "Deployment" and e.get("name") == "oauth-connect"
            and e.get("namespace") == CLUSTER for e in target.data.get("ownedResources", [])):
        raise ValueError("register OAuthConnect with --simulated-input-file for future coffer epochs")
    pki.private_directory(directory, create=True)
    state = {"schema": SCHEMA, "lease_path": str(args.lease.resolve()), "engine": engine,
             "identity": {key: target.data[key] for key in IDENTITY}, "epoch": None,
              "registry_state": str(registry.directory) if registry else None, "github_input": reference,
              "simulated_input": simulated_reference}
    persist(directory / "environment.json", state)


def operate(args, runtime, read_github, make_secret):
    directory = pki.checked_path(args.environment_dir)
    state_file = directory / "environment.json"
    if not directory.exists():
        if args.action == "status":
            return {"status": "not-initialized", "next": "first up requires an explicit anchored --lease"}
        _register(args, directory)
    pki.private_directory(directory)
    state = json.loads(state_file.read_text(encoding="utf-8"), object_pairs_hook=unique_json)
    if state.get("schema") != SCHEMA:
        raise ValueError("unknown local environment state")
    lease = Path(state["lease_path"])
    if args.lease and args.lease.resolve() != lease:
        raise ValueError("environment already registered to a different original lease")
    if args.registry_state and str(args.registry_state.resolve()) != state["registry_state"]:
        raise ValueError("environment registry reference changed")
    if args.github_input_file and input_reference(args.github_input_file) != state.get("github_input"):
        raise ValueError("environment private source changed; explicit rotation required")
    if getattr(args, "simulated_input_file", None) and input_reference(args.simulated_input_file) != state.get("simulated_input"):
        raise ValueError("environment private simulated source changed; explicit rotation required")
    target = LocalClusterTargetAnchor(lease, expected_cluster=CLUSTER)
    if {key: target.data[key] for key in IDENTITY} != state["identity"]:
        raise ValueError("original environment anchor identity changed")
    def save():
        persist(state_file, state)
    registry = Registry(Path(state["registry_state"])) if state["registry_state"] else None
    if registry and any(set(value["node_ids"]) != set(state["identity"]["nodeContainerIds"])
            for value in registry.state.get("kind_networks", {}).values()):
        raise ValueError("registered registry is also leased to another environment; do not power it down here")
    with runtime.operation_lock(lease, "local-" + args.action):
        nodes = target.lifecycle_nodes(state["engine"])
        if args.action == "up" and state.get("github_input"):
            if input_reference(state["github_input"]["path"]) != state["github_input"]:
                raise ValueError("private GitHub source changed; no implicit rotation")
        if args.action == "up" and state.get("simulated_input"):
            if input_reference(state["simulated_input"]["path"]) != state["simulated_input"]:
                raise ValueError("private simulated source changed; no implicit rotation")
        with power_cleanup(target, registry, nodes, state["engine"], args.action == "up"):
            return running_operation(args, state, save, target, registry, nodes, runtime, read_github, make_secret)


def resync_api_policies(target):
    """Keep the existing API-only allowlist aligned after Docker reassigns node IPs."""
    nodes = json.loads(target.kubectl(["get", "nodes", "-l", "node-role.kubernetes.io/control-plane", "-o", "json"], text=True, timeout=30))["items"]
    if len(nodes) != 1:
        raise ValueError("unique local control-plane required for API policy")
    addresses = [entry["address"] for entry in nodes[0]["status"]["addresses"] if entry["type"] == "InternalIP"]
    if len(addresses) != 1:
        raise ValueError("unique local control-plane IPv4 required for API policy")
    address = ipaddress.IPv4Address(addresses[0])
    if not address.is_private or address.is_loopback or address.is_unspecified or address.is_multicast:
        raise ValueError("local control-plane address refused for API policy")
    changes = []
    for namespace in (CLUSTER, "aiforall-local-apparatus"):
        current = owned(target, "NetworkPolicy", "allow-api", namespace)
        desired = copy.deepcopy(current["spec"])
        rules = desired.get("egress", [])
        if (desired.get("policyTypes") != ["Egress"] or len(rules) != 1
                or rules[0].get("ports") != [{"protocol": "TCP", "port": 443}, {"protocol": "TCP", "port": 6443}]
                or len(rules[0].get("to", [])) != 2
                or rules[0]["to"][0] != {"ipBlock": {"cidr": "10.96.0.1/32"}}):
            raise ValueError("unexpected local API policy; no automatic overwrite")
        previous = rules[0]["to"][1]
        cidr = ipaddress.IPv4Network(previous.get("ipBlock", {}).get("cidr", ""))
        if cidr.prefixlen != 32 or previous != {"ipBlock": {"cidr": str(cidr)}}:
            raise ValueError("unexpected local API endpoint peer")
        rules[0]["to"][1] = {"ipBlock": {"cidr": str(address) + "/32"}}
        if desired != current["spec"]:
            changes.append((namespace, current, desired))
    # Validate both owned preimages before writing either; CAS rejects concurrent edits.
    for namespace, current, desired in changes:
        patch = [{"op": "test", "path": "/metadata/uid", "value": current["metadata"]["uid"]},
                 {"op": "test", "path": "/metadata/resourceVersion", "value": current["metadata"]["resourceVersion"]},
                 {"op": "test", "path": "/spec", "value": current["spec"]},
                 {"op": "replace", "path": "/spec", "value": desired}]
        target.kubectl(["patch", "networkpolicy/allow-api", "-n", namespace, "--type=json", "-p", json.dumps(patch)], text=True, timeout=30)


def running_operation(args, state, save, target, registry, nodes, runtime, read_github, make_secret):
    if args.action == "stop":
        target.stop_nodes(state["engine"])
        if registry:
            with registry.operation():
                registry.check()
                for identity in [*registry.state["helpers"], registry.state["container"]]:
                    registry.stop_container(identity)
        return {"status": "stopped", "node_ids": [n["id"] for n in nodes], "data_certificates_images_caches": "retained",
                "openbao": "local volatile state intentionally lost"}
    if args.action == "status" and not all(n["running"] for n in nodes):
        return {"status": "stopped" if not any(n["running"] for n in nodes) else "partially-stopped",
                "nodes": [{"id": n["id"], "running": n["running"]} for n in nodes], "recovery_pending": bool(state.get("pending_epoch"))}
    selected_oauth = target.data.get("connectorProfile") == "oauth-connect"
    if selected_oauth:
        selected_connectors(target.data)  # A partial activation is not an installed profile.
    if args.action == "up":
        if registry:
            with registry.operation():
                registry.up(None, registry.state["port"])
        target.resume_nodes(state["engine"])
        target.kubectl(["wait", "--for=condition=Ready", "nodes", "--all", "--timeout=300s"], text=True, timeout=330)
        resync_api_policies(target)
        target.kubectl(["rollout", "status", "deployment/openbao", "-n", CLUSTER, "--timeout=300s"], text=True, timeout=330)
    epoch = coffer_epoch(target)
    changed = state.get("pending_epoch") or (state["epoch"] is not None and epoch != state["epoch"])
    if args.action == "up" and (changed or any(not n["running"] for n in nodes)):
        if state.get("pending_epoch") and state["pending_epoch"] != epoch:
            raise ValueError("another coffer epoch interrupted recovery; inspect the recorded attempt")
        state["pending_epoch"] = epoch
        save()
        recover_coffer(target, state, save, runtime, read_github, make_secret)
        state.pop("pending_epoch", None)
        state.pop("recovery_job", None)
        state.pop("recovery_seeded_epoch", None)
    if args.action == "up":
        state["epoch"] = epoch
        save()
    deployments = []
    for entry in target.data.get("ownedResources", []):
        if entry.get("kind") != "Deployment":
            continue
        if selected_oauth and entry.get("namespace") == CLUSTER and entry.get("name") in {"github-connect", "gitlab-connect"}:
            continue  # Historical resources remain leased, but are not the selected service.
        obj = owned(target, "Deployment", entry["name"], entry["namespace"])
        if args.action == "up":
            target.kubectl(["rollout", "status", "deployment/" + entry["name"], "-n", entry["namespace"], "--timeout=300s"], text=True, timeout=330)
            obj = owned(target, "Deployment", entry["name"], entry["namespace"])
        deployments.append({"name": entry["name"], "namespace": entry["namespace"],
            "ready_replicas": obj.get("status", {}).get("readyReplicas", 0), "replicas": obj["spec"].get("replicas", 1),
            "images": [c["image"] for c in obj["spec"]["template"]["spec"]["containers"]]})
    return {"status": "running", "ready": bool(deployments) and all(d["ready_replicas"] == d["replicas"] for d in deployments),
            "deployments": deployments, "recovery_pending": bool(state.get("pending_epoch")) or (args.action == "status" and bool(changed)),
            "openbao": "intentionally volatile; business/auth proof deferred to E2E"}
