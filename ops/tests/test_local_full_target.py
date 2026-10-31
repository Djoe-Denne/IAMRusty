"""Fake-command S-11 tests and a native Windows ACL invocation boundary."""
import base64
import hashlib
import importlib.util
import io
import json
import os
import re
import tempfile
import sys
import unittest
from email.message import Message
from pathlib import Path
from unittest.mock import MagicMock, Mock, patch
from urllib.parse import unquote

import yaml

spec = importlib.util.spec_from_file_location(
    "local_full_target", Path(__file__).resolve().parents[1] / "deploy/local_full_target.py")
anchor = importlib.util.module_from_spec(spec)
sys.modules["local_full_target"] = anchor
spec.loader.exec_module(anchor)


class FakeCommands:
    def __init__(self, fixture, exists=True):
        self.fixture, self.exists, self.calls, self.uid = fixture, exists, [], "owned-system-uid"

    def __call__(self, cmd, **kwargs):
        self.calls.append(cmd)
        f = self.fixture
        if cmd[:2] == ["docker", "ps"]:
            return " ".join(f.ids) if self.exists else ""
        if cmd[:2] == ["docker", "inspect"]:
            return json.dumps(f.nodes[cmd[-1]])
        if cmd[:3] == ["kind", "create", "cluster"]:
            self.exists = True
            f.write_config()
            return ""
        if cmd[0] == "kubectl":
            # Model effective global options, not merely a trusted prefix.
            local = cmd[1:cmd.index("--")] if "--" in cmd else cmd[1:]
            config_path = context = server = None
            for i, flag in enumerate(local):
                if flag in {"--kubeconfig", "--context", "--server", "-s"}:
                    value = local[i + 1]
                    if flag == "--kubeconfig": config_path = value
                    elif flag == "--context": context = value
                    else: server = value
                elif flag.startswith("--server="): server = flag.split("=", 1)[1]
                elif flag.startswith("-s") and not flag.startswith("--"): server = flag[2:].lstrip("=")
                elif flag.startswith(("-is", "-As", "-its")):
                    # Boolean shorthand cluster followed by string server flag.
                    server = flag[flag.index("s") + 1:].lstrip("=")
                elif flag.startswith("--context="): context = flag.split("=", 1)[1]
                elif flag.startswith("--kubeconfig="): config_path = flag.split("=", 1)[1]
            assert config_path == str(f.config_path)
            assert kwargs["env"]["KUBECONFIG"] == config_path
            effective = server or yaml.safe_load(f.config_path.read_bytes())["clusters"][0]["cluster"]["server"]
            assert effective == "https://127.0.0.1:60123", "foreign endpoint contacted"
            assert context == f.data["context"]
            return self.uid if "jsonpath={.metadata.uid}" in cmd else "ok"
        if cmd[:3] == ["kind", "load", "docker-image"]:
            return ""
        raise AssertionError("unexpected command")

    def api(self):
        return [c for c in self.calls if c[0] == "kubectl"]

    def kind_actions(self):
        return [c for c in self.calls if c[0] == "kind"]


class Fixture:
    def __init__(self, root, pending=False):
        self.root = Path(root)
        self.ids = [c * 64 for c in "abc"]
        self.config_path = self.root / ("cluster-anchor-" + "d" * 32) / "kubeconfig.yaml"
        self.config = {"apiVersion": "v1", "kind": "Config", "current-context": "kind-aiforall-local-full",
                       "contexts": [{"name": "kind-aiforall-local-full", "context": {"cluster": "owned", "user": "owned"}}],
                       "clusters": [{"name": "owned", "cluster": {"server": "https://127.0.0.1:60123", "certificate-authority-data": "Y2E="}}],
                       "users": [{"name": "owned", "user": {"client-certificate-data": "Y2VydA==", "client-key-data": "a2V5"}}]}
        self.nodes = {i: {"id": i, "running": True, "image": anchor.NODE_IMAGE, "cluster": "aiforall-local-full", "role": "control-plane" if i == self.ids[0] else "worker", "ports": {"6443/tcp": [{"HostIp": "127.0.0.1", "HostPort": "60123"}], "30080/tcp": [{"HostIp": "127.0.0.1", "HostPort": "18080"}]}} for i in self.ids}
        self.data = {"version": 2, "task_id": "d" * 32, "cluster": "aiforall-local-full", "context": "kind-aiforall-local-full", "anchor_state": "anchored", "nodeContainerIds": self.ids,
                     "control_plane_container_id": self.ids[0], "api_host": "127.0.0.1", "api_port": 60123, "kube_system_uid": "owned-system-uid", "kubeconfig_path": str(self.config_path),
                     "creation": {"task_id": "d" * 32, "created_ids": self.ids, "before_ids": [], "phase": "anchored"}}
        if pending:
            self.data.update(anchor_state="pending_creation", nodeContainerIds=[])
            for key in ["creation", "control_plane_container_id", "api_host", "api_port", "kube_system_uid"]:
                self.data.pop(key)
        else:
            self.config_path.parent.mkdir()
            self.write_config()
            self.data["kubeconfig_sha256"] = hashlib.sha256(self.config_path.read_bytes()).hexdigest()
        self.lease = self.root / "lease.json"
        self.save()

    def write_config(self):
        self.config_path.write_text(yaml.safe_dump(self.config))

    def save(self):
        self.lease.write_text(json.dumps(self.data))

    def target(self, commands):
        return anchor.LocalClusterTargetAnchor(self.lease, commands, self.root, lambda _: True, "aiforall-local-full")


class AnchorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.env = patch.dict(os.environ, {}, clear=True)
        self.env.start()
        self.addCleanup(self.env.stop)
        self.f = Fixture(self.temp.name)
        self.cmd = FakeCommands(self.f)

    def refused_before_api(self):
        self.f.save()
        with self.assertRaises(anchor.AnchorError):
            self.f.target(self.cmd).kubectl(["create", "namespace", "candidate"], text=True)
        self.assertEqual(self.cmd.api(), [])
        self.assertEqual(self.cmd.kind_actions(), [])

    def test_forward_rejects_foreign_target_bind_service_and_mapping_before_api(self):
        for change in ({"namespace": "foreign"}, {"service": "iam"},
                       {"bind": "0.0.0.0"}, {"bind": "localhost"}, {"mapping": "18082:8080"}):
            with self.subTest(change=change), patch.object(anchor.socket, "socket") as probe:
                with self.assertRaisesRegex(anchor.AnchorError, "before any probe"):
                    with self.f.target(self.cmd).oauth_forward(**change):
                        self.fail("foreign forward accepted")
                probe.assert_not_called()
                self.assertEqual(self.cmd.calls, [])
        with self.assertRaises(anchor.AnchorError):
            anchor.parse_kubectl_operation(["port-forward", "service/oauth-idp-simulator", "18081:8080"])

    def test_forward_requires_owned_service_and_stops_only_owned_pid(self):
        self.f.data["namespaceUids"] = {"aiforall-local-full": "owned-namespace"}
        entry = dict(kind="Service", name="oauth-idp-simulator", namespace="aiforall-local-full",
                     uid="owned-service", context="kind-aiforall-local-full", cluster="aiforall-local-full")
        self.f.data["ownedResources"] = [entry]
        self.f.save()
        value = dict(metadata=dict(uid="owned-service"), spec=dict(type="ClusterIP",
                     selector={"app.kubernetes.io/name": "oauth-idp-simulator"},
                     ports=[dict(port=8080, targetPort=8080)]))
        target = self.f.target(self.cmd)
        process = Mock(pid=12345)
        process.poll.return_value = None
        launcher = Mock(return_value=process)
        with patch.object(target, "check"), patch.object(target, "kubectl", side_effect=[
                "owned-namespace", json.dumps(value)]), patch.object(anchor.socket, "socket"):
            with target.oauth_forward(launcher=launcher) as child:
                self.assertIs(child, process)
                marker = json.loads(target.path.with_name(target.path.name + ".oauth-forward").read_text())
                self.assertEqual((marker["pid"], marker["service_uid"]), (12345, "owned-service"))
        if os.name == "nt":
            process.send_signal.assert_called_once_with(anchor.signal.CTRL_BREAK_EVENT)
            process.terminate.assert_not_called()
        else:
            process.terminate.assert_called_once_with()
        process.wait.assert_called_once_with(timeout=15)
        self.assertFalse(target.path.with_name(target.path.name + ".oauth-forward").exists())
        command = launcher.call_args.args[0]
        self.assertEqual(command, target.prefix() + ["port-forward", "-n", "aiforall-local-full",
                         "service/oauth-idp-simulator", "--address", "127.0.0.1", "18081:8080"])
        # A foreign Service with the same name never launches a child.
        value["metadata"]["uid"] = "foreign-service"
        launcher.reset_mock()
        with patch.object(target, "check"), patch.object(target, "kubectl", side_effect=[
                "owned-namespace", json.dumps(value)]):
            with self.assertRaisesRegex(anchor.AnchorError, "Service UID not owned"):
                with target.oauth_forward(launcher=launcher):
                    self.fail("foreign UID accepted")
        launcher.assert_not_called()

    def test_forward_cli_propagates_child_failure_and_preserves_owned_interrupt(self):
        for outcome, expected in ((0, 0), (1, 1), (KeyboardInterrupt, 0)):
            with self.subTest(outcome=outcome):
                target = MagicMock()
                process = Mock()
                target.oauth_forward.return_value.__enter__.return_value = process
                target.oauth_forward.return_value.__exit__.return_value = False
                if outcome is KeyboardInterrupt:
                    process.wait.side_effect = KeyboardInterrupt
                else:
                    process.wait.return_value = outcome
                arguments = ["local_full_target.py", "oauth-forward", "--lease", str(self.f.lease),
                             "--cluster", "aiforall-local-full"]
                with patch.object(anchor, "LocalClusterTargetAnchor", return_value=target), \
                        patch.object(anchor.sys, "argv", arguments), patch("builtins.print") as output:
                    self.assertEqual(anchor.main(), expected)
                process.wait.assert_called_once_with()
                target.oauth_forward.assert_called_once_with()
                target.kubectl.assert_not_called()
                target.kind_load.assert_not_called()
                if expected:
                    output.assert_called_once_with(
                        "STOP: target anchor validation/operation failed (redacted); no adoption/reset",
                        file=anchor.sys.stderr)
                else:
                    output.assert_not_called()

    def test_wrong_loopback_alias_zero_get_mutation_and_kind(self):
        self.f.config["clusters"][0]["cluster"]["server"] = "https://127.0.0.1:60999"
        self.f.write_config()
        self.f.data["kubeconfig_sha256"] = hashlib.sha256(self.f.config_path.read_bytes()).hexdigest()
        self.refused_before_api()
        with self.assertRaises(anchor.AnchorError):
            self.f.target(self.cmd).kind_load("image:sha256-test")
        self.assertEqual(self.cmd.api(), [])
        self.assertEqual(self.cmd.kind_actions(), [])

    def test_valid_fixed_config_and_uid_before_write(self):
        self.f.target(self.cmd).kubectl(["create", "namespace", "candidate"], text=True)
        self.assertIn("kube-system", self.cmd.api()[0])
        self.assertIn("create", self.cmd.api()[1])

    def test_url_userinfo_suffix_and_hostname_alias_zero_api(self):
        for server in ["https://@127.0.0.1:60123", "https://localhost:60123", "https://127.0.0.1:60123/", "https://127.0.0.1:60123?", "https://127.0.0.1:60123#"]:
            self.f.config["clusters"][0]["cluster"]["server"] = server
            self.f.write_config()
            self.f.data["kubeconfig_sha256"] = hashlib.sha256(self.f.config_path.read_bytes()).hexdigest()
            self.refused_before_api()

    def test_stale_uid_one_metadata_get_zero_mutation(self):
        self.cmd.uid = "foreign-uid"
        with self.assertRaises(anchor.AnchorError):
            self.f.target(self.cmd).kubectl(["create", "namespace", "candidate"], text=True)
        self.assertEqual(len(self.cmd.api()), 1)
        self.assertNotIn("create", self.cmd.api()[0])
        self.assertEqual(json.loads(self.f.lease.read_text())["kube_system_uid"], "owned-system-uid")

    def test_no_prior_uid_or_v1(self):
        for field, value in [("version", 1), ("kube_system_uid", None), ("control_plane_container_id", self.f.ids[1])]:
            original = self.f.data[field]
            self.f.data[field] = value
            self.refused_before_api()
            self.f.data[field] = original

    def test_mapping_missing_multiple_wildcard_wrong_port(self):
        cp = self.f.nodes[self.f.ids[0]]
        for mappings in [None, [], [{"HostIp": "0.0.0.0", "HostPort": "60123"}], [{"HostIp": "::", "HostPort": "60123"}], [{"HostIp": "127.0.0.1", "HostPort": "60999"}], [{"HostIp": "127.0.0.1", "HostPort": "60123"}] * 2]:
            cp["ports"]["6443/tcp"] = mappings
            self.refused_before_api()

    def test_hash_env_and_external_auth_override(self):
        original = self.f.data["kubeconfig_sha256"]
        self.f.data["kubeconfig_sha256"] = "0" * 64
        self.refused_before_api()
        self.f.data["kubeconfig_sha256"] = original
        with patch.dict(os.environ, {"KUBECONFIG": "ambient-other.yaml"}):
            self.refused_before_api()
        self.f.config["users"][0]["user"]["exec"] = {"command": "foreign-provider"}
        self.f.write_config()
        self.f.data["kubeconfig_sha256"] = hashlib.sha256(self.f.config_path.read_bytes()).hexdigest()
        self.refused_before_api()

    def test_unattested_created_ids_and_protected_alias(self):
        self.f.data["creation"]["before_ids"] = [self.f.ids[0]]
        self.refused_before_api()
        self.f.data["creation"]["before_ids"] = []
        self.f.data["context"] = "kind-apparatus-p4-it"
        self.refused_before_api()

    def test_kubeconfig_outside_approved_root_zero_commands(self):
        with tempfile.TemporaryDirectory() as outside:
            self.f.data["kubeconfig_path"] = str((Path(outside) / "kubeconfig.yaml").resolve())
            self.f.save()
            with self.assertRaisesRegex(anchor.AnchorError, "^kubeconfig outside task-owned approved directory$"):
                self.f.target(self.cmd).kubectl(["get", "nodes"], text=True)
        self.assertEqual(self.cmd.calls, [])

    def test_linked_or_reparse_kubeconfig_zero_commands(self):
        # Model filesystem metadata, avoiding Windows symlink privileges or real links.
        original_stat = Path.stat
        for metadata in ("symlink", "reparse"):
            with self.subTest(metadata=metadata):
                def linked(path):
                    return metadata == "symlink" and path == self.f.config_path

                def attributes(path, *args, **kwargs):
                    result = original_stat(path, *args, **kwargs)
                    if metadata == "reparse" and path == self.f.config_path:
                        result = Mock(wraps=result)
                        result.st_file_attributes = anchor.stat.FILE_ATTRIBUTE_REPARSE_POINT
                    return result

                with patch.object(Path, "is_symlink", linked), patch.object(Path, "stat", attributes):
                    with self.assertRaisesRegex(anchor.AnchorError, "^linked kubeconfig path forbidden$"):
                        self.f.target(self.cmd).kubectl(["get", "nodes"], text=True)
                self.assertEqual(self.cmd.calls, [])

    def test_prefabricated_anchored_without_creation_attestation_zero_commands(self):
        self.f.data.pop("creation")
        self.f.save()
        with self.assertRaisesRegex(anchor.AnchorError, "^exact task-created node delta not attested$"):
            anchor.bootstrap(self.f.lease, "aiforall-local-full", "unused", True,
                             self.cmd, self.f.root, lambda _: True)
        self.assertEqual(self.cmd.calls, [])

    def test_command_override_zero_api_and_missing_privacy(self):
        for flag in ["-shttps://127.0.0.1:60999", "-ishttps://127.0.0.1:60999", "-ns", "--server=https://127.0.0.1:60999", "-s", "--context=kind-other", "--cluster=other", "--kubeconfig=other", "--insecure-skip-tls-verify=true", "--token=untrusted"]:
            with self.assertRaises(anchor.AnchorError):
                self.f.target(self.cmd).kubectl(["get", "nodes", flag], text=True)
        self.assertEqual(self.cmd.api(), [])
        target = anchor.LocalClusterTargetAnchor(self.f.lease, self.cmd, self.f.root, lambda _: False)
        with self.assertRaises(anchor.AnchorError):
            target.check()
        self.assertEqual(self.cmd.api(), [])

    def test_pending_creation_attests_endpoint_then_seals_uid(self):
        with tempfile.TemporaryDirectory() as root:
            fixture = Fixture(root, pending=True)
            commands = FakeCommands(fixture, exists=False)
            config = Path(root) / "kind.yaml"
            config.write_text(yaml.safe_dump({"name": "aiforall-local-full", "nodes": [{"role": "control-plane"}, {"role": "worker"}, {"role": "worker"}]}))
            with patch.object(anchor, "secure", lambda *a, **k: None), patch.object(anchor.socket, "socket"):
                result = anchor.bootstrap(fixture.lease, "aiforall-local-full", config, True, commands, root, lambda _: True)
            self.assertEqual(result["anchor_state"], "anchored")
            self.assertEqual(result["kube_system_uid"], "owned-system-uid")
            self.assertEqual(len(commands.api()), 1)
            self.assertEqual(commands.kind_actions()[0][1:3], ["create", "cluster"])
            self.assertIn("--retain", commands.kind_actions()[0])

    def test_pending_cannot_adopt_existing_or_run_before_final_it(self):
        with tempfile.TemporaryDirectory() as root:
            fixture = Fixture(root, pending=True)
            commands = FakeCommands(fixture, exists=True)
            for consent in [False, True]:
                with self.assertRaises(anchor.AnchorError):
                    anchor.bootstrap(fixture.lease, "aiforall-local-full", "unused", consent, commands, root, lambda _: True)
            self.assertEqual(commands.kind_actions(), [])
            self.assertEqual(commands.api(), [])

    def test_envoy_patch_json_ack_grammar_is_exact_before_any_probe(self):
        payload = json.dumps([{"op": "test", "path": "/metadata/uid", "value": "owned-policy"}])
        args = ["patch", "networkpolicy/allow-envoy", "-n", "aiforall-local-full",
                "--type=json", "-p", payload, "-o", "json"]
        for valid in (args, ["patch", "-o", "json", "--namespace=aiforall-local-full",
                            "-p", payload, "networkpolicy/allow-envoy", "--type", "json"]):
            self.assertEqual(anchor.parse_kubectl_operation(valid).argv, tuple(valid))
        self.f.target(self.cmd).kubectl(args, text=True)
        self.assertEqual(self.cmd.api()[-1][-len(args):], args)
        self.cmd.calls.clear()
        invalid = []
        for target in ("secret/platform-mesh-certs", "se/private", "secrets/private", "deployment/iam",
                       "networkpolicy/other", "networkpolicies/allow-envoy", "NetworkPolicy/allow-envoy"):
            invalid.append([args[0], target, *args[2:]])
        for namespace in ("other", "aiforall-local", "kube-system"):
            invalid.append([*args[:3], namespace, *args[4:]])
        invalid.append([*args[:2], *args[4:]])
        for output in ("yaml", "name", "jsonpath={.metadata.uid}"):
            invalid.append([*args[:-1], output])
        for suffix in (["-n", "aiforall-local-full"], ["--namespace", "aiforall-local-full"],
                       ["--type=json"], ["-p", payload], ["-o", "json"], ["another-target"],
                       ["--"], ["--", "remote"], ["--record=true"], ["--dry-run=client"],
                       ["--local"], ["--subresource=status"], ["--patch-file", "patch.json"],
                       ["--server=https://127.0.0.1:60999"], ["--context=kind-other"],
                       ["--kubeconfig=other"], ["--token=untrusted"], ["--as=other"],
                       ["--insecure-skip-tls-verify=true"]):
            invalid.append([*args, *suffix])
        for compact in ("-ojson", "-o=json", "--output=json"):
            invalid.append([*args[:-2], compact])
        for kind in ("merge", "strategic"):
            invalid.append([*args[:4], "--type=" + kind, *args[5:]])
        for malformed in ("[]", "{}", "null", "[1]", '[{"op":1,"path":"/x"}]'):
            invalid.append([*args[:6], malformed, *args[7:]])
        invalid.append([*args[:5], "--patch-file", "patch.json", *args[7:]])
        invalid.append([*args[:4], *args[5:]])
        for index, command in enumerate(invalid):
            with self.subTest(case=index):
                with self.assertRaises(anchor.AnchorError):
                    self.f.target(self.cmd).kubectl(command, text=True)
                self.assertEqual(self.cmd.calls, [])
        # Existing grammar without this one output mode is unchanged. This is
        # parsing only, never a Secret read or mutation.
        legacy = ["patch", "secret/owned", "-n", "aiforall-local-full", "--type=json", "-p", payload]
        self.assertEqual(anchor.parse_kubectl_operation(legacy).argv, tuple(legacy))
        self.assertEqual(anchor.parse_kubectl_operation(["get", "nodes", "-o", "yaml"]).argv,
                         ("get", "nodes", "-o", "yaml"))

    def test_positive_grammar_rejects_unknown_before_any_probe(self):
        for args in [["get", "nodes", "--raw=/x"], ["get", "nodes", "-ojson"], ["get", "nodes", "-Ashttps://other"], ["delete", "namespace", "owned"], ["get", "nodes", "--", "-sother"]]:
            with self.assertRaises(anchor.AnchorError):
                self.f.target(self.cmd).kubectl(args, text=True)
        self.assertEqual(self.cmd.calls, [])

    def test_exec_remote_flags_are_not_local_target_overrides(self):
        self.f.target(self.cmd).kubectl(["exec", "pod", "--", "tool", "-sforeign"], text=True)
        self.assertEqual(len(self.cmd.api()), 2)

    def test_fixed_config_rechecked_after_uid_before_mutation(self):
        def changed_after_uid(cmd, **kwargs):
            result = self.cmd(cmd, **kwargs)
            if cmd[0] == "kubectl":
                self.f.config["clusters"][0]["cluster"]["server"] = "https://127.0.0.1:60999"
                self.f.write_config()
            return result
        with self.assertRaises(anchor.AnchorError):
            self.f.target(changed_after_uid).kubectl(["create", "namespace", "candidate"], text=True)
        self.assertEqual(len(self.cmd.api()), 1)

    def test_valid_guarded_kind_load(self):
        image = "image:sha256-" + "a" * 64
        self.f.target(self.cmd).kind_load(image)
        self.assertEqual(len(self.cmd.api()), 1)
        self.assertEqual(self.cmd.kind_actions(), [["kind", "load", "docker-image", image, "--name", "aiforall-local-full"]])


class FakePodLogCommands(FakeCommands):
    """Only fake metadata projections and fake log capture; never starts a child."""
    def __init__(self, fixture):
        super().__init__(fixture)
        self.namespace_uid = "owned-namespace"
        self.post_uid = "owned-pod"
        self.post_error = None
        self.log_error = None
        self.log_output = "connection refused\n"
        self.log_options = []
        self.native_result = None
        self.projections = {
            "pod": "owned-pod\napp-abc\naiforall-local-full\napp sidecar\nReplicaSet\tapp-rs\towned-rs\ttrue\n",
            "replicaset": "owned-rs\napp-rs\naiforall-local-full\nDeployment\tapp\towned-deployment\ttrue\n",
            "deployment": "owned-deployment\napp\naiforall-local-full\n",
        }

    def __call__(self, cmd, **kwargs):
        result = super().__call__(cmd, **kwargs)  # Verify effective endpoint/config/context too.
        if cmd[0] != "kubectl":
            return result
        args = cmd[5:]
        if args[0] == "logs":
            self.log_options.append(kwargs)
            if self.log_error:
                raise self.log_error
            if self.native_result is not None:
                with patch.object(anchor.subprocess, "run", return_value=self.native_result):
                    return anchor.run(cmd, **kwargs)
            return self.log_output
        if args[:3] == ["get", "namespace", "aiforall-local-full"]:
            return self.namespace_uid
        if args[:2] == ["get", "pod"] and args[-1] == "jsonpath={.metadata.uid}":
            if self.post_error:
                raise self.post_error
            return self.post_uid
        if args[0] == "get" and args[1] in self.projections:
            return self.projections[args[1]]
        return result

    def log_calls(self):
        return [cmd for cmd in self.api() if cmd[5] == "logs"]


class BoundedPodLogTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        env = patch.dict(os.environ, {}, clear=True)
        env.start()
        self.addCleanup(env.stop)
        self.f = Fixture(self.temp.name)
        self.f.data["namespaceUids"] = {"aiforall-local-full": "owned-namespace"}
        self.f.data["ownedResources"] = [dict(kind="Deployment", name="app", uid="owned-deployment",
            namespace="aiforall-local-full", context="kind-aiforall-local-full", cluster="aiforall-local-full")]
        self.f.save()
        self.cmd = FakePodLogCommands(self.f)
        self.target = self.f.target(self.cmd)
        self.args = ["logs", "app-abc", "-n", "aiforall-local-full", "-c", "app",
                     "--tail=200", "--limit-bytes=32768"]
        self.attestation = dict(namespace_uid="owned-namespace", pod_uid="owned-pod", container="app",
            owner_chain=[dict(kind="ReplicaSet", name="app-rs", uid="owned-rs"),
                         dict(kind="Deployment", name="app", uid="owned-deployment")])

    def request(self, args=None, **changes):
        return self.target.diagnostic_logs(self.args if args is None else args,
                                           **{**self.attestation, **changes})

    def test_logs_accepts_exact_owned_pod_container_and_bounded_previous(self):
        for previous in ([], ["--previous"], ["--previous=true"], ["--previous=false"]):
            with self.subTest(previous=previous):
                self.cmd.calls.clear()
                self.assertEqual(self.request(self.args + previous), ("connection-refused",))
                expected = self.args + (["--previous=" + ("false" if previous == ["--previous=false"] else "true")]
                                        if previous else [])
                self.assertEqual(self.cmd.log_calls(), [self.target.prefix() + expected])
                options = self.cmd.log_options[-1]
                self.assertEqual(options["timeout"], 30)
                self.assertIs(options["text"], True)
                self.assertEqual(options["stdin"], anchor.subprocess.DEVNULL)
                self.assertNotIn("input", options)
                api = [cmd[5:] for cmd in self.cmd.api()]
                log_index = next(i for i, args in enumerate(api) if args[0] == "logs")
                self.assertEqual(api[log_index - 1][:3], ["get", "namespace", "kube-system"])
                self.assertEqual(api[-1], ["get", "pod", "app-abc", "-n", "aiforall-local-full",
                                          "-o", "jsonpath={.metadata.uid}"])
                for args in api:
                    if args[0] == "get" and args[1] in {"pod", "replicaset", "deployment"}:
                        self.assertTrue(args[-1].startswith("jsonpath={.metadata.uid}"))
                        self.assertNotIn(".spec.initContainers", args[-1])
                        self.assertNotIn(".spec.ephemeralContainers", args[-1])
                        self.assertNotIn(".env", args[-1])
                        self.assertNotIn(".args", args[-1])
                        self.assertNotIn(".volumes", args[-1])
        minimal = ["logs", "app-abc", "--namespace=aiforall-local-full", "--container=app",
                   "--tail", "1", "--limit-bytes", "32"]
        self.assertEqual(self.request(minimal), ("connection-refused",))

    def test_logs_rejects_invalid_grammar_before_any_probe(self):
        invalid = [[], ["logs"], ["logs", "-"], ["logs", "pod/app-abc", *self.args[2:]],
                   ["logs", "deployment/app", *self.args[2:]],
                   self.args[:6], self.args[:6] + ["--tail=1"], self.args[:6] + ["--limit-bytes=1"],
                   [*self.args[:3], "foreign", *self.args[4:]],
                   [*self.args[:5], "app.sidecar", *self.args[6:]]]
        for tail in ("0", "201", "-1", "all", "1s", "1.0", ""):
            invalid.append([*self.args[:6], "--tail=" + tail, self.args[7]])
        for size in ("0", "32769", "-1", "999999", "1Ki", ""):
            invalid.append([*self.args[:7], "--limit-bytes=" + size])
        for extra in (["--follow"], ["--follow=false"], ["-f"], ["-l", "app=app"], ["--all-containers"],
                      ["--selector=app=app"], ["--since=1s"], ["--since-time=x"], ["--timestamps"],
                      ["--prefix"], ["--ignore-errors"], ["--pod-running-timeout=1s"], ["--max-log-requests=1"],
                      ["--server=https://127.0.0.1:60124"], ["--context=kind-aiforall-local"],
                      ["--kubeconfig=other"], ["--token=sentinel"], ["--as=user"], ["-sother"], ["-nother"],
                      ["-capp"], ["-n=aiforall-local-full"], ["--tail=1"], ["--limit-bytes=1"],
                      ["--namespace=aiforall-local-full"], ["--container=app"], ["--previous=no"],
                      ["--previous", "false"], ["--previous", "--previous=false"], ["--"], ["--", "anything"],
                      ["-i"], ["--stdin"], ["--unknown"], ["extra-pod"], ["\x00"]):
            invalid.append(self.args + extra)
        for args in invalid:
            with self.subTest(args=args):
                with self.assertRaises(anchor.AnchorError):
                    self.request(args)
                self.assertEqual(self.cmd.calls, [])
        with self.assertRaises(anchor.AnchorError):
            self.target.kubectl(self.args, text=True, input="")
        with self.assertRaises(TypeError):
            self.request(input="not permitted")
        self.assertEqual(self.cmd.calls, [])
        for changes in (dict(namespace_uid="foreign"), dict(container="sidecar"), dict(owner_chain=[])):
            with self.subTest(attestation=changes), self.assertRaises(anchor.AnchorError):
                self.request(**changes)
            self.assertEqual(self.cmd.calls, [])

    def test_logs_rejects_wrong_endpoint_and_stale_cluster_before_log_request(self):
        original = self.f.config["clusters"][0]["cluster"]["server"]
        for endpoint in ("https://127.0.0.1:60124", "https://localhost:60123", "https://user@127.0.0.1:60123"):
            with self.subTest(endpoint=endpoint):
                self.f.config["clusters"][0]["cluster"]["server"] = endpoint
                self.f.write_config()
                self.target.data["kubeconfig_sha256"] = hashlib.sha256(self.f.config_path.read_bytes()).hexdigest()
                self.cmd.calls.clear()
                with self.assertRaises(anchor.AnchorError):
                    self.request()
                self.assertEqual(self.cmd.api(), [])
        self.f.config["clusters"][0]["cluster"]["server"] = original
        self.f.write_config()
        self.target.data["kubeconfig_sha256"] = hashlib.sha256(self.f.config_path.read_bytes()).hexdigest()
        self.cmd.uid = "replacement-cluster"
        self.cmd.calls.clear()
        with self.assertRaises(anchor.AnchorError):
            self.request()
        self.assertEqual(len(self.cmd.api()), 1)
        self.assertEqual(self.cmd.log_calls(), [])
        self.cmd.uid = "owned-system-uid"
        self.cmd.calls.clear()
        self.f.nodes[self.f.ids[0]]["ports"]["6443/tcp"][0]["HostPort"] = "60124"
        with self.assertRaises(anchor.AnchorError):
            self.request()
        self.assertEqual(self.cmd.api(), [])
        self.cmd.calls.clear()
        self.target.data["cluster"] = "aiforall-local"
        with self.assertRaises(anchor.AnchorError):
            self.request()
        self.assertEqual(self.cmd.calls, [])

    def test_logs_require_owned_namespace_pod_uid_and_named_regular_container(self):
        for changes in (dict(namespace_uid="foreign"), dict(pod_uid="foreign"), dict(container="sidecar"),
                        dict(owner_chain=[]), dict(owner_chain=[dict(kind="Secret", name="app", uid="foreign")]),
                        dict(owner_chain=[dict(kind=[], name="app", uid="foreign")])):
            with self.subTest(changes=changes):
                with self.assertRaises(anchor.AnchorError):
                    self.request(**changes)
                self.assertEqual(self.cmd.log_calls(), [])
        mutations = [("namespace", "foreign"), ("pod", self.cmd.projections["pod"].replace("owned-pod", "foreign")),
                     ("pod", self.cmd.projections["pod"].replace("app sidecar", "sidecar")),
                     ("pod", self.cmd.projections["pod"].replace("owned-rs", "foreign")),
                     ("pod", self.cmd.projections["pod"].replace("\ttrue", "\tfalse")),
                     ("replicaset", self.cmd.projections["replicaset"].replace("owned-deployment", "foreign")),
                     ("deployment", self.cmd.projections["deployment"].replace("owned-deployment", "foreign"))]
        for resource, output in mutations:
            with self.subTest(resource=resource, output=output):
                original = self.cmd.namespace_uid if resource == "namespace" else self.cmd.projections[resource]
                if resource == "namespace":
                    self.cmd.namespace_uid = output
                else:
                    self.cmd.projections[resource] = output
                with self.assertRaises(anchor.AnchorError):
                    self.request()
                self.assertEqual(self.cmd.log_calls(), [])
                if resource == "namespace":
                    self.cmd.namespace_uid = original
                else:
                    self.cmd.projections[resource] = original
        for key, value in (("context", "foreign"), ("cluster", "foreign"), ("uid", "foreign"),
                           ("namespace", "foreign"), ("name", "foreign")):
            original = self.target.data["ownedResources"][0][key]
            self.target.data["ownedResources"][0][key] = value
            with self.subTest(key=key), self.assertRaises(anchor.AnchorError):
                self.request()
            self.target.data["ownedResources"][0][key] = original
            self.assertEqual(self.cmd.log_calls(), [])
        self.target.data["ownedResources"] = []
        with self.assertRaises(anchor.AnchorError):
            self.request()
        self.assertEqual(self.cmd.log_calls(), [])

    def test_logs_discard_capture_when_pod_identity_changes(self):
        for uid, error in (("replacement-pod", None), ("", None),
                           ("owned-pod", anchor.AnchorError("private missing-Pod details"))):
            with self.subTest(uid=uid, error=type(error).__name__):
                self.cmd.calls.clear()
                self.cmd.post_uid, self.cmd.post_error = uid, error
                with self.assertRaisesRegex(anchor.AnchorError, "^INCONCLUSIVE:.*redacted$"):
                    self.request()
                self.assertEqual(len(self.cmd.log_calls()), 1)
                self.assertEqual(sum(cmd[-1] == "jsonpath={.metadata.uid}" and cmd[5:7] == ["get", "pod"]
                                     for cmd in self.cmd.api()), 1)

    def test_logs_failures_do_not_echo_sensitive_output(self):
        sentinel = "PRIVATE_SENTINEL JWT password PEM code-state cookie header https://private.invalid/token"
        cases = [dict(log_error=anchor.AnchorError(sentinel)),
                 dict(log_error=anchor.subprocess.TimeoutExpired([sentinel], 30, output=sentinel, stderr=sentinel)),
                 dict(log_output=sentinel), dict(log_output="connection refused " + sentinel * 500),
                 dict(log_output="connection refused\n" * 201), dict(log_output="connection refused\udcff"),
                 dict(native_result=Mock(returncode=1, stdout=sentinel.encode(), stderr=sentinel.encode())),
                 dict(native_result=Mock(returncode=0, stdout=b"connection refused\xff", stderr=sentinel.encode())),
                 dict(native_result=Mock(returncode=0, stdout=b"connection refused", stderr=b"\xff"))]
        for changes in cases:
            with self.subTest(case=tuple(changes)):
                self.cmd.calls.clear()
                self.cmd.log_error = self.cmd.native_result = None
                self.cmd.log_output = "connection refused\n"
                for key, value in changes.items():
                    setattr(self.cmd, key, value)
                stdout, stderr = io.StringIO(), io.StringIO()
                with patch.object(anchor.sys, "stdout", stdout), patch.object(anchor.sys, "stderr", stderr):
                    with self.assertRaises(anchor.AnchorError) as caught:
                        self.request()
                self.assertEqual(str(caught.exception), "INCONCLUSIVE: owned Pod diagnostic unavailable; output redacted")
                self.assertEqual(stdout.getvalue(), "")
                self.assertEqual(stderr.getvalue(), "")
                self.assertNotIn(sentinel, str(caught.exception))
                self.assertEqual(len(self.cmd.log_calls()), 1)
        # Classified captures containing secrets still return only closed labels.
        self.cmd.native_result = None
        self.cmd.log_output = "connection refused " + sentinel
        self.assertEqual(self.request(), ("connection-refused",))

    def test_logs_return_only_closed_startup_categories(self):
        sentinel = ("PRIVATE_SENTINEL password=private JWT=eyJ.private.signature "
                    "PEM=-----BEGIN PRIVATE KEY----- code-state=private cookie=private "
                    "header=Authorization:Bearer-private https://private.invalid/token "
                    "C:/private/config.yaml field=private-field")
        cases = (("no such file or directory", "missing-file"),
                 ("error initializing configuration", "configuration-initialization-failed"),
                 ("missing field", "configuration-missing-field"),
                 ("unknown field", "configuration-unknown-field"),
                 ("invalid type", "configuration-type-mismatch"),
                 ("panicked at", "process-panicked"))
        for motif, label in cases:
            for text in (motif, sentinel + " " + motif + " " + sentinel):
                with self.subTest(motif=motif, with_sentinel=text != motif):
                    self.cmd.log_output = text
                    stdout, stderr = io.StringIO(), io.StringIO()
                    with patch.object(anchor.sys, "stdout", stdout), patch.object(anchor.sys, "stderr", stderr):
                        self.assertEqual(self.request(), (label,))
                    self.assertEqual(stdout.getvalue(), "")
                    self.assertEqual(stderr.getvalue(), "")
        self.cmd.log_output = "\n".join(motif + " " + sentinel for motif, _ in cases)
        self.assertEqual(self.request(), tuple(label for _, label in cases))
        self.cmd.log_output = sentinel
        with self.assertRaisesRegex(anchor.AnchorError, "^INCONCLUSIVE:.*redacted$"):
            self.request()


class FakeEnvoyAdminCommands(FakeCommands):
    """Extends the existing endpoint-aware fake with identity-only projections."""
    def __init__(self, fixture, incarnation):
        super().__init__(fixture)
        self.namespace_uid = "owned-namespace"
        self.pod_reads = 0
        self.change_at = None
        self.change = lambda value: value
        self.projections = {
            "pod": ("owned-pod\nenvoy-mesh-abc-def\naiforall-local-full\nenvoy\n\n\n"
                    + anchor._ENVOY_IMAGE + "\n\n\nS\tenvoy\t" + incarnation["container_id"]
                    + "\t0\t" + incarnation["image_id"] + "\t2026-10-29T01:02:03Z\n"
                    + "R\tReplicaSet\tenvoy-mesh-abc\towned-rs\ttrue\n"),
            "replicaset": "owned-rs\nenvoy-mesh-abc\naiforall-local-full\nR\tDeployment\tenvoy-mesh\towned-deployment\ttrue\n",
            "deployment": "owned-deployment\nenvoy-mesh\naiforall-local-full\n",
        }

    def __call__(self, cmd, **kwargs):
        result = super().__call__(cmd, **kwargs)
        if cmd[0] != "kubectl":
            return result
        args = cmd[5:]
        if args[:3] == ["get", "namespace", "aiforall-local-full"]:
            return self.namespace_uid
        if args[0] == "get" and args[1] in self.projections:
            value = self.projections[args[1]]
            if args[1] == "pod":
                self.pod_reads += 1
                if self.pod_reads == self.change_at:
                    value = self.change(value)
            return value
        return result


class FakeAdminResponse:
    def __init__(self, body, status=200, headers=(), chunk_size=137):
        self.stream = io.BytesIO(body)
        self.status, self.chunk_size = status, chunk_size
        self.headers = Message()
        for key, value in headers:
            self.headers[key] = value
        self.length = None
        self.reads, self.bytes_read, self.closed, self.on_read = [], 0, False, lambda: None

    def read1(self, size):
        self.reads.append(size)
        self.on_read()
        chunk = self.stream.read(min(size, self.chunk_size))
        self.bytes_read += len(chunk)
        return chunk

    def close(self):
        self.closed = True
        self.stream.close()


