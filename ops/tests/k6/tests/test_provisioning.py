import importlib.util
import io
import json
from pathlib import Path
import stat
import unittest
from unittest import mock
import zipfile

from test_runner import ROOT, runner
from test_boundaries import scratch

SPEC = importlib.util.spec_from_file_location("k6_provision", ROOT / "provision-linux.py")
provision = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(provision)


class ProvisioningTests(unittest.TestCase):
    def test_no_execution_or_installation_imported(self):
        self.assertNotIn("subprocess", provision.__dict__)
        self.assertNotIn("Child", provision.__dict__)

    def test_zip_traversal_and_symlink_rejected(self):
        for name, mode in [("../escape", stat.S_IFREG | 0o644),
                           ("/absolute", stat.S_IFREG | 0o644),
                           ("link", stat.S_IFLNK | 0o777)]:
            buffer = io.BytesIO()
            with zipfile.ZipFile(buffer, "w") as archive:
                info = zipfile.ZipInfo(name)
                info.external_attr = mode << 16
                archive.writestr(info, b"content")
            with scratch() as root, self.assertRaises(runner.Rejected):
                provision.extract(root, buffer.getvalue(), "chromium")

    def test_safe_regular_zip(self):
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as archive:
            info = zipfile.ZipInfo("browser/chrome")
            info.external_attr = (stat.S_IFREG | 0o755) << 16
            archive.writestr(info, b"fake-ELF-not-executed")
        with scratch() as root:
            provision.extract(root, buffer.getvalue(), "chromium")
            self.assertEqual((root / "browser/chrome").read_bytes(), b"fake-ELF-not-executed")

    def test_missing_pin_directory_never_executes(self):
        with mock.patch.dict(runner.os.environ, {}, clear=True), mock.patch.object(runner, "Child") as child:
            with self.assertRaises(runner.Rejected) as found:
                runner.pinned_tools()
            self.assertEqual(found.exception.code, "TOOL_UNPINNED")
            child.assert_not_called()

    def test_pins_are_exact_fixed_official_assets(self):
        lock = json.loads((ROOT / "config/tools.lock.json").read_text())
        self.assertEqual(lock["k6"]["version"], "2.3.0")
        self.assertEqual(lock["platform"], "linux-amd64")
        for name in ("k6", "chromium"):
            self.assertNotRegex(lock[name]["url"], r"latest|master")
            self.assertRegex(lock[name]["executable_sha256"], r"^[a-f0-9]{64}$")
            self.assertRegex(lock[name]["archive_sha256"], r"^[a-f0-9]{64}$")

    def test_workflow_is_prerequisite_only_and_has_explicit_artifact_files(self):
        workflow = (ROOT.parents[2] / ".github/workflows/k6-functional-smoke.yml").read_text()
        for forbidden in ["pull_request_target", "continue-on-error", "secrets.", "ubuntu-latest", "workflow_dispatch"]:
            self.assertNotIn(forbidden, workflow)
        self.assertIn("contents: read", workflow)
        self.assertIn("persist-credentials: false", workflow)
        self.assertIn("runs-on: ubuntu-24.04", workflow)
        self.assertNotIn("/private/", workflow)
        self.assertNotIn("run-linux.py run ", workflow)
        self.assertNotIn("run-linux.py selftest ", workflow)
        self.assertEqual(workflow.count("/result.json"), 2)
        self.assertEqual(workflow.count("/provenance.json"), 2)


if __name__ == "__main__":
    unittest.main()
