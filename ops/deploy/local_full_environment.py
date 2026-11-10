"""Durable local-full references and reversible, metadata-selected power operations."""
import hashlib
import json
from pathlib import Path
import uuid
from contextlib import contextmanager
from types import SimpleNamespace

from local_full_target import LocalClusterTargetAnchor, persist, private_file, unique_json
from local_registry import Registry, pki
import local_full_lifecycle as lifecycle

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
    declaration = lifecycle.coffer(target.data)
    deployment = owned(target, "Deployment", declaration["name"], declaration["namespace"])
    replicas = json.loads(target.kubectl(["get", "replicasets", "-n", CLUSTER, "-o", "json"], text=True, timeout=30))["items"]
    owners = {r["metadata"]["uid"] for r in replicas if any(
        o.get("controller") and o.get("kind") == "Deployment" and o.get("uid") == deployment["metadata"]["uid"]
        for o in r["metadata"].get("ownerReferences", []))}
    pods = json.loads(target.kubectl(["get", "pods", "-n", declaration["namespace"], "-l", "app.kubernetes.io/name=" + declaration["name"], "-o", "json"],
                                     text=True, timeout=30))["items"]
    pods = [pod for pod in pods if not pod["metadata"].get("deletionTimestamp") and any(
        o.get("controller") and o.get("kind") == "ReplicaSet" and o.get("uid") in owners
        for o in pod["metadata"].get("ownerReferences", []))]
    if len(pods) != 1:
        raise ValueError("expected one owned OpenBao pod")
    container_name = declaration["containerName"]
    statuses = [c for c in pods[0].get("status", {}).get("containerStatuses", []) if c["name"] == container_name]
    if len(statuses) != 1 or not statuses[0].get("ready") or not statuses[0].get("containerID"):
        raise ValueError("OpenBao bootstrap is not ready")
    # Public pod/container identities, never tokens/keys or their hashes.
    return hashlib.sha256((pods[0]["metadata"]["uid"] + "|" + statuses[0]["containerID"]).encode()).hexdigest()