class EnvoyAdminTests(unittest.TestCase):
    """Five predeclared fake oracles. No subprocess, sockets, cluster or browser."""
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        env = patch.dict(os.environ, {}, clear=True)
        env.start()
        self.addCleanup(env.stop)
        self.f = Fixture(self.temp.name)
        self.f.data["namespaceUids"] = {"aiforall-local-full": "owned-namespace"}
        self.f.data["ownedResources"] = [dict(kind="Deployment", name="envoy-mesh", uid="owned-deployment",
            namespace="aiforall-local-full", context="kind-aiforall-local-full", cluster="aiforall-local-full")]
        self.f.save()
        self.incarnation = dict(container_id="containerd://" + "e" * 64, restart_count=0,
                                image_id="docker.io/envoyproxy/envoy@sha256:" + "f" * 64)
        self.attestation = dict(pod="envoy-mesh-abc-def", namespace_uid="owned-namespace", pod_uid="owned-pod",
            container="envoy", owner_chain=[dict(kind="ReplicaSet", name="envoy-mesh-abc", uid="owned-rs"),
                dict(kind="Deployment", name="envoy-mesh", uid="owned-deployment")], incarnation=self.incarnation)
        self.cmd = FakeEnvoyAdminCommands(self.f, self.incarnation)
        self.target = self.f.target(self.cmd)
        self.marker = self.target.config.parent / ".envoy-admin"
        self.info = {"version": "a" * 40 + "/1.31.2/Clean/RELEASE/BoringSSL", "state": "LIVE",
                     "uptime_current_epoch": "1.123456789s", "uptime_all_epochs": "2s",
                     "command_line_options": {"restart_epoch": 0, "config_path": "PRIVATE-SENTINEL"}}
        self.stats = {"stats": [dict(name=name, value=0) for name in anchor._ENVOY_STATS]}
        self.responses, self.connections, self.requests = [], [], []
        http = patch.object(anchor.http.client, "HTTPConnection", side_effect=self.connection)
        self.http = http.start()
        self.addCleanup(http.stop)
        process = self.process()
        launch = patch.object(anchor.subprocess, "Popen", return_value=process)
        self.launcher = launch.start()
        self.addCleanup(launch.stop)
        acl = patch.object(anchor, "_windows_path_script", return_value="")
        self.acl = acl.start()
        self.addCleanup(acl.stop)
        # Hard tripwires: every production boundary above is faked explicitly.
        for module, name in ((anchor.socket, "create_connection"), (anchor.os, "kill"), (anchor.subprocess, "run")):
            guard = patch.object(module, name, side_effect=AssertionError("unexpected live operation"))
            guard.start()
            self.addCleanup(guard.stop)

    def process(self, stdout=b"Forwarding from 127.0.0.1:49152 -> 9901\n", stderr=b"", blocked=False, exited=False):
        code = [0 if exited else None]
        process = Mock(pid=654321, stdout=io.BytesIO(stdout), stderr=io.BytesIO(stderr))
        process.poll.side_effect = lambda: code[0]
        def stop(*_):
            if not blocked:
                code[0] = 0
        def wait(timeout):
            if blocked:
                raise anchor.subprocess.TimeoutExpired("private-child", timeout)
            code[0] = 0
            return 0
        process.send_signal.side_effect = stop
        process.terminate.side_effect = stop
        process.wait.side_effect = wait
        return process

    def connection(self, host, port, timeout):
        self.assertEqual((host, port), ("127.0.0.1", 49152))
        self.assertGreater(timeout, 0)
        self.assertLessEqual(timeout, 5)
        connection = Mock(sock=Mock())
        connection.getresponse.side_effect = lambda: self.responses.pop(0)
        connection.request.side_effect = lambda *args, **kwargs: self.requests.append((args, kwargs))
        self.connections.append(connection)
        return connection

    def capture(self, **changes):
        self.responses = [FakeAdminResponse(json.dumps(value).encode()) for value in (self.info, self.stats)]
        return self.target.envoy_admin_snapshot(**{**self.attestation, **changes})

    def server_info_body(self, size):
        value = {**self.info, "node": {"opaque": "OPAQUE-SERVER-INFO-V2-SENTINEL"}}
        raw = json.dumps(value, separators=(",", ":")).encode()
        self.assertGreaterEqual(size, len(raw))
        value["node"]["opaque"] += "x" * (size - len(raw))
        raw = json.dumps(value, separators=(",", ":")).encode()
        self.assertEqual(len(raw), size)
        return raw

    def test_envoy_admin_rejects_foreign_target_and_free_requests_before_probe(self):
        invalid = [dict(pod="iam-abc"), dict(pod="pod/envoy-mesh-abc-def"), dict(container="sidecar"),
            dict(namespace_uid="foreign"), dict(pod_uid=""), dict(owner_chain=[]),
            dict(owner_chain=[self.attestation["owner_chain"][0], dict(kind="Deployment", name="iam", uid="owned-deployment")]),
            dict(incarnation={}), dict(incarnation={**self.incarnation, "restart_count": True}),
            dict(incarnation={**self.incarnation, "container_id": "foreign"}),
            dict(incarnation={**self.incarnation, "image_id": "PRIVATE-SENTINEL"})]
        for change in invalid:
            with self.subTest(change=change), self.assertRaisesRegex(anchor.AnchorError, "before any probe"):
                self.capture(**change)
            self.assertEqual(self.cmd.calls, [])
        for change in ({"cluster": "aiforall-local"}, {"context": "foreign"}, {"ownedResources": []}):
            target = self.f.target(self.cmd)
            target.data.update(change)
            with self.subTest(target=change), self.assertRaises(anchor.AnchorError):
                target.envoy_admin_snapshot(**self.attestation)
            self.assertEqual(self.cmd.calls, [])
        for free in (dict(url="http://foreign"), dict(path="/config_dump"), dict(query=".*"),
                     dict(port=9901), dict(bind="0.0.0.0"), dict(callback=lambda: None),
                     dict(namespace="foreign"), dict(method="POST"), dict(launcher=lambda: None)):
            with self.subTest(free=tuple(free)), self.assertRaises(TypeError):
                self.target.envoy_admin_snapshot(**self.attestation, **free)
            self.assertEqual(self.cmd.calls, [])
        self.launcher.assert_not_called()
        self.http.assert_not_called()
        self.assertFalse(self.marker.exists())
        for args in (["port-forward", "pod/envoy-mesh-abc-def", ":9901"], ["logs", "envoy-mesh-abc-def"]):
            with self.assertRaises(anchor.AnchorError):
                anchor.parse_kubectl_operation(args)

    def test_envoy_admin_requires_owned_chain_and_stable_container_incarnation(self):
        positive = self.capture()
        self.assertEqual(positive["status"], "COMPLETE")
        self.assertEqual(positive["identity"]["container_id"], self.incarnation["container_id"])
        self.assertEqual(self.cmd.pod_reads, 4)  # Before launch, each GET, then post-capture.
        self.assertEqual(positive["restart_epoch"], 0)
        mutations = [lambda v: v.replace("owned-pod", "replaced-pod"),
            lambda v: v.replace("owned-rs", "foreign-rs"),
            lambda v: v.replace("envoy\n\n\n", "envoy sidecar\n\n\n"),
            lambda v: v.replace("envoy\n\n\n", "envoy\ninit\n\n"),
            lambda v: v.replace("envoy\n\n\n", "envoy\n\nephemeral\n"),
            lambda v: v.replace(self.incarnation["container_id"], "containerd://" + "1" * 64),
            lambda v: v.replace("\t0\t", "\t1\t"),
            lambda v: v.replace(self.incarnation["image_id"], "sha256:" + "2" * 64),
            lambda v: v.replace("2026-10-29T01:02:03Z", ""),
            lambda v: v.replace(anchor._ENVOY_IMAGE, "envoyproxy/envoy:latest")]
        for stage in (1, 2, 3, 4):
            for mutation in mutations:
                with self.subTest(stage=stage, mutation=mutations.index(mutation)):
                    self.cmd = FakeEnvoyAdminCommands(self.f, self.incarnation)
                    self.target = self.f.target(self.cmd)
                    self.cmd.change_at, self.cmd.change = stage, mutation
                    self.launcher.return_value = self.process()
                    self.launcher.reset_mock()
                    self.http.reset_mock()
                    with self.assertRaisesRegex(anchor.AnchorError, "INCONCLUSIVE"):
                        self.capture()
                    self.assertEqual(self.http.call_count, max(0, stage - 2))
                    self.assertEqual(self.launcher.call_count, 0 if stage == 1 else 1)
                    self.assertFalse(self.marker.exists())
        for kind in ("replicaset", "deployment"):
            self.cmd = FakeEnvoyAdminCommands(self.f, self.incarnation)
            self.target = self.f.target(self.cmd)
            self.cmd.projections[kind] = self.cmd.projections[kind].replace("owned-deployment", "foreign-deployment")
            self.launcher.reset_mock()
            with self.subTest(kind=kind), self.assertRaises(anchor.AnchorError):
                self.capture()
            self.launcher.assert_not_called()
        self.cmd = FakeEnvoyAdminCommands(self.f, self.incarnation)
        self.target = self.f.target(self.cmd)
        self.cmd.namespace_uid = "foreign-namespace"
        with self.assertRaises(anchor.AnchorError):
            self.capture()
        self.launcher.assert_not_called()
        # Wrong endpoint/ACL/hash are rejected before even kube-system UID GET.
        self.cmd.calls.clear()
        self.f.config["clusters"][0]["cluster"]["server"] = "https://127.0.0.1:60124"
        self.f.write_config()
        with self.assertRaises(anchor.AnchorError):
            self.capture()
        self.assertEqual(self.cmd.api(), [])

    def test_envoy_admin_exact_child_mapping_and_closed_gets(self):
        # kubectl v1.36.1 reports the local port, not the remote Admin port.
        self.launcher.return_value = self.process(stdout=(
            b"Forwarding from 127.0.0.1:49152 -> 9901\n"
            b"Handling connection for 49152\nHandling connection for 49152\n"))
        result = self.capture()
        command, options = self.launcher.call_args.args[0], self.launcher.call_args.kwargs
        self.assertEqual(command, self.target.prefix() + ["port-forward", "-n", "aiforall-local-full",
                         "pod/envoy-mesh-abc-def", "--address", "127.0.0.1", ":9901"])
        self.assertEqual(options["env"]["KUBECONFIG"], str(self.target.config))
        self.assertEqual(options["stdin"], anchor.subprocess.DEVNULL)
        self.assertEqual((options["stdout"], options["stderr"], options["bufsize"]),
                         (anchor.subprocess.PIPE, anchor.subprocess.PIPE, 0))
        if os.name == "nt":
            self.acl.assert_called_once()
            script, path = self.acl.call_args.args
            self.assertEqual(path, self.marker)
            self.assertIn("FileSecurity", script)
            self.assertIn("SetAccessRuleProtection($true,$false)", script)
            self.assertNotIn("IsReadOnly", script)
        else:
            self.acl.assert_not_called()
        self.assertEqual(self.requests, [(("GET", path), {"headers": {"Accept": "application/json",
            "Accept-Encoding": "identity", "Connection": "close"}}) for path in ("/server_info", anchor._ENVOY_STATS_PATH)])
        self.assertEqual(result["uptime_current_epoch_ns"], 1123456789)
        self.assertNotIn("PRIVATE-SENTINEL", repr(result))
        self.assertEqual(set(result["stats"]), set(anchor._ENVOY_STATS))
        self.assertTrue(all(value == 0 for value in result["stats"].values()))
        pattern = unquote(anchor._ENVOY_STATS_PATH.split("filter=", 1)[1])
        for name in anchor._ENVOY_STATS:
            self.assertIsNotNone(re.fullmatch(pattern, name))
        for name in ("cluster.iamXssl.handshake", "cluster.other.ssl.handshake", "xcluster.iam.ssl.handshake"):
            self.assertIsNone(re.fullmatch(pattern, name))
        for args in (cmd[5:] for cmd in self.cmd.api()):
            self.assertEqual(args[0], "get")
            self.assertIn(args[1], {"namespace", "pod", "replicaset", "deployment"})
            self.assertTrue(args[-1].startswith("jsonpath="))
            for forbidden in (".env", ".args", ".volumes", "Secret"):
                self.assertNotIn(forbidden, args[-1])
        mappings = [b"Forwarding from 0.0.0.0:49152 -> 9901\n", b"Forwarding from localhost:49152 -> 9901\n",
            b"Forwarding from 127.0.0.1:49152 -> 8080\n", b"Forwarding from 127.0.0.1:0 -> 9901\n",
            b"Forwarding from 127.0.0.1:65536 -> 9901\n",
            b"Forwarding from 127.0.0.1:49152 -> 9901\nForwarding from [::1]:49152 -> 9901\n",
            b"Forwarding from 127.0.0.1:49152 -> 9901\nHandling connection for 9901\n",
            b"Forwarding from 127.0.0.1:49152 -> 9901\nHandling connection for 49153\n",
            b"PRIVATE-SENTINEL\n", b"x" * 5000]
        for output in mappings:
            self.launcher.return_value = self.process(stdout=output)
            self.http.reset_mock()
            with self.subTest(mapping=output[:50]), self.assertRaises(anchor.AnchorError) as error:
                self.capture()
            self.assertNotIn("PRIVATE-SENTINEL", str(error.exception))
            self.http.assert_not_called()
            self.assertFalse(self.marker.exists())

    def test_envoy_admin_server_info_v2_fixed_limit_and_opaque_projection(self):
        for size in (16385, 65536, 131072):
            for headers in ((), (("Content-Length", str(size)),), (("Transfer-Encoding", "chunked"),)):
                with self.subTest(size=size, headers=headers):
                    response = FakeAdminResponse(self.server_info_body(size), headers=headers, chunk_size=4096)
                    self.launcher.return_value = self.process()
                    self.responses = [response, FakeAdminResponse(json.dumps(self.stats).encode())]
                    result = self.target.envoy_admin_snapshot(**self.attestation)
                    self.assertEqual(result["state"], "LIVE")
                    self.assertEqual(result["version"], "1.31.2")
                    self.assertEqual(set(result), {"state", "version", "restart_epoch",
                        "uptime_current_epoch_ns", "uptime_all_epochs_ns", "identity", "stats", "status"})
                    self.assertNotIn("OPAQUE-SERVER-INFO-V2-SENTINEL", repr(result))
                    self.assertNotIn("PRIVATE-SENTINEL", repr(result))
                    self.assertEqual(response.bytes_read, size)
                    self.assertLessEqual(max(response.reads), 4096)
                    self.assertTrue(response.closed)
                    self.assertFalse(self.marker.exists())
                    self.assertIsNotNone(self.launcher.return_value.poll())
                    for connection in self.connections[-2:]:
                        connection.close.assert_called_once_with()
        # The amendment must not change the statistics ceiling.
        raw = json.dumps(self.stats).encode()
        raw += b" " * (65536 - len(raw))
        self.responses = [FakeAdminResponse(raw, headers=(("Transfer-Encoding", "chunked"),))]
        self.assertEqual(anchor._admin_stats_projection(anchor._admin_get(
            49152, True, anchor.time.monotonic() + 30)), dict.fromkeys(anchor._ENVOY_STATS, 0))

    def test_envoy_admin_server_info_v2_overflow_discards_and_closes_without_retry(self):
        for size in (131073, 262144):
            for headers in ((), (("Content-Length", str(size)),), (("Transfer-Encoding", "chunked"),)):
                with self.subTest(size=size, headers=headers):
                    response = FakeAdminResponse(self.server_info_body(size), headers=headers, chunk_size=4096)
                    self.launcher.reset_mock()
                    self.launcher.return_value = self.process()
                    self.http.reset_mock()
                    self.responses = [response]
                    with patch.object(anchor, "_admin_json") as parse:
                        with self.assertRaises(anchor.AnchorError) as error:
                            self.target.envoy_admin_snapshot(**self.attestation)
                        parse.assert_not_called()
                    self.assertNotIn("OPAQUE-SERVER-INFO-V2-SENTINEL", str(error.exception))
                    self.assertNotIn(str(size), str(error.exception))
                    self.assertEqual(response.bytes_read, 131073)
                    self.assertLessEqual(max(response.reads), 4096)
                    self.assertTrue(response.closed)
                    self.http.assert_called_once()
                    self.launcher.assert_called_once()
                    self.connections[-1].close.assert_called_once_with()
                    self.assertIsNotNone(self.launcher.return_value.poll())
                    self.assertFalse(self.marker.exists())

    def test_envoy_admin_limits_and_missing_stats_are_not_zero(self):
        self.stats = {"stats": [{"name": anchor._ENVOY_STATS[0], "value": 0}]}
        result = self.capture()
        self.assertEqual(result["stats"][anchor._ENVOY_STATS[0]], 0)
        self.assertEqual(result["stats"][anchor._ENVOY_STATS[1]], "MISSING")
        self.assertEqual(result["status"], "INCONCLUSIVE")
        self.assertEqual(set(result["stats"].values()), {0, "MISSING"})
        bodies = [(False, b" " * 131072 + b"{}"), (True, b" " * 65536 + b"{}"),
                  (False, b"\xff"), (False, b'{"state":0,"state":1}'), (False, b'{"x":NaN}'),
                  (True, b'{"stats":[],"stats":[]}'), (False, b'{"x":')]
        for stats, body in bodies:
            response = FakeAdminResponse(body, headers=(("Transfer-Encoding", "chunked"),))
            self.responses = [response]
            with self.subTest(body=body[:30]), self.assertRaises((anchor.AnchorError, ValueError)):
                anchor._admin_get(49152, stats, anchor.time.monotonic() + 30)
            self.assertTrue(response.closed)
            self.assertLessEqual(response.bytes_read, (65536 if stats else 131072) + 1)
            self.assertLessEqual(max(response.reads), 4096)  # Never read-all, even for chunked input.
            self.connections[-1].close.assert_called_once_with()
        for status, headers in ((302, (("Location", "http://foreign/PRIVATE-SENTINEL"),)),
                                (500, ()), (200, (("Content-Encoding", "gzip"),)),
                                (200, (("Content-Length", "1"), ("Content-Length", "1")))):
            response = FakeAdminResponse(b"PRIVATE-SENTINEL", status=status, headers=headers)
            self.responses = [response]
            with self.subTest(status=status, headers=headers), self.assertRaises(anchor.AnchorError) as error:
                anchor._admin_get(49152, False, anchor.time.monotonic() + 30)
            self.assertEqual(response.reads, [])
            self.assertTrue(response.closed)
            self.assertNotIn("PRIVATE-SENTINEL", str(error.exception))
        name = anchor._ENVOY_STATS[0]
        for entries in ([dict(name=name, value=-1)], [dict(name=name, value=True)], [dict(name=name, value=1.0)],
                        [dict(name="cluster.other.ssl.handshake", value=0)],
                        [dict(name=name, value=0), dict(name=name, value=1)], [dict(name=name, value=2**64)]):
            with self.subTest(entries=entries), self.assertRaises(anchor.AnchorError):
                anchor._admin_stats_projection({"stats": entries})
        for change in (dict(state="PRIVATE-SENTINEL"), dict(version=self.info["version"].replace("1.31.2", "1.31.1")),
                       dict(command_line_options={}), dict(command_line_options=dict(restart_epoch=True)),
                       dict(uptime_all_epochs="-1s"), dict(uptime_current_epoch="3s")):
            with self.subTest(change=change), self.assertRaises(anchor.AnchorError):
                anchor._admin_server_projection({**self.info, **change})
        # A slow trickle exceeds the whole GET budget, not just each socket read.
        clock = [0]
        response = FakeAdminResponse(b"{}")
        response.on_read = lambda: clock.__setitem__(0, 6)
        self.responses = [response]
        with patch.object(anchor.time, "monotonic", side_effect=lambda: clock[0]), \
                patch.object(anchor.threading, "Timer") as timer:
            with self.assertRaises(anchor.AnchorError):
                anchor._admin_get(49152, False, 30)
            self.assertEqual(timer.call_args.args[0], 5)
            timer.call_args.args[1]()
            self.connections[-1].sock.shutdown.assert_called_once_with(anchor.socket.SHUT_RDWR)
            timer.return_value.cancel.assert_called_once_with()
        self.assertTrue(response.closed)
        response = FakeAdminResponse(b"{}")
        response.length = 10
        self.responses = [response]
        with self.assertRaisesRegex(anchor.AnchorError, "truncated"):
            anchor._admin_get(49152, False, anchor.time.monotonic() + 30)
        self.assertTrue(response.closed)

    def test_envoy_admin_stale_marker_and_cleanup_use_only_owned_handle(self):
        self.marker.write_text('{"pid":99999,"ownership":"stale"}')
        before = self.marker.read_bytes()
        with self.assertRaisesRegex(anchor.AnchorError, "STOP"):
            self.capture()
        self.assertEqual(self.marker.read_bytes(), before)
        self.assertEqual(self.cmd.calls, [])
        self.launcher.assert_not_called()
        self.marker.unlink()
        oauth = self.target.path.with_name(self.target.path.name + ".oauth-forward")
        oauth.write_text("active-parent")
        with self.assertRaises(anchor.AnchorError):
            self.capture()
        self.assertEqual(oauth.read_text(), "active-parent")
        self.assertEqual(self.cmd.calls, [])
        oauth.unlink()
        process = self.process()
        self.launcher.return_value = process
        self.capture()
        stopped = process.send_signal if os.name == "nt" else process.terminate
        stopped.assert_called_once()
        if os.name == "nt":
            stopped.assert_called_once_with(anchor.signal.CTRL_BREAK_EVENT)
            process.terminate.assert_not_called()
        else:
            stopped.assert_called_once_with()
            process.send_signal.assert_not_called()
        process.wait.assert_called_once()
        self.assertLessEqual(process.wait.call_args.kwargs["timeout"], 15)
        self.assertTrue(process.stdout.closed and process.stderr.closed)
        self.assertFalse(self.marker.exists())
        process = self.process()
        self.launcher.return_value = process
        self.info["state"] = "PRIVATE-SENTINEL"
        with self.assertRaises(anchor.AnchorError) as error:
            self.capture()
        self.info["state"] = "LIVE"
        self.assertNotIn("PRIVATE-SENTINEL", str(error.exception))
        process.wait.assert_called_once()
        self.assertTrue(process.stdout.closed and process.stderr.closed)
        self.assertFalse(self.marker.exists())
        for failure in (OSError("PRIVATE-SENTINEL"), RuntimeError("PRIVATE-SENTINEL")):
            self.launcher.side_effect = failure
            with self.subTest(launch=type(failure)), self.assertRaises(anchor.AnchorError) as error:
                self.capture()
            self.assertNotIn("PRIVATE-SENTINEL", str(error.exception))
            self.assertEqual(self.marker.exists(), not isinstance(failure, OSError))
            if self.marker.exists():
                self.marker.unlink()  # Fake-only unknown-launch marker, no actual child.
        self.launcher.side_effect = None
        for output, err, exited in ((b"Forwarding from 127.0.0.1:49152 -> 9901\n", b"PRIVATE-SENTINEL", False),
                                    (b"", b"", True)):
            process = self.process(stdout=output, stderr=err, exited=exited)
            self.launcher.return_value = process
            with self.subTest(startup=(bool(err), exited)), self.assertRaises(anchor.AnchorError):
                self.capture()
            process.wait.assert_called_once()
            self.assertFalse(self.marker.exists())
        # Readiness has an independent five-second budget, with no real wait.
        forward = anchor._EnvoyAdminForward()
        forward.process, forward.deadline, forward.changed = self.process(stdout=b""), 30, Mock()
        with patch.object(anchor.time, "monotonic", side_effect=[0, 0, 6]):
            with self.assertRaises(anchor.AnchorError):
                forward.ready()
        forward.changed.wait.assert_called_once_with(5)
        forward.process.stdout.close()
        forward.process.stderr.close()
        process = self.process()
        self.launcher.return_value = process
        with patch.object(anchor._EnvoyAdminForward, "ready", side_effect=anchor.AnchorError("timeout")):
            with self.assertRaises(anchor.AnchorError):
                self.capture()
        process.wait.assert_called_once()
        self.assertFalse(self.marker.exists())
        # The thirty-second watchdog is independent of metadata/HTTP progress.
        process = self.process()
        self.launcher.return_value = process
        forward = anchor._EnvoyAdminForward()
        with patch.object(anchor.threading, "Timer") as timer:
            timer.return_value.is_alive.return_value = False
            forward.start(self.target.prefix() + ["port-forward", "-n", "aiforall-local-full",
                          "pod/envoy-mesh-abc-def", "--address", "127.0.0.1", ":9901"], self.target.environment())
            self.assertEqual(timer.call_args.args[0], 30)
            timer.call_args.args[1]()
            timer.call_args.args[1]()
            forward.close()
        (process.send_signal if os.name == "nt" else process.terminate).assert_called_once()
        process.wait.assert_called_once()
        # Expiry uses the handle, once; blocked cleanup retains its exclusive marker.
        process = self.process(blocked=True)
        self.launcher.return_value = process
        with self.assertRaisesRegex(anchor.AnchorError, "INCONCLUSIVE"):
            self.capture()
        self.assertTrue(self.marker.exists())
        marker = json.loads(self.marker.read_text())
        self.assertEqual((marker["pid"], marker["ownership"]), (process.pid, "live-child-handle-only"))
        self.launcher.reset_mock()
        with self.assertRaisesRegex(anchor.AnchorError, "STOP"):
            self.capture()
        self.launcher.assert_not_called()
        self.assertEqual(json.loads(self.marker.read_text()), marker)


