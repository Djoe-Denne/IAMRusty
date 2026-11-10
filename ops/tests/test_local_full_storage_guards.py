"""Offline storage CAS/ACK oracles. No cluster or runtime used."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/deploy"))
spec = importlib.util.spec_from_file_location("storage_guards", ROOT / "ops/deploy/local-full-storage.py")
storage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(storage)


class StorageGuards(unittest.TestCase):
    def test_replica_patch_is_uid_rv_and_preimage_bound_and_ack_checked(self):
        original = {"metadata": {"uid": "owned", "resourceVersion": "42"}, "spec": {"replicas": 1}}
        for mutation in (None, "uid", "replicas", "ack"):
            with self.subTest(mutation=mutation):
                live = copy.deepcopy(original)
                if mutation == "uid": live["metadata"]["uid"] = "foreign"
                if mutation == "replicas": live["spec"]["replicas"] = 2
                ack = copy.deepcopy(original)
                ack["spec"]["replicas"] = 0
                if mutation == "ack": ack["metadata"]["uid"] = "foreign"
                with patch.object(storage, "kubectl", side_effect=[json.dumps(live), json.dumps(ack)]) as query:
                    if mutation:
                        with self.assertRaises(ValueError): storage.guarded_replicas("namespace", "source", "owned", 0, 1)
                    else:
                        storage.guarded_replicas("namespace", "source", "owned", 0, 1)
                    self.assertEqual(query.call_count, 1 if mutation in {"uid", "replicas"} else 2)
                    if query.call_count == 2:
                        args = query.call_args.args
                        operations = json.loads(args[args.index("-p") + 1])
                        self.assertEqual(operations[:3], [
                            {"op": "test", "path": "/metadata/uid", "value": "owned"},
                            {"op": "test", "path": "/metadata/resourceVersion", "value": "42"},
                            {"op": "test", "path": "/spec/replicas", "value": 1}])

    def test_incomplete_operation_blocks_mutators_but_keeps_status_and_stop(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lease.json"
            path.write_text(json.dumps({"localFullOperation": {"state": "in-progress"}}))
            for action in ("deploy", "backup", "restore", "refresh", "local-up"):
                with self.subTest(action=action), self.assertRaisesRegex(ValueError, "unresolved"):
                    with storage.operation_lock(path, action): self.fail("mutator entered")
            for action in ("local-status", "local-stop"):
                with storage.operation_lock(path, action): pass
            self.assertFalse(path.with_name(path.name + ".local-full-lock").exists())

    def test_restore_requires_original_source_pvc_ack_before_new_target(self):
        from local_registry import pki
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "synthetic.archive"
            metadata = {"volume": "zot", "format": "tar", "source": {
                "namespace": storage.SOURCE, "name": "zot-data", "uid": "original", "volume": "original-pv"}}
            archive.with_suffix(".archive.json").write_text(json.dumps(metadata))
            args = SimpleNamespace(archive=archive, confirm_restore=True, target_namespace="aiforall-local-restore-synthetic")
            with patch.object(pki, "checked_path"), patch.object(storage, "private_file", return_value=True), \
                 patch.object(storage, "assert_namespace_owned"), patch.object(storage, "kubectl") as query, patch.object(storage, "create") as create:
                with self.assertRaisesRegex(ValueError, "source PVC UID"):
                    storage.restore(args, {"ownedResources": []})
                query.assert_not_called(); create.assert_not_called()


if __name__ == "__main__": unittest.main()
