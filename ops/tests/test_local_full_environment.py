"""Bounded local lifecycle oracles; no Docker/Kind/credentials contacted."""
import copy
from contextlib import ExitStack, nullcontext, redirect_stdout, redirect_stderr
import io
import json
import os
import shutil
from pathlib import Path
import stat
import sys
import tempfile
import traceback
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "deploy"))
from ops.tests.test_local_full_target import Fixture, FakeCommands, anchor
from ops.tests.test_openbao_local import load
import local_full_environment as environment

deployer = load("github_private_input_tests", "ops/deploy/deploy-local-full.py")


class PrivateGitHubInputTests(unittest.TestCase):
    """G2/G5: synthetic files and modeled metadata, no credentials/runtime."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name).resolve() / "github-input.json"
        self.inputs = {"version": 1, "client_id": "synthetic-client", "client_secret": "SYNTHETIC_OAUTH_PRIVATE",
                       "hmac_secret": "b" * 64}
        self.path.write_text(json.dumps(self.inputs), encoding="utf-8")
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        # Same synthetic-file privacy seam as the existing OAuth input tests.
        self.privacy = self.stack.enter_context(patch.object(environment, "private_file", return_value=True))
        self.stack.enter_context(patch.object(deployer, "private_file", return_value=True))

    def assert_redacted_refusal(self, operation):
        before = self.path.read_bytes() if self.path.exists() else None
        output = io.StringIO()
        with redirect_stdout(output), redirect_stderr(output):
            try:
                operation()
            except (ValueError, anchor.AnchorError) as error:
                # The contract requires a redacted refusal, not one exception
                # type: duplicate JSON uses the existing unique_json guard.
                diagnostic = "".join(traceback.format_exception(error))
            else:
                self.fail("G2: invalid or changed private input must be refused")
        public = output.getvalue() + diagnostic
        for sentinel in (self.inputs["client_secret"], self.inputs["hmac_secret"]):
            self.assertTrue(sentinel not in public, "G5: private input escaped through diagnostics")
        after = self.path.read_bytes() if self.path.exists() else None
        self.assertTrue(after == before, "G2: rejected input was overwritten")

    def test_g2_g5_valid_private_json_round_trips_with_metadata_only_reference(self):
        reference = environment.input_reference(self.path)
        self.assertEqual(reference, {"path": str(self.path), "size": self.path.stat().st_size,
                                     "mtime_ns": self.path.stat().st_mtime_ns})
        before = self.path.read_bytes()
        output = io.StringIO()
        with redirect_stdout(output), redirect_stderr(output):
            result = deployer.github_inputs(self.path)
        self.assertTrue(result == self.inputs, "G2: valid private JSON did not round trip")
        self.assertEqual(self.path.read_bytes(), before)
        for value in (self.inputs["client_secret"], self.inputs["hmac_secret"]):
            self.assertTrue(value not in output.getvalue() + json.dumps(reference), "G5: private input was published")

    def test_g2_changed_reference_before_read_is_refused_without_calling_reader(self):
        reference = environment.input_reference(self.path)
        self.path.write_text(json.dumps({**self.inputs, "client_id": "changed-synthetic-client"}), encoding="utf-8")
        reader = Mock()
        self.assert_redacted_refusal(lambda: environment.read_private_input(reference, reader))
        reader.assert_not_called()

    def test_g2_source_change_during_read_cannot_return_credentials(self):
        reference = environment.input_reference(self.path)
        original = copy.deepcopy(reference)
        def reader(path):
            self.assertEqual(path, self.path)
            value = json.loads(path.read_text(encoding="utf-8"))
            path.write_text(json.dumps({**value, "client_id": "changed-synthetic-client"}), encoding="utf-8")
            return value
        output = io.StringIO()
        with redirect_stdout(output), redirect_stderr(output):
            with self.assertRaisesRegex(ValueError, "changed during reading"):
                environment.read_private_input(reference, reader)
        self.assertEqual(reference, original, "G2: changed source silently re-registered")
        self.assertNotEqual(self.path.stat().st_size, reference["size"], "fixture must actually change the reference")
        for value in (self.inputs["client_secret"], self.inputs["hmac_secret"]):
            self.assertTrue(value not in output.getvalue(), "G5: source change leaked private values")

    def test_g2_missing_source_or_unqualified_acl_is_refused_before_reader(self):
        reference = environment.input_reference(self.path)
        reader = Mock()
        self.privacy.return_value = False
        self.assert_redacted_refusal(lambda: environment.read_private_input(reference, reader))
        reader.assert_not_called()
        self.privacy.return_value = True
        self.path.unlink()
        self.assert_redacted_refusal(lambda: deployer.github_inputs(self.path))

    def test_g2_symlink_and_reparse_metadata_are_refused_before_private_read(self):
        # Reuse the existing target fixture's modeled filesystem metadata pattern;
        # never create real links or require Windows symlink privileges.
        original_lstat = Path.lstat
        for metadata in ("symlink", "reparse"):
            with self.subTest(metadata=metadata):
                def attributes(path, *args, **kwargs):
                    result = original_lstat(path, *args, **kwargs)
                    if path == self.path:
                        result = Mock(wraps=result)
                        result.st_mode = stat.S_IFLNK if metadata == "symlink" else stat.S_IFREG
                        result.st_file_attributes = 0x400 if metadata == "reparse" else 0
                    return result
                with patch.object(Path, "lstat", attributes), patch.object(Path, "read_text") as read:
                    self.assert_redacted_refusal(lambda: deployer.github_inputs(self.path))
                read.assert_not_called()

    def test_g2_g5_invalid_duplicate_or_unknown_json_is_refused_and_redacted(self):
        valid = json.dumps(self.inputs)
        cases = {"syntax": valid[:-1], "duplicate": valid[:-1] + ',"client_secret":"duplicate"}',
                 "unknown-field": json.dumps({**self.inputs, "unexpected": self.inputs["client_secret"]}),
                 "wrong-version": json.dumps({**self.inputs, "version": True}),
                 "invalid-hmac": json.dumps({**self.inputs, "hmac_secret": "SYNTHETIC_OAUTH_PRIVATE"}),
                 "utf8": b'{"client_secret":"SYNTHETIC_OAUTH_PRIVATE",\xff}',
                 "oversized": valid + " " * 65536}
        for case, text in cases.items():
            with self.subTest(case=case):
                self.path.write_bytes(text if isinstance(text, bytes) else text.encode("utf-8"))
                self.assert_redacted_refusal(lambda: deployer.github_inputs(self.path))

    def test_g5_reader_exceptions_cannot_escape_private_values_or_chained_tracebacks(self):
        reference = environment.input_reference(self.path)
        for exception in (OSError, ValueError, KeyError, TypeError, UnicodeError):
            with self.subTest(exception=exception.__name__):
                reader = Mock(side_effect=exception(self.inputs["client_secret"] + self.inputs["hmac_secret"]))
                self.assert_redacted_refusal(lambda: environment.read_private_input(reference, reader))
                reader.assert_called_once_with(self.path)


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
        tool = os.environ.get("AIFORALL_HELM") or shutil.which("helm")
        self.variables = patch.dict("os.environ", {"AIFORALL_HELM": tool} if tool else {}, clear=True)
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
        import local_full_helm
        objects = local_full_helm.render(phase="foundation", values={"network": {"apiCidr": "172.19.0.3/32", "ingressCidr": ""}})
        self.policies = {o["metadata"]["namespace"]: o for o in objects if o["kind"] == "NetworkPolicy" and o["metadata"]["name"] == "allow-api"}
        for namespace, obj in self.policies.items():
            obj["metadata"].update(uid=namespace, resourceVersion="12")
        def call(args, **kwargs):
            if args[0] == "patch":
                namespace = args[args.index("-n") + 1]
                ack = copy.deepcopy(self.policies[namespace])
                ack["spec"] = json.loads(args[args.index("-p") + 1])[-1]["value"]
                ack["metadata"]["resourceVersion"] = "13"
                return json.dumps(ack)
            return self.target.kubectl.return_value
        self.target.kubectl.side_effect = call

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
    def recovery_fixture(self):
        target = Mock()
        target.data = {"ownedResources": [{"kind": "Deployment", "name": "github-connect", "namespace": environment.CLUSTER}]}
        reference = {"path": "synthetic-private-file", "size": 1, "mtime_ns": 1}
        state = {"lease_path": "synthetic-original-lease", "github_input": reference, "pending_epoch": "epoch-two"}
        inputs = {"version": 1, "client_id": "synthetic-client", "client_secret": "SYNTHETIC_OAUTH_PRIVATE", "hmac_secret": "b" * 64}
        deployment = {"metadata": {"uid": "owned", "resourceVersion": "10"}, "spec": {"template": {
            "metadata": {"labels": {}}, "spec": {"initContainers": [{"name": "openbao-snapshot"}, {"name": "openbao-agent"}],
            "volumes": [{"name": "github-config", "configMap": {"name": "github-config"}},
                        {"name": "openbao-scripts", "configMap": {"name": "scripts"}}]}}}}
        def resource(_target, kind, name, *_args):
            if kind == "ConfigMap":
                return {"data": {"development.toml": '[github]\nclient_id="synthetic-client"'}}
            return deployment
        return target, state, inputs, deployment, resource

    def test_g2_g4_recovery_refusals_precede_seed_and_all_patches(self):
        for refusal in ("missing-source", "changed-source", "wrong-client", "legacy-identity"):
            with self.subTest(refusal=refusal):
                target, state, inputs, deployment, resource = self.recovery_fixture()
                reference = state["github_input"]
                if refusal == "missing-source":
                    state.pop("github_input")
                if refusal == "wrong-client":
                    inputs["client_id"] = "foreign-client"
                if refusal == "legacy-identity":
                    deployment["spec"]["template"]["spec"]["initContainers"] = []
                runtime, save, secret = Mock(), Mock(), Mock()
                original = copy.deepcopy(state)
                with patch.object(environment, "owned", side_effect=resource), \
                        patch.object(environment, "input_reference", return_value={**reference, "size": 2} if refusal == "changed-source" else reference):
                    with self.assertRaises(ValueError) as raised:
                        environment.recover_coffer(target, state, save, runtime, Mock(return_value=inputs), secret)
                runtime.create.assert_not_called()
                secret.assert_not_called()
                save.assert_not_called()
                target.kubectl.assert_not_called()
                self.assertEqual(state, original)
                for value in (inputs["client_secret"], inputs["hmac_secret"]):
                    self.assertNotIn(value, str(raised.exception) + json.dumps(state))

    def test_g4_lost_create_ack_records_uncertainty_and_refuses_adoption(self):
        target, state, inputs, _deployment, resource = self.recovery_fixture()
        runtime, save = Mock(), Mock()
        runtime.create.side_effect = ValueError("synthetic ACK lost")
        with patch.object(environment, "owned", side_effect=resource), \
                patch.object(environment, "input_reference", return_value=state["github_input"]):
            with self.assertRaisesRegex(ValueError, "ACK lost"):
                environment.recover_coffer(target, state, save, runtime, Mock(return_value=inputs), Mock())
        self.assertIn("recovery_job", state)
        self.assertNotIn("recovery_seeded_epoch", state)
        self.assertEqual(state["pending_epoch"], "epoch-two")
        save.assert_called_once()
        recorded = copy.deepcopy(state)
        runtime.reset_mock()
        def expired_job(t, kind, name, *args):
            if kind == "Job":
                raise ValueError("job ACK absent; no adoption")
            return resource(t, kind, name, *args)
        with patch.object(environment, "owned", side_effect=expired_job), \
                patch.object(environment, "input_reference", return_value=state["github_input"]):
            with self.assertRaisesRegex(ValueError, "no adoption"):
                environment.recover_coffer(target, state, save, runtime, Mock(return_value=inputs), Mock())
        runtime.create.assert_not_called()
        target.kubectl.assert_not_called()
        self.assertEqual(state, recorded)

    def test_g2_g3_initial_source_is_not_silently_registered_by_ordinary_up(self):
        # Activation is a separate operation; local-up must retain its strict guard.
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp) / "synthetic-input.json"
            source.write_text("{}")
            reference = {"path": str(source), "size": 2, "mtime_ns": 1}
            directory = Path(temp) / "environment"
            directory.mkdir()
            state = {"schema": environment.SCHEMA, "lease_path": str(Path(temp) / "original-lease"),
                     "registry_state": None, "github_input": None}
            state_file = directory / "environment.json"
            state_file.write_text(json.dumps(state))
            original = state_file.read_bytes()
            args = SimpleNamespace(action="up", environment_dir=directory, lease=None,
                                   registry_state=None, github_input_file=source)
            with patch.object(environment.pki, "private_directory"), \
                    patch.object(environment, "input_reference", return_value=reference), \
                    patch.object(environment, "LocalClusterTargetAnchor") as target:
                with self.assertRaisesRegex(ValueError, "private source changed"):
                    environment.operate(args, Mock(), Mock(), Mock())
            target.assert_not_called()
            self.assertEqual(state_file.read_bytes(), original)

    def test_child_secret_ack_loss_refuses_retry_even_with_acknowledged_job(self):
        target, state, inputs, _deployment, resource = self.recovery_fixture()
        runtime, save = Mock(), Mock()
        runtime.create.side_effect = [{"metadata": {"uid": "parent-job"}}, ValueError("child ACK lost")]
        secret = lambda name, values: {"kind": "Secret", "metadata": {"name": name}, "data": values}
        def read(t, kind, name, *args):
            if kind == "Job": return {"metadata": {"uid": "parent-job"}}
            return resource(t, kind, name, *args)
        with patch.object(environment, "owned", side_effect=read), \
             patch.object(environment, "input_reference", return_value=state["github_input"]):
            with self.assertRaisesRegex(ValueError, "child ACK lost"):
                environment.recover_coffer(target, state, save, runtime, Mock(return_value=inputs), secret)
            self.assertEqual(state["recovery_job_uid"], "parent-job")
            self.assertNotIn("recovery_input_uid", state)
            self.assertNotIn("recovery_seeded_epoch", state)
            runtime.create.reset_mock()
            with self.assertRaisesRegex(ValueError, "no adoption"):
                environment.recover_coffer(target, state, save, runtime, Mock(return_value=inputs), secret)
            runtime.create.assert_not_called()
            target.kubectl.assert_not_called()

    def test_g4_seed_foundations_require_ack_uid_and_original_lease(self):
        for kind, name in (("ServiceAccount", "github-seed"),
                           ("NetworkPolicy", "allow-openbao-from-github-seed"),
                           ("NetworkPolicy", "allow-github-seed-to-openbao")):
            for mismatch in ("none", "uid", "cluster", "context", "namespace", "ack"):
                with self.subTest(kind=kind, mismatch=mismatch):
                    entry = {"kind": kind, "name": name, "namespace": environment.CLUSTER,
                             "uid": "acked-uid", "cluster": environment.CLUSTER, "context": "owned-context"}
                    target = Mock()
                    target.data = {"cluster": environment.CLUSTER, "context": "owned-context",
                                   "namespaceUids": {environment.CLUSTER: "owned-ns"}, "ownedResources": [entry]}
                    if mismatch in ("uid", "cluster", "context"):
                        entry[mismatch] = "foreign"
                    if mismatch == "ack":
                        target.data["ownedResources"] = []
                    obj = {"metadata": {"uid": "acked-uid"}, "spec": {"synthetic": True}}
                    target.kubectl.side_effect = ["foreign-ns" if mismatch == "namespace" else "owned-ns", json.dumps(obj)]
                    if mismatch == "none":
                        self.assertEqual(environment.owned(target, kind, name), obj)
                    else:
                        with self.assertRaisesRegex(ValueError, "not owned"):
                            environment.owned(target, kind, name)
                    self.assertTrue(all(c.args[0][0] == "get" for c in target.kubectl.call_args_list))

    def test_g4_second_epoch_cannot_complete_interrupted_attempt(self):
        state = {"engine": "engine", "epoch": "epoch-one", "pending_epoch": "epoch-two",
                 "recovery_job": "same-attempt", "recovery_seeded_epoch": "epoch-two"}
        original = copy.deepcopy(state)
        target, save, runtime = Mock(), Mock(), Mock()
        target.data = {"connectorProfile": "base", "ownedResources": []}
        with patch.object(environment, "coffer_epoch", return_value="epoch-three"), \
                patch.object(environment, "resync_api_policies"), \
                patch.object(environment, "recover_coffer") as recover:
            with self.assertRaisesRegex(ValueError, "another coffer epoch interrupted"):
                environment.running_operation(SimpleNamespace(action="up"), state, save, target,
                                              None, [{"running": True}], runtime, Mock(), Mock())
        recover.assert_not_called()
        save.assert_not_called()
        self.assertEqual(state, original)
        self.assertFalse(any(c.args[0][0] in {"patch", "apply", "delete"} for c in target.kubectl.call_args_list))

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
