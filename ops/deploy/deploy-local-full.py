"""Parent-run local-full facade and authority guards; sequencing lives in just."""
import copy
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import time
import uuid
import sys
import bootstrap_local_kind
from pathlib import Path
import local_full_target as target_authority
from local_full_target import private_file
from local_full_secrets import envelope as secret

ROOT = Path(__file__).resolve().parents[2]
CONTEXT = "kind-aiforall-local-full"
CLUSTER = "aiforall-local-full"


def load_module(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


runtime = load_module("storage_runtime", "local-full-storage.py")
renderer = load_module("local_full_renderer", "render-local-full.py")
image_provenance = load_module("local_full_image_provenance", "local_full_image_provenance.py")


def k(*args, stdin=None, timeout=360):
    return runtime.kubectl(*args, stdin=stdin, timeout=timeout)


def apply(items):
    if items:
        # Server-side apply avoids client-side last-applied annotations containing
        # local key material. Output is object names, never configuration data.
        k("apply", "--server-side", "--field-manager=local-full", "-f", "-", "-o", "name",
          stdin=json.dumps({"apiVersion": "v1", "kind": "List", "items": items}))


def resource_state(obj, data):
    """One ownership rule for preflight and apply, including missing ACKed objects."""
    kind, metadata = obj["kind"], obj["metadata"]
    namespace, name = metadata.get("namespace"), metadata["name"]
    flags = ["get", kind, name, "--ignore-not-found", "-o", "json"]
    if namespace:
        flags += ["-n", namespace]
    raw = k(*flags)
    expected = {e["uid"] for e in data.get("ownedResources", []) if e.get("kind") == kind
                and e.get("name") == name and e.get("namespace") == namespace
                and e.get("context") == CONTEXT and e.get("cluster") == CLUSTER}
    if kind == "Namespace":
        expected = set(filter(None, [data.get("namespaceUids", {}).get(name)]))
    if kind in {"CustomResourceDefinition", "ClusterRole", "ClusterRoleBinding"}:
        expected = set(filter(None, [data.get("clusterResourceUids", {}).get(kind + "/" + name)]))
    current = json.loads(raw) if raw else None
    if raw:
        if current["metadata"]["uid"] not in expected:
            raise ValueError("resource UID is not explicitly owned")
        if kind == "Namespace" and current["metadata"].get("labels", {}).get("aiforall.dev/profile") != "local-full":
            raise ValueError("namespace belongs to another profile")
    elif expected:
        raise ValueError("previously owned resource disappeared; no implicit recreation")
    return current


def apply_owned(items, args, data):
    """Create-only or UID/RV-bound apply, with an ACK before the next object."""
    for item in items:
        if item["kind"] == "Secret":
            raise ValueError("confidential envelopes require the create-only/CAS boundary")
        current = resource_state(item, data)
        if current:
            item = copy.deepcopy(item)
            item["metadata"].update(uid=current["metadata"]["uid"], resourceVersion=current["metadata"]["resourceVersion"])
            apply([item])
        else:
            item = runtime.create(item, args, data)
            if not item["metadata"].get("uid"):
                raise ValueError("resource creation UID ACK missing")
        record_resources([item], args, data)


def prepare_namespaces(args, data, objects):
    for obj in objects:
        if obj["kind"] == "Namespace" and not resource_state(obj, data):
            runtime.create(obj, args, data)


def preflight_resources(objects, data):
    """Validate every live preimage before quiescence; never a storage/schema migration."""
    for obj in objects:
        if obj["kind"] == "Secret":
            continue  # Create-only collision handling never reads a live Secret.
        current = resource_state(obj, data)
        if not current:
            continue
        if obj["kind"] == "CustomResourceDefinition":
            expected, actual = obj["spec"], current["spec"]
            if actual["names"]["kind"] != expected["names"]["kind"] or actual["scope"] != expected["scope"] or actual["versions"] != expected["versions"]:
                raise ValueError("existing AdmissionRecord schema differs; preserve it and escalate")
        encoded = obj["metadata"].get("annotations", {}).get("aiforall.dev/persistence")
        if obj["kind"] != "Deployment" or not encoded:
            continue
        volume_name = json.loads(encoded)["volume"]
        def selected(deployment):
            return [v for v in deployment["spec"]["template"]["spec"].get("volumes", []) if v["name"] == volume_name]
        if selected(current) != selected(obj):
            raise ValueError("storage transition refused for " + obj["metadata"]["namespace"] + "/" + obj["metadata"]["name"]
                              + "; preserve and verify the existing data before an explicit cutover; --confirm-quiesce is not a migration approval")


def record_resources(objects, args, data):
    for obj in objects:
        if obj["kind"] == "Secret":
            # Track local input ownership without GET of a live Secret.
            data.setdefault("localSecretInputs", {})[obj["metadata"]["namespace"] + "/" + obj["metadata"]["name"]] = True
            continue
        flags = ["get", obj["kind"], obj["metadata"]["name"], "-o", "jsonpath={.metadata.uid}"]
        if obj["metadata"].get("namespace"):
            flags += ["-n", obj["metadata"]["namespace"]]
        uid = obj["metadata"].get("uid") or k(*flags)
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
    from local_registry import pki
    for _, name, _ in (row for row in pki.leaf_profiles() if row[0] == "mesh"):
        for suffix in ["crt", "key"]:
            file = name + "." + suffix
            files[file] = (mesh / file).read_bytes()
    identity_files = {name: (workload / name).read_bytes() for name in ["ca.crt", "ca.key", "server.crt", "server.key"]}
    if files["ca.crt"] == identity_files["ca.crt"]:
        raise ValueError("transport and workload CA must be distinct")
    # Public rollout identity only: never send a private-key fingerprint to Helm.
    digest = hashlib.sha256(b"".join(files[key] for key in sorted(files) if key.endswith(".crt"))
                            + b"".join(identity_files[key] for key in sorted(identity_files) if key.endswith(".crt"))).hexdigest()
    return files, identity_files, digest


def oauth_inputs(path):
    if not path or not path.is_file() or path.is_symlink() or not private_file(path):
        raise ValueError("private read-only OAuth proof input and private parent directory required")
    if path.stat().st_size > 4096:
        raise ValueError("private OAuth proof input exceeds bound")
    try:
        return renderer.validate_proof_inputs(json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=target_authority.unique_json))
    except target_authority.AnchorError:
        raise ValueError("ambiguous private OAuth proof input") from None


