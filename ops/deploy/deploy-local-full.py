"""Parent-run local-full entrypoint; offline render is the only worker-safe action."""
import argparse
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import time
import uuid
from pathlib import Path
import local_full_target as target_authority
from local_full_target import private_file

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


def oauth_inputs(path):
    if not path or not path.is_file() or path.is_symlink() or not private_file(path):
        raise ValueError("private read-only OAuth proof input and private parent directory required")
    # Duplicate JSON keys must not silently select credentials/configuration.
    def unique(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError("ambiguous private OAuth proof input")
            value[key] = item
        return value
    if path.stat().st_size > 4096:
        raise ValueError("private OAuth proof input exceeds bound")
    return renderer.validate_proof_inputs(json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique))


def oauth_cert_inputs(path, run_id):
    mesh = path / "mesh"
    public = (mesh / "ca.crt").read_bytes()
    leaf = {"tls.crt": (mesh / "gitlab-connect.crt").read_bytes(),
            "tls.key": (mesh / "gitlab-connect.key").read_bytes()}
    # New per-run objects, never copies of the parent CA or gateway private key.
    objects = [secret("oauth-proof-public-ca-" + run_id, {"ca.crt": public}),
               secret("oauth-proof-connector-tls-" + run_id, leaf)]
    digest = hashlib.sha256(public + leaf["tls.crt"] + leaf["tls.key"]).hexdigest()
    return objects, digest


def preflight_oauth_objects(objects, data):
    for obj in objects:
        name = obj["metadata"]["name"]
        if not (name.startswith("oauth-proof-") or name in {"gitlab-connect", "oauth-idp-simulator"}):
            continue
        if obj["kind"] == "Secret":
            continue  # CREATE only later: collision fails without GET/overwrite.
        uid = k("get", obj["kind"], name, "-n", "aiforall-local-full", "--ignore-not-found", "-o", "jsonpath={.metadata.uid}")
        if uid and not any(entry.get("kind") == obj["kind"] and entry.get("name") == name
                           and entry.get("namespace") == "aiforall-local-full" and entry.get("uid") == uid
                           and entry.get("context") == CONTEXT and entry.get("cluster") == CLUSTER
                           for entry in data.get("ownedResources", [])):
            raise ValueError("existing OAuth proof resource UID not explicitly owned")


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