class FakeTupleCommands(FakeEnvoyAdminCommands):
    """Endpoint-aware identity fake; diagnostic argv are separately asserted."""
    def __init__(self, fixture, incarnation):
        super().__init__(fixture, incarnation)
        self.names = {fixture.ids[0]: "aiforall-local-full-control-plane",
                      fixture.ids[1]: "aiforall-local-full-worker",
                      fixture.ids[2]: "aiforall-local-full-worker2"}
        self.addresses = {container_id: "172.18.0." + str(index + 2)
                          for index, container_id in enumerate(fixture.ids)}
        self.network = (self.names[fixture.ids[1]] + '\n10.244.1.4\n[{"ip":"10.244.1.4"}]\nfalse\n'
            '[{"name":"http","containerPort":10000,"protocol":"TCP"}]\nenvoy-mesh\n')
        self.service = ('owned-service\nenvoy-mesh\naiforall-local-full\nNodePort\n10.96.0.42\n'
            '["10.96.0.42"]\nCluster\nCluster\n{"app.kubernetes.io/name":"envoy-mesh"}\n'
            '[{"name":"http","port":10000,"protocol":"TCP","targetPort":"http","nodePort":30080}]\n')
        self.node = "owned-node\naiforall-local-full-worker\n172.18.0.3\n"
        self.backend = "nf_tables"
        self.diagnostics = []
        self.mutate = lambda cmd, value: value
        self.tuple_rows = ""
        self.chain_packets = self.chain_bytes = 0

    def __call__(self, cmd, **kwargs):
        if cmd[:2] == ["docker", "exec"]:
            self.calls.append(cmd)
            self.diagnostics.append(cmd)
            if cmd[3:] == ["conntrack", "--version"]:
                value = "conntrack v1.4.7 (conntrack-tools)\n"
            elif cmd[3:] == ["iptables", "--version"]:
                value = "iptables v1.8.11 (" + self.backend + ")\n"
            elif cmd[3:] in (["test", "-d", "/sys/module/nf_conntrack"],
                               ["test", "-d", "/sys/module/nf_conntrack_netlink"]):
                value = ""
            elif cmd[3:5] == ["conntrack", "-L"]:
                value = (self.tuple_rows, "conntrack v1.4.7 (conntrack-tools): "
                         + str(len(self.tuple_rows.splitlines())) + " flow entries have been shown.\n")
            elif cmd[3:10] == ["ip", "-j", "-4", "route", "show", "exact", "10.244.1.4/32"]:
                value = json.dumps([{"dst": "10.244.1.4", "scope": "link", "dev": "cali123456789ab"}])
            elif cmd[3:] == ["iptables", "-t", "filter", "-L", "cali-tw-cali123456789ab",
                             "-n", "-v", "-x", "--line-numbers"]:
                value = ("Chain cali-tw-cali123456789ab (1 references)\n"
                    "num pkts bytes target prot opt in out source destination\n"
                    "1 0 0 cali-pi-abc all -- * * 0.0.0.0/0 0.0.0.0/0 mark match 0x0/0x20000\n"
                    + f"2 {self.chain_packets} {self.chain_bytes} DROP all -- * * 0.0.0.0/0 0.0.0.0/0 "
                    "/* cali:opaque */ /* Drop if no policies passed packet */ mark match 0x0/0x20000\n")
            else:
                raise AssertionError("unexpected diagnostic argv")
        elif cmd[:2] == ["docker", "inspect"] and cmd[3] != anchor.METADATA:
            self.calls.append(cmd)
            name = self.names[cmd[-1]]
            value = json.dumps(dict(id=cmd[-1], hostname=name, name="/" + name, ipv4=self.addresses[cmd[-1]]))
        else:
            value = super().__call__(cmd, **kwargs)
            if cmd[0] == "kubectl":
                args = cmd[5:]
                if args[:2] == ["get", "pod"] and "{.spec.nodeName}" in cmd[-1]:
                    value = ("owned-pod\nenvoy-mesh-abc-def\naiforall-local-full\n" + self.network
                             + "2026-10-29T01:02:03Z\n")
                elif args[:2] == ["get", "node"]:
                    value = ("owned-cp-node\n" + self.names[self.fixture.ids[0]] + "\n" + self.addresses[self.fixture.ids[0]] + "\n"
                             if args[2] == self.names[self.fixture.ids[0]] else self.node)
                elif args[:2] == ["get", "service"]:
                    value = self.service
        return self.mutate(cmd, value)


