"""Private failure bytes preserved, public outputs and metadata stay redacted."""
from contextlib import redirect_stderr
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "deploy"))
import local_full_target as target
import local_registry
import private_diagnostics as diagnostics


class DiagnosticsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "evidence"
        self.stderr = io.StringIO()

    def test_real_child_stderr_exit_preserved_without_public_leak_or_argv_stdin(self):
        # Native child, not a mocked runner. No infrastructure or actual secret.
        cmd = [sys.executable, "-c", "import sys;sys.stderr.buffer.write(b'PRIVATE_OUTPUT_SENTINEL');sys.exit(17)", "PRIVATE_ARG_SENTINEL"]
        with redirect_stderr(self.stderr), self.assertRaises(target.AnchorError) as error:
            with diagnostics.operation("child-probe", self.root):
                target.run(cmd, input=b"PRIVATE_STDIN_SENTINEL", text=True)
        self.assertNotIn("PRIVATE_", str(error.exception) + self.stderr.getvalue())
        directory = next(self.root.iterdir())
        self.assertEqual((directory / "001.stderr.bin").read_bytes(), b"PRIVATE_OUTPUT_SENTINEL")
        metadata = (directory / "001.json").read_text(encoding="utf-8")
        self.assertEqual(json.loads(metadata)["returncode"], 17)
        self.assertNotIn("PRIVATE_ARG_SENTINEL", metadata)
        self.assertNotIn("PRIVATE_STDIN_SENTINEL", metadata)
        local_registry.pki.private_directory(self.root)  # Actual owner-only ACL/modes guard.

    def test_success_and_unscoped_failure_create_no_private_directory(self):
        with diagnostics.operation("success", self.root):
            self.assertEqual(target.run([sys.executable, "-c", "print('ok')"], text=True).strip(), "ok")
        self.assertFalse(self.root.exists())
        with self.assertRaises(target.AnchorError):
            target.run([sys.executable, "-c", "raise SystemExit(9)"], text=True)
        self.assertFalse(self.root.exists())

    def test_wrapper_exception_metadata_never_copies_arbitrary_exception_text(self):
        error = ValueError("PRIVATE_ARG_SENTINEL PRIVATE_STDIN_SENTINEL")
        with redirect_stderr(self.stderr), self.assertRaises(ValueError) as caught:
            with diagnostics.operation("wrapper", self.root):
                raise error
        self.assertIs(caught.exception, error)
        directory = next(self.root.iterdir())
        metadata = (directory / "001.json").read_text(encoding="utf-8")
        self.assertEqual(json.loads(metadata)["reason"], "ValueError")
        self.assertNotIn("PRIVATE_", metadata + self.stderr.getvalue())

    def test_timeout_partial_bytes_and_explicit_truncation(self):
        error = subprocess.TimeoutExpired(["docker", "wait"], 1, output=b"partial", stderr=b"z" * 20)
        self.root.mkdir(mode=0o700)
        with patch.object(target.subprocess, "run", side_effect=error), patch.object(diagnostics, "LIMIT", 8), \
                patch.object(diagnostics, "_directory", return_value=self.root), \
                redirect_stderr(self.stderr), self.assertRaises(subprocess.TimeoutExpired):
            with diagnostics.operation("timeout", self.root):
                target.run(["docker", "wait", "PRIVATE_ARG_SENTINEL"], timeout=1)
        directory = self.root
        metadata = json.loads((directory / "001.json").read_text())
        self.assertTrue(metadata["timeout"])
        self.assertTrue(metadata["outputs"]["stderr"]["truncated"])
        self.assertEqual(metadata["outputs"]["stderr"]["bytes"], 20)
        self.assertEqual((directory / "001.stderr.bin").read_bytes(), b"z" * 8)

    def test_failed_capture_never_masks_original_error(self):
        with patch.object(diagnostics, "_directory", side_effect=OSError("PRIVATE_PERMISSION_SENTINEL")), \
                redirect_stderr(self.stderr), self.assertRaises(target.AnchorError) as error:
            with diagnostics.operation("unavailable", self.root):
                target.run([sys.executable, "-c", "raise SystemExit(7)"], text=True)
        self.assertEqual(str(error.exception), "target operation failed; output redacted")
        self.assertNotIn("PRIVATE_PERMISSION_SENTINEL", self.stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