def _ingress_json(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def _ingress_require(condition):
    if not condition:
        raise ValueError("INCONCLUSIVE: Envoy ingress gate refused; details redacted")


def _ingress_admin_pair(before, after, identity):
    expected = {key: identity[key] for key in ("namespace_uid", "pod", "pod_uid", "owner_chain",
                "container", "container_id", "restart_count", "image_id")}
    for item in (before, after):
        _ingress_require(item["identity"] == expected and item["state"] == "LIVE"
                         and item["version"] == "1.31.2"
                         and type(item["restart_epoch"]) is int and item["restart_epoch"] == 0)
    for key in ("uptime_current_epoch_ns", "uptime_all_epochs_ns"):
        _ingress_require(type(before[key]) is int and type(after[key]) is int
                         and 0 <= before[key] <= after[key])
    # Only explicit counters count. Other collector fields can legitimately be
    # MISSING; never turn those into zero or claim per-flow packet attribution.
    for suffix in ("upstream_cx_total", "upstream_rq_total", "ssl.handshake",
                   "ssl.fail_verify_san", "ssl.fail_verify_error"):
        key = "cluster.iam." + suffix
        a, b = before["stats"][key], after["stats"][key]
        _ingress_require(type(a) is int and type(b) is int and 0 <= a <= b)


def _ingress_evidence(directory):
    """Read only the seven closed, private, nonsecret parent projections."""
    directory = Path(directory)
    _ingress_require(directory.is_dir() and not directory.is_symlink()
                     and directory.absolute() == directory.resolve())
    files, hashes = {}, {}
    for name in ("observation", "summary", "baseline", "passive-baseline",
                 "admin-before", "admin-after", "causal-review"):
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
    _ingress_require(review["schema"] == "aiforall.envoy.ingress-causal-review.v1"
        and review["decision"] == "QUALIFIED_EQUIVALENT_LAB_ONLY"
        and review["observation_sha256"] == hashes["observation"]
        and review["namespace"] == "aiforall-local-full"
        and review["measured_source_ipv4"] == source
        and review["source_annotation"] == "projectcalico.org/IPv4IPIPTunnelAddr"
        and review["source_provenance"] == "UNIQUE_MEASURED_BINDING_AND_EXACT_K8S_NODE_IPIP_ANNOTATION"
        and isinstance(review["reason"], str) and len(review["reason"].strip()) >= 80
        and isinstance(review["limitations"], list) and len(review["limitations"]) >= 5)
    required_basis = {"ONE_EXACT_FRESH_TCP_REQUEST_UNDER_PARENT_LEASE",
        "UNIQUE_NEW_CLOSED_FILTERED_NAT_BINDING", "EXACT_OWNED_CP_NODE_UID_AND_UNIQUE_IPIP_SOURCE",
        "POD_LINKED_CALICO_TO_WORKLOAD_POLICY_DENY_TEMPLATE", "STABLE_IDENTITIES_RULES_AND_NAMESPACE_POLICIES",
        "PASSIVE_COUNTERS_STABLE_WITH_RESIDENT_NODE_PROBES", "NO_OTHER_KNOWN_MISSION_CLIENT_OR_TEST_FOR_THIS_ENTRY",
        "IAM_UPSTREAM_TLS_DELTAS_ZERO_WITH_STABLE_LIVE_ENVOY"}
    _ingress_require(set(review["causal_basis"]) == required_basis)
    for key, identity_key in (("control_plane_container_id", "control_plane_container_id"),
            ("control_plane_node_uid", "control_plane_node_uid"), ("envoy_pod_uid", "pod_uid"),
            ("service_uid", "service_uid")):
        _ingress_require(review[key] == identity[identity_key])
    _ingress_require(binding["original"]["dst"] == identity["control_plane_ipv4"]
        and binding["original"]["dport"] == 30080 and binding["reply"]["src"] == identity["pod_ipv4"]
        and binding["reply"]["sport"] == 10000 and binding["id"] == review["nat_id"]
        and binding["zone_evidence"] == "SOURCE_FILTERED" and binding["requested_zone"] == 0
        and obs["status"] == "PARENT_CORRELATION_REQUIRED"
        and obs["counter_scope"] == "POD_CHAIN_SHARED"
        and obs["packet_attribution"] == "NOT_ESTABLISHED_BY_COUNTER")
    for item in (files["baseline"], files["passive-baseline"]):
        _ingress_require(item["status"] == "BASELINE_ONLY" and item["identity"] == identity
            and item["rules_fingerprint"] == obs["rules_fingerprint"])
    _ingress_require(files["baseline"]["deny"] == files["passive-baseline"]["deny"]
        and review["rules_fingerprint"] == obs["rules_fingerprint"]
        and summary["exit"] == 0 and summary["stage"] == "complete"
        and summary["cluster_mutation"] is False and summary["policy_fingerprint_stable"] is True
        and summary["verdict"] == "PENDING_LEAD_CAUSAL_REVIEW"
        and bool(re.fullmatch(r"[a-f0-9]{40}", summary["ops_head"]))
        and summary["known_probe_transport"] == "TCP_RESIDENT_NODE_TO_POD_10000"
        and summary["counter_scope"] == obs["counter_scope"]
        and summary["packet_attribution"] == obs["packet_attribution"]
        and summary["envoy_pod_uid"] == identity["pod_uid"]
        and summary["source_equals_anchored_cp"] is False
        and summary["restart_epoch"] == files["admin-after"]["restart_epoch"]
        and summary["http_attempts"] == review["http_attempts"] == 1
        and summary["passive_deny_delta"] == review["passive_deny_delta"] == {"packets": 0, "bytes": 0}
        and summary["nat_binding"] == binding
        and summary["endpoint_deny_delta"] == review["endpoint_deny_delta"] == obs["endpoint_deny_delta"]
        and type(obs["endpoint_deny_delta"]["packets"]) is int and obs["endpoint_deny_delta"]["packets"] > 0
        and type(obs["endpoint_deny_delta"]["bytes"]) is int and obs["endpoint_deny_delta"]["bytes"] > 0)
    # Check the actual imported authority, not a sibling or a claimed delivery hash.
    _ingress_require(runtime.LocalClusterTargetAnchor is target_authority.LocalClusterTargetAnchor
        and summary["helper_sha256"] == hashlib.sha256(Path(target_authority.__file__).read_bytes()).hexdigest())
    _ingress_admin_pair(files["admin-before"], files["admin-after"], identity)
    _ingress_require(files["admin-before"]["stats"] == files["admin-after"]["stats"])
    _ingress_require(summary["iam_upstream_deltas"] == {"cluster.iam." + suffix: 0 for suffix in (
        "upstream_cx_total", "upstream_rq_total", "ssl.handshake", "ssl.fail_verify_san", "ssl.fail_verify_error")})
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
        _ingress_require(item["ports"] == [{"name": "http", "port": 10000, "protocol": "TCP"}])
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


class EnvoyIngressChange:
    """Parent-only pending change. No HTTP callback, URL or deployment actor.

    The parent must call finish with the fixed-route witness or rollback on any
    failed/abandoned validation. No-op owns nothing. Receipts are copies, never
    aliases of the state used for conditional rollback. No packet proof implied.
    """

    def __init__(self, guard, directory, files, hashes, source, before, desired, journal_sha256, added, post_version):
        self._guard, self._directory = guard, directory
        self._frozen = _ingress_json({"files": files, "hashes": hashes, "source": source,
            "before": before, "desired": desired, "journal_sha256": journal_sha256, "post_version": post_version})
        self._added, self._closed, self._validated = added, False, False
        self._result = "PENDING_PARENT_VALIDATION" if added else "NOOP_NOT_OWNED"

    @property
    def receipt(self):
        data = json.loads(self._frozen)
        return {"state": self._result,
                "journal_sha256": data["journal_sha256"], "source": data["source"], "added": self._added,
                "policy_uid": data["before"]["allow-envoy"]["uid"], "policy_resource_version": data["post_version"]}

    def rollback(self):
        if not self._added or self._closed:
            return "NOT_OWNED" if not self._added else "ALREADY_CLOSED"
        try:
            fresh = runtime.target()
            _ingress_require(fresh.data == self._guard.data)
            fresh.check()
            data = json.loads(self._frozen)
            item = json.loads(fresh.kubectl(["get", "networkpolicy", "allow-envoy", "-n", CLUSTER,
                "-o", "json"], text=True, timeout=30))
            old, desired = data["before"]["allow-envoy"], data["desired"]
            _ingress_require(item["metadata"]["uid"] == old["uid"] and item["spec"] == desired
                and item["metadata"]["resourceVersion"] == data["post_version"]
                and item["metadata"]["namespace"] == CLUSTER and not item["metadata"].get("deletionTimestamp"))
            patch = [{"op": "test", "path": "/metadata/uid", "value": old["uid"]},
                {"op": "test", "path": "/metadata/resourceVersion", "value": item["metadata"]["resourceVersion"]},
                {"op": "test", "path": "/spec", "value": desired},
                {"op": "test", "path": "/spec/ingress/1", "value": desired["ingress"][1]},
                {"op": "remove", "path": "/spec/ingress/1"}]
            fresh.kubectl(["patch", "networkpolicy/allow-envoy", "-n", CLUSTER, "--type=json", "-p",
                _ingress_json(patch)], text=True, timeout=30)
            after = json.loads(fresh.kubectl(["get", "networkpolicy", "allow-envoy", "-n", CLUSTER,
                "-o", "json"], text=True, timeout=30))
            _ingress_require(after["metadata"]["uid"] == old["uid"] and after["spec"] == old["spec"])
            self._closed = True
            self._result = "ROLLED_BACK_OWN_ENTRY"
            return self._result
        except Exception:
            raise ValueError("STOP: conditional Envoy rollback blocked; parent intervention required") from None

    def _revalidate(self):
        data = json.loads(self._frozen)
        files, hashes, source = _ingress_evidence(self._directory)
        _ingress_require(hashes == data["hashes"] and files == data["files"] and source == data["source"])
        live = _ingress_live(self._guard, files["observation"]["identity"], files["causal-review"], source)
        expected = json.loads(_ingress_json(data["before"]))
        expected["allow-envoy"]["spec"] = data["desired"]
        # Freeze the first observed postimage RV, not just its spec. A later
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
            _ingress_require(witness["schema"] == "aiforall.envoy.ingress-parent-witness.v1"
                and witness["journal_sha256"] == data["journal_sha256"]
                and witness["entry"] == "127.0.0.1:18080" and witness["method"] == "GET"
                and witness["path"] == "/iam/api/auth/gitlab/login"
                and witness["outcome"] == "IAM_CONNECTOR_NONCE_REDIRECT"
                and witness["connector"] == "gitlab" and witness["status"] == 303
                and witness["protected_without_bearer"] == "DENIED")
            _ingress_admin_pair(data["files"]["admin-after"], witness["admin_before"], data["files"]["observation"]["identity"])
            _ingress_admin_pair(witness["admin_before"], witness["admin_after"], data["files"]["observation"]["identity"])
            _ingress_require(witness["admin_after"]["stats"]["cluster.iam.upstream_rq_total"]
                             > witness["admin_before"]["stats"]["cluster.iam.upstream_rq_total"])
            for key in ("cluster.iam.ssl.fail_verify_san", "cluster.iam.ssl.fail_verify_error"):
                _ingress_require(witness["admin_before"]["stats"][key] == witness["admin_after"]["stats"][key])
            self._validated = True
            self._result = "PARENT_WITNESS_ACCEPTED_LAB_ONLY"
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
        declared = {item["metadata"]["name"]: item["spec"] for item in target_authority.yaml.safe_load_all(
            (Path(__file__).parent / "apps/overlays/local-full/platform/policies.yaml").read_text()) if item}
        for name, policy in before.items():
            selector = policy["spec"]["podSelector"]
            # Unknown expressions are not guessed non-applicable. The declared
            # policy itself must match before its selector can be trusted.
            _ingress_require(name in declared)
            if name == "allow-envoy":
                base = json.loads(_ingress_json(policy["spec"]))
                base["ingress"] = [client]
                _ingress_require(base == declared[name])
            elif (name == "allow-test-client" or selector == {}
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
            "source": source, "before": before, "expected_spec": desired, "patch": patch,
            "state": "PENDING_PARENT_VALIDATION" if added else "NOOP_NOT_OWNED"}
        path = Path(journal_path)
        _ingress_require(path.parent.is_dir() and not path.parent.is_symlink()
            and path.absolute() == path.resolve() and not path.exists())
        raw = _ingress_json(record).encode()
        with path.open("xb") as file:
            file.write(raw)
            file.flush()
            os.fsync(file.fileno())
        target_authority.secure(path)
        _ingress_require(private_file(path) and path.read_bytes() == raw)
        # Re-read evidence and all semantics immediately before mutation. There
        # is no multi-object atomicity claim; only this JSONPatch is atomic.
        again, current_hashes, current_source = _ingress_evidence(evidence_directory)
        _ingress_require(again == files and current_hashes == hashes and current_source == source
            and _ingress_live(guard, identity, review, source) == before)
        if added:
            # A transport error can follow an applied patch. Without an ACK we
            # cannot distinguish ours from a concurrent equal entry: STOP and
            # retain the journal, never adopt/remove an uncertain rule.
            guard.kubectl(["patch", "networkpolicy/allow-envoy", "-n", CLUSTER, "--type=json", "-p",
                _ingress_json(patch)], text=True, timeout=30)
        post = json.loads(guard.kubectl(["get", "networkpolicy", "allow-envoy", "-n", CLUSTER,
                                        "-o", "json"], text=True, timeout=30))
        _ingress_require(post["metadata"]["uid"] == old["uid"] and post["spec"] == desired
            and not post["metadata"].get("deletionTimestamp")
            and (added or post["metadata"]["resourceVersion"] == old["rv"]))
        change = EnvoyIngressChange(guard, evidence_directory, files, hashes, source, before, desired,
                                   hashlib.sha256(raw).hexdigest(), added, post["metadata"]["resourceVersion"])
        change._revalidate()
        return change
    except Exception:
        if change is not None and change._added:
            change.rollback()
        raise ValueError("STOP: Envoy ingress refused; details redacted; inspect parent journal") from None


def deploy_locked(args):
    images = json.loads(args.image_manifest.read_text())
    mesh, identity, cert_hash = cert_inputs(args.cert_directory)
    inputs = oauth_inputs(args.oauth_proof_input_file) if args.oauth_proof else None
    if args.oauth_proof_input_file and not args.oauth_proof:
        raise ValueError("private proof input requires --oauth-proof")
    run_id = inputs["run_id"] if inputs else uuid.uuid4().hex[:16]
    proof_certs, proof_cert_hash = oauth_cert_inputs(args.cert_directory, run_id) if inputs else ([], "")
    objects = renderer.render(run_id, images, cert_hash + proof_cert_hash, args.oauth_proof, inputs)
    objects += proof_certs
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
    if args.oauth_proof:
        preflight_oauth_objects(objects, data)
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
    proof_secrets = [obj for obj in groups["foundation"] if obj["kind"] == "Secret" and obj["metadata"]["name"].startswith("oauth-proof-")]
    groups["foundation"] = [obj for obj in groups["foundation"] if obj not in proof_secrets]
    try:
        if original_replicas and not args.confirm_quiesce:
            raise ValueError("existing owned deployment: --confirm-quiesce required before serial migration/rollout")
        for obj in groups["applications"]:
            namespace, name = obj["metadata"]["namespace"], obj["metadata"]["name"]
            if (namespace, name) in original_replicas:
                k("scale", "deployment/" + name, "-n", namespace, "--replicas=0")
                k("wait", "--for=delete", "pod", "-n", namespace,
                  "-l", "app.kubernetes.io/name=" + name, "--timeout=180s")
        for obj in proof_secrets:
            runtime.create(obj, args, data)  # No overwrite/adoption of existing secrets, even a same-name one.
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
    parser.add_argument("--oauth-proof", action="store_true")
    parser.add_argument("--oauth-proof-input-file", type=Path)
    args = parser.parse_args()
    if args.action == "render":
        # No runtime inspection, startup or API contact. Source secrets remain
        # in render artifacts; report only resource kind/name metadata.
        images = json.loads(args.image_manifest.read_text()) if args.image_manifest else None
        if args.oauth_proof_input_file:
            raise ValueError("offline render does not accept private runtime inputs")
        objects = renderer.render("staticvalidation", images, oauth_proof=args.oauth_proof)
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