class FakeTuplePipe(io.BytesIO):
    def __init__(self, value, broken=False):
        super().__init__(value)
        self.bytes_read, self.read_sizes, self.broken = 0, [], broken

    def read(self, size):
        self.read_sizes.append(size)
        if self.broken:
            raise OSError("PRIVATE-TUPLE-SENTINEL")
        value = super().read(size)
        self.bytes_read += len(value)
        return value


class EnvoyTuplePreflightTests(unittest.TestCase):
    """The two predeclared tooling-only oracles; no native command/network."""
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        env = patch.dict(os.environ, {}, clear=True)
        env.start()
        self.addCleanup(env.stop)
        self.f = Fixture(self.temp.name)
        self.f.data["namespaceUids"] = {"aiforall-local-full": "owned-namespace"}
        self.f.data["ownedResources"] = [dict(kind=kind, name="envoy-mesh", uid=uid,
            namespace="aiforall-local-full", context="kind-aiforall-local-full", cluster="aiforall-local-full")
            for kind, uid in (("Deployment", "owned-deployment"), ("Service", "owned-service"))]
        self.f.save()
        self.incarnation = dict(container_id="containerd://" + "e" * 64, restart_count=0,
                                image_id="docker.io/envoyproxy/envoy@sha256:" + "f" * 64)
        self.attestation = dict(pod="envoy-mesh-abc-def", namespace_uid="owned-namespace", pod_uid="owned-pod",
            container="envoy", owner_chain=[dict(kind="ReplicaSet", name="envoy-mesh-abc", uid="owned-rs"),
                dict(kind="Deployment", name="envoy-mesh", uid="owned-deployment")], incarnation=self.incarnation,
            node_uid="owned-node", service_uid="owned-service")
        self.cmd = FakeTupleCommands(self.f, self.incarnation)
        self.target = self.f.target(self.cmd)
        self.children = []
        self.streams = None
        self.blocked = self.ignore_stop = self.broken_pipe = False
        self.exit_status = lambda command: 0
        self.acl_result = "yes\n"
        self.on_launch = lambda: None
        launcher = patch.object(anchor.subprocess, "Popen", side_effect=self.launch)
        self.launcher = launcher.start()
        self.addCleanup(launcher.stop)
        for module, name in ((anchor.subprocess, "run"), (anchor.socket, "create_connection"),
                             (anchor.http.client, "HTTPConnection"), (anchor.os, "kill")):
            guard = patch.object(module, name, side_effect=AssertionError("unexpected live operation"))
            guard.start()
            self.addCleanup(guard.stop)

    def launch(self, command, **kwargs):
        self.assertEqual(kwargs["stdin"], anchor.subprocess.DEVNULL)
        self.assertEqual(kwargs["stdout"], anchor.subprocess.PIPE)
        self.assertEqual(kwargs["stderr"], anchor.subprocess.PIPE)
        self.assertEqual(kwargs["bufsize"], 0)
        self.assertNotIn("shell", kwargs)
        self.on_launch()
        if self.streams is None:
            # Existing fake verifies endpoint/context and supplies only projections.
            value = self.acl_result if command[0] == "powershell.exe" else self.cmd(command, **kwargs)
            if isinstance(value, tuple):
                out, err = (part.encode() for part in value)
            else:
                out, err = value.encode() if isinstance(value, str) else value, b""
        else:
            out, err = self.streams
        process = Mock(stdout=FakeTuplePipe(out, self.broken_pipe), stderr=FakeTuplePipe(err))
        state = [None if self.blocked else self.exit_status(command)]
        process.poll.side_effect = lambda: state[0]
        def stop(*_):
            if not self.ignore_stop:
                state[0] = -15
        def kill():
            state[0] = -9
        def wait(timeout):
            self.assertGreaterEqual(timeout, 0)
            self.assertLessEqual(timeout, 5)
            if state[0] is None:
                raise anchor.subprocess.TimeoutExpired("PRIVATE-TUPLE-SENTINEL", timeout)
            return state[0]
        process.send_signal.side_effect = stop
        process.terminate.side_effect = stop
        process.kill.side_effect = kill
        process.wait.side_effect = wait
        self.children.append(process)
        return process

    def capture(self, **changes):
        return self.target.envoy_tuple_tooling_preflight(**{**self.attestation, **changes})

    def assert_closed(self):
        for process in self.children:
            self.assertIsNotNone(process.poll())
            self.assertTrue(process.stdout.closed)
            self.assertTrue(process.stderr.closed)
            process.wait.assert_called()

    def assert_opaque_failure(self, operation):
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch("sys.stdout", stdout), patch("sys.stderr", stderr):
            with self.assertRaises(anchor.AnchorError) as caught:
                operation()
        self.assertNotIn("PRIVATE-TUPLE-SENTINEL", str(caught.exception) + stdout.getvalue() + stderr.getvalue())
        self.assertIsNone(caught.exception.__cause__)
        self.assert_closed()

    def test_envoy_tuple_preflight_rejects_missing_tools_modules_and_foreign_nodes_before_dump(self):
        expected = [["docker", "exec", self.f.ids[0], "conntrack", "--version"],
            ["docker", "exec", self.f.ids[1], "iptables", "--version"],
            ["docker", "exec", self.f.ids[0], "test", "-d", "/sys/module/nf_conntrack"],
            ["docker", "exec", self.f.ids[0], "test", "-d", "/sys/module/nf_conntrack_netlink"]]
        for backend in ("nf_tables", "legacy"):
            with self.subTest(backend=backend):
                self.cmd.backend = backend
                self.cmd.mutate = lambda cmd, value: value.replace("\n", "\r\n") if backend == "legacy" else value
                self.cmd.diagnostics.clear()
                result = self.capture()
                self.assertEqual(self.cmd.diagnostics, expected)
                self.assertEqual(result["status"], "TOOLING_PREFLIGHT_ONLY")
                self.assertEqual(result["iptables_backend"], backend)
                self.assertEqual(result["conntrack_version"], "1.4.7")
                self.assertEqual(result["identity"]["worker_container_id"], self.f.ids[1])
                self.assertEqual(result["identity"]["node_uid"], "owned-node")
                self.assertEqual(result["identity"]["node_internal_ipv4"], "172.18.0.3")
                self.assertEqual(result["identity"]["pod_ipv4"], "10.244.1.4")
                self.assertNotIn("NAT_BINDING", json.dumps(result))
                self.assert_closed()
        self.cmd.mutate = lambda cmd, value: value
        # Invalid caller grammar/attestation: not even an anchor/identity process.
        invalid = [dict(pod="foreign"), dict(container="sidecar"), dict(namespace_uid="foreign"),
            dict(node_uid=[]), dict(service_uid=""), dict(owner_chain=[]), dict(incarnation={}),
            dict(incarnation={**self.incarnation, "restart_count": True})]
        for changes in invalid:
            with self.subTest(invalid=changes):
                self.launcher.reset_mock()
                self.assert_opaque_failure(lambda: self.capture(**changes))
                self.launcher.assert_not_called()
        for free in (dict(argv=["conntrack", "-L"]), dict(worker_id=self.f.ids[1]),
                     dict(control_plane_id=self.f.ids[0]), dict(table="filter"), dict(filter="tcp"),
                     dict(callback=lambda: None)):
            with self.subTest(free=tuple(free)):
                self.launcher.reset_mock()
                with self.assertRaises(TypeError):
                    self.target.envoy_tuple_tooling_preflight(**self.attestation, **free)
                self.launcher.assert_not_called()
        self.target.data["cluster"] = "aiforall-local"
        self.launcher.reset_mock()
        self.assert_opaque_failure(self.capture)
        self.launcher.assert_not_called()
        self.target.data["cluster"] = "aiforall-local-full"
        resources = self.target.data["ownedResources"]
        self.target.data["ownedResources"] = resources[:1]
        self.launcher.reset_mock()
        self.assert_opaque_failure(self.capture)
        self.launcher.assert_not_called()
        self.target.data["ownedResources"] = resources
        # Missing binary, permission refusal and either missing resident module:
        # no later command, no dump/netlink/install/fallback is attempted.
        for failure in range(1, 5):
            with self.subTest(missing_or_refused=failure):
                self.cmd.diagnostics.clear()
                self.exit_status = lambda cmd: (127 if failure <= 2 else 1) if cmd == expected[failure - 1] else 0
                self.assert_opaque_failure(self.capture)
                self.assertEqual(self.cmd.diagnostics, expected[:failure])
        self.exit_status = lambda command: 0
        for output in ("conntrack v1.4.8 (conntrack-tools)\n", "1.4.7\n",
                       "conntrack v1.4.7 (conntrack-tools)\nPRIVATE-TUPLE-SENTINEL"):
            with self.subTest(conntrack=output):
                self.cmd.diagnostics.clear()
                self.cmd.mutate = lambda cmd, value: output if cmd[3:] == ["conntrack", "--version"] else value
                self.assert_opaque_failure(self.capture)
                self.assertEqual(self.cmd.diagnostics, expected[:1])
        self.cmd.mutate = lambda cmd, value: value
        self.cmd.backend = "unknown"
        self.cmd.diagnostics.clear()
        self.assert_opaque_failure(self.capture)
        self.assertEqual(self.cmd.diagnostics, expected[:2])
        self.cmd.backend = "nf_tables"
        network, node = self.cmd.network, self.cmd.node
        for name in ("aiforall-local-full-control-plane", "foreign-worker"):
            with self.subTest(foreign_or_cp_node=name):
                self.cmd.network = network.replace("aiforall-local-full-worker", name)
                self.cmd.node = "owned-node\n" + name + "\n172.18.0.2\n"
                self.cmd.diagnostics.clear()
                self.assert_opaque_failure(self.capture)
                self.assertEqual(self.cmd.diagnostics, [])
        self.cmd.network, self.cmd.node = network, node
        other_name = self.cmd.names[self.f.ids[2]]
        other_address = self.cmd.addresses[self.f.ids[2]]
        self.cmd.names[self.f.ids[2]] = self.cmd.names[self.f.ids[1]]
        self.cmd.addresses[self.f.ids[2]] = self.cmd.addresses[self.f.ids[1]]
        self.cmd.diagnostics.clear()
        self.assert_opaque_failure(self.capture)
        self.assertEqual(self.cmd.diagnostics, [])
        self.cmd.names[self.f.ids[2]] = other_name
        self.cmd.addresses[self.f.ids[2]] = other_address
        # Identity projection errors before diagnostic exec, and the SAME changes
        # after all four execs. None may return a stale successful attestation.
        mutations = [
            lambda cmd, v: v.replace("10.244.1.4", "10.244.1.5") if cmd[5:7] == ["get", "pod"] else v,
            lambda cmd, v: v.replace("owned-node", "foreign-node") if cmd[5:7] == ["get", "node"] else v,
            lambda cmd, v: v.replace("172.18.0.3", "172.18.0.99") if cmd[5:7] == ["get", "node"] else v,
            lambda cmd, v: v + "172.18.0.99\n" if cmd[5:7] == ["get", "node"] else v,
            lambda cmd, v: v.replace("owned-service", "foreign-service") if cmd[5:7] == ["get", "service"] else v,
            lambda cmd, v: v.replace('"nodePort":30080', '"nodePort":30081') if cmd[5:7] == ["get", "service"] else v,
            lambda cmd, v: v.replace("Cluster", "Local") if cmd[5:7] == ["get", "service"] else v,
            lambda cmd, v: v.replace("owned-deployment", "foreign-deployment") if cmd[5:7] == ["get", "replicaset"] else v,
            lambda cmd, v: v.replace("owned-deployment", "foreign-deployment") if cmd[5:7] == ["get", "deployment"] else v,
            lambda cmd, v: v.replace(self.incarnation["container_id"], "containerd://" + "1" * 64) if cmd[5:7] == ["get", "pod"] else v,
            lambda cmd, v: v.replace("\t0\t", "\t1\t") if cmd[5:7] == ["get", "pod"] else v,
            lambda cmd, v: v.replace(self.incarnation["image_id"], "sha256:" + "2" * 64) if cmd[5:7] == ["get", "pod"] else v,
            lambda cmd, v: v.replace('"hostname": "aiforall-local-full-worker"', '"hostname": "foreign-worker"') if cmd[:2] == ["docker", "inspect"] else v,
            lambda cmd, v: v.replace('"ipv4": "172.18.0.3"', '"ipv4": "172.18.0.99"') if cmd[:2] == ["docker", "inspect"] else v,
            lambda cmd, v: v.replace('"ipv4": "172.18.0.3"', '"ipv4": null') if cmd[:2] == ["docker", "inspect"] else v,
            lambda cmd, v: v.replace('"running": true', '"running": false') if cmd[:2] == ["docker", "inspect"] else v,
            lambda cmd, v: v.replace('"HostPort": "18080"', '"HostPort": "18081"') if cmd[:2] == ["docker", "inspect"] else v,
            lambda cmd, v: v + " " + "1" * 64 if cmd[:2] == ["docker", "ps"] else v,
            lambda cmd, v: v.replace('"ip":"10.244.1.4"', '"ip":"10.244.1.4"},{"ip":"10.244.1.6"') if cmd[5:7] == ["get", "pod"] else v,
            lambda cmd, v: v + "PRIVATE-TUPLE-SENTINEL\n" if cmd[5:7] == ["get", "service"] else v,
            lambda cmd, v: v.replace('"nodePort":30080', '"nodePort":30080,"nodePort":30080') if cmd[5:7] == ["get", "service"] else v,
        ]
        for mutation in mutations:
            # A different but valid Pod IP is permitted initially, never as drift.
            for post in ((True,) if mutation is mutations[0] else (False, True)):
                with self.subTest(mutation=mutations.index(mutation), post=post):
                    self.cmd.diagnostics.clear()
                    self.cmd.mutate = lambda cmd, value: mutation(cmd, value) if not post or len(self.cmd.diagnostics) == 4 else value
                    self.assert_opaque_failure(self.capture)
                    self.assertEqual(self.cmd.diagnostics, expected if post else [])
        self.cmd.mutate = lambda cmd, value: value
        self.f.data["control_plane_container_id"] = "1" * 64
        self.f.save()
        self.cmd.diagnostics.clear()
        self.assert_opaque_failure(self.capture)
        self.assertEqual(self.cmd.diagnostics, [])

    def test_envoy_tuple_bounds_and_failures_never_echo_private_output(self):
        # Exercise the actual byte reader, not a fake pre-truncated run() result.
        for size in (16384, 16385, 200000):
            for stream in (0, 1):
                with self.subTest(size=size, stream=stream):
                    self.children.clear()
                    prefix = b"PRIVATE-TUPLE-SENTINEL"
                    value = prefix + b"x" * (size - len(prefix))
                    self.streams = (value, b"") if stream == 0 else (b"", value)
                    child = anchor._EnvoyTupleChild(anchor.time.monotonic() + 30)
                    if size == 16384 and stream == 0:
                        self.assertEqual(len(child.run(["private-fake-command"])), 16384)
                        self.assert_closed()
                    else:
                        self.assert_opaque_failure(lambda: child.run(["private-fake-command"]))
                    pipe = (self.children[0].stdout, self.children[0].stderr)[stream]
                    self.assertLessEqual(pipe.bytes_read, 16385)
                    self.assertTrue(all(0 < count <= 1024 for count in pipe.read_sizes))
        for streams in ((b"\xffPRIVATE-TUPLE-SENTINEL", b""), (b"", b"\xffPRIVATE-TUPLE-SENTINEL")):
            self.streams = streams
            self.assert_opaque_failure(self.capture)
        self.streams = (b"PRIVATE-TUPLE-SENTINEL", b"")
        self.broken_pipe = True
        self.assert_opaque_failure(self.capture)
        self.broken_pipe = False
        # Native spawn refusal: never consult another runner after rejection.
        with patch.object(anchor.subprocess, "Popen", side_effect=PermissionError("PRIVATE-TUPLE-SENTINEL")) as refused:
            self.assert_opaque_failure(self.capture)
            refused.assert_called_once()
        # Command timeout, including ignored graceful stop -> kill ONLY this child.
        for ignored in (False, True):
            self.children.clear()
            self.blocked, self.ignore_stop = True, ignored
            clock = [0.0]
            def ticking():
                clock[0] += 0.05
                return clock[0]
            with patch.object(anchor.time, "monotonic", side_effect=ticking):
                self.assert_opaque_failure(self.capture)
            self.assertLessEqual(clock[0], 5.1)
            self.assertEqual(len(self.children), 1)
            self.assertEqual(self.children[0].kill.called, ignored)
        self.blocked = self.ignore_stop = False
        self.streams = None
        # Overall snapshot deadline applies even if EACH process exits promptly.
        clock = [0.0]
        self.on_launch = lambda: clock.__setitem__(0, clock[0] + 1)
        with patch.object(anchor.time, "monotonic", side_effect=lambda: clock[0]):
            self.assert_opaque_failure(self.capture)
        self.assertLessEqual(clock[0], 30)
        self.on_launch = lambda: None
        if os.name == "nt":
            # The default Owner/Access guard must use the SAME bounded child,
            # never escape to the existing generic 30s PowerShell runner.
            privacy = self.target.privacy
            self.target.privacy = anchor.private_file
            self.cmd.mutate = lambda cmd, value: value
            self.assertEqual(self.capture()["status"], "TOOLING_PREFLIGHT_ONLY")
            self.assert_closed()
            for refused in ("no\n", "PRIVATE-TUPLE-SENTINEL", b"\xffPRIVATE-TUPLE-SENTINEL", "x" * 16385):
                self.acl_result = refused
                self.cmd.diagnostics.clear()
                self.assert_opaque_failure(self.capture)
                self.assertEqual(self.cmd.diagnostics, [])
            self.acl_result = "yes\n"
            self.target.privacy = privacy
        # Opaque unknown projection data must never appear in a successful result.
        self.cmd.mutate = lambda cmd, value: value + "PRIVATE-TUPLE-SENTINEL" if cmd[:2] == ["docker", "inspect"] else value
        self.assert_opaque_failure(self.capture)


    def test_envoy_tuple_reads_have_fixed_bounds_and_closed_stderr_summaries(self):
        for mode, limit in ((None, 16384), ("conntrack", 16384), ("route", 16384), ("chain", 65536)):
            for overflow in (False, True):
                with self.subTest(mode=mode, overflow=overflow):
                    payload = b"x" * (limit - 1 + int(overflow)) + b"\n"
                    summary = b"conntrack v1.4.7 (conntrack-tools): 1 flow entries have been shown.\n" if mode == "conntrack" else b""
                    self.streams = (payload, summary)
                    child = anchor._EnvoyTupleChild(anchor.time.monotonic() + 30)
                    if overflow:
                        self.assert_opaque_failure(lambda: child.run(["private-fake-command"], _tuple_reading=mode))
                    else:
                        self.assertEqual(len(child.run(["private-fake-command"], _tuple_reading=mode)), limit)
                    self.assertEqual(self.children[-1].stdout.bytes_read, limit + int(overflow))
                    self.assertLessEqual(max(self.children[-1].stdout.read_sizes), 1024)
        for output, error in [(b"", b"PRIVATE-TUPLE-SENTINEL"),
                              (b"", b"conntrack v1.4.8 (conntrack-tools): 0 flow entries have been shown.\n"),
                              (b"one\n", b"conntrack v1.4.7 (conntrack-tools): 0 flow entries have been shown.\n"),
                              (b"", b"conntrack v1.4.7 (conntrack-tools): 9 flow entries have been shown.\n")]:
            self.streams = (output, error)
            child = anchor._EnvoyTupleChild(anchor.time.monotonic() + 30)
            self.assert_opaque_failure(lambda: child.run(["private-fake-command"], _tuple_reading="conntrack"))
        self.streams = (b"", b"conntrack v1.4.7 (conntrack-tools): 0 flow entries have been shown.\n")
        child = anchor._EnvoyTupleChild(anchor.time.monotonic() + 30)
        self.assertEqual(child.run(["private-fake-command"], _tuple_reading="conntrack"), "")
        self.assert_opaque_failure(lambda: anchor._tuple_rows(
            "ipv4 2 tcp 6 100 SYN_SENT src=PRIVATE-TUPLE-SENTINEL dst=172.18.0.2 sport=1 dport=30080 "
            "[UNREPLIED] src=10.244.1.4 dst=172.18.0.2 sport=10000 dport=1 zone=0 id=1", "172.18.0.2", "10.244.1.4", source_filtered=True))
        self.assert_closed()

    def test_envoy_tuple_observation_is_single_use_and_never_accepts_http_callbacks(self):
        for extra in ({"url": "PRIVATE-URL"}, {"callback": "PRIVATE-CALLBACK"}, {"container": "foreign"},
                      {"source_filtered": True}):
            target = self.f.target(self.cmd)
            self.cmd.diagnostics.clear()
            self.assert_opaque_failure(lambda: target.envoy_tuple_begin(**{**self.attestation, **extra}))
            self.assertEqual(self.cmd.diagnostics, [])
        self.assert_opaque_failure(self.target.envoy_tuple_finish)
        baseline = self.target.envoy_tuple_begin(**self.attestation)
        self.assertEqual(baseline["status"], "BASELINE_ONLY")
        self.assertNotIn("rows", baseline)
        self.assertNotIn("original", repr(baseline))
        baseline["identity"]["pod_uid"] = "caller-mutated"
        baseline["deny"]["packets"] = 999
        self.assertEqual(len(self.cmd.diagnostics), 7)
        cp, worker = self.f.ids[0], self.f.ids[1]
        self.assertEqual(self.cmd.diagnostics[-3:], [
            ["docker", "exec", cp, "conntrack", "-L", "-f", "ipv4", "-p", "tcp",
             "--orig-dst", "172.18.0.2", "--orig-port-dst", "30080", "--reply-src", "10.244.1.4",
             "--reply-port-src", "10000", "-w", "0", "-o", "extended,id"],
            ["docker", "exec", worker, "ip", "-j", "-4", "route", "show", "exact", "10.244.1.4/32"],
            ["docker", "exec", worker, "iptables", "-t", "filter", "-L", "cali-tw-cali123456789ab",
             "-n", "-v", "-x", "--line-numbers"]])
        self.attestation["incarnation"]["restart_count"] = 2
        self.cmd.tuple_rows = ("ipv4 2 tcp 6 100 SYN_SENT src=172.18.0.1 dst=172.18.0.2 sport=45678 dport=30080 "
            "[UNREPLIED] src=10.244.1.4 dst=172.18.0.2 sport=10000 dport=45678 mark=0 zone=0 use=1 id=101\n")
        self.cmd.chain_packets, self.cmd.chain_bytes = 3, 180
        result = self.target.envoy_tuple_finish()
        self.assertEqual(result["endpoint_deny_delta"], {"packets": 3, "bytes": 180})
        self.assertEqual(result["identity"]["restart_count"], 0)
        self.assertEqual(result["packet_attribution"], "NOT_ESTABLISHED_BY_COUNTER")
        self.assertNotIn("_tuple_pending", vars(self.target))
        count = len(self.cmd.diagnostics)
        self.assert_opaque_failure(self.target.envoy_tuple_finish)
        self.assert_opaque_failure(lambda: self.target.envoy_tuple_begin(**self.attestation))
        self.assertEqual(len(self.cmd.diagnostics), count)
        self.assert_closed()
        target = self.f.target(self.cmd)
        before = {"identity": {"pod_uid": "owned-pod"}, "chain": {"fingerprint": "fixed", "deny": "MISSING"},
                  "captured_ns": anchor.time.monotonic_ns()}
        with patch.object(target, "_envoy_tuple_capture", side_effect=[before, anchor.AnchorError("PRIVATE-TUPLE-SENTINEL")]) as capture:
            target.envoy_tuple_begin(**self.attestation)
            self.assert_opaque_failure(target.envoy_tuple_finish)
            self.assert_opaque_failure(target.envoy_tuple_finish)
            self.assertEqual(capture.call_count, 2)
            self.assertNotIn("_tuple_pending", vars(target))
        target = self.f.target(self.cmd)
        before["captured_ns"] = anchor.time.monotonic_ns() - 70_000_000_000
        with patch.object(target, "_envoy_tuple_capture", return_value=before) as capture:
            target.envoy_tuple_begin(**self.attestation)
            self.assert_opaque_failure(target.envoy_tuple_finish)
            self.assertEqual(capture.call_count, 1)
            self.assertNotIn("_tuple_pending", vars(target))


