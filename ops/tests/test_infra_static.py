"""Exercise the gate's actual source-YAML check without a shell or cluster."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SOURCE_CHECK = (ROOT / "ops/tests/infra-static.sh").read_text(encoding="utf-8").split(
    '"$PYTHON" - <<\'PY\'\n', 1
)[1].split("\nPY\n", 1)[0]


class SourceYamlTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="infra-source-yaml-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.write(".github/workflows/ci.yml", "name: fixture\n")
        self.write("ops/plain.yaml", "apiVersion: v1\nkind: ConfigMap\n")

    def write(self, path, content):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")

    def check(self):
        return subprocess.run([sys.executable, "-c", SOURCE_CHECK], cwd=self.root,
                              capture_output=True, text=True, timeout=10)

    def test_plain_yaml_and_compose_tags_are_checked(self):
        self.write("ops/compose.yml", "ports: !override [443]\nvolumes: !reset []\n")
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("OK YAML syntax: 3 files", result.stdout)

    def assert_helm_inputs(self, directory):
        self.write(directory + "/Chart.yaml", "apiVersion: v2\nname: fixture\nversion: 0.1.0\n")
        self.write(directory + "/values.yaml", "enabled: true\n")
        self.write(directory + "/templates/fixtures.yaml", '{{- define "fixture" -}}\n')
        self.write(directory + "/files/config.yaml", "rendered: {{ .Values.enabled }}\n")
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_native_k6_singular_chart_uses_helm_not_raw_yaml(self):
        self.assert_helm_inputs("ops/tests/k6/native/chart")

    def test_deployment_charts_use_helm_not_raw_yaml(self):
        self.assert_helm_inputs("ops/deploy/charts/example")

    def test_chart_metadata_not_directory_spelling_identifies_helm_inputs(self):
        self.assert_helm_inputs("ops/another-renderer-location")

    def test_chart_metadata_and_values_still_reject_invalid_yaml(self):
        directory = "ops/tests/k6/native/chart"
        for filename in ("Chart.yaml", "values.yaml"):
            with self.subTest(filename=filename):
                self.write(directory + "/Chart.yaml", "apiVersion: v2\n")
                self.write(directory + "/values.yaml", "enabled: true\n")
                self.write(directory + "/" + filename, "broken: [\n")
                result = self.check()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(filename, result.stderr)

    def test_templates_outside_a_chart_are_not_silently_excluded(self):
        self.write("ops/charts/not-a-chart/templates/broken.yaml", "broken: [\n")
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("broken.yaml", result.stderr)

    def test_diagnostics_do_not_echo_yaml_values(self):
        self.write("ops/private-shape.yaml", "token: DO-NOT-ECHO-TEST-SENTINEL\nbroken: [\n")
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Invalid YAML:", result.stderr)
        self.assertNotIn("DO-NOT-ECHO-TEST-SENTINEL", result.stdout + result.stderr)
