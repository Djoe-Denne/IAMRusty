"""Fresh-runner IAM cache qualification; no compiler or external runtime is started."""

import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

import yaml


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/ci"))
import build


class IamCacheQualificationTests(unittest.TestCase):
    def test_workflow_uses_sequential_fresh_jobs_and_no_target_transfer(self):
        workflow = yaml.load((ROOT / ".github/workflows/iam-cache-qualification.yml").read_text(),
                             Loader=yaml.BaseLoader)
        self.assertIn("workflow_dispatch", workflow["on"])
        self.assertEqual(workflow["on"]["push"]["branches"], ["ci-iam-cache-qualification"])
        self.assertNotIn("pull_request", workflow["on"])
        self.assertEqual(workflow["permissions"], {"contents": "read"})
        jobs = workflow["jobs"]
        self.assertNotIn("needs", jobs["seed"])
        self.assertEqual(jobs["warm"]["needs"], "seed")
        self.assertEqual(jobs["summary"]["needs"], ["seed", "warm"])
        for name in ("seed", "warm"):
            job = jobs[name]
            self.assertEqual(job["runs-on"], "ubuntu-24.04")
            builds = [step for step in job["steps"] if step.get("uses") == "./.github/actions/build-service"]
            self.assertEqual([step["with"] for step in builds], [{"service": "iam"}])
            commands = "\n".join(step.get("run", "") for step in job["steps"])
            self.assertIn('test ! -e "$CARGO_TARGET_DIR"', commands)
            self.assertIn("CI_BUILD_TIMING_FILE=", commands)
            self.assertIn("python3 -B ops/ci/compiler_cache.py\nsccache --stop-server", commands)
            self.assertFalse(any("download-artifact" in step.get("uses", "") for step in job["steps"]))
            uploads = [step["with"]["path"] for step in job["steps"]
                       if "upload-artifact" in step.get("uses", "")]
            self.assertEqual(uploads, ["${{ runner.temp }}/iam-build-timing.json"])

    def test_timing_records_exact_command_outcome_and_empty_initial_target(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for status in (0, 101):
                output = root / f"timing-{status}.json"
                environment = {
                    "CI_BUILD_TIMING_FILE": str(output), "CARGO_TARGET_DIR": str(root / "target"),
                    "GITHUB_ENV": "", "GITHUB_SHA": "a" * 40, "GITHUB_RUN_ID": "123",
                    "GITHUB_RUN_ATTEMPT": "1", "GITHUB_JOB": "warm",
                }
                with self.subTest(status=status), patch.dict(os.environ, environment), \
                        patch.object(sys, "argv", ["build.py", "--service", "iam"]), \
                        patch.object(build.time, "perf_counter", side_effect=[100.0, 123.5]), \
                        patch.object(build.subprocess, "run", return_value=Mock(returncode=status)) as runner:
                    self.assertEqual(build.main(), status)
                    value = json.loads(output.read_text())
                    self.assertEqual(value["elapsed_seconds"], 23.5)
                    self.assertEqual(value["exit_code"], status)
                    self.assertIs(value["target_initially_empty"], True)
                    self.assertEqual(value["command"], runner.call_args.args[0][1:])
                    self.assertEqual(value["job"], "warm")
                    self.assertEqual(value["source_sha"], environment["GITHUB_SHA"])

    def test_existing_target_artifacts_cannot_masquerade_as_remote_cache_reuse(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / "target"
            target.mkdir()
            (target / "old-artifact").write_text("not a fresh runner")
            output = root / "timing.json"
            with patch.dict(os.environ, {"CI_BUILD_TIMING_FILE": str(output),
                                         "CARGO_TARGET_DIR": str(target)}), \
                    patch.object(sys, "argv", ["build.py", "--service", "iam"]), \
                    patch.object(build.subprocess, "run") as runner:
                self.assertEqual(build.main(), 2)
                runner.assert_not_called()
                self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