class EnvoyTupleDecoderTests(unittest.TestCase):
    cp = "172.19.0.5"
    pod = "10.244.75.206"
    interface = "cali123456789ab"

    def line(self, id=101, port=45678):
        return (f"ipv4 2 tcp 6 100 SYN_SENT src=172.19.0.1 dst={self.cp} sport={port} dport=30080 "
                f"[UNREPLIED] src={self.pod} dst={self.cp} sport=10000 dport={port} mark=0 zone=0 use=1 id={id}\n")

    def chain(self, packets=0, size=0, target="DROP", direction="tw"):
        return (f"Chain cali-{direction}-{self.interface} (1 references)\n"
                "num pkts bytes target prot opt in out source destination\n"
                "1 0 0 cali-pi-abc all -- * * 0.0.0.0/0 0.0.0.0/0 mark match 0x0/0x20000\n"
                f"2 {packets} {size} {target} all -- * * 0.0.0.0/0 0.0.0.0/0 "
                f"/* cali:opaqueRuleId */ /* {target} if no policies passed packet */ mark match 0x0/0x20000\n")

    def sample(self, rows="", packets=0, size=0, timestamp=1):
        return {"identity": {"pod_uid": "owned-pod"}, "interface": self.interface,
                "backend": "nf_tables", "versions": ("1.4.7", "1.8.9"),
                "chain": anchor._tuple_deny(self.chain(packets, size), self.interface),
                "rows": anchor._tuple_rows(rows, self.cp, self.pod, source_filtered=True), "captured_ns": timestamp}

    def test_envoy_tuple_queries_are_closed_and_original_reply_nat_mapping_is_exact(self):
        row, = anchor._tuple_rows(self.line(), self.cp, self.pod, source_filtered=True)
        self.assertEqual(row["original"], {"src": "172.19.0.1", "dst": self.cp, "sport": 45678, "dport": 30080})
        self.assertEqual(row["reply"], {"src": self.pod, "dst": self.cp, "sport": 10000, "dport": 45678})
        self.assertNotEqual(row["original"]["src"], row["reply"]["dst"])
        self.assertTrue(row["unreplied"])
        self.assertNotIn("reply_observed", row)
        for old, new in [("dport=30080", "dport=30081"), ("sport=10000", "sport=9901"),
                         ("zone=0", "zone=1"), ("id=101", "id=101 id=102"),
                         ("ipv4 2 tcp 6", "ipv4 2 udp 17"), ("100 SYN_SENT", "0 SYN_SENT")]:
            with self.subTest(replacement=new), self.assertRaises(anchor.AnchorError):
                anchor._tuple_rows(self.line().replace(old, new), self.cp, self.pod, source_filtered=True)
        with self.assertRaises(anchor.AnchorError):
            anchor._tuple_rows(self.line() * 2, self.cp, self.pod, source_filtered=True)
        with self.assertRaises(anchor.AnchorError):
            anchor._tuple_rows(" " * 16385, self.cp, self.pod, source_filtered=True)

    def test_envoy_tuple_zone_missing_preserves_source_filtered_provenance(self):
        absent, = anchor._tuple_rows(self.line().replace("zone=0 ", ""), self.cp, self.pod, source_filtered=True)
        explicit, = anchor._tuple_rows(self.line(), self.cp, self.pod, source_filtered=True)
        self.assertEqual(absent["requested_zone"], 0)
        self.assertEqual(absent["observed_zone"], "MISSING")
        self.assertEqual(absent["zone_evidence"], "SOURCE_FILTERED")
        self.assertEqual(explicit["observed_zone"], 0)
        self.assertEqual(explicit["zone_evidence"], "OBSERVED_AND_SOURCE_FILTERED")
        self.assertEqual(absent["fingerprint"], explicit["fingerprint"])
        self.assertNotIn("zone", absent)

    def test_envoy_tuple_zone_conflicts_and_unattested_filter_fail_closed(self):
        for token in ("zone=1", "zone=00", "zone=0 zone=0", "zone-orig=0", "zone-reply=0", "zone-unknown=0"):
            with self.subTest(token=token), self.assertRaises(anchor.AnchorError):
                anchor._tuple_rows(self.line().replace("zone=0", token), self.cp, self.pod, source_filtered=True)
        for proof in (False, None, 1, "conntrack1.4.7"):
            with self.subTest(proof_type=type(proof).__name__), self.assertRaises(anchor.AnchorError):
                anchor._tuple_rows(self.line(), self.cp, self.pod, source_filtered=proof)

    def test_envoy_tuple_requires_unique_new_candidate_and_stable_identity(self):
        before, after = self.sample(), self.sample(self.line(), 3, 180, 20)
        result = anchor._tuple_correlate(before, after)
        self.assertEqual(result["endpoint_deny_delta"], {"packets": 3, "bytes": 180})
        self.assertEqual(result["nat_binding"]["reply"]["dst"], self.cp)
        self.assertEqual(result["status"], "PARENT_CORRELATION_REQUIRED")
        self.assertEqual(result["packet_attribution"], "NOT_ESTABLISHED_BY_COUNTER")
        for sample in [self.sample(timestamp=20),
                       self.sample(self.line() + self.line(102, 45679), timestamp=20),
                       self.sample(self.line(), timestamp=60_000_000_002)]:
            with self.assertRaises(anchor.AnchorError):
                anchor._tuple_correlate(before, sample)
        for key in ("identity", "interface", "backend", "versions"):
            changed = {**after, key: "changed"}
            with self.subTest(key=key), self.assertRaises(anchor.AnchorError):
                anchor._tuple_correlate(before, changed)
        old = self.sample(self.line(), timestamp=1)
        with self.assertRaises(anchor.AnchorError):
            anchor._tuple_correlate(old, self.sample(self.line(102), timestamp=20))

    def test_envoy_tuple_drop_chain_is_to_workload_and_ambiguous_deltas_do_not_pass(self):
        self.assertEqual(anchor._tuple_route(json.dumps([{"dst": self.pod, "dev": self.interface, "scope": "link"}]), self.pod), self.interface)
        for route in [[], [{"dst": self.pod, "dev": "eth0", "scope": "link"}],
                      [{"dst": self.pod, "dev": self.interface, "scope": "link", "gateway": self.cp}]]:
            with self.assertRaises(anchor.AnchorError):
                anchor._tuple_route(json.dumps(route), self.pod)
        for target in ("DROP", "REJECT"):
            self.assertEqual(anchor._tuple_deny(self.chain(3, 180, target), self.interface)["deny"], {"packets": 3, "bytes": 180})
        with self.assertRaises(anchor.AnchorError):
            anchor._tuple_deny(self.chain(direction="fw"), self.interface)
        invalid = self.chain().replace("if no policies passed packet", "if INVALID")
        self.assertEqual(anchor._tuple_deny(invalid, self.interface)["deny"], "AMBIGUOUS")
        self.assertEqual(anchor._tuple_deny(self.chain().replace("mark match 0x0/0x20000", "ctstate INVALID"), self.interface)["deny"], "AMBIGUOUS")
        before, after = self.sample(packets=3, size=180), self.sample(self.line(), packets=0, size=0, timestamp=20)
        with self.assertRaises(anchor.AnchorError):
            anchor._tuple_correlate(before, after)
        before["chain"]["deny"] = "MISSING"
        result = anchor._tuple_correlate(before, after)
        self.assertEqual(result["endpoint_deny_delta"], "MISSING")
        self.assertEqual(result["packet_attribution"], "NOT_ESTABLISHED_BY_COUNTER")


