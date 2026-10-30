import importlib.util
import hashlib
import io
import json
from pathlib import Path
import stat
import unittest
from unittest import mock
import zipfile

from test_runner import ROOT, runner
from test_boundaries import posix_tree, scratch

SPEC = importlib.util.spec_from_file_location("k6_provision", ROOT / "provision-linux.py")
provision = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(provision)


class ProvisioningTests(unittest.TestCase):
    def fixture_tools(self, root):
        lock = json.loads((ROOT / "config/tools.lock.json").read_text())
        tools = root / "tools"
        tools.mkdir()
        archive = tools / "k6.tar.gz"
        archive.write_bytes(b"not-an-archive-not-executed")
        binary = tools / lock["k6"]["executable"]
        binary.parent.mkdir()
        binary.write_bytes(b"\x7fELFfixture-not-an-executable")
        lock["k6"]["archive_sha256"] = hashlib.sha256(archive.read_bytes()).hexdigest()
        lock["k6"]["executable_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
        return tools, binary, lock

    def substitute_tools(self, root, tools, lock):
        tools.rename(root / "retired-tools")
        tools.mkdir()
        (tools / "k6.tar.gz").write_bytes(b"replacement-not-executed")
        binary = tools / lock["k6"]["executable"]
        binary.parent.mkdir()
        binary.write_bytes(b"\x7fELFreplacement-not-executed")

    def test_unsafe_tool_ancestor_refused_before_version_child(self):
        with scratch() as root:
            tools, _, lock = self.fixture_tools(root)
            with posix_tree(root, {root.parent: (124, 0o755)}), mock.patch.dict(runner.os.environ, {"K6_TOOLS_DIR": str(tools)}), \
                    mock.patch.object(runner, "strict_json", return_value=lock), mock.patch.object(runner, "Child") as child:
                child.return_value.stdout = b"k6 v2.3.0 fixture"
                child.return_value.execute.return_value = 0
                with self.assertRaises(runner.Rejected) as found:
                    pinned, actual_lock = runner.pinned_tools()
                    runner.version_checks(pinned, actual_lock, root)
                self.assertEqual(found.exception.code, "PATH_INVALID")
                child.assert_not_called()

    def test_tool_directory_substitution_after_hash_before_use_never_executes(self):
        with scratch() as root:
            tools, _, lock = self.fixture_tools(root)
            with posix_tree(root), mock.patch.dict(runner.os.environ, {"K6_TOOLS_DIR": str(tools)}), \
                    mock.patch.object(runner, "strict_json", return_value=lock), mock.patch.object(runner, "Child") as child:
                pinned, actual_lock = runner.pinned_tools()
                self.substitute_tools(root, tools, lock)
                child.return_value.stdout = b"k6 v2.3.0 fixture"
                child.return_value.execute.return_value = 0
                with self.assertRaises(runner.Rejected) as found:
                    runner.version_checks(pinned, actual_lock, root)
                self.assertEqual(found.exception.code, "TOOL_UNPINNED")
                child.assert_not_called()

    def test_tool_directory_substitution_during_last_digest_never_executes(self):
        with scratch() as root:
            tools, binary, lock = self.fixture_tools(root)
            original = runner.digest
            def substitute(path):
                result = original(path)
                if path == binary:
                    self.substitute_tools(root, tools, lock)
                return result
            with posix_tree(root), mock.patch.dict(runner.os.environ, {"K6_TOOLS_DIR": str(tools)}), \
                    mock.patch.object(runner, "strict_json", return_value=lock), mock.patch.object(runner, "digest", side_effect=substitute), \
                    mock.patch.object(runner, "Child") as child:
                child.return_value.stdout = b"k6 v2.3.0 fixture"
                child.return_value.execute.return_value = 0
                with self.assertRaises(runner.Rejected) as found:
                    pinned, actual_lock = runner.pinned_tools()
                    runner.version_checks(pinned, actual_lock, root)
                self.assertEqual(found.exception.code, "TOOL_UNPINNED")
                child.assert_not_called()

    def test_verified_tool_survives_unrelated_parent_entry_change(self):
        with scratch() as root:
            tools, _, lock = self.fixture_tools(root)
            with posix_tree(root), mock.patch.dict(runner.os.environ, {"K6_TOOLS_DIR": str(tools)}), \
                    mock.patch.object(runner, "strict_json", return_value=lock), mock.patch.object(runner, "Child") as child:
                pinned, actual_lock = runner.pinned_tools()
                (root / "unrelated-entry").write_text("ordinary sibling change")
                child.return_value.stdout = b"k6 v2.3.0 fixture"
                child.return_value.execute.return_value = 0
                self.assertEqual(runner.version_checks(pinned, actual_lock, root), {"k6": "2.3.0"})
                child.return_value.execute.assert_called_once()

    def test_tool_mapping_cannot_detach_from_verified_identity(self):
        with scratch() as root:
            tools, _, lock = self.fixture_tools(root)
            with posix_tree(root), mock.patch.dict(runner.os.environ, {"K6_TOOLS_DIR": str(tools)}), \
                    mock.patch.object(runner, "strict_json", return_value=lock), mock.patch.object(runner, "Child") as child:
                pinned, actual_lock = runner.pinned_tools()
                pinned["k6"] = root / "different-path-not-executed"
                with self.assertRaises(runner.Rejected) as found:
                    runner.version_checks(pinned, actual_lock, root)
                self.assertEqual(found.exception.code, "TOOL_UNPINNED")
                child.assert_not_called()

    def test_substitution_prevents_scenario_child_even_with_stubbed_target_guard(self):
        with scratch() as root:
            tools, _, lock = self.fixture_tools(root)
            with posix_tree(root), mock.patch.dict(runner.os.environ, {"K6_TOOLS_DIR": str(tools)}), \
                    mock.patch.object(runner, "strict_json", return_value=lock), mock.patch.object(runner, "live_target"), \
                    mock.patch.object(runner, "Child") as child:
                pinned, _ = runner.pinned_tools()
                self.substitute_tools(root, tools, lock)
                with self.assertRaises(runner.Rejected) as found:
                    runner.scenario_run("HTTP", {"origin": "http://127.0.0.1:18080"}, root, pinned)
                self.assertEqual(found.exception.code, "TOOL_UNPINNED")
                child.assert_not_called()

    def test_substitution_prevents_provenance_and_publication(self):
        with scratch() as root:
            tools, _, lock = self.fixture_tools(root)
            with posix_tree(root), mock.patch.dict(runner.os.environ, {"K6_TOOLS_DIR": str(tools)}), \
                    mock.patch.object(runner, "strict_json", return_value=lock), mock.patch.object(runner.subprocess, "run") as git, \
                    mock.patch.object(runner, "publish") as publish:
                pinned, actual_lock = runner.pinned_tools()
                self.substitute_tools(root, tools, lock)
                with self.assertRaises(runner.Rejected) as found:
                    runner.publish(root, {}, runner.provenance(pinned, actual_lock, {"k6": "2.3.0"}))
                self.assertEqual(found.exception.code, "TOOL_UNPINNED")
                git.assert_not_called()
                publish.assert_not_called()
                self.assertFalse((root / "result.json").exists())

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
        published = workflow.split("path: |", 1)[1].split("      - name:", 1)[0]
        self.assertNotIn("/private/", published)  # private parent input is not an upload
        self.assertNotIn("run-linux.py run ", workflow)
        self.assertNotIn("run-linux.py selftest ", workflow)
        self.assertEqual(workflow.count("/result.json"), 2)
        self.assertEqual(workflow.count("/provenance.json"), 2)


if __name__ == "__main__":
    unittest.main()