def oauth_cert_inputs(path, run_id):
    mesh = path / "mesh"
    public = (mesh / "ca.crt").read_bytes()
    leaf = {"tls.crt": (mesh / "gitlab-connect.crt").read_bytes(),
            "tls.key": (mesh / "gitlab-connect.key").read_bytes()}
    # New per-run objects, never copies of the parent CA or gateway private key.
    objects = [secret("oauth-proof-public-ca-" + run_id, {"ca.crt": public}),
               secret("oauth-proof-connector-tls-" + run_id, leaf)]
    digest = hashlib.sha256(public + leaf["tls.crt"]).hexdigest()
    return objects, digest


def github_inputs(path):
    from local_full_environment import input_reference, read_private_input
    try:
        reference = input_reference(path)
        return read_private_input(reference, _parse_github_inputs)
    except (OSError, ValueError, KeyError, TypeError, UnicodeError):
        raise ValueError("invalid private GitHub input source (values redacted)") from None


def _parse_github_inputs(path):
    if not path or not path.is_file() or path.is_symlink() or not private_file(path) or path.stat().st_size > 65536:
        raise ValueError("private read-only GitHub input file in a private directory required")
    text = path.read_text(encoding="utf-8")
    if text.lstrip().startswith("{"):
        value = json.loads(text, object_pairs_hook=target_authority.unique_json)
    else:
        # An allowlist, NOT a shell/dotenv evaluator. No expansion or execution.
        keys = {"GITHUB_CLIENT_ID": "client_id", "GITHUB_CLIENT_SECRET": "client_secret",
                "GITHUB_IAM_HMAC_SECRET": "hmac_secret"}
        value = {"version": 1}
        for line in text.splitlines():
            key, separator, item = line.removeprefix("export ").partition("=")
            if separator and key.strip() in keys:
                field = keys[key.strip()]
                if field in value:
                    raise ValueError("duplicate private GitHub input")
                item = item.strip()
                if item.startswith('"'):
                    item = json.loads(item)
                elif item.startswith("'") and item.endswith("'"):
                    item = item[1:-1]
                value[field] = item
    if (set(value) != {"version", "client_id", "client_secret", "hmac_secret"}
            or type(value["version"]) is not int or value["version"] != 1
            or not isinstance(value["client_id"], str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", value["client_id"])
            or not isinstance(value["client_secret"], str) or not 1 <= len(value["client_secret"]) <= 4096
            or not isinstance(value["hmac_secret"], str) or not re.fullmatch(r"[a-f0-9]{64}", value["hmac_secret"])):
        raise ValueError("invalid private GitHub input schema (values redacted)")
    return value


def openbao_cert_inputs(path, run_id, github, oauth_connect=False, *, validate=True):
    extension = path / ("extensions/openbao-oauthconnect" if oauth_connect else "extensions/openbao-github")
    if not extension.is_dir():
        raise ValueError("explicit OpenBao connector certificate extension must be prepared first")
    # Same shared PKI verifier on both OS; an existing complete bundle is mandatory.
    if validate:
        validation = subprocess.run([sys.executable, "-B", str(ROOT / "ops/scripts/prepare-local-full-certs.py"),
            "--output-dir", str(path), "--oauth-connect" if oauth_connect else "--openbao-github"], capture_output=True, timeout=300)
        if validation.returncode:
            raise ValueError("OpenBao certificate bundle validation failed (details redacted)")
    from local_full_lifecycle import _objects
    from local_full_declarations import identity as declarations_identity
    profile = "oauth-connect" if oauth_connect else "github-openbao" if github else "base"
    declarations = _objects(profile, declarations_identity())
    leaves = {match.group(1) for obj in declarations if obj["kind"] == "Deployment"
              for volume in obj["spec"]["template"]["spec"].get("volumes", [])
              if (match := re.fullmatch(r"([a-z0-9-]+)-server-tls-[a-z0-9]{12,32}", volume.get("secret", {}).get("secretName", "")))}
    result = []
    for name in sorted(leaves):
        result.append(secret(name + "-server-tls-" + run_id, {
            "tls.crt": (extension / (name + ".crt")).read_bytes(), "tls.key": (extension / (name + ".key")).read_bytes()}))
    return result


def verify_images(images, registry_state=None):
    runtime.target().check()
    if any("@sha256:" in image for image in images.values()):
        if not registry_state or any("@sha256:" not in image for image in images.values()):
            raise ValueError("OCI deployment requires --registry-state and an all-digest mapping; no kind-load fallback")
        from local_registry import Registry
        registry = Registry(registry_state)
        with registry.operation():
            registry.verify_mapping(images)
            runtime.target().registry_connect(registry)
            for image in set(images.values()):
                runtime.target().registry_pull(registry, image)
            registry.verify_mapping(images)
        return
    for image in set(images.values()):
        match = re.fullmatch(r"[a-z0-9./:_-]+:sha256-([a-f0-9]{64})", image)
        if not match:
            raise ValueError("application image must use a full image-ID content tag")
        result = subprocess.run(["docker", "image", "inspect", "--format", "{{.Id}}", image], capture_output=True, text=True, timeout=30)
        if result.returncode or result.stdout.strip() != "sha256:" + match.group(1):
            raise ValueError("local immutable image identity mismatch")
        runtime.target().kind_load(image)


def ingress_policy_catalog(profile="base"):
    """Read the same declared policy objects; never maintain an overlay copy."""
    from local_full_declarations import identity
    from local_full_lifecycle import _objects
    return {obj["metadata"]["name"]: copy.deepcopy(obj["spec"]) for obj in _objects(profile, identity())
            if obj["kind"] == "NetworkPolicy" and obj["metadata"]["namespace"] == CLUSTER}


_INGRESS_PROTOCOL = json.loads((renderer.helm.CHART / "files/ingress-attestation.json").read_text(encoding="utf-8"))
_INGRESS_HISTORICAL_OBSERVATION = _INGRESS_PROTOCOL["_INGRESS_HISTORICAL_OBSERVATION"]
_INGRESS_HISTORICAL_CAPTURE_HEAD = _INGRESS_PROTOCOL["_INGRESS_HISTORICAL_CAPTURE_HEAD"]
_INGRESS_PROJECTION_NAMES = tuple(_INGRESS_PROTOCOL["_INGRESS_PROJECTION_NAMES"])
# Exact producer/mutation/manifest qualified by the lead and sealed audit receipt.
# No runtime file, caller argument or environment variable can authorize a pair.
_INGRESS_CAPTURE_COMPATIBILITY = tuple((head, helper, tuple(map(tuple, manifest)), mutation, receipt)
    for head, helper, manifest, mutation, receipt in _INGRESS_PROTOCOL["_INGRESS_CAPTURE_COMPATIBILITY"])


def _ingress_json(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def _ingress_require(condition):
    if not condition:
        raise ValueError("INCONCLUSIVE: Envoy ingress gate refused; details redacted")


def _ingress_fields(value, group):
    return all(type(value[key]) is type(expected) and value[key] == expected
               for key, expected in _INGRESS_PROTOCOL[group].items())


def _ingress_authority(files, hashes):
    _ingress_require(runtime.LocalClusterTargetAnchor is target_authority.LocalClusterTargetAnchor)
    path = Path(target_authority.__file__).resolve()
    current = hashlib.sha256(path.read_bytes()).hexdigest()
    summary = files["summary"]
    manifest = tuple((name, hashes[name]) for name in _INGRESS_PROJECTION_NAMES)
    hex_value = lambda value, count: isinstance(value, str) and bool(re.fullmatch("[a-f0-9]{" + str(count) + "}", value))
    _ingress_require(isinstance(_INGRESS_CAPTURE_COMPATIBILITY, tuple))
    for entry in _INGRESS_CAPTURE_COMPATIBILITY:
        _ingress_require(isinstance(entry, tuple) and len(entry) == 5
            and hex_value(entry[0], 40) and all(hex_value(entry[index], 64) for index in (1, 3, 4))
            and isinstance(entry[2], tuple) and len(entry[2]) == len(_INGRESS_PROJECTION_NAMES))
        for pair, name in zip(entry[2], _INGRESS_PROJECTION_NAMES):
            _ingress_require(isinstance(pair, tuple) and len(pair) == 2
                and pair[0] == name and hex_value(pair[1], 64))
    matches = [entry for entry in _INGRESS_CAPTURE_COMPATIBILITY if entry[2][0][1] == hashes["observation"]]
    reserved = (hashes["observation"] == _INGRESS_HISTORICAL_OBSERVATION
        or summary["ops_head"] == _INGRESS_HISTORICAL_CAPTURE_HEAD
        or bool(matches) or any(entry[0] == summary["ops_head"] for entry in _INGRESS_CAPTURE_COMPATIBILITY))
    if reserved:
        # A reserved producer cannot relabel an altered observation as a new
        # capture by rewriting summary to the current helper's hash.
        _ingress_require(len(matches) == 1)
        qualified = matches[0]
        _ingress_require(qualified[0] == summary["ops_head"] and qualified[1] == summary["helper_sha256"]
            and qualified[2] == manifest and qualified[3] == current)
        compatibility = {"mode": "QUALIFIED_EXACT_PAIR", "entry": qualified}
        qualification_sha256 = qualified[4]
    else:
        _ingress_require(summary["helper_sha256"] == current)
        compatibility = {"mode": "IDENTICAL_RAW"}
        qualification_sha256 = None
    return json.loads(_ingress_json({"capture_ops_head": summary["ops_head"], "capture_helper_raw_sha256": summary["helper_sha256"],
            "capture_projections_raw_sha256": manifest, "qualification_record_sha256": qualification_sha256,
            "mutation_helper_raw_sha256": current, "mutation_module_path": str(path),
            "compatibility": compatibility}))


def _ingress_write_record(path, record):
    path = Path(path)
    _ingress_require(path.parent.is_dir() and not path.parent.is_symlink()
        and path.absolute() == path.resolve() and not path.exists())
    raw = _ingress_json(record).encode()
    with path.open("xb") as file:
        file.write(raw)
        file.flush()
        os.fsync(file.fileno())
    target_authority.secure(path)
    _ingress_require(private_file(path) and path.read_bytes() == raw)
    return hashlib.sha256(raw).hexdigest()


def _ingress_policy_postimage(raw, uid, spec, *, previous_version=None):
    item = json.loads(raw, object_pairs_hook=target_authority.unique_json)
    meta = item["metadata"]
    _ingress_require(item["apiVersion"] == "networking.k8s.io/v1" and item["kind"] == "NetworkPolicy"
        and meta["name"] == "allow-envoy" and meta["namespace"] == CLUSTER and meta["uid"] == uid
        and isinstance(meta["resourceVersion"], str) and bool(meta["resourceVersion"].strip())
        and not meta.get("deletionTimestamp") and item["spec"] == spec
        and (previous_version is None or meta["resourceVersion"] != previous_version))
    return json.loads(_ingress_json({"uid": meta["uid"], "rv": meta["resourceVersion"], "spec": item["spec"]}))


def _ingress_admin_pair(before, after, identity):
    expected = {key: identity[key] for key in _INGRESS_PROTOCOL["adminIdentityFields"]}
    for item in (before, after):
        _ingress_require(_ingress_json(item["identity"]) == _ingress_json(expected) and item["state"] == "LIVE"
                         and item["version"] == "1.31.2"
                         and type(item["restart_epoch"]) is int and item["restart_epoch"] == 0)
    for key in ("uptime_current_epoch_ns", "uptime_all_epochs_ns"):
        _ingress_require(type(before[key]) is int and type(after[key]) is int
                         and 0 <= before[key] <= after[key])
    # Only explicit counters count. Other collector fields can legitimately be
    # MISSING; never turn those into zero or claim per-flow packet attribution.
    for suffix in _INGRESS_PROTOCOL["upstreamCounters"]:
        key = "cluster.iam." + suffix
        a, b = before["stats"][key], after["stats"][key]
        _ingress_require(type(a) is int and type(b) is int and 0 <= a <= b)


def _ingress_evidence(directory):
    """Read only the seven closed, private, nonsecret parent projections."""
    directory = Path(directory)
    _ingress_require(directory.is_dir() and not directory.is_symlink()
                     and directory.absolute() == directory.resolve())
    files, hashes = {}, {}
    for name in _INGRESS_PROJECTION_NAMES:
        path = directory / (name + ".json")
        _ingress_require(path.is_file() and not path.is_symlink() and path.absolute() == path.resolve() and private_file(path)
                         and path.stat().st_size <= 131072)
        raw = path.read_bytes()
        hashes[name] = hashlib.sha256(raw).hexdigest()
        files[name] = json.loads(raw, object_pairs_hook=target_authority.unique_json)
    obs, summary, review = (files[name] for name in ("observation", "summary", "causal-review"))
    identity, binding = obs["identity"], obs["nat_binding"]
    import ipaddress
    address = ipaddress.IPv4Address(binding["reply"]["dst"])
    source = str(address)
    _ingress_require(source == binding["reply"]["dst"] and source != identity["control_plane_ipv4"]
        and not any((address.is_unspecified, address.is_loopback, address.is_multicast, address.is_link_local)))
    _ingress_require(_ingress_fields(review, "review")
        and review["observation_sha256"] == hashes["observation"]
        and review["measured_source_ipv4"] == source
        and isinstance(review["reason"], str) and len(review["reason"].strip()) >= 80
        and isinstance(review["limitations"], list) and len(review["limitations"]) >= 5)
    _ingress_require(set(review["causal_basis"]) == set(_INGRESS_PROTOCOL["causalBasis"]))
    for key, identity_key in (("control_plane_container_id", "control_plane_container_id"),
            ("control_plane_node_uid", "control_plane_node_uid"), ("envoy_pod_uid", "pod_uid"),
            ("service_uid", "service_uid")):
        _ingress_require(review[key] == identity[identity_key])
    _ingress_require(binding["original"]["dst"] == identity["control_plane_ipv4"]
        and binding["original"]["dport"] == 30080 and binding["reply"]["src"] == identity["pod_ipv4"]
        and binding["reply"]["sport"] == 10000 and binding["id"] == review["nat_id"]
        and binding["zone_evidence"] == "SOURCE_FILTERED" and binding["requested_zone"] == 0
        and _ingress_fields(obs, "observation"))
    for item in (files["baseline"], files["passive-baseline"]):
        _ingress_require(item["status"] == "BASELINE_ONLY" and item["identity"] == identity
            and item["rules_fingerprint"] == obs["rules_fingerprint"])
    _ingress_require(files["baseline"]["deny"] == files["passive-baseline"]["deny"]
        and review["rules_fingerprint"] == obs["rules_fingerprint"]
        and _ingress_fields(summary, "summary")
        and bool(re.fullmatch(r"[a-f0-9]{40}", summary["ops_head"]))
        and summary["counter_scope"] == obs["counter_scope"]
        and summary["packet_attribution"] == obs["packet_attribution"]
        and summary["envoy_pod_uid"] == identity["pod_uid"]
        and summary["restart_epoch"] == files["admin-after"]["restart_epoch"]
        and summary["http_attempts"] == review["http_attempts"] == 1
        and summary["passive_deny_delta"] == review["passive_deny_delta"] == {"packets": 0, "bytes": 0}
        and summary["nat_binding"] == binding
        and summary["endpoint_deny_delta"] == review["endpoint_deny_delta"] == obs["endpoint_deny_delta"]
        and type(obs["endpoint_deny_delta"]["packets"]) is int and obs["endpoint_deny_delta"]["packets"] > 0
        and type(obs["endpoint_deny_delta"]["bytes"]) is int and obs["endpoint_deny_delta"]["bytes"] > 0)
    # The imported mutation authority and the immutable capture producer are
    # distinct only through the explicitly qualified, versioned exact pair.
    _ingress_authority(files, hashes)
    _ingress_admin_pair(files["admin-before"], files["admin-after"], identity)
    _ingress_require(files["admin-before"]["stats"] == files["admin-after"]["stats"])
    _ingress_require(summary["iam_upstream_deltas"] == {"cluster.iam." + suffix: 0 for suffix in _INGRESS_PROTOCOL["upstreamCounters"]})
    return files, hashes, source


def _ingress_live(guard, identity, review, source):
    """Existing anchor/owner/incarnation authority plus public network metadata."""
    data = guard.check()
    _ingress_require(data["cluster"] == CLUSTER and data["context"] == CONTEXT
        and data["control_plane_container_id"] == identity["control_plane_container_id"]
        and len(data["nodeContainerIds"]) == 3)
    incarnation = {key: identity[key] for key in ("container_id", "restart_count", "image_id")}
    owners, incarnation = guard._envoy_tuple_attestation(pod=identity["pod"], namespace_uid=identity["namespace_uid"],
        pod_uid=identity["pod_uid"], container="envoy", owner_chain=identity["owner_chain"],
        incarnation=incarnation, node_uid=identity["node_uid"], service_uid=identity["service_uid"])
    actual = guard._envoy_tuple_identity(identity["pod"], identity["namespace_uid"], identity["pod_uid"],
        owners, incarnation, identity["node_uid"], identity["service_uid"], time.monotonic() + 30)
    # The tuple helper's identity predates the CP annotation extension. Heartbeat
    # resourceVersions are intentionally not semantic identity or proof expiry.
    _ingress_require(_ingress_json(actual) == _ingress_json({key: value for key, value in identity.items()
                     if key not in {"control_plane_node_uid", "control_plane_ipv4"}}))
    fields = ('jsonpath={range .items[*]}{.metadata.uid}{"\\t"}{.metadata.name}{"\\t"}'
        '{.metadata.labels.node-role\\.kubernetes\\.io/control-plane}{"\\t"}'
        '{.metadata.annotations.projectcalico\\.org/IPv4IPIPTunnelAddr}{"\\t"}'
        '{.metadata.annotations.projectcalico\\.org/IPv4Address}{"\\t"}'
        '{range .status.addresses[?(@.type=="InternalIP")]}{.address}{end}{"\\n"}{end}')
    rows = [line.split("\t") for line in guard.kubectl(["get", "nodes", "-o", fields], text=True, timeout=30).splitlines()]
    _ingress_require(len(rows) == 3 and all(len(row) == 6 for row in rows)
        and len({row[0] for row in rows}) == 3 and len({row[1] for row in rows}) == 3)
    cp = [row for row in rows if row[0] == identity["control_plane_node_uid"]]
    _ingress_require(len(cp) == 1 and cp[0][3] == source
        and cp[0][5] == identity["control_plane_ipv4"]
        and cp[0][4].split("/")[0] == identity["control_plane_ipv4"]
        and len([row for row in rows if row[3] == source]) == 1)
    # A separate label query proves role even when the label's value is empty.
    cp_uids = guard.kubectl(["get", "nodes", "-l", "node-role.kubernetes.io/control-plane", "-o",
        "jsonpath={range .items[*]}{.metadata.uid}{'\\n'}{end}"], text=True, timeout=30).splitlines()
    _ingress_require(cp_uids == [identity["control_plane_node_uid"]])
    slices = json.loads(guard.kubectl(["get", "endpointslices", "-n", CLUSTER, "-l",
        "kubernetes.io/service-name=envoy-mesh", "-o", "json"], text=True, timeout=30))
    endpoints = []
    for item in slices["items"]:
        _ingress_require(any(ref.get("kind") == "Service" and ref.get("uid") == identity["service_uid"]
            and ref.get("name") == "envoy-mesh" and ref.get("controller") is True
            for ref in item["metadata"]["ownerReferences"]))
        _ingress_require(item["ports"] == _INGRESS_PROTOCOL["endpointPorts"])
        endpoints.extend(item["endpoints"])
    _ingress_require(len(endpoints) == 1 and endpoints[0]["conditions"]["ready"] is True
        and endpoints[0]["conditions"].get("terminating", False) is False
        and endpoints[0]["addresses"] == [identity["pod_ipv4"]]
        and endpoints[0]["nodeName"] == identity["node_name"]
        and endpoints[0]["targetRef"]["uid"] == identity["pod_uid"])
    items = json.loads(guard.kubectl(["get", "networkpolicies", "-n", CLUSTER, "-o", "json"],
                                   text=True, timeout=30))["items"]
    policies = {}
    for item in items:
        meta = item["metadata"]
        _ingress_require(meta["namespace"] == CLUSTER and not meta.get("deletionTimestamp")
            and meta["name"] not in policies and any(all(entry.get(key) == value for key, value in {
                "kind": "NetworkPolicy", "name": meta["name"], "namespace": CLUSTER,
                "uid": meta["uid"], "context": CONTEXT, "cluster": CLUSTER}.items())
                for entry in data.get("ownedResources", [])))
        policies[meta["name"]] = {"uid": meta["uid"], "rv": meta["resourceVersion"], "spec": item["spec"]}
    _ingress_require(policies["allow-envoy"]["uid"] == review["policy_uid"]
        and policies["default-deny"]["uid"] == review["default_deny_uid"]
        and policies["default-deny"]["spec"] == {"podSelector": {}, "policyTypes": ["Ingress", "Egress"]})
    return policies


def _ingress_base_full_witness(witness, identity):
    """Validate closed parent observations, not the truth of HTTP/profile claims."""
    _ingress_require(set(witness) == set(_INGRESS_PROTOCOL["witnessFields"]))
    profile = witness["profile_before"]
    _ingress_require(profile == witness["profile_after"] and set(profile) == set(_INGRESS_PROTOCOL["profileFields"]))
    _ingress_require(profile["name"] == "local-full" and profile["connectors"] == []
        and profile["namespace_uid"] == identity["namespace_uid"]
        and bool(re.fullmatch(r"iam-dev-config(?:-[a-z0-9]+)?", profile["config_map_name"]))
        and bool(re.fullmatch(r"[a-f0-9]{64}", profile["config_sha256"]))
        and bool(re.fullmatch(r"containerd://[a-f0-9]{64}", profile["iam_container_id"]))
        and type(profile["iam_restart_count"]) is int and profile["iam_restart_count"] >= 0
        and isinstance(profile["config_resource_version"], str) and profile["config_resource_version"].isdigit())
    for field in ("config_map_uid", "iam_pod_uid"):
        _ingress_require(str(uuid.UUID(profile[field])) == profile[field])
    _ingress_require(witness["profile_sha256"] == hashlib.sha256(_ingress_json(profile).encode()).hexdigest())
    probes = witness["probes"]
    _ingress_require(isinstance(probes, list) and len(probes) == 3)
    for probe, (path, statuses, outcome) in zip(probes, _INGRESS_PROTOCOL["probes"]):
        _ingress_require(set(probe) == set(_INGRESS_PROTOCOL["probeFields"])
            and probe["method"] == "GET" and probe["path"] == path
            and type(probe["status"]) is int and probe["status"] in statuses and probe["outcome"] == outcome
            and bool(re.fullmatch(r"[a-f0-9]{64}", probe["response_sha256"])))
    _ingress_require(witness["observations_sha256"] == hashlib.sha256(_ingress_json(probes).encode()).hexdigest())


class EnvoyIngressChange:
    """Parent-only pending change. No HTTP callback, URL or deployment actor.

    The parent must call finish with the fixed-route witness or rollback on any
    failed/abandoned validation. No-op owns nothing. Receipts are copies, never
    aliases of the state used for conditional rollback. No packet proof implied.
    """

    def __init__(self, guard, directory, files, hashes, source, before, desired, journal_sha256, added, ack, authority, journal_path):
        self._guard, self._directory = guard, directory
        self._authority_class = target_authority.LocalClusterTargetAnchor
        self._journal_path = Path(journal_path)
        self._frozen = _ingress_json({"files": files, "hashes": hashes, "source": source,
            "before": before, "desired": desired, "journal_sha256": journal_sha256, "ack": ack,
            "authority": authority, "post_version": ack["rv"]})
        self._added, self._closed, self._validated, self._blocked = added, False, False, False
        self._rollback_ack = None
        self._result = "PENDING_PARENT_VALIDATION" if added else "NOOP_NOT_OWNED"

    @property
    def receipt(self):
        data = json.loads(self._frozen)
        return {"state": self._result,
                "journal_sha256": data["journal_sha256"], "source": data["source"], "added": self._added,
                "policy_uid": data["before"]["allow-envoy"]["uid"], "policy_resource_version": data["post_version"],
                "authority": data["authority"], "ack_source": "PATCH_RESPONSE" if self._added else "NOT_OWNED",
                "rollback_ack": json.loads(self._rollback_ack) if self._rollback_ack is not None else None}

    def rollback(self):
        if self._blocked:
            raise ValueError("STOP: conditional Envoy rollback blocked; parent intervention required")
        if not self._added or self._closed:
            return "NOT_OWNED" if not self._added else "ALREADY_CLOSED"
        try:
            data = self._revalidate_provenance()
            fresh = runtime.target()
            _ingress_require(fresh.data == self._guard.data)
            fresh.check()
            old, desired = data["before"]["allow-envoy"], data["desired"]
            observed = _ingress_policy_postimage(fresh.kubectl(["get", "networkpolicy", "allow-envoy", "-n", CLUSTER,
                "-o", "json"], text=True, timeout=30), old["uid"], desired)
            _ingress_require(observed == data["ack"])
            patch = [{"op": "test", "path": "/metadata/uid", "value": old["uid"]},
                {"op": "test", "path": "/metadata/resourceVersion", "value": data["ack"]["rv"]},
                {"op": "test", "path": "/spec", "value": desired},
                {"op": "test", "path": "/spec/ingress/1", "value": desired["ingress"][1]},
                 {"op": "remove", "path": "/spec/ingress/1"}]
            self._revalidate_provenance()
            raw = fresh.kubectl(["patch", "networkpolicy/allow-envoy", "-n", CLUSTER, "--type=json", "-p",
                _ingress_json(patch), "-o", "json"], text=True, timeout=30)
            ack = _ingress_policy_postimage(raw, old["uid"], old["spec"], previous_version=data["ack"]["rv"])
            self._rollback_ack = _ingress_json(ack)
            _ingress_write_record(self._journal_path.with_name(self._journal_path.name + ".rollback-ack.json"),
                {"schema": "aiforall.envoy.ingress-rollback-ack.v1", "journal_sha256": data["journal_sha256"],
                 "authority": data["authority"], "ack": ack})
            self._revalidate_provenance()
            after = _ingress_policy_postimage(fresh.kubectl(["get", "networkpolicy", "allow-envoy", "-n", CLUSTER,
                "-o", "json"], text=True, timeout=30), old["uid"], old["spec"])
            _ingress_require(after == ack)
            self._closed = True
            self._result = "ROLLED_BACK_OWN_ENTRY"
            return self._result
        except Exception:
            self._blocked = True
            self._result = "STOP_ROLLBACK_UNCERTAIN"
            raise ValueError("STOP: conditional Envoy rollback blocked; parent intervention required") from None

    def _revalidate_provenance(self):
        data = json.loads(self._frozen)
        _ingress_require(target_authority.LocalClusterTargetAnchor is self._authority_class
            and runtime.LocalClusterTargetAnchor is self._authority_class)
        files, hashes, source = _ingress_evidence(self._directory)
        _ingress_require(hashes == data["hashes"] and files == data["files"] and source == data["source"])
        _ingress_require(_ingress_authority(files, hashes) == data["authority"])
        return data

    def _revalidate(self):
        data = self._revalidate_provenance()
        live = _ingress_live(self._guard, data["files"]["observation"]["identity"],
                             data["files"]["causal-review"], data["source"])
        expected = json.loads(_ingress_json(data["before"]))
        expected["allow-envoy"]["spec"] = data["desired"]
        # Freeze the mutation ACK's RV, not just its spec. A later
        # equal-spec update is still concurrency and never grants ownership.
        expected["allow-envoy"]["rv"] = data["post_version"]
        _ingress_require(live == expected)

    def finish(self, witness):
        """Consume a nonsecret parent witness; failure conditionally removes ours.

        This validates the receipt's shape/binding, not the truth of an HTTP
        claim. The parent owns the real fixed-entry request and independent QA.
        """
        _ingress_require(not self._closed and not self._validated)
        try:
            self._revalidate()
            data = json.loads(self._frozen)
            _ingress_require(witness["journal_sha256"] == data["journal_sha256"]
                and witness["entry"] == "127.0.0.1:18080")
            if witness["schema"] == "aiforall.envoy.ingress-parent-witness.v1":
                _ingress_require(witness["method"] == "GET" and witness["path"] == "/iam/api/auth/gitlab/login"
                    and witness["outcome"] == "IAM_CONNECTOR_NONCE_REDIRECT"
                    and witness["connector"] == "gitlab" and witness["status"] == 303
                    and witness["protected_without_bearer"] == "DENIED")
            else:
                _ingress_require(witness["schema"] == "aiforall.envoy.ingress-base-full-witness.v1")
                _ingress_base_full_witness(witness, data["files"]["observation"]["identity"])
            _ingress_admin_pair(data["files"]["admin-after"], witness["admin_before"], data["files"]["observation"]["identity"])
            _ingress_admin_pair(witness["admin_before"], witness["admin_after"], data["files"]["observation"]["identity"])
            _ingress_require(witness["admin_after"]["stats"]["cluster.iam.upstream_rq_total"]
                             > witness["admin_before"]["stats"]["cluster.iam.upstream_rq_total"])
            for key in ("cluster.iam.ssl.fail_verify_san", "cluster.iam.ssl.fail_verify_error"):
                _ingress_require(witness["admin_before"]["stats"][key] == witness["admin_after"]["stats"][key])
            self._validated = True
            self._result = "PARENT_WITNESS_ACCEPTED_LAB_ONLY"
            # Preservation authority, not adoption of the rule or a runtime proof
            # for another source/node. Future foundation renders must reverify it.
            lease = self._guard.check()
            lease["localFullIngress"] = {"source": data["source"],
                "nodeUid": data["files"]["observation"]["identity"]["control_plane_node_uid"],
                "policyUid": data["ack"]["uid"], "spec": data["desired"],
                "journalSha256": data["journal_sha256"], "authority": data["authority"]}
            target_authority.persist(self._guard.path, lease)
            self._guard.data = lease
            return self._result
        except Exception:
            self.rollback()
            raise ValueError("STOP: parent Envoy validation refused; no qualification claimed") from None


def configure_envoy_ingress(*, evidence_directory, journal_path):
    """Specialized parent operation, deliberately NOT wired into deploy/main.

    Requires the ratified causal review, measured IPIP source and current owned
    identities. Returns a pending validation/rollback seam, never runs HTTP.
    """
    change = None
    try:
        files, hashes, source = _ingress_evidence(evidence_directory)
        authority = _ingress_authority(files, hashes)
        authority_class = target_authority.LocalClusterTargetAnchor
        identity, review = files["observation"]["identity"], files["causal-review"]
        guard = runtime.target()
        before = _ingress_live(guard, identity, review, source)
        client = {"from": [{"podSelector": {"matchLabels": {"app.kubernetes.io/name": "local-full-client"}}}],
                  "ports": [{"protocol": "TCP", "port": 10000}]}
        entry = {"from": [{"ipBlock": {"cidr": source + "/32"}}], "ports": [{"protocol": "TCP", "port": 10000}]}
        old = before["allow-envoy"]
        _ingress_require(old["spec"]["podSelector"] == {"matchLabels": {"app.kubernetes.io/name": "envoy-mesh"}}
            and old["spec"]["policyTypes"] == ["Ingress", "Egress"]
            and old["spec"]["ingress"] in ([client], [client, entry], [entry, client]))
        # Freeze all applicable policies against the declared, unchanged source.
        declared = ingress_policy_catalog()
        # The existing opt-in browser profile adds three declared policies.
        # Recognize their exact source, never unknown live policies or selectors.
        oauth = {name: spec for name, spec in ingress_policy_catalog("gitlab-proof").items() if name.startswith("oauth-proof-")}
        _ingress_require(set(oauth) == {"oauth-proof-iam-egress", "oauth-proof-connector", "oauth-proof-provider"}
            and not set(oauth).intersection(declared))
        declared.update(oauth)
        # Retained original bootstrap policy, removed from new renders by
        # f6091058. Exact spec from 678d22fd, never an arbitrary legacy selector.
        # It cannot select Envoy; preserve it, do not clean up another phase here.
        retained = {"allow-bao-seed": {"podSelector": {"matchLabels": {"app.kubernetes.io/name": "openbao-seed"}},
            "policyTypes": ["Egress"], "egress": [{"to": [{"podSelector": {
                "matchLabels": {"app.kubernetes.io/name": "openbao"}}}],
                "ports": [{"protocol": "TCP", "port": 8200}]}]}}
        _ingress_require(not set(retained).intersection(declared))
        declared.update(retained)
        for name, policy in before.items():
            selector = policy["spec"]["podSelector"]
            # Unknown expressions are not guessed non-applicable. The declared
            # policy itself must match before its selector can be trusted.
            _ingress_require(name in declared)
            if name == "allow-envoy":
                base = json.loads(_ingress_json(policy["spec"]))
                base["ingress"] = [client]
                _ingress_require(base == declared[name])
            elif (name in oauth or name in retained or name == "allow-test-client" or selector == {}
                  or selector.get("matchLabels", {}).get("app.kubernetes.io/name") == "envoy-mesh"):
                _ingress_require(policy["spec"] == declared[name])
            else:
                _ingress_require(selector == declared[name]["podSelector"])
        _ingress_require({"default-deny", "allow-dns", "allow-envoy", "allow-test-client"} <= set(before))
        desired = json.loads(_ingress_json(old["spec"]))
        added = desired["ingress"] == [client]
        if added:
            desired["ingress"].append(entry)
        patch = [{"op": "test", "path": "/metadata/uid", "value": old["uid"]},
            {"op": "test", "path": "/metadata/resourceVersion", "value": old["rv"]},
            {"op": "test", "path": "/spec", "value": old["spec"]},
            {"op": "add", "path": "/spec/ingress/-", "value": entry}] if added else []
        record = {"schema": "aiforall.envoy.ingress-journal.v1", "operation": uuid.uuid4().hex,
            "evidence_sha256": hashes, "decision": review, "identity": identity,
            "authority": authority,
            "source": source, "before": before, "expected_spec": desired, "patch": patch,
            "state": "PENDING_PARENT_VALIDATION" if added else "NOOP_NOT_OWNED"}
        path = Path(journal_path)
        journal_sha256 = _ingress_write_record(path, record)
        # Re-read evidence and all semantics immediately before mutation. There
        # is no multi-object atomicity claim; only this JSONPatch is atomic.
        again, current_hashes, current_source = _ingress_evidence(evidence_directory)
        _ingress_require(again == files and current_hashes == hashes and current_source == source
            and _ingress_authority(again, current_hashes) == authority
            and _ingress_live(guard, identity, review, source) == before)
        _ingress_require(target_authority.LocalClusterTargetAnchor is authority_class
            and runtime.LocalClusterTargetAnchor is authority_class
            and _ingress_authority(files, hashes) == authority)
        if added:
            # A transport error can follow an applied patch. Without an ACK we
            # cannot distinguish ours from a concurrent equal entry: STOP and
            # retain the journal, never adopt/remove an uncertain rule.
            raw = guard.kubectl(["patch", "networkpolicy/allow-envoy", "-n", CLUSTER, "--type=json", "-p",
                _ingress_json(patch), "-o", "json"], text=True, timeout=30)
            ack = _ingress_policy_postimage(raw, old["uid"], desired, previous_version=old["rv"])
            # This frozen response, never a following GET, attributes ownership.
            frozen_ack = _ingress_json(ack)
            _ingress_write_record(path.with_name(path.name + ".ack.json"),
                {"schema": "aiforall.envoy.ingress-patch-ack.v1", "journal_sha256": journal_sha256,
                 "authority": authority, "ack": json.loads(frozen_ack)})
        else:
            frozen_ack = _ingress_json({"uid": old["uid"], "rv": old["rv"], "spec": desired})
        _ingress_require(target_authority.LocalClusterTargetAnchor is authority_class
            and runtime.LocalClusterTargetAnchor is authority_class
            and _ingress_authority(files, hashes) == authority)
        post = _ingress_policy_postimage(guard.kubectl(["get", "networkpolicy", "allow-envoy", "-n", CLUSTER,
                                        "-o", "json"], text=True, timeout=30), old["uid"], desired)
        _ingress_require(post == json.loads(frozen_ack))
        change = EnvoyIngressChange(guard, evidence_directory, files, hashes, source, before, desired,
                                   journal_sha256, added, json.loads(frozen_ack), authority, path)
        change._revalidate()
        return change
    except Exception:
        if change is not None and change._added:
            change.rollback()
        raise ValueError("STOP: Envoy ingress refused; details redacted; inspect parent journal") from None


def deploy_locked(args, operation=None):
    from types import SimpleNamespace
    from local_full_phases import run
    run(SimpleNamespace(**globals()), args, operation)


def deploy(args):
    # A persistent exclusive-operation marker prevents concurrent deploys from
    # sharing a parent ledger. A killed process leaves a stale marker: STOP and
    # ask the parent to inspect ownership; never reclaim it by PID/name alone.
    with runtime.operation_lock(args.lease, "deploy") as operation:
        deploy_locked(args, operation)


def refresh(args):
    """Image-only CAS patches; never invoke full deploy/bootstrap or read secrets."""
    from image_catalog import FRESHNESS_POLICY, TARGETS, select
    from image_sources import capture, require_unchanged
    from local_registry import Registry, refresh_inputs
    names = list(dict.fromkeys(args.targets.split(",")))
    select(names)
    before = json.loads(args.previous_release_manifest.read_text(encoding="utf-8"))
    after = json.loads(args.release_manifest.read_text(encoding="utf-8"))
    snapshot = capture()
    for receipt in (before, after):
        if (receipt.get("schema") != "aiforall.image-release/v1"
                or receipt.get("freshness_policy") != FRESHNESS_POLICY
                or receipt.get("source", {}).get("mode") != "working-tree"
                or receipt.get("refresh_inputs_id") != refresh_inputs(snapshot)):
            raise ValueError("refresh needs two publication receipts with unchanged recipe/config/migration inputs")
    if before.get("profile") != after.get("profile") or before.get("toolchain_id") != after.get("toolchain_id"):
        raise ValueError("refresh profile/transport toolchain changed; explicit delivery plan required")
    with runtime.operation_lock(args.lease, "refresh"):
        data = runtime.lease(args.lease)
        from local_full_declarations import identity as declarations_identity
        if data.get("localFullDeclarationsSha256") != declarations_identity():
            raise ValueError("refresh refuses changed or unattested manifests/configuration/phase declarations; explicit full deployment required")
        from local_full_lifecycle import objects as declared_objects
        workloads = {o["metadata"]["annotations"]["aiforall.dev/refresh-target"]:
                     (o["metadata"]["namespace"], o["metadata"]["name"])
                     for o in declared_objects(data) if o["kind"] == "Deployment"
                     and o["metadata"].get("annotations", {}).get("aiforall.dev/refresh-target")
                     and not o["metadata"].get("annotations", {}).get("aiforall.dev/migration-scope")}
        if any(name not in workloads for name in names):
            raise ValueError("image-only refresh refuses automatic-migration services; use an explicit migration plan")
        if any(not receipt.get("images", {}).get(name, {}).get("oci_ref") for receipt in (before, after) for name in names):
            raise ValueError("refresh publication receipts must cover every selected target")
        images = {TARGETS[name]["source"]: after["images"][name]["oci_ref"] for name in names}
        registry = Registry(args.registry_state)
        with registry.operation():
            registry.verify_mapping(images)
            changes = []
            for name in names:
                namespace, deployment = workloads[name]
                old, new = before["images"][name]["oci_ref"], after["images"][name]["oci_ref"]
                # Re-read the old immutable object too, not just the receipt.
                prefix = "aiforall-apps-registry:5000/apps/" + name + "@"
                if not old.startswith(prefix) or not re.fullmatch(r"sha256:[a-f0-9]{64}", old[len(prefix):]):
                    raise ValueError("previous receipt has the wrong application repository")
                _, old_config = registry.remote("apps/" + name, old[len(prefix):], include_config=True)
                _, new_config = registry.remote("apps/" + name, new[len(prefix):], include_config=True)
                for config, receipt in ((old_config, before), (new_config, after)):
                    if config["config"].get("Labels", {}).get("aiforall.dev.cargo-profile") != receipt["profile"]:
                        raise ValueError("refresh receipt profile disagrees with OCI configuration")
                for field in ("Env", "Cmd", "Entrypoint", "User", "WorkingDir", "ExposedPorts", "Volumes", "Healthcheck"):
                    if old_config["config"].get(field) != new_config["config"].get(field):
                        raise ValueError("refresh runtime configuration changed; explicit deployment plan required")
                live = json.loads(k("get", "deployment", deployment, "-n", namespace, "-o", "json"))
                metadata = live["metadata"]
                if (data.get("namespaceUids", {}).get(namespace) != k("get", "namespace", namespace, "-o", "jsonpath={.metadata.uid}")
                        or not any(entry.get("kind") == "Deployment" and entry.get("namespace") == namespace
                            and entry.get("name") == deployment and entry.get("uid") == metadata["uid"]
                            and entry.get("context") == CONTEXT and entry.get("cluster") == CLUSTER
                            for entry in data.get("ownedResources", []))):
                    raise ValueError("refresh deployment/namespace UID is not leased")
                containers = live["spec"]["template"]["spec"]["containers"]
                matches = [index for index, container in enumerate(containers) if container["image"] == old]
                if len(matches) != 1:
                    raise ValueError("live image differs from the previous publication receipt")
                if old != new:
                    index = matches[0]
                    path = "/spec/template/spec/containers/" + str(index)
                    patch = [{"op": "test", "path": "/metadata/uid", "value": metadata["uid"]},
                             {"op": "test", "path": "/metadata/resourceVersion", "value": metadata["resourceVersion"]},
                             {"op": "test", "path": path + "/image", "value": old},
                             {"op": "replace", "path": path + "/image", "value": new},
                             {"op": "add", "path": path + "/imagePullPolicy", "value": "IfNotPresent"}]
                    changes.append((namespace, deployment, patch, new))
            # All targets preflight before a pull or workload mutation.
            if changes:
                runtime.target().registry_connect(registry)
            for _, _, _, image in changes:
                runtime.target().registry_pull(registry, image)
            registry.verify_mapping(images)
            require_unchanged(snapshot)
            for namespace, deployment, patch, image in changes:
                require_unchanged(snapshot)
                k("patch", "deployment/" + deployment, "-n", namespace, "--type=json", "-p", json.dumps(patch))
                k("rollout", "status", "deployment/" + deployment, "-n", namespace, "--timeout=300s")
                post = json.loads(k("get", "deployment", deployment, "-n", namespace, "-o", "json"))
                if post["metadata"]["uid"] != patch[0]["value"] or image not in [
                        c["image"] for c in post["spec"]["template"]["spec"]["containers"]]:
                    raise ValueError("refresh deployment identity/image changed during rollout; no automatic rollback")
            require_unchanged(snapshot)
            print("image-only refresh complete: " + str(len(changes)) + " deployment(s); no provisioning or migrations")


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "_phase":
        from types import SimpleNamespace
        from local_full_phases import main as phase_main
        phase_main(SimpleNamespace(**globals()))
        return
    parser = renderer.helm.parser("deploy")
    args = parser.parse_args()
    if args.preserve_platform_tls and (args.action != "deploy" or not args.oauth_connect):
        parser.error("--preserve-platform-tls is only valid for deploy --oauth-connect")
    if args.action == "render":
        # No runtime inspection, startup or API contact. Source secrets remain
        # in render artifacts; report only resource kind/name metadata.
        images = json.loads(args.image_manifest.read_text()) if args.image_manifest else None
        if args.oauth_proof_input_file or args.github_input_file or args.simulated_input_file:
            raise ValueError("offline render does not accept private runtime inputs")
        objects = renderer.render("staticvalidation", images, oauth_proof=args.oauth_proof,
                                  github_openbao=args.github_openbao, oauth_connect=args.oauth_connect)
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
                          "http_endpoint": data.get("publicEndpoint", "http://127.0.0.1:18080"), "leased_node_ids": data["nodeContainerIds"]}))
        for namespace in data.get("namespaceUids", {}):
            uid = k("get", "namespace", namespace, "-o", "jsonpath={.metadata.uid}")
            if data.get("namespaceUids", {}).get(namespace) != uid:
                raise ValueError("inventory namespace is not owned")
            print(k("get", "deployment,pod,pvc,job,networkpolicy", "-n", namespace, "-o", "name"))
    elif args.action in {"up", "stop", "status"}:
        from local_full_environment import operate
        previous = os.umask(0o077)
        try:
            print(json.dumps(operate(args, runtime, github_inputs, secret)))
        except (ValueError, target_authority.AnchorError) as error:
            raise SystemExit(str(error)) from None
        except (OSError, KeyError, TypeError, subprocess.TimeoutExpired):
            raise SystemExit("local environment refused/not ready; private state retained, no reset") from None
        finally:
            os.umask(previous)
    elif args.action == "refresh":
        if not all((args.lease, args.registry_state, args.release_manifest, args.previous_release_manifest, args.targets)):
            raise ValueError("refresh requires --lease, --registry-state, --release-manifest, --previous-release-manifest and --targets")
        refresh(args)
    else:
        if not args.lease or not args.image_manifest or not args.cert_directory:
            raise ValueError("deploy requires --lease, --image-manifest and --cert-directory; no build or key generation is implicit")
        deploy(args)


if __name__ == "__main__":
    from private_diagnostics import operation
    stage = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] in {"render", "deploy", "inventory", "refresh", "up", "stop", "status"} else "arguments"
    with operation("local-full-" + stage):
        main()
