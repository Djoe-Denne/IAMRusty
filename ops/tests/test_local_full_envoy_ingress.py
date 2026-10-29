"""Offline fakes only; four contract oracles named in QA before implementation."""
import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
import uuid


DEPLOY = Path(__file__).resolve().parents[1] / "deploy"
sys.path.insert(0, str(DEPLOY))
spec = importlib.util.spec_from_file_location("envoy_ingress_deploy", DEPLOY / "deploy-local-full.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def clone(value):
    return json.loads(json.dumps(value))


def uid():
    return str(uuid.uuid4())


class FakeGuard:
    def __init__(self, directory):
        self.directory = directory
        self.calls, self.patches = [], []
        self.on_patch = None
        self.on_nodes = None
        self.node_reads = 0
        self.source = "192.0.2.61"
        self.cp_uid = uid()
        self.worker_uid = uid()
        self.third_uid = uid()
        self.identity = {"namespace_uid": uid(), "pod": "envoy-mesh-synthetic-pod", "pod_uid": uid(),
            "owner_chain": [{"kind": "ReplicaSet", "name": "envoy-mesh-synthetic", "uid": uid()},
                            {"kind": "Deployment", "name": "envoy-mesh", "uid": uid()}],
            "container": "envoy", "container_id": "containerd://" + "a" * 64, "restart_count": 0,
            "image_id": "synthetic@sha256:" + "b" * 64, "started_at": "2026-01-01T00:00:00Z",
            "pod_ipv4": "192.0.2.72", "node_uid": self.worker_uid, "node_name": "synthetic-worker",
            "node_internal_ipv4": "198.51.100.21", "worker_container_id": "c" * 64,
            "control_plane_container_id": "d" * 64, "service_uid": uid(), "service_cluster_ipv4": "192.0.2.80",
            "host_port": 18080, "node_port": 30080, "service_port": 10000, "pod_port": 10000,
            "traffic_policy": "Cluster", "control_plane_node_uid": self.cp_uid, "control_plane_ipv4": "198.51.100.20"}
        self.data = {"cluster": runner.CLUSTER, "context": runner.CONTEXT,
            "control_plane_container_id": self.identity["control_plane_container_id"],
            "nodeContainerIds": ["c" * 64, "d" * 64, "e" * 64], "ownedResources": []}
        declared = {item["metadata"]["name"]: item["spec"] for item in runner.target_authority.yaml.safe_load_all(
            (DEPLOY / "apps/overlays/local-full/platform/policies.yaml").read_text()) if item}
        self.policies = []
        for name in ("allow-envoy", "default-deny", "allow-dns", "allow-test-client"):
            item = {"apiVersion": "networking.k8s.io/v1", "kind": "NetworkPolicy",
                    "metadata": {"name": name, "namespace": runner.CLUSTER, "uid": uid(), "resourceVersion": "10"},
                    "spec": clone(declared[name])}
            self.policies.append(item)
            self.data["ownedResources"].append({"kind": "NetworkPolicy", "name": name, "namespace": runner.CLUSTER,
                "uid": item["metadata"]["uid"], "context": runner.CONTEXT, "cluster": runner.CLUSTER})
        self.nodes = [[self.cp_uid, "synthetic-cp", "", self.source, "198.51.100.20/24", "198.51.100.20"],
            [self.worker_uid, "synthetic-worker", "", "192.0.2.62", "198.51.100.21/24", "198.51.100.21"],
            [self.third_uid, "synthetic-third", "", "192.0.2.63", "198.51.100.22/24", "198.51.100.22"]]
        self.slices = {"items": [{"metadata": {"ownerReferences": [{"kind": "Service", "name": "envoy-mesh",
            "uid": self.identity["service_uid"], "controller": True}]},
            "ports": [{"name": "http", "port": 10000, "protocol": "TCP"}], "endpoints": [{
                "conditions": {"ready": True}, "addresses": [self.identity["pod_ipv4"]],
                "nodeName": self.identity["node_name"], "targetRef": {"uid": self.identity["pod_uid"]}}]}]}
        self.write_evidence()

    @property
    def policy(self):
        return next(item for item in self.policies if item["metadata"]["name"] == "allow-envoy")

    @property
    def entry(self):
        return {"from": [{"ipBlock": {"cidr": self.source + "/32"}}], "ports": [{"protocol": "TCP", "port": 10000}]}

    def admin(self, count=0):
        return {"identity": {key: self.identity[key] for key in ("namespace_uid", "pod", "pod_uid", "owner_chain",
                    "container", "container_id", "restart_count", "image_id")},
            "state": "LIVE", "version": "1.31.2", "restart_epoch": 0,
            "uptime_current_epoch_ns": count + 1, "uptime_all_epochs_ns": count + 1,
            "stats": {"cluster.iam." + suffix: count for suffix in ("upstream_cx_total", "upstream_rq_total",
                "ssl.handshake", "ssl.fail_verify_san", "ssl.fail_verify_error")}}

    def write_evidence(self):
        binding = {"original": {"src": "198.51.100.10", "dst": self.identity["control_plane_ipv4"],
            "sport": 54321, "dport": 30080}, "reply": {"src": self.identity["pod_ipv4"], "dst": self.source,
            "sport": 10000, "dport": 45678}, "id": 1234567, "requested_zone": 0,
            "observed_zone": "MISSING", "zone_evidence": "SOURCE_FILTERED", "state": "SYN_SENT", "unreplied": True}
        observation = {"identity": self.identity, "nat_binding": binding, "rules_fingerprint": "f" * 64,
            "status": "PARENT_CORRELATION_REQUIRED",
            "endpoint_deny_delta": {"packets": 3, "bytes": 180}, "counter_scope": "POD_CHAIN_SHARED",
            "packet_attribution": "NOT_ESTABLISHED_BY_COUNTER"}
        self.write("observation", observation)
        baseline = {"status": "BASELINE_ONLY", "identity": self.identity, "rules_fingerprint": "f" * 64,
            "deny": {"packets": 7, "bytes": 420}}
        self.write("baseline", baseline)
        self.write("passive-baseline", baseline)
        self.write("admin-before", self.admin())
        self.write("admin-after", self.admin())
        summary = {"exit": 0, "stage": "complete", "cluster_mutation": False, "policy_fingerprint_stable": True,
            "verdict": "PENDING_LEAD_CAUSAL_REVIEW", "ops_head": "1" * 40,
            "known_probe_transport": "TCP_RESIDENT_NODE_TO_POD_10000", "counter_scope": "POD_CHAIN_SHARED",
            "packet_attribution": "NOT_ESTABLISHED_BY_COUNTER", "envoy_pod_uid": self.identity["pod_uid"],
            "source_equals_anchored_cp": False, "restart_epoch": 0,
            "iam_upstream_deltas": {"cluster.iam." + suffix: 0 for suffix in ("upstream_cx_total", "upstream_rq_total",
                "ssl.handshake", "ssl.fail_verify_san", "ssl.fail_verify_error")},
            "helper_sha256": hashlib.sha256(Path(runner.target_authority.__file__).read_bytes()).hexdigest(),
            "http_attempts": 1, "passive_deny_delta": {"packets": 0, "bytes": 0}, "nat_binding": binding,
            "endpoint_deny_delta": observation["endpoint_deny_delta"]}
        self.write("summary", summary)
        review = {"schema": "aiforall.envoy.ingress-causal-review.v1", "decision": "QUALIFIED_EQUIVALENT_LAB_ONLY",
            "observation_sha256": hashlib.sha256((self.directory / "observation.json").read_bytes()).hexdigest(),
            "namespace": runner.CLUSTER, "control_plane_container_id": self.identity["control_plane_container_id"],
            "control_plane_node_uid": self.cp_uid, "envoy_pod_uid": self.identity["pod_uid"],
            "service_uid": self.identity["service_uid"], "policy_uid": self.policy["metadata"]["uid"],
            "default_deny_uid": self.policies[1]["metadata"]["uid"], "nat_id": binding["id"],
            "measured_source_ipv4": self.source, "source_annotation": "projectcalico.org/IPv4IPIPTunnelAddr",
            "source_provenance": "UNIQUE_MEASURED_BINDING_AND_EXACT_K8S_NODE_IPIP_ANNOTATION",
            "rules_fingerprint": "f" * 64, "http_attempts": 1, "passive_deny_delta": {"packets": 0, "bytes": 0},
            "endpoint_deny_delta": observation["endpoint_deny_delta"],
            "causal_basis": ["ONE_EXACT_FRESH_TCP_REQUEST_UNDER_PARENT_LEASE", "UNIQUE_NEW_CLOSED_FILTERED_NAT_BINDING",
                "EXACT_OWNED_CP_NODE_UID_AND_UNIQUE_IPIP_SOURCE", "POD_LINKED_CALICO_TO_WORKLOAD_POLICY_DENY_TEMPLATE",
                "STABLE_IDENTITIES_RULES_AND_NAMESPACE_POLICIES", "PASSIVE_COUNTERS_STABLE_WITH_RESIDENT_NODE_PROBES",
                "NO_OTHER_KNOWN_MISSION_CLIENT_OR_TEST_FOR_THIS_ENTRY", "IAM_UPSTREAM_TLS_DELTAS_ZERO_WITH_STABLE_LIVE_ENVOY"],
            "reason": "Synthetic reason: the exclusive request, passive reference and exact source binding exclude the known laboratory alternatives only.",
            "limitations": ["shared counter", "not packet trace", "not production", "no hostile daemon guarantee", "not CRD proof"]}
        self.write("causal-review", review)

    def write(self, name, value):
        (self.directory / (name + ".json")).write_text(json.dumps(value), encoding="utf-8")

    def edit(self, name, edit):
        value = json.loads((self.directory / (name + ".json")).read_text())
        edit(value)
        self.write(name, value)

    def check(self):
        return clone(self.data)

    def _envoy_tuple_attestation(self, **kwargs):
        return clone(kwargs["owner_chain"]), clone(kwargs["incarnation"])

    def _envoy_tuple_identity(self, *args):
        return {key: clone(value) for key, value in self.identity.items()
                if key not in {"control_plane_node_uid", "control_plane_ipv4"}}

    def kubectl(self, args, **kwargs):
        self.calls.append(clone(args))
        if args[:2] == ["get", "nodes"]:
            if "-l" in args:
                return self.cp_uid + "\n"
            self.node_reads += 1
            if self.on_nodes:
                self.on_nodes(self)
            return "\n".join("\t".join(row) for row in self.nodes)
        if args[:2] == ["get", "endpointslices"]:
            return json.dumps(self.slices)
        if args[:2] == ["get", "networkpolicies"]:
            return json.dumps({"items": self.policies})
        if args[:3] == ["get", "networkpolicy", "allow-envoy"]:
            return json.dumps(self.policy)
        if args[:2] == ["patch", "networkpolicy/allow-envoy"]:
            operations = json.loads(args[args.index("-p") + 1])
            self.patches.append(operations)
            if self.on_patch:
                self.on_patch(self, operations)
            # Emulate atomic JSONPatch tests: work on a copy, publish at end.
            candidate = clone(self.policy)
            for operation in operations:
                parts = operation["path"].strip("/").split("/")
                parent = candidate
                for part in parts[:-1]:
                    parent = parent[int(part)] if isinstance(parent, list) else parent[part]
                part = int(parts[-1]) if isinstance(parent, list) and parts[-1] != "-" else parts[-1]
                if operation["op"] == "test":
                    if parent[part] != operation["value"]:
                        raise ValueError("synthetic CAS conflict")
                elif operation["op"] == "add":
                    assert part == "-"
                    parent.append(clone(operation["value"]))
                elif operation["op"] == "remove":
                    del parent[part]
                else:
                    raise AssertionError("unexpected mutation")
            candidate["metadata"]["resourceVersion"] = str(int(candidate["metadata"]["resourceVersion"]) + 1)
            policy = self.policy
            policy.clear()
            policy.update(candidate)
            return json.dumps(candidate) if args[-2:] == ["-o", "json"] else "patched"
        raise AssertionError("unapproved fake command")


class EnvoyIngressTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="envoy-ingress-offline-")
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.guard = FakeGuard(self.directory)
        for mock in (patch.object(runner.runtime, "target", return_value=self.guard),
                     patch.object(runner, "private_file", return_value=True),
                     patch.object(runner.target_authority, "secure")):
            mock.start()
            self.addCleanup(mock.stop)

    def configure(self):
        return runner.configure_envoy_ingress(evidence_directory=self.directory,
            journal_path=self.directory / ("journal-" + uuid.uuid4().hex + ".json"))

    def historical_pair(self, guard):
        guard.edit("summary", lambda value: value.__setitem__("helper_sha256", "9" * 64))
        names = ("observation", "summary", "baseline", "passive-baseline",
                 "admin-before", "admin-after", "causal-review")
        raw = {name: (self.directory / (name + ".json")).read_bytes() for name in names}
        hashes = {name: hashlib.sha256(value).hexdigest() for name, value in raw.items()}
        key = ("1" * 40, "9" * 64, hashes["observation"])
        entry = (key[0], key[1], tuple((name, hashes[name]) for name in names),
                 hashlib.sha256(Path(runner.target_authority.__file__).read_bytes()).hexdigest(), "c" * 64)
        return key, entry, raw

    def test_envoy_ingress_requires_measured_tunnel_of_exact_owned_cp(self):
        original = clone(self.guard.policies)
        change = self.configure()
        self.assertNotEqual(self.guard.source, self.guard.identity["control_plane_ipv4"])
        self.assertEqual(self.guard.policy["spec"]["ingress"][-1], self.guard.entry)
        self.assertEqual(change.receipt["state"], "PENDING_PARENT_VALIDATION")
        self.assertEqual(len(self.guard.patches), 1)
        self.assertEqual(self.guard.policies[1:], original[1:])
        # Heartbeat RV is deliberately not requested or compared for Nodes.
        self.assertTrue(all("resourceVersion" not in " ".join(call) for call in self.guard.calls if call[:2] == ["get", "nodes"]))
        module = ast.parse((DEPLOY / "deploy-local-full.py").read_text())
        for name in ("main", "deploy", "deploy_locked", "configure_api_policy"):
            node = next(n for n in module.body if isinstance(n, ast.FunctionDef) and n.name == name)
            self.assertFalse(any(isinstance(n, ast.Name) and n.id == "configure_envoy_ingress" for n in ast.walk(node)))
        specialized = [n for n in module.body if isinstance(n, (ast.ClassDef, ast.FunctionDef))
                       and (n.name.startswith("_ingress_") or n.name in {"EnvoyIngressChange", "configure_envoy_ingress"})]
        text = "\n".join(ast.unparse(n) for n in specialized)
        self.assertNotIn("subprocess.run", text)
        self.assertNotIn("docker", text.lower())
        self.assertNotIn("urlopen", text)
        self.assertNotIn("exec", [v for n in specialized for v in ast.walk(n) if isinstance(v, ast.Constant) for v in [v.value] if isinstance(v, str)])
        after = self.guard.admin(1)
        for key in ("cluster.iam.ssl.fail_verify_san", "cluster.iam.ssl.fail_verify_error"):
            after["stats"][key] = 0
        witness = {"schema": "aiforall.envoy.ingress-parent-witness.v1",
            "journal_sha256": change.receipt["journal_sha256"], "entry": "127.0.0.1:18080", "method": "GET",
            "path": "/iam/api/auth/gitlab/login", "connector": "gitlab", "status": 303,
            "outcome": "IAM_CONNECTOR_NONCE_REDIRECT", "protected_without_bearer": "DENIED",
            "admin_before": self.guard.admin(), "admin_after": after}
        self.assertEqual(change.finish(witness), "PARENT_WITNESS_ACCEPTED_LAB_ONLY")
        # Validation does not revoke our ability to remove our own entry if the
        # parent subsequently observes source drift during this active lease.
        self.guard.nodes[0][3] = "192.0.2.99"
        self.assertEqual(change.rollback(), "ROLLED_BACK_OWN_ENTRY")

    def test_envoy_ingress_accepts_native_admin_owner_chain_tuple_and_json_roundtrip(self):
        before = self.guard.admin()
        before["identity"]["owner_chain"] = tuple(clone(before["identity"]["owner_chain"]))
        after = clone(before)
        after["identity"]["owner_chain"] = tuple(after["identity"]["owner_chain"])
        after["uptime_current_epoch_ns"] += 1
        after["uptime_all_epochs_ns"] += 1
        after["stats"]["cluster.iam.upstream_rq_total"] += 1
        runner._ingress_admin_pair(before, after, self.guard.identity)
        runner._ingress_admin_pair(clone(before), after, self.guard.identity)
        runner._ingress_admin_pair(before, clone(after), self.guard.identity)
        self.assertIsInstance(before["identity"]["owner_chain"], tuple)
        for key, value in (("pod_uid", uid()), ("container_id", "containerd://" + "f" * 64),
                           ("restart_count", 1), ("restart_count", False), ("image_id", "other-image")):
            with self.subTest(identity_field=key, value=value):
                wrong = clone(after)
                wrong["identity"][key] = value
                with self.assertRaises(ValueError):
                    runner._ingress_admin_pair(before, wrong, self.guard.identity)
        wrong = clone(after)
        wrong["identity"]["owner_chain"][1]["uid"] = uid()
        with self.assertRaises(ValueError):
            runner._ingress_admin_pair(before, wrong, self.guard.identity)
        change = self.configure()
        witness = {"schema": "aiforall.envoy.ingress-parent-witness.v1",
            "journal_sha256": change.receipt["journal_sha256"], "entry": "127.0.0.1:18080", "method": "GET",
            "path": "/iam/api/auth/gitlab/login", "connector": "gitlab", "status": 303,
            "outcome": "IAM_CONNECTOR_NONCE_REDIRECT", "protected_without_bearer": "DENIED",
            "admin_before": before, "admin_after": after}
        self.assertEqual(change.finish(witness), "PARENT_WITNESS_ACCEPTED_LAB_ONLY")
        self.assertEqual(change.rollback(), "ROLLED_BACK_OWN_ENTRY")

    def test_envoy_ingress_does_not_adopt_equal_spec_update_between_ack_and_first_read(self):
        original_transport = self.guard.kubectl
        concurrent_read = []

        def transport(args, **kwargs):
            if (args[:3] == ["get", "networkpolicy", "allow-envoy"]
                    and self.guard.patches and not concurrent_read):
                # Another writer removes/reintroduces an equal entry after the
                # patch ACK but before its first read. The spec alone cannot
                # establish ownership of this new revision.
                self.guard.policy["metadata"]["resourceVersion"] = "13"
                concurrent_read.append(True)
            return original_transport(args, **kwargs)

        with patch.object(self.guard, "kubectl", side_effect=transport):
            with self.assertRaises(ValueError):
                self.configure()
        self.assertEqual(concurrent_read, [True])
        self.assertEqual(len(self.guard.patches), 1)
        self.assertEqual(self.guard.policy["metadata"]["resourceVersion"], "13")
        self.assertEqual(self.guard.policy["spec"]["ingress"][-1], self.guard.entry)
        self.assertFalse(any(op["op"] == "remove" for ops in self.guard.patches for op in ops))

    def test_envoy_ingress_patch_ack_is_frozen_and_lost_ack_never_grants_rollback(self):
        change = self.configure()
        receipt = change.receipt
        self.assertEqual(receipt["ack_source"], "PATCH_RESPONSE")
        self.assertEqual(receipt["policy_resource_version"], "11")
        receipt["authority"].clear()
        self.assertTrue(change.receipt["authority"])
        records = list(self.directory.glob("journal-*.json.ack.json"))
        self.assertEqual(len(records), 1)
        self.assertEqual(json.loads(records[0].read_text())["ack"]["rv"], "11")
        self.assertEqual(change.rollback(), "ROLLED_BACK_OWN_ENTRY")
        modes = ("text", "malformed", "uid", "empty-rv", "unchanged-rv", "spec", "api",
                 "kind", "namespace", "name", "deleting", "lost-ack")
        for mode in modes:
            with self.subTest(mode=mode):
                guard = FakeGuard(self.directory)
                original = guard.kubectl

                def transport(args, **kwargs):
                    response = original(args, **kwargs)
                    if args[:2] != ["patch", "networkpolicy/allow-envoy"]:
                        return response
                    self.assertEqual(args[-2:], ["-o", "json"])
                    if mode == "lost-ack":
                        raise TimeoutError("synthetic ACK lost after application")
                    if mode == "text":
                        return "patched"
                    if mode == "malformed":
                        return "{"
                    value = json.loads(response)
                    if mode == "uid": value["metadata"]["uid"] = uid()
                    elif mode == "empty-rv": value["metadata"]["resourceVersion"] = ""
                    elif mode == "unchanged-rv": value["metadata"]["resourceVersion"] = "10"
                    elif mode == "spec": value["spec"]["egress"].clear()
                    elif mode == "api": value["apiVersion"] = "other/v1"
                    elif mode == "kind": value["kind"] = "Secret"
                    elif mode == "namespace": value["metadata"]["namespace"] = "other"
                    elif mode == "name": value["metadata"]["name"] = "other"
                    elif mode == "deleting": value["metadata"]["deletionTimestamp"] = "2026-01-01T00:00:00Z"
                    return json.dumps(value)

                with patch.object(runner.runtime, "target", return_value=guard), patch.object(guard, "kubectl", side_effect=transport):
                    with self.assertRaises(ValueError):
                        self.configure()
                self.assertEqual(len(guard.patches), 1)
                self.assertEqual(len(guard.policy["spec"]["ingress"]), 2)
                self.assertFalse(any(call[:3] == ["get", "networkpolicy", "allow-envoy"] for call in guard.calls))

    def test_envoy_ingress_rollback_uses_ack_rv_and_does_not_adopt_postrollback_get(self):
        for mode in ("clean", "before-read", "between-read-and-patch", "lost-ack", "malformed-ack", "post-read-rv"):
            with self.subTest(mode=mode):
                guard = FakeGuard(self.directory)
                original_spec = clone(guard.policy["spec"])
                with patch.object(runner.runtime, "target", return_value=guard):
                    change = self.configure()
                    original = guard.kubectl
                    if mode == "before-read":
                        guard.policy["metadata"]["resourceVersion"] = "13"

                    def transport(args, **kwargs):
                        is_patch = args[:2] == ["patch", "networkpolicy/allow-envoy"]
                        if is_patch and mode == "between-read-and-patch":
                            guard.policy["metadata"]["resourceVersion"] = "13"
                        if (args[:3] == ["get", "networkpolicy", "allow-envoy"]
                                and len(guard.patches) == 2 and mode == "post-read-rv"):
                            guard.policy["metadata"]["resourceVersion"] = "13"
                        response = original(args, **kwargs)
                        if is_patch:
                            self.assertEqual(args[-2:], ["-o", "json"])
                            operations = json.loads(args[args.index("-p") + 1])
                            self.assertEqual(operations[1], {"op": "test", "path": "/metadata/resourceVersion", "value": "11"})
                            if mode == "lost-ack": raise TimeoutError("synthetic rollback ACK loss")
                            if mode == "malformed-ack": return "{"
                        return response

                    with patch.object(guard, "kubectl", side_effect=transport):
                        if mode == "clean":
                            self.assertEqual(change.rollback(), "ROLLED_BACK_OWN_ENTRY")
                            self.assertEqual(change.receipt["rollback_ack"]["rv"], "12")
                            self.assertEqual(guard.policy["spec"], original_spec)
                        else:
                            with self.assertRaisesRegex(ValueError, "rollback blocked"):
                                change.rollback()
                            calls = len(guard.calls)
                            with self.assertRaisesRegex(ValueError, "rollback blocked"):
                                change.rollback()
                            self.assertEqual(len(guard.calls), calls)
                            self.assertEqual(change.receipt["state"], "STOP_ROLLBACK_UNCERTAIN")
                            if mode == "post-read-rv":
                                self.assertEqual(change.receipt["rollback_ack"]["rv"], "12")
                                self.assertNotEqual(change.receipt["rollback_ack"]["rv"], guard.policy["metadata"]["resourceVersion"])
                    self.assertEqual(len(guard.patches), 1 if mode == "before-read" else 2)

    def test_envoy_ingress_qualified_capture_helper_pair_preserves_immutable_evidence(self):
        key, entry, raw = self.historical_pair(self.guard)
        with patch.object(runner, "_INGRESS_HISTORICAL_OBSERVATION", key[2]), \
                patch.object(runner, "_INGRESS_CAPTURE_COMPATIBILITY", (entry,)):
            change = self.configure()
            authority = change.receipt["authority"]
            self.assertEqual(authority["capture_helper_raw_sha256"], "9" * 64)
            self.assertEqual(authority["mutation_helper_raw_sha256"], entry[3])
            self.assertEqual(authority["qualification_record_sha256"], entry[4])
            self.assertEqual(authority["capture_projections_raw_sha256"], clone(entry[2]))
            self.assertEqual(authority["compatibility"]["mode"], "QUALIFIED_EXACT_PAIR")
            authority["compatibility"]["entry"][2].clear()
            self.assertEqual(change.rollback(), "ROLLED_BACK_OWN_ENTRY")
        for name, value in raw.items():
            self.assertEqual((self.directory / (name + ".json")).read_bytes(), value)

    def test_envoy_ingress_unknown_helper_pair_or_rewritten_historical_summary_is_rejected(self):
        for mode in ("empty", "unknown-mutation", "producer", "ops-head", "manifest", "rewritten-equality",
                     "short-head", "short-producer", "short-mutation", "short-record", "record-bool",
                     "manifest-order", "manifest-duplicate", "entry-extra", "entry-list", "manifest-list",
                     "table-list", "duplicate-entry"):
            with self.subTest(mode=mode):
                guard = FakeGuard(self.directory)
                key, entry, _ = self.historical_pair(guard)
                changed = list(entry)
                table = (entry,)
                if mode == "empty": table = ()
                elif mode == "unknown-mutation": changed[3] = "0" * 64
                elif mode == "producer": guard.edit("summary", lambda value: value.__setitem__("helper_sha256", "8" * 64))
                elif mode == "ops-head": guard.edit("summary", lambda value: value.__setitem__("ops_head", "2" * 40))
                elif mode == "manifest": changed[2] = tuple((name, "0" * 64 if name == "summary" else value) for name, value in entry[2])
                elif mode == "rewritten-equality":
                    guard.edit("summary", lambda value: value.__setitem__("helper_sha256", entry[3]))
                elif mode == "short-head": changed[0] = entry[0][:7]
                elif mode == "short-producer": changed[1] = entry[1][:8]
                elif mode == "short-mutation": changed[3] = entry[3][:8]
                elif mode == "short-record": changed[4] = entry[4][:8]
                elif mode == "record-bool": changed[4] = True
                elif mode == "manifest-order": changed[2] = tuple(reversed(entry[2]))
                elif mode == "manifest-duplicate": changed[2] = (entry[2][0], entry[2][0], *entry[2][2:])
                elif mode == "entry-extra": changed.append("unexpected")
                elif mode == "manifest-list": changed[2] = list(entry[2])
                if mode not in {"empty", "table-list", "duplicate-entry"}:
                    table = (changed if mode == "entry-list" else tuple(changed),)
                elif mode == "table-list": table = [entry]
                elif mode == "duplicate-entry": table = (entry, entry)
                with patch.object(runner.runtime, "target", return_value=guard), \
                        patch.object(runner, "_INGRESS_HISTORICAL_OBSERVATION", key[2]), \
                        patch.object(runner, "_INGRESS_CAPTURE_COMPATIBILITY", table):
                    with self.assertRaises(ValueError):
                        self.configure()
                self.assertEqual(guard.calls, [])
                self.assertEqual(guard.patches, [])

    def test_envoy_ingress_reserved_capture_head_cannot_reclassify_altered_observation(self):
        key, entry, _ = self.historical_pair(self.guard)
        self.guard.edit("observation", lambda value: value.__setitem__("synthetic_untrusted_edit", True))
        changed_observation = hashlib.sha256((self.directory / "observation.json").read_bytes()).hexdigest()
        self.guard.edit("causal-review", lambda value: value.__setitem__("observation_sha256", changed_observation))
        self.guard.edit("summary", lambda value: value.__setitem__("helper_sha256", entry[3]))
        with patch.object(runner, "_INGRESS_HISTORICAL_OBSERVATION", key[2]), \
                patch.object(runner, "_INGRESS_CAPTURE_COMPATIBILITY", (entry,)):
            with self.assertRaises(ValueError):
                self.configure()
        self.assertEqual(self.guard.calls, [])
        self.assertEqual(self.guard.patches, [])

    def test_envoy_ingress_mutation_authority_cannot_change_after_ack(self):
        for mode in ("raw-even-if-other-qualified", "class-identity"):
            with self.subTest(mode=mode):
                guard = FakeGuard(self.directory)
                key, entry, _ = self.historical_pair(guard)
                with patch.object(runner.runtime, "target", return_value=guard), \
                        patch.object(runner, "_INGRESS_HISTORICAL_OBSERVATION", key[2]), \
                        patch.object(runner, "_INGRESS_CAPTURE_COMPATIBILITY", (entry,)):
                    change = self.configure()
                    calls = len(guard.calls)
                    if mode == "raw-even-if-other-qualified":
                        replacement = self.directory / "not-executed-helper-source.py"
                        replacement.write_bytes(Path(runner.target_authority.__file__).read_bytes() + b"\n")
                        other = list(entry)
                        other[3] = hashlib.sha256(replacement.read_bytes()).hexdigest()
                        other[4] = "d" * 64
                        other = tuple(other)
                        with patch.object(runner.target_authority, "__file__", str(replacement)), \
                                patch.object(runner, "_INGRESS_CAPTURE_COMPATIBILITY", (other,)):
                            files, hashes, _ = runner._ingress_evidence(self.directory)
                            self.assertEqual(runner._ingress_authority(files, hashes)["mutation_helper_raw_sha256"],
                                             other[3])
                            with self.assertRaisesRegex(ValueError, "rollback blocked"):
                                change.rollback()
                    else:
                        replacement_class = type("ReplacementAuthority", (), {})
                        with patch.object(runner.target_authority, "LocalClusterTargetAnchor", replacement_class), \
                                patch.object(runner.runtime, "LocalClusterTargetAnchor", replacement_class):
                            with self.assertRaisesRegex(ValueError, "rollback blocked"):
                                change.rollback()
                    self.assertEqual(len(guard.calls), calls)
                    self.assertEqual(len(guard.patches), 1)
                    with self.assertRaisesRegex(ValueError, "rollback blocked"):
                        change.rollback()
                    self.assertEqual(len(guard.calls), calls)

    def test_envoy_ingress_accepts_declared_oauth_profile_and_rejects_unknown_policy(self):
        declarations = [item for item in runner.target_authority.yaml.safe_load_all(
            (DEPLOY / "apps/overlays/local-full/oauth-proof/resources.yaml").read_text())
            if item and item["kind"] == "NetworkPolicy"]
        self.assertEqual({item["metadata"]["name"] for item in declarations},
                         {"oauth-proof-iam-egress", "oauth-proof-connector", "oauth-proof-provider"})
        for mode in ("declared", "unknown", "selector-drift"):
            with self.subTest(mode=mode):
                guard = FakeGuard(self.directory)
                for item in declarations:
                    policy = clone(item)
                    policy["metadata"].update(uid=uid(), resourceVersion="10")
                    guard.policies.append(policy)
                    guard.data["ownedResources"].append({"kind": "NetworkPolicy", "name": policy["metadata"]["name"],
                        "namespace": runner.CLUSTER, "uid": policy["metadata"]["uid"],
                        "context": runner.CONTEXT, "cluster": runner.CLUSTER})
                if mode == "unknown":
                    guard.policies[-1]["metadata"]["name"] = "oauth-proof-foreign"
                    guard.data["ownedResources"][-1]["name"] = "oauth-proof-foreign"
                elif mode == "selector-drift":
                    guard.policies[-1]["spec"]["podSelector"] = {}
                preserved = clone(guard.policies[4:])
                with patch.object(runner.runtime, "target", return_value=guard):
                    if mode == "declared":
                        change = self.configure()
                        self.assertEqual(len(guard.patches), 1)
                        self.assertEqual(guard.policies[4:], preserved)
                        self.assertEqual(change.rollback(), "ROLLED_BACK_OWN_ENTRY")
                    else:
                        with self.assertRaises(ValueError):
                            self.configure()
                        self.assertEqual(guard.patches, [])

    def test_envoy_ingress_parent_witness_requires_exact_303_and_rolls_back_302(self):
        for status in (303, 302, 301, 307, 308, 200, 500):
            with self.subTest(status=status):
                guard = FakeGuard(self.directory)
                original_spec = clone(guard.policy["spec"])
                with patch.object(runner.runtime, "target", return_value=guard):
                    change = self.configure()
                    after = guard.admin(1)
                    for key in ("cluster.iam.ssl.fail_verify_san", "cluster.iam.ssl.fail_verify_error"):
                        after["stats"][key] = 0
                    witness = {"schema": "aiforall.envoy.ingress-parent-witness.v1",
                        "journal_sha256": change.receipt["journal_sha256"], "entry": "127.0.0.1:18080", "method": "GET",
                        "path": "/iam/api/auth/gitlab/login", "connector": "gitlab", "status": status,
                        "outcome": "IAM_CONNECTOR_NONCE_REDIRECT", "protected_without_bearer": "DENIED",
                        "admin_before": guard.admin(), "admin_after": after}
                    if status == 303:
                        self.assertEqual(change.finish(witness), "PARENT_WITNESS_ACCEPTED_LAB_ONLY")
                        self.assertEqual(len(guard.patches), 1)
                        self.assertEqual(len(guard.policy["spec"]["ingress"]), 2)
                        self.assertEqual(change.rollback(), "ROLLED_BACK_OWN_ENTRY")
                    else:
                        with self.assertRaisesRegex(ValueError, "validation refused"):
                            change.finish(witness)
                        self.assertEqual(change.receipt["state"], "ROLLED_BACK_OWN_ENTRY")
                        self.assertEqual(len(guard.patches), 2)
                        rollback = guard.patches[-1]
                        self.assertEqual([op["op"] for op in rollback], ["test", "test", "test", "test", "remove"])
                        self.assertEqual(rollback[-1]["path"], "/spec/ingress/1")
                    self.assertEqual(guard.policy["spec"], original_spec)

    def test_envoy_ingress_tunnel_mismatch_ambiguity_and_stale_proof_block_patch(self):
        mutations = [
            lambda g: g.nodes[0].__setitem__(3, "192.0.2.99"),
            lambda g: g.nodes[1].__setitem__(3, g.source),
            lambda g: g.nodes[0].__setitem__(0, uid()),
            lambda g: g.data.__setitem__("control_plane_container_id", "f" * 64),
            lambda g: g.slices["items"][0]["endpoints"][0]["conditions"].__setitem__("ready", False),
            lambda g: g.edit("causal-review", lambda v: v.__setitem__("decision", "PENDING_LEAD_CAUSAL_REVIEW")),
            lambda g: g.edit("causal-review", lambda v: v.__setitem__("observation_sha256", "0" * 64)),
            lambda g: g.edit("causal-review", lambda v: v.__setitem__("causal_basis", [])),
            lambda g: g.edit("causal-review", lambda v: v.__setitem__("measured_source_ipv4", "192.0.2.99")),
            lambda g: g.edit("summary", lambda v: v.__setitem__("helper_sha256", "0" * 64)),
            lambda g: g.edit("summary", lambda v: v["iam_upstream_deltas"].__setitem__("cluster.iam.upstream_rq_total", 1)),
            lambda g: g.edit("passive-baseline", lambda v: v["deny"].__setitem__("packets", 8)),
            lambda g: g.edit("admin-after", lambda v: v.__setitem__("restart_epoch", 1)),
            lambda g: g.policy["spec"]["egress"].clear(),
            lambda g: g.policies[1]["spec"].__setitem__("ingress", [{}]),
            lambda g: g.data["ownedResources"].clear(),
            lambda g: g.policies.append({"metadata": {"name": "unknown-policy", "namespace": runner.CLUSTER,
                "uid": uid(), "resourceVersion": "10"}, "spec": {"podSelector": {}, "policyTypes": ["Ingress"], "ingress": [{}]}}),
            lambda g: (g.directory / "causal-review.json").unlink(),
        ]
        for mutation in mutations:
            with self.subTest(mutation=mutations.index(mutation)):
                self.guard = FakeGuard(self.directory)
                with patch.object(runner.runtime, "target", return_value=self.guard):
                    mutation(self.guard)
                    with self.assertRaisesRegex(ValueError, "redacted"):
                        self.configure()
                    self.assertEqual(self.guard.patches, [])
        self.guard = FakeGuard(self.directory)
        def drift(g):
            if g.node_reads == 2:
                g.nodes[0][3] = "192.0.2.99"
        self.guard.on_nodes = drift
        with patch.object(runner.runtime, "target", return_value=self.guard):
            with self.assertRaises(ValueError):
                self.configure()
            self.assertEqual(self.guard.patches, [])

    def test_envoy_ingress_patch_tests_uid_version_and_old_value_and_preserves_peers(self):
        old = clone(self.guard.policy)
        journal_seen = []
        def inspect_patch(g, operations):
            records = list(self.directory.glob("journal-*.json"))
            self.assertEqual(len(records), 1)
            record = json.loads(records[0].read_text())
            self.assertEqual(record["patch"], operations)
            self.assertEqual(record["before"]["allow-envoy"]["spec"], old["spec"])
            journal_seen.append(record)
        self.guard.on_patch = inspect_patch
        change = self.configure()
        operations = self.guard.patches[0]
        self.assertEqual(operations[:3], [
            {"op": "test", "path": "/metadata/uid", "value": old["metadata"]["uid"]},
            {"op": "test", "path": "/metadata/resourceVersion", "value": old["metadata"]["resourceVersion"]},
            {"op": "test", "path": "/spec", "value": old["spec"]}])
        self.assertEqual(operations[3], {"op": "add", "path": "/spec/ingress/-", "value": self.guard.entry})
        self.assertEqual(self.guard.policy["spec"]["egress"], old["spec"]["egress"])
        self.assertEqual(self.guard.policy["spec"]["ingress"][0], old["spec"]["ingress"][0])
        # Returned receipts and journal dictionaries cannot mutate rollback state.
        journal_seen[0]["before"]["allow-envoy"]["spec"].clear()
        change.receipt.clear()
        self.guard.on_patch = None
        self.assertEqual(change.rollback(), "ROLLED_BACK_OWN_ENTRY")
        self.assertEqual(self.guard.policy["spec"], old["spec"])
        self.assertEqual([p["op"] for p in self.guard.patches[1]], ["test", "test", "test", "test", "remove"])
        for field in ("uid", "resourceVersion", "spec"):
            with self.subTest(add_cas=field):
                g = FakeGuard(self.directory)
                def conflict(g, operations):
                    if field == "spec":
                        g.policy["spec"]["egress"].clear()
                    else:
                        g.policy["metadata"][field] = uid() if field == "uid" else "999"
                g.on_patch = conflict
                with patch.object(runner.runtime, "target", return_value=g):
                    with self.assertRaises(ValueError):
                        self.configure()
                self.assertEqual(len(g.patches), 1)
                self.assertEqual(len(g.policy["spec"]["ingress"]), 1)

    def test_envoy_ingress_noop_and_conditional_rollback_do_not_adopt_replacement(self):
        self.guard.policy["spec"]["ingress"].append(self.guard.entry)
        change = self.configure()
        self.assertFalse(change.receipt["added"])
        self.assertEqual(change.rollback(), "NOT_OWNED")
        self.assertEqual(self.guard.patches, [])
        self.guard.policy["spec"]["ingress"][-1]["from"][0]["ipBlock"]["cidr"] = "192.0.2.0/24"
        with self.assertRaises(ValueError):
            self.configure()
        self.assertEqual(self.guard.patches, [])
        for mode in ("replacement", "concurrent-spec", "concurrent-equal-spec-rv", "cas-rv",
                     "post-source-drift", "witness-failure", "changed-lease", "unacknowledged-patch"):
            with self.subTest(mode=mode):
                g = FakeGuard(self.directory)
                with patch.object(runner.runtime, "target", return_value=g):
                    if mode == "unacknowledged-patch":
                        def lost_ack(g, operations):
                            g.policy["spec"]["ingress"].append(g.entry)
                            g.policy["metadata"]["resourceVersion"] = "11"
                            raise TimeoutError("synthetic lost ACK")
                        g.on_patch = lost_ack
                        with self.assertRaises(ValueError):
                            self.configure()
                        self.assertEqual(len(g.patches), 1)
                        self.assertEqual(len(g.policy["spec"]["ingress"]), 2)
                        continue
                    if mode == "post-source-drift":
                        def drift(g, operations):
                            if operations[-1]["op"] == "add":
                                g.nodes[0][3] = "192.0.2.99"
                        g.on_patch = drift
                        with self.assertRaises(ValueError):
                            self.configure()
                        self.assertEqual(len(g.patches), 2)
                        self.assertEqual(len(g.policy["spec"]["ingress"]), 1)
                        continue
                    change = self.configure()
                    if mode == "replacement":
                        g.policy["metadata"]["uid"] = uid()
                    elif mode == "concurrent-spec":
                        g.policy["spec"]["egress"].append({"to": [{"podSelector": {}}]})
                    elif mode == "concurrent-equal-spec-rv":
                        g.policy["metadata"]["resourceVersion"] = "999"
                    elif mode == "cas-rv":
                        g.on_patch = lambda guard, operations: guard.policy["metadata"].__setitem__("resourceVersion", "999")
                    elif mode == "changed-lease":
                        fresh = FakeGuard(self.directory)
                        with patch.object(runner.runtime, "target", return_value=fresh):
                            with self.assertRaisesRegex(ValueError, "rollback blocked"):
                                change.rollback()
                        self.assertEqual(len(g.patches), 1)
                        continue
                    else:
                        with self.assertRaisesRegex(ValueError, "validation refused"):
                            change.finish({})
                        self.assertEqual(len(g.policy["spec"]["ingress"]), 1)
                        continue
                    with self.assertRaisesRegex(ValueError, "rollback blocked"):
                        change.rollback()
                    self.assertEqual(len(g.policy["spec"]["ingress"]), 2)
                    self.assertEqual(len(g.patches), 2 if mode == "cas-rv" else 1)


if __name__ == "__main__":
    unittest.main()
