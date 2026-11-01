"""Image-only mutation boundary; no Docker/cluster is contacted."""
from contextlib import ExitStack, nullcontext
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/deploy"))
import image_sources
import local_registry
from local_full_target import parse_kubectl_operation

spec = importlib.util.spec_from_file_location("refresh_deploy", ROOT / "ops/deploy/deploy-local-full.py")
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)


class RefreshTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        directory = Path(self.directory.name)
        self.snapshot = {"files": [{"path": "workers/ext-authz/src/main.rs", "sha256": "a", "mode": 420}]}
        self.old = "aiforall-apps-registry:5000/apps/ext-authz@sha256:" + "a" * 64
        self.new = "aiforall-apps-registry:5000/apps/ext-authz@sha256:" + "b" * 64
        receipt = {"schema": "aiforall.image-release/v1", "freshness_policy": "snapshot-final-v1",
                   "source": {"mode": "working-tree"}, "profile": "release", "toolchain_id": "pins",
                   "refresh_inputs_id": local_registry.refresh_inputs(self.snapshot),
                   "images": {"ext-authz": {"oci_ref": self.old}}}
        before, after = directory / "before.json", directory / "after.json"
        before.write_text(json.dumps(receipt), encoding="utf-8")
        receipt["images"]["ext-authz"]["oci_ref"] = self.new
        after.write_text(json.dumps(receipt), encoding="utf-8")
        self.args = SimpleNamespace(targets="ext-authz", previous_release_manifest=before,
            release_manifest=after, lease=directory / "lease.json", registry_state=directory)
        self.live = {"metadata": {"uid": "deployment-uid", "resourceVersion": "10"},
                     "spec": {"replicas": 2, "template": {"spec": {"containers": [
                         {"name": "ext-authz", "image": self.old, "imagePullPolicy": "Never"}]}}}}
        self.data = {"namespaceUids": {deploy.CLUSTER: "namespace-uid"}, "ownedResources": [
            {"kind": "Deployment", "namespace": deploy.CLUSTER, "name": "ext-authz", "uid": "deployment-uid",
             "context": deploy.CONTEXT, "cluster": deploy.CLUSTER}]}
        self.registry = Mock()
        self.registry.operation.side_effect = nullcontext
        self.registry.remote.return_value = ({}, {"config": {"Labels": {"aiforall.dev.cargo-profile": "release"}}})
        self.target = Mock()
        self.calls = []
        def kubectl(*args, **kwargs):
            parse_kubectl_operation(list(args))
            self.calls.append(args)
            if args[:2] == ("get", "namespace"):
                return "namespace-uid"
            if args[:2] == ("get", "deployment"):
                return json.dumps(self.live)
            if args[0] == "patch":
                self.live["spec"]["template"]["spec"]["containers"][0]["image"] = self.new
            return ""
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        for obj, name, value in [(image_sources, "capture", Mock(return_value=self.snapshot)),
                (image_sources, "require_unchanged", Mock()), (local_registry, "Registry", Mock(return_value=self.registry)),
                (deploy.runtime, "lease", Mock(return_value=self.data)),
                (deploy.runtime, "operation_lock", Mock(side_effect=lambda *_: nullcontext())),
                (deploy.runtime, "target", Mock(return_value=self.target)), (deploy, "k", kubectl)]:
            self.stack.enter_context(patch.object(obj, name, value))

    def test_changes_only_image_and_pull_policy_with_cas(self):
        deploy.refresh(self.args)
        writes = [args for args in self.calls if args[0] not in {"get", "rollout"}]
        self.assertEqual(len(writes), 1)
        change = json.loads(writes[0][-1])
        self.assertEqual([(p["op"], p["path"]) for p in change], [
            ("test", "/metadata/uid"), ("test", "/metadata/resourceVersion"),
            ("test", "/spec/template/spec/containers/0/image"),
            ("replace", "/spec/template/spec/containers/0/image"),
            ("add", "/spec/template/spec/containers/0/imagePullPolicy")])
        self.assertEqual(self.live["spec"]["replicas"], 2)
        self.target.registry_pull.assert_called_once_with(self.registry, self.new)

    def test_foreign_uid_or_old_image_refused_before_node_operations(self):
        for key in ("uid", "image"):
            with self.subTest(key=key):
                self.live["metadata"]["uid"] = "foreign" if key == "uid" else "deployment-uid"
                self.live["spec"]["template"]["spec"]["containers"][0]["image"] = "foreign" if key == "image" else self.old
                with self.assertRaises(ValueError):
                    deploy.refresh(self.args)
                self.target.registry_connect.assert_not_called()
                self.assertFalse(any(call[0] == "patch" for call in self.calls))

    def test_config_changes_and_automatic_migrations_refused(self):
        self.snapshot["files"].append({"path": "services/Hive/config/default.toml", "sha256": "changed", "mode": 420})
        with self.assertRaises(ValueError):
            deploy.refresh(self.args)
        self.args.targets = "hive"
        with self.assertRaisesRegex(ValueError, "automatic-migration"):
            deploy.refresh(self.args)
        self.assertEqual(self.calls, [])
        self.target.registry_pull.assert_not_called()

    def test_unchanged_image_is_noop_without_node_operations(self):
        self.args.release_manifest.write_bytes(self.args.previous_release_manifest.read_bytes())
        deploy.refresh(self.args)
        self.assertFalse(any(call[0] in {"patch", "rollout"} for call in self.calls))
        self.target.registry_connect.assert_not_called()
        self.target.registry_pull.assert_not_called()


if __name__ == "__main__":
    unittest.main()
