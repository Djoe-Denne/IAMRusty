"""Small offline boundary checks; real mTLS/push is verified separately."""
import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import Mock

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import local_registry as registry


class RegistryTests(unittest.TestCase):
    def test_mtls_policy_separates_publisher_and_reader(self):
        config = registry.config()
        policy = config["http"]["accessControl"]["repositories"]["apps/**"]
        self.assertEqual(policy["anonymousPolicy"], [])
        self.assertEqual(policy["defaultPolicy"], [])
        self.assertEqual(policy["policies"][1], {"users": ["registry-reader"], "actions": ["read"]})
        self.assertNotIn("delete", policy["policies"][0]["actions"])
        self.assertNotIn("ca.key", json.dumps(config))
        self.assertEqual(config["http"]["tls"]["cacert"], "/runtime/ca.crt")

    def test_archive_hashes_final_binary_not_lower_layer(self):
        def layer(files):
            data = io.BytesIO()
            with tarfile.open(fileobj=data, mode="w") as archive:
                for name, contents in files.items():
                    item = tarfile.TarInfo(name)
                    item.size = len(contents)
                    archive.addfile(item, io.BytesIO(contents))
            return data.getvalue()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "image.tar"
            payloads = {"one.tar": layer({"app/hive-service": b"old", "app/migration": b"old"}),
                        "two.tar": layer({"app/.wh..wh..opq": b"", "app/hive-service": b"new", "app/migration": b"migration"})}
            payloads["manifest.json"] = json.dumps([{"Layers": ["one.tar", "two.tar"]}]).encode()
            with tarfile.open(path, "w") as archive:
                for name, contents in payloads.items():
                    item = tarfile.TarInfo(name)
                    item.size = len(contents)
                    archive.addfile(item, io.BytesIO(contents))
            result = registry.artifact_hashes(path, "hive")
            self.assertEqual({r["binary"]: r["sha256"] for r in result}, {
                "hive-service": registry.digest(b"new"), "hivemigration": registry.digest(b"migration")})

    def test_remote_manifest_and_config_digests_verified_separately(self):
        source = {"RootFS": {"Layers": ["sha256:" + "a" * 64]}, "Config": {"Labels": {"source": "public"}}}
        config = json.dumps({"os": "linux", "architecture": "amd64", "rootfs": {"diff_ids": source["RootFS"]["Layers"]},
                             "config": source["Config"]}).encode()
        manifest = json.dumps({"mediaType": "application/vnd.oci.image.manifest.v1+json",
                               "config": {"digest": registry.digest(config)}}).encode()
        instance = registry.Registry.__new__(registry.Registry)
        instance.request = Mock(side_effect=[(200, manifest, registry.digest(manifest)), (200, config, None)])
        result = instance.remote("apps/hive", registry.digest(manifest), source)
        self.assertNotEqual(result["manifest_digest"], result["config_digest"])
        instance.request = Mock(return_value=(200, manifest, "sha256:" + "b" * 64))
        with self.assertRaises(ValueError):
            instance.remote("apps/hive", registry.digest(manifest), source)

    def test_foreign_container_refused_before_stop(self):
        instance = registry.Registry.__new__(registry.Registry)
        instance.state = {"owner": "a" * 32}
        instance.inspect = Mock(return_value={"Id": "b" * 64, "Config": {"Labels": {"aiforall.registry.owner": "foreign"}}})
        instance.docker = Mock()
        with self.assertRaises(ValueError):
            instance.stop_container("b" * 64)
        instance.docker.assert_not_called()


if __name__ == "__main__":
    unittest.main()
