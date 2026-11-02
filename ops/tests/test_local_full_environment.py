"""Bounded local lifecycle oracles; no Docker/Kind/credentials contacted."""
import copy
from contextlib import ExitStack, nullcontext
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "deploy"))
from ops.tests.test_local_full_target import Fixture, FakeCommands, anchor
import local_full_environment as environment


class PowerCommands(FakeCommands):
    def __init__(self, fixture):
        super().__init__(fixture)
        for node in fixture.nodes.values():
            node["configured_ports"] = copy.deepcopy(node["ports"])

    def __call__(self, cmd, **kwargs):
        if cmd[:2] == ["docker", "info"]:
            self.calls.append(cmd)
            return "engine"
        if cmd[:2] in (["docker", "start"], ["docker", "stop"]):
            self.calls.append(cmd)
            node = self.fixture.nodes[cmd[-1]]
            node["running"] = cmd[1] == "start"
            node["ports"] = copy.deepcopy(node["configured_ports"]) if node["running"] else {}
            return node["id"]
        return super().__call__(cmd, **kwargs)


class PowerFixture(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.f = Fixture(self.temp.name)
        self.cmd = PowerCommands(self.f)
        self.target = self.f.target(self.cmd)
        self.variables = patch.dict("os.environ", {}, clear=True)
        self.variables.start()
        self.addCleanup(self.variables.stop)


class PowerTests(PowerFixture):
    def test_power_journal_preserves_resources_recorded_since_anchor_creation(self):
        data = json.loads(self.f.lease.read_text())
        data["ownedResources"] = [{"kind": "Job", "uid": "new-recovery-job"}]
        self.f.lease.write_text(json.dumps(data))
        self.target.stop_nodes("engine")
        self.assertEqual(json.loads(self.f.lease.read_text())["ownedResources"], data["ownedResources"])

    def test_resume_noop_stop_resume_keeps_ids_ports_config_and_uid(self):
        original = self.f.config_path.read_bytes()
        ids = list(self.f.ids)
        self.assertEqual(self.target.resume_nodes("engine"), [])
        self.target.stop_nodes("engine")
        self.assertTrue(all(not n["running"] and n["ports"] == {} for n in self.f.nodes.values()))
        before_api = len(self.cmd.api())
        self.target.lifecycle_nodes("engine")
        self.assertEqual(len(self.cmd.api()), before_api)
        self.assertEqual(self.target.resume_nodes("engine"), ids)
        self.assertEqual(self.f.config_path.read_bytes(), original)
        self.assertEqual(self.target.data["kube_system_uid"], "owned-system-uid")
        self.assertEqual(self.target.data["nodeContainerIds"], ids)
        self.assertEqual([c[-1] for c in self.cmd.calls if c[:2] == ["docker", "stop"]], ids[1:] + ids[:1])
        self.assertFalse(any(c[0] == "kind" for c in self.cmd.calls))

    def test_stopped_target_still_refused_by_ordinary_api(self):
        self.target.stop_nodes("engine")
        self.cmd.calls.clear()
        with self.assertRaises(anchor.AnchorError):
            self.target.kubectl(["get", "nodes"], text=True)
        self.assertEqual(self.cmd.api(), [])

    def test_bad_configured_port_or_engine_refused_before_start_or_api(self):
        self.target.stop_nodes("engine")
        self.cmd.calls.clear()
        with self.assertRaises(anchor.AnchorError):
            self.target.resume_nodes("foreign-engine")
        self.f.nodes[self.f.ids[0]]["configured_ports"]["6443/tcp"][0]["HostPort"] = "60124"
        with self.assertRaises(anchor.AnchorError):
            self.target.resume_nodes("engine")
        self.assertFalse(any(c[:2] == ["docker", "start"] for c in self.cmd.calls))
        self.assertEqual(self.cmd.api(), [])

    def test_stale_uid_after_start_rolls_back_only_started_ids(self):
        self.target.stop_nodes("engine")
        self.cmd.calls.clear()
        self.cmd.uid = "foreign-uid"
        with self.assertRaises(anchor.AnchorError):
            self.target.resume_nodes("engine")
        self.assertTrue(all(not node["running"] for node in self.f.nodes.values()))
        self.assertEqual(self.target.data["kube_system_uid"], "owned-system-uid")


class ApiPolicyTests(unittest.TestCase):
    def setUp(self):
        self.target = Mock()
        self.target.kubectl.return_value = json.dumps({"items": [{"status": {
            "addresses": [{"type": "InternalIP", "address": "172.19.0.2"}]}}]})
        self.policies = {namespace: {"metadata": {"uid": namespace, "resourceVersion": "12"},
            "spec": {"podSelector": {"matchLabels": {"app": namespace}}, "policyTypes": ["Egress"],
                     "egress": [{"to": [{"ipBlock": {"cidr": "10.96.0.1/32"}}, {"ipBlock": {"cidr": "172.19.0.3/32"}}],
                                 "ports": [{"protocol": "TCP", "port": 443}, {"protocol": "TCP", "port": 6443}]}]}}
            for namespace in (environment.CLUSTER, "aiforall-local-apparatus")}

    def run_sync(self):
        with patch.object(environment, "owned", side_effect=lambda _t, _k, _n, ns: self.policies[ns]):
            environment.resync_api_policies(self.target)

    def test_changed_cp_ip_patches_only_endpoint_with_uid_rv_and_spec_tests(self):
        self.run_sync()
        writes = [call.args[0] for call in self.target.kubectl.call_args_list if call.args[0][0] == "patch"]
        self.assertEqual(len(writes), 2)
        for args in writes:
            namespace = args[args.index("-n") + 1]
            operations = json.loads(args[args.index("-p") + 1])
            self.assertEqual([p["op"] for p in operations], ["test", "test", "test", "replace"])
            self.assertEqual(operations[0]["value"], namespace)
            self.assertEqual(operations[1]["value"], "12")
            self.assertEqual(operations[2]["value"], self.policies[namespace]["spec"])
            desired = copy.deepcopy(self.policies[namespace]["spec"])
            desired["egress"][0]["to"][1]["ipBlock"]["cidr"] = "172.19.0.2/32"
            self.assertEqual(operations[3]["value"], desired)

    def test_current_cp_ip_is_a_noop(self):
        for policy in self.policies.values():
            policy["spec"]["egress"][0]["to"][1]["ipBlock"]["cidr"] = "172.19.0.2/32"
        self.run_sync()
        self.assertEqual(self.target.kubectl.call_count, 1)  # Node read only.

    def test_unexpected_second_policy_refused_before_first_write(self):
        self.policies["aiforall-local-apparatus"]["spec"]["egress"].append({"ports": [{"port": 53}]})
        with self.assertRaisesRegex(ValueError, "unexpected local API policy"):
            self.run_sync()
        self.assertEqual(self.target.kubectl.call_count, 1)

    def test_invalid_or_ambiguous_cp_refused_without_mutation(self):
        for items in ([], [{"status": {"addresses": []}}], [{"status": {
                "addresses": [{"type": "InternalIP", "address": "127.0.0.1"}]}}]):
            with self.subTest(nodes=items):
                self.target.reset_mock()
                self.target.kubectl.return_value = json.dumps({"items": items})
                with self.assertRaises(ValueError):
                    self.run_sync()
                self.assertEqual(self.target.kubectl.call_count, 1)


class EnvironmentTests(PowerFixture):
    def setUp(self):
        super().setUp()
        self.f.data["ownedResources"] = [{"kind": "Deployment", "name": "openbao", "namespace": environment.CLUSTER}]
        self.f.save()
        self.args = SimpleNamespace(action="up", environment_dir=Path(self.temp.name) / "stable-env",
            lease=self.f.lease, registry_state=None, github_input_file=None)
        self.runtime = SimpleNamespace(operation_lock=lambda *_: nullcontext(), create=Mock(), lease=Mock())
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        self.stack.enter_context(patch.object(environment, "LocalClusterTargetAnchor", side_effect=lambda *_a, **_k: self.f.target(self.cmd)))
        self.stack.enter_context(patch.object(environment.pki, "private_directory",
            side_effect=lambda path, create=False: path.mkdir(mode=0o700) if create else None))
        self.epoch = self.stack.enter_context(patch.object(environment, "coffer_epoch", return_value="epoch-one"))
        self.recover = self.stack.enter_context(patch.object(environment, "recover_coffer"))
        self.api_resync = self.stack.enter_context(patch.object(environment, "resync_api_policies"))
        self.stack.enter_context(patch.object(environment, "owned", side_effect=lambda *_a, **_k: {
            "metadata": {"uid": "dep", "resourceVersion": "1"}, "spec": {"replicas": 1,
            "template": {"metadata": {"labels": {}}, "spec": {"containers": [{"image": "unchanged-image"}]}}},
            "status": {"readyReplicas": 1}}))

    def operate(self):
        return environment.operate(self.args, self.runtime, Mock(), Mock())

    def test_up_up_stop_up_status_uses_original_registration(self):
        self.assertEqual(self.operate()["status"], "running")
        self.args.lease = None  # Every subsequent session needs only the stable directory.
        self.assertFalse(self.operate()["recovery_pending"])
        self.recover.assert_not_called()
        self.args.action = "stop"
        self.assertEqual(self.operate()["status"], "stopped")
        self.args.action = "status"
        self.cmd.calls.clear()
        self.assertEqual(self.operate()["status"], "stopped")
        self.assertEqual(self.cmd.api(), [])
        self.args.action = "up"
        self.epoch.return_value = "epoch-two"
        self.assertFalse(self.operate()["recovery_pending"])
        self.recover.assert_called_once()
        self.operate()
        self.recover.assert_called_once()  # No reseed or forced rollout on unchanged up.
        self.assertEqual(self.api_resync.call_count, 4)  # Up only, never stop/status.
        self.assertNotIn("kind", [c[0] for c in self.cmd.calls])
        self.assertFalse(any("apply" in c or "scale" in c or "delete" in c for c in self.cmd.calls))

    def test_unknown_environment_or_changed_anchor_not_adopted(self):
        self.args.lease = None
        with self.assertRaisesRegex(ValueError, "first local-up"):
            self.operate()
        self.assertFalse(self.args.environment_dir.exists())
        self.args.lease = self.f.lease
        self.operate()
        self.f.data["kube_system_uid"] = "different"
        self.f.save()
        self.cmd.calls.clear()
        with self.assertRaisesRegex(ValueError, "anchor identity changed"):
            self.operate()
        self.assertEqual(self.cmd.calls, [])

    def test_failed_up_stops_only_previously_stopped_nodes(self):
        self.operate()
        self.target.stop_nodes("engine")
        for error in (ValueError("not ready"), KeyboardInterrupt()):
            with self.subTest(error=type(error).__name__):
                self.epoch.side_effect = error
                with self.assertRaises(type(error)):
                    self.operate()
                self.assertTrue(all(not n["running"] for n in self.f.nodes.values()))

    def test_api_policy_failure_precedes_coffer_and_preserves_original_power_state(self):
        original = {key: node["running"] for key, node in self.f.nodes.items()}
        self.api_resync.side_effect = ValueError("policy refused")
        with self.assertRaisesRegex(ValueError, "policy refused"):
            self.operate()
        self.epoch.assert_not_called()
        self.recover.assert_not_called()
        self.assertEqual({key: node["running"] for key, node in self.f.nodes.items()}, original)


class CofferRecoveryTests(unittest.TestCase):
    def test_reuses_nonroot_seed_and_only_patches_identity_annotations(self):
        target = Mock()
        target.data = {"ownedResources": [{"kind": "Deployment", "name": "github-connect", "namespace": environment.CLUSTER}]}
        target.kubectl.return_value = ""
        inputs = {"version": 1, "client_id": "public-client", "client_secret": "PRIVATE_SENTINEL", "hmac_secret": "f" * 64}
        reference = {"path": "private-file", "size": 1, "mtime_ns": 1}
        state = {"lease_path": "original-lease", "github_input": reference, "pending_epoch": "new-epoch"}
        deployment = {"metadata": {"uid": "owned-deployment", "resourceVersion": "10"}, "spec": {"template": {
            "metadata": {"labels": {}}, "spec": {"initContainers": [{"name": "openbao-snapshot"}, {"name": "openbao-agent"}], "volumes": [
                {"name": "github-config", "configMap": {"name": "github-config"}},
                {"name": "openbao-scripts", "configMap": {"name": "scripts"}}]}}}}
        def resource(_target, kind, name, *_args):
            if kind == "ConfigMap":
                return {"data": {"development.toml": '[github]\nclient_id="public-client"'}}
            return deployment
        runtime = Mock()
        runtime.create.side_effect = lambda obj, *_: {**obj, "metadata": {**obj["metadata"], "uid": "new-job-uid"}}
        secret = lambda name, data: {"kind": "Secret", "metadata": {"name": name}, "data": data}
        with patch.object(environment, "owned", side_effect=resource), patch.object(environment, "input_reference", return_value=reference), \
                patch.object(environment, "coffer_epoch", return_value="new-epoch"):
            environment.recover_coffer(target, state, Mock(), runtime, Mock(return_value=inputs), secret)
        job, private = [call.args[0] for call in runtime.create.call_args_list]
        self.assertEqual(job["kind"], "Job")
        self.assertEqual(job["spec"]["template"]["spec"]["serviceAccountName"], "github-seed")
        self.assertEqual(job["spec"]["ttlSecondsAfterFinished"], 300)
        self.assertEqual(private["metadata"]["ownerReferences"][0]["uid"], "new-job-uid")
        self.assertNotIn("PRIVATE_SENTINEL", json.dumps(state))
        writes = [c.args[0] for c in target.kubectl.call_args_list if c.args[0][0] == "patch"]
        self.assertEqual({c[1] for c in writes}, {"deployment/lazaret", "deployment/iam", "deployment/github-connect"})
        for call in writes:
            changes = json.loads(call[-1])
            self.assertEqual([p["path"] for p in changes], ["/metadata/uid", "/metadata/resourceVersion", "/spec/template/metadata/annotations"])
        self.assertFalse(any(c.args[0][0] in {"apply", "delete", "scale"} for c in target.kubectl.call_args_list))


if __name__ == "__main__":
    unittest.main()
