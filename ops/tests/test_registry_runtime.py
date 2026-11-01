"""Opt-in local registry protocol smoke; never starts or touches Kind."""
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/deploy"))
import local_registry as registry


@unittest.skipUnless(os.environ.get("AIFORALL_REGISTRY_SMOKE") == "1", "explicit Docker registry protocol smoke")
class RegistryRuntimeTests(unittest.TestCase):
    def test_publish_reader_denial_and_persistent_restart(self):
        parent = Path(os.environ["LOCALAPPDATA"]) / "Temp/opencode" if os.name == "nt" else Path(os.environ.get("TMPDIR", "/tmp"))
        root = parent / ("aiforall-registry-" + uuid.uuid4().hex[:12])
        root.mkdir()
        certs, state = root / "certificates", root / "state"
        result = subprocess.run([sys.executable, "-B", str(ROOT / "ops/scripts/prepare-local-full-certs.py"),
                                 "--output-dir", str(certs), "--registry"], capture_output=True, timeout=180)
        self.assertEqual(result.returncode, 0, "synthetic registry PKI failed; private output withheld")
        instance = registry.Registry(state, create=True)
        evidence = ROOT / ".tmp/registry-smoke-current.json"
        evidence.parent.mkdir(exist_ok=True)
        evidence.write_text(json.dumps({"state_dir": str(state), "owner": instance.state["owner"]}), encoding="utf-8")
        try:
            with instance.operation():
                instance.up(certs, 15443)
                local = instance.inspect("image", instance.lock["zot"]["reference"])
                remote = instance.copy(local, "apps/registry-smoke", lambda archive: None)
                path = "/v2/apps/registry-smoke/manifests/" + remote["manifest_digest"]
                status, body, _ = instance.request(path, headers={"Accept": "application/vnd.oci.image.manifest.v1+json"})
                self.assertEqual(status, 200)
                denied, _, _ = instance.request("/v2/apps/registry-smoke/manifests/reader-must-not-write", method="PUT",
                    body=body, headers={"Content-Type": "application/vnd.oci.image.manifest.v1+json"})
                self.assertIn(denied, (401, 403), "reader unexpectedly gained publication permission")
                identity = instance.state["container"]
                instance.stop_container(identity)
                instance.up(None, 15443)
                self.assertEqual(instance.state["container"], identity)
                instance.remote("apps/registry-smoke", remote["manifest_digest"], local)
                evidence.write_text(json.dumps({"state_dir": str(state), "owner": instance.state["owner"],
                    "registry_id": identity, "remote": remote, "reader_denied": denied, "restart_preserved": True}), encoding="utf-8")
                print("registry protocol proof: " + str(evidence.relative_to(ROOT)))
        finally:
            for identity in [*instance.state["helpers"], *([instance.state["container"]] if instance.state.get("container") else [])]:
                instance.stop_container(identity)


if __name__ == "__main__":
    unittest.main()
