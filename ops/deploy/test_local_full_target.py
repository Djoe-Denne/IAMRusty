"""Pure fake-command S-11 tests. Parent executes after source integration only."""
import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import yaml
import local_full_target as anchor


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


if __name__ == "__main__":
    unittest.main()
