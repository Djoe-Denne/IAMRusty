"""Fake-command S-11 tests and a native Windows ACL invocation boundary."""
import base64
import hashlib
import importlib.util
import io
import json
import os
import tempfile
import sys
import unittest
from pathlib import Path
from unittest.mock import MagicMock, Mock, patch

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

    def test_pending_cannot_adopt_existing_or_run_before_final_it(self):
        with tempfile.TemporaryDirectory() as root:
            fixture = Fixture(root, pending=True)
            commands = FakeCommands(fixture, exists=True)
            for consent in [False, True]:
                with self.assertRaises(anchor.AnchorError):
                    anchor.bootstrap(fixture.lease, "aiforall-local-full", "unused", consent, commands, root, lambda _: True)
            self.assertEqual(commands.kind_actions(), [])
            self.assertEqual(commands.api(), [])

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