class WindowsAclBindingTests(unittest.TestCase):
    @unittest.skipUnless(os.name == "nt", "native Windows PowerShell boundary")
    def test_windows_kubectl_pipeline_preserves_payload_and_target_arguments(self):
        common = Path(__file__).resolve().parents[1] / "deploy/common.ps1"
        # Shadow only the outbound helper in this child. No API/Kind operation.
        setup = ". $p;function Invoke-LocalTarget {param($Action,$Arguments,$LeasePath,$Context) " \
                "$values=@($input);@{action=$Action;arguments=$Arguments;lease=$LeasePath;context=$Context;values=$values}|ConvertTo-Json -Compress};"
        for source, values in [("@('line-one','line-two') | ", ["line-one", "line-two"]),
                               ("@([string]::Concat('first',[char]10,'second')) | ", ["first\nsecond"]),
                               ("", [])]:
            with self.subTest(values=values):
                script = setup + source + "Invoke-LocalKubectl -Arguments @('apply','-f','-') " \
                         "-LeasePath 'owned-non-secret-lease' -Context 'kind-aiforall-local-full'"
                value = json.loads(anchor._windows_path_script(script, common))
                self.assertEqual(value, {"action": "kubectl", "arguments": ["apply", "-f", "-"],
                                        "lease": "owned-non-secret-lease", "context": "kind-aiforall-local-full",
                                        "values": values})

    @unittest.skipUnless(os.name == "nt", "native Windows PowerShell boundary")
    def test_windows_acl_directory_and_private_probe_bind_literal_paths(self):
        # Own non-secret files only. Spaces, apostrophe and semicolon must remain
        # literal data, never PowerShell syntax. No Docker, API or kubeconfig.
        with tempfile.TemporaryDirectory(dir=anchor.approved_root(), prefix="acl-binding-") as directory:
            path = Path(directory) / "literal space' ; éΩ directory"
            path.mkdir(mode=0o700)
            anchor.secure(path, directory=True)
            permissions = anchor._windows_path_script(
                "$a=Get-Acl -LiteralPath $p;$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User;"
                "$rules=@($a.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier]));"
                "$ok=$a.AreAccessRulesProtected -and $a.GetOwner([Security.Principal.SecurityIdentifier]).Equals($sid) -and $rules.Count -eq 2;"
                "foreach($r in $rules){if($r.IdentityReference.Value -notin @($sid.Value,'S-1-5-18') -or "
                "$r.AccessControlType -ne 'Allow' -or $r.FileSystemRights -ne 'FullControl' -or $r.IsInherited){$ok=$false}};"
                "if($ok){'exact-private-dacl'}else{'invalid-dacl'}", path)
            self.assertEqual(permissions.strip(), "exact-private-dacl")
            file = path / "mutable.txt"
            file.write_text("non-secret test marker", encoding="utf-8")
            # A policy-probe fixture needs a known private DACL but must retain
            # WriteAttributes for teardown. Do not undo secure(file)'s seal.
            anchor._windows_path_script(
                "$a=[Security.AccessControl.FileSecurity]::new();"
                "$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User;"
                "$a.SetOwner($sid);$a.SetAccessRuleProtection($true,$false);"
                "$a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','Allow'));"
                "$a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new("
                "[Security.Principal.SecurityIdentifier]::new('S-1-5-18'),'FullControl','Allow'));"
                "Set-Acl -LiteralPath $p -AclObject $a", file)
            try:
                file.chmod(0o400)
                self.assertTrue(anchor.private_file(file), "own protected read-only input must be accepted")
            finally:
                file.chmod(0o600)
            self.assertFalse(anchor.private_file(file), "mutable input must remain rejected")
            self.assertEqual(anchor._windows_path_script("Write-Output $p", path).strip(), str(path))
            self.assertEqual(anchor._windows_path_script("if($dir){'directory'}else{'file'}", path).strip(), "file")

    def test_windows_path_command_uses_encoded_data_and_propagates_failure(self):
        path = Path("literal space' ; Write-Output INJECTION")
        with patch.object(anchor, "run", return_value="marker") as runner:
            self.assertEqual(anchor._windows_path_script("Write-Output $p", path, directory=True), "marker")
        command = runner.call_args.args[0]
        self.assertEqual(command[:3], ["powershell.exe", "-NoProfile", "-EncodedCommand"])
        self.assertEqual(len(command), 4)
        self.assertEqual(runner.call_args.kwargs["encoding"], "utf-8")
        script = base64.b64decode(command[3], validate=True).decode("utf-16-le")
        self.assertNotIn(str(path), script)
        self.assertIn(base64.b64encode(str(path).encode("utf-8")).decode("ascii"), script)
        self.assertIn("$dir=$true;", script)
        self.assertIn("$ErrorActionPreference='Stop';", script)
        with patch.object(anchor, "run", side_effect=anchor.AnchorError("opaque command failure")):
            with self.assertRaises(anchor.AnchorError):
                anchor._windows_path_script("Write-Output $p", path)

    def test_windows_seal_marks_readonly_before_withdrawing_write_attributes(self):
        path = Path("owned-file")
        with patch.object(anchor.os, "name", "nt"), patch.object(anchor, "_windows_path_script") as invoke:
            anchor.secure(path)
        script = invoke.call_args.args[0]
        self.assertLess(script.index(".IsReadOnly=$true"), script.index(".SetAccessControl($a)"))
        self.assertNotIn("Set-Acl", script)
        self.assertNotIn("Audit", script)
        self.assertIn("'ReadAndExecute'", script)
        self.assertEqual(invoke.call_args.args[2], False)


class NativeRunnerTextTests(unittest.TestCase):
    def test_native_runner_decodes_utf8_without_reader_thread_loss(self):
        text = "native \U0001f30f\r\n"
        data = text.encode("utf-8").hex()
        command = [sys.executable, "-c", "import sys;sys.stdout.buffer.write(bytes.fromhex('" + data + "'));"
                   "sys.stderr.buffer.write(bytes.fromhex('" + data + "'))"]
        # Reproduce the locale of the actual Windows bootstrap independently of
        # the MCP server's Python UTF-8 mode. Production must not rely on locale.
        for options in ({"text": True}, {"text": True, "universal_newlines": True}):
            with self.subTest(options=options), patch("subprocess._text_encoding", return_value="cp1252"):
                self.assertEqual(anchor.run(command, **options), text.replace("\r\n", "\n"))

    def test_native_runner_rejects_invalid_text_without_reflecting_output(self):
        sentinel = "DO_NOT_REFLECT_NATIVE_OUTPUT"
        for stream in ("stdout", "stderr"):
            for code in (0, 3):
                with self.subTest(stream=stream, exit=code):
                    command = [sys.executable, "-c", "import sys;sys." + stream + ".buffer.write(bytes([255]));"
                               "sys.stderr.write('" + sentinel + "');sys.exit(" + str(code) + ")"]
                    with self.assertRaises(anchor.AnchorError) as caught:
                        anchor.run(command, text=True, encoding="utf-8")
                    self.assertNotIn(sentinel, str(caught.exception))
        with self.assertRaises(anchor.AnchorError) as caught:
            anchor.run([sys.executable, "-c", "pass"], text=True, input=sentinel + "\udcff")
        self.assertNotIn(sentinel, str(caught.exception))

    def test_native_runner_preserves_binary_io(self):
        value = bytes([0, 255, 13, 10, 128])
        command = [sys.executable, "-c", "import sys;sys.stdout.buffer.write(sys.stdin.buffer.read())"]
        self.assertEqual(anchor.run(command, input=value, text=False), value)


if __name__ == "__main__":
    unittest.main()