def recover_coffer(target, state, save, runtime, read_github, make_secret):
    from local_full_lifecycle import objects, profile, profiles, validate_public_input
    from local_full_helm import render
    selected_profile = profile(target.data)
    contract = profiles()[selected_profile]
    sources = {}
    for key, declaration in contract['inputs'].items():
        reference = state.get(key)
        if not reference:
            raise ValueError('new coffer epoch needs every declared unchanged private input source')
        sources[key] = read_private_input(reference, read_github)
        validate_public_input(target, owned, declaration, sources[key])
    seed = None
    if sources:
        if contract['seedContract'] == 'oauth-connect':
            from local_full_oauthconnect import seed_inputs
            seed = seed_inputs(sources['github_input'], sources['simulated_input'])
        else:
            seed = sources['github_input']
    deployments = []
    for desired in objects(target.data):
        initializer = desired['metadata'].get('annotations', {}).get('aiforall.dev/epoch-initializer')
        if not initializer:
            continue
        namespace, name = desired['metadata']['namespace'], desired['metadata']['name']
        deployment = owned(target, 'Deployment', name, namespace)
        if not any(c['name'] == initializer for c in deployment['spec']['template']['spec'].get('initContainers', [])):
            raise ValueError('existing deployment predates declared OpenBao identity; explicit initial upgrade required')
        deployments.append((namespace, name))
    def require_sources():
        for key in sources:
            reference = state[key]
            if input_reference(reference['path']) != reference:
                raise ValueError('private source changed during epoch recovery')
    if seed is not None and state.get('recovery_seeded_epoch') == state['pending_epoch'] and not state.get('recovery_input_uid'):
        raise ValueError('legacy epoch receipt lacks private input ACK; explicit parent arbitration required')
    if seed is not None and state.get('recovery_seeded_epoch') != state['pending_epoch']:
        require_sources()
        if state.get('recovery_job'):
            job_name = state['recovery_job']
            acknowledged = owned(target, 'Job', job_name)
            if (acknowledged['metadata']['uid'] != state.get('recovery_job_uid')
                    or not state.get('recovery_input_uid')
                    or not any(e.get('kind') == 'Secret' and e.get('name') == state.get('recovery_input_name')
                               and e.get('uid') == state['recovery_input_uid'] and e.get('namespace') == CLUSTER
                               and e.get('context') == target.data.get('context') and e.get('cluster') == target.data.get('cluster')
                               for e in target.data.get('ownedResources', []))):
                raise ValueError('private epoch transfer ACK absent; no adoption or implicit retry')
        else:
            run_id = uuid.uuid4().hex[:16]
            jobs = [o for o in render(selected_profile, 'bootstrap', values={'runId': run_id})
                    if o['metadata'].get('annotations', {}).get('aiforall.dev/seed') == contract['seedContract']]
            if len(jobs) != 1:
                raise ValueError('unique declared epoch seed required')
            job = jobs[0]
            for volume in job['spec']['template']['spec']['volumes']:
                if volume.get('configMap'):
                    owned(target, 'ConfigMap', volume['configMap']['name'])
            state['recovery_job'] = job['metadata']['name']
            save()
            data = runtime.lease(Path(state['lease_path']))
            created = runtime.create(job, SimpleNamespace(lease=Path(state['lease_path'])), data)
            if not created['metadata'].get('uid'):
                raise ValueError('epoch seed Job UID ACK missing')
            input_name = next(v['secret']['secretName'] for v in job['spec']['template']['spec']['volumes'] if v.get('secret'))
            state['recovery_job_uid'] = created['metadata']['uid']
            state['recovery_input_name'] = input_name
            save()  # Persist the acknowledged parent before confidential transfer.
            secret = make_secret(input_name, {'inputs.json': json.dumps(seed).encode()})
            secret['metadata']['ownerReferences'] = [{'apiVersion': job['apiVersion'], 'kind': 'Job',
                'name': job['metadata']['name'], 'uid': created['metadata']['uid'], 'controller': True}]
            acknowledged = runtime.create(secret, SimpleNamespace(lease=Path(state['lease_path'])), data)
            if not acknowledged['metadata'].get('uid'):
                raise ValueError('private epoch input UID ACK missing')
            state['recovery_input_uid'] = acknowledged['metadata']['uid']
            save()
            job_name = job['metadata']['name']
        target.kubectl(['wait', '--for=condition=complete', 'job/' + job_name, '-n', CLUSTER, '--timeout=240s'], text=True, timeout=270)
        state['recovery_seeded_epoch'] = state['pending_epoch']
        save()
    for namespace, name in deployments:
        require_sources()
        deployment = owned(target, 'Deployment', name, namespace)
        metadata = deployment['metadata']
        annotations = deployment['spec']['template'].get('metadata', {}).get('annotations')
        path = '/spec/template/metadata/annotations'
        value = {'aiforall.dev/openbao-epoch': state['pending_epoch']}
        if annotations is not None:
            path += '/aiforall.dev~1openbao-epoch'
            value = state['pending_epoch']
        patch = [{'op': 'test', 'path': '/metadata/uid', 'value': metadata['uid']},
                 {'op': 'test', 'path': '/metadata/resourceVersion', 'value': metadata['resourceVersion']},
                 {'op': 'add', 'path': path, 'value': value}]
        target.kubectl(['patch', 'deployment/' + name, '-n', namespace, '--type=json', '-p', json.dumps(patch)], text=True, timeout=30)
        target.kubectl(['rollout', 'status', 'deployment/' + name, '-n', namespace, '--timeout=300s'], text=True, timeout=330)
    if coffer_epoch(target) != state['pending_epoch']:
        raise ValueError('OpenBao restarted during recovery; environment not ready')



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
    coffer = lifecycle.coffer(target.data)
    if not any(e.get("kind") == "Deployment" and e.get("name") == coffer["name"] and e.get("namespace") == coffer["namespace"]
            for e in target.data.get("ownedResources", [])):
        raise ValueError("initial deployment must be explicitly installed before registering local-up")
    required = lifecycle.profiles()[lifecycle.profile(target.data)]["inputs"]
    if any({"github_input": reference, "simulated_input": simulated_reference}.get(key) is None for key in required):
        raise ValueError("register every declared private profile source for future coffer epochs")
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
    """Measure under the existing authority; reconcile only declared API deltas."""
    from local_full_network import api_preimages, apply_changes, control_plane
    nodes = json.loads(target.kubectl(["get", "nodes", "-l", "node-role.kubernetes.io/control-plane", "-o", "json"], text=True, timeout=30))["items"]
    _, address = control_plane(nodes)
    changes = api_preimages(target, address, owned)
    apply_changes(target, changes)


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
    operation = target.data.get("localFullOperation")
    if operation and operation.get("state") != "complete":
        if args.action == "up":
            raise ValueError("previous local-full operation is unresolved; parent arbitration required")
        return {"status": "incomplete-operation", "ready": False, "phase": operation.get("phase"),
                "requires_parent_arbitration": True}
    declarations = None if args.action == "status" and not target.data.get("connectorProfile") else lifecycle.objects(target.data)
    if args.action == "up":
        if registry:
            with registry.operation():
                registry.up(None, registry.state["port"])
        target.resume_nodes(state["engine"])
        target.kubectl(["wait", "--for=condition=Ready", "nodes", "--all", "--timeout=300s"], text=True, timeout=330)
        resync_api_policies(target)
        coffer = lifecycle.coffer(target.data)
        target.kubectl(["rollout", "status", "deployment/" + coffer["name"], "-n", coffer["namespace"], "--timeout=300s"], text=True, timeout=330)
    epoch = coffer_epoch(target)
    changed = state.get("pending_epoch") or (state["epoch"] is not None and epoch != state["epoch"])
    if args.action == "up" and (changed or any(not n["running"] for n in nodes)):
        if state.get("pending_epoch") and state["pending_epoch"] != epoch:
            raise ValueError("another coffer epoch interrupted recovery; inspect the recorded attempt")
        state["pending_epoch"] = epoch
        save()
        recover_coffer(target, state, save, runtime, read_github, make_secret)
        for key in ("pending_epoch", "recovery_job", "recovery_seeded_epoch", "recovery_job_uid", "recovery_input_name", "recovery_input_uid"):
            state.pop(key, None)
    if args.action == "up":
        state["epoch"] = epoch
        save()
    deployments = []
    selected = None if declarations is None else {(o["metadata"].get("namespace"), o["metadata"]["name"])
                for o in declarations if o["kind"] == "Deployment"}
    for entry in target.data.get("ownedResources", []):
        if entry.get("kind") != "Deployment" or (selected is not None and (entry.get("namespace"), entry.get("name")) not in selected):
            continue
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
