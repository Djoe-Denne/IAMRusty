"""Offline native Helm gates for the new-coffer branch, never the existing lab."""
import os
from pathlib import Path
import shutil
import subprocess
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
CHART = ROOT / "ops/deploy/charts/kind-minimal"


class NativeOpenBaoDeclarationsTests(unittest.TestCase):
    def render(self, *settings, expected=0):
        self.assertTrue((CHART / "Chart.yaml").is_file(), "native profile is not implemented")
        helm = os.environ.get("AIFORALL_HELM") or shutil.which("helm")
        if not helm:
            self.skipTest("qualified Helm CLI unavailable (NOT RUN)")
        env = dict(os.environ, KUBECONFIG="nonexistent-offline-sentinel")
        args = [helm, "template", "kind-local", str(CHART), "--namespace", "aiforall-local-full"]
        for setting in settings:
            args += ["--set-string" if setting.endswith("=not-a-boolean") else "--set", setting]
        result = subprocess.run(args, cwd=ROOT, env=env, capture_output=True, text=True, timeout=30, check=False)
        self.assertEqual(result.returncode, expected, result.stderr)
        return [obj for obj in yaml.safe_load_all(result.stdout) if obj] if expected == 0 else []

    def test_existing_coffer_default_renders_no_mutation(self):
        self.assertEqual(self.render(), [])

    def test_new_coffer_is_native_tls_raft_and_preserves_its_pvc(self):
        objects = self.render("openbao.create=true")
        self.assertEqual({obj["kind"] for obj in objects}, {
            "ConfigMap", "Deployment", "Service", "PersistentVolumeClaim",
            "ServiceAccount", "ClusterRole", "ClusterRoleBinding",
        })
        self.assertEqual(len(objects), 7)
        for obj in objects:
            if obj["kind"] in {"ClusterRole", "ClusterRoleBinding"}:
                self.assertNotIn("namespace", obj["metadata"])
            else:
                self.assertEqual(obj["metadata"]["namespace"], "aiforall-local-full")
        deployment = next(obj for obj in objects if obj["kind"] == "Deployment")
        self.assertEqual(deployment["metadata"]["name"], "openbao")
        self.assertEqual(deployment["spec"]["strategy"]["type"], "Recreate")
        self.assertEqual(deployment["spec"]["replicas"], 1)
        pod = deployment["spec"]["template"]["spec"]
        self.assertEqual(pod["serviceAccountName"], "openbao")
        container = pod["containers"][0]
        self.assertNotIn("-dev", container["args"])
        self.assertIn("openbao/openbao:2.6.2@sha256:", container["image"])
        volumes = {volume["name"]: volume for volume in pod["volumes"]}
        self.assertEqual(volumes["data"]["persistentVolumeClaim"]["claimName"], "openbao-data")
        self.assertFalse(any("emptyDir" in volume for volume in pod["volumes"]))
        self.assertNotIn("startupProbe", container, "waiting for unseal must not restart this coffer")
        live = container["livenessProbe"]["exec"]["command"][-1]
        self.assertIn("-eq 2", live, "sealed native status is live, not ready")
        self.assertEqual(container["readinessProbe"]["exec"]["command"], ["bao", "status"])
        config = next(obj for obj in objects if obj["kind"] == "ConfigMap")["data"]["openbao.hcl"]
        self.assertIn('storage "raft"', config)
        self.assertIn('node_id = "openbao-local"', config)
        self.assertIn('tls_cert_file = "/tls/tls.crt"', config)
        self.assertNotIn("tls_disable", config)
        self.assertNotIn("8200", config)
        role = next(obj for obj in objects if obj["kind"] == "ClusterRole")
        self.assertEqual(role["rules"], [{
            "apiGroups": ["authentication.k8s.io"], "resources": ["tokenreviews"], "verbs": ["create"],
        }])
        binding = next(obj for obj in objects if obj["kind"] == "ClusterRoleBinding")
        self.assertEqual(binding["roleRef"]["name"], role["metadata"]["name"])
        self.assertEqual(binding["subjects"], [{
            "kind": "ServiceAccount", "name": "openbao", "namespace": "aiforall-local-full",
        }])
        for obj in objects:
            self.assertNotEqual(obj["kind"], "Secret", "unseal parts/root must not be mounted in this server")

    def test_native_schema_refuses_nonboolean_creation_switch(self):
        self.render("openbao.create=not-a-boolean", expected=1)


if __name__ == "__main__":
    unittest.main()
