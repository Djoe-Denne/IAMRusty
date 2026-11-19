"""Static and data-only checks for the IAM compile-once experiment."""

import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest import mock

import yaml

ROOT = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(ROOT / "ops/ci"), str(ROOT / "ops/tests/k6/ci")]
import iam_compile_once as pilot
import runtime


class PilotTests(unittest.TestCase):
    def test_admits_pinned_coverage_wrapper_and_refuses_uninstrumented_modes(self):
        env = {
            "RUSTC_WRAPPER": "/home/runner/.cargo/bin/cargo-llvm-cov",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER": "1",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS": "-C\x1finstrument-coverage\x1f--cfg=coverage",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_PRE_EXISTING": "sccache",
        }
        pilot.verify_coverage_environment(env)
        for key in env:
            with self.subTest(missing=key), self.assertRaises(ValueError):
                pilot.verify_coverage_environment({k: v for k, v in env.items() if k != key})
        with self.assertRaisesRegex(ValueError, "coverage wrapper"):
            pilot.verify_coverage_environment({"RUSTC_WRAPPER": "sccache", "RUSTFLAGS": "-C instrument-coverage"})

    def test_archive_and_negative_tamper(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = b"\x7fELF\x02\x01" + bytes(10) + b"\x03\x00\x3e\x00" + bytes(70)
            (root / "binary").write_bytes(data)
            archive = root / "bundle.tar.gz"
            expected = {"iam-service|test|example"}
            pilot.bundle(archive, {"run": "sample"}, "tests-0",
                         [(next(iter(expected)), "services/IAMRusty", "test-000", root / "binary")])
            pilot.unpack(archive, root / "good", {"run": "sample"}, "tests-0", expected)
            self.assertEqual((root / "good/test-000").read_bytes(), data)
            with self.assertRaisesRegex(ValueError, "identity mismatch"):
                pilot.unpack(archive, root / "wrong", {"run": "other"}, "tests-0", expected)
            with tarfile.open(archive) as original, tarfile.open(root / "bad.tar.gz", "w:gz") as bad:
                member = original.getmember("manifest.json")
                bad.addfile(member, io.BytesIO(original.extractfile(member).read()))
                member = original.getmember("test-000")
                bad.addfile(member, io.BytesIO(data[:-1] + b"x"))
            with self.assertRaisesRegex(ValueError, "digest mismatch"):
                pilot.unpack(root / "bad.tar.gz", root / "bad", {"run": "sample"}, "tests-0", expected)

    def test_raw_bundle_cap_reports_partition_and_size_before_reading_binaries(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binary = root / "binary"
            binary.write_bytes(bytes(65))
            with mock.patch.object(pilot, "MAX_BUNDLE", 64), mock.patch.object(pilot, "digest") as digest:
                with self.assertRaisesRegex(ValueError, r"tests-1: 65 uncompressed bytes exceeds limit 64"):
                    pilot.bundle(root / "bundle.tar.gz", {}, "tests-1", [("test", ".", "test-000", binary)])
                digest.assert_not_called()

    def test_coverage_requires_a_positive_count_for_a_real_iam_source(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            iam = root / "services/IAMRusty/src/lib.rs"
            iam.parent.mkdir(parents=True)
            iam.touch()
            other = root / "other.rs"
            other.touch()
            report = root / "pilot.lcov"
            report.write_text(f"SF:{other.as_posix()}\nDA:1,7\nend_of_record\n"
                              "SF:services/IAMRusty/src/lib.rs\nDA:1,0\nend_of_record\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "no executed IAM source lines"):
                pilot.require_iam_coverage(report, root)
            with report.open("a", encoding="utf-8") as out:
                out.write(f"SF:{iam.as_posix()}\nDA:1,1\nend_of_record\n")
            pilot.require_iam_coverage(report, root)

    def test_pilot_never_uses_native_workflow_or_consumer_compilation(self):
        raw = (ROOT / pilot.WORKFLOW).read_text()
        config = yaml.load(raw, Loader=yaml.BaseLoader)
        self.assertEqual(config["on"], {"push": {"branches": [pilot.BRANCH]}})
        self.assertEqual(config["permissions"], {"contents": "read", "actions": "read"})
        jobs = config["jobs"]
        self.assertCountEqual(jobs, ["producer", "native-oauth", "native-ext-authz", "pilot-e2e"])
        self.assertCountEqual(jobs["pilot-e2e"]["needs"], ["producer", "native-oauth", "native-ext-authz"])
        self.assertNotIn("secrets.", json.dumps(jobs["producer"]))
        producer = jobs["producer"]
        for profile in ("DEV", "TEST"):
            self.assertEqual(producer["env"][f"CARGO_PROFILE_{profile}_DEBUG"], "line-tables-only")
        handoff = [(i, step) for i, step in enumerate(producer["steps"])
                   if step.get("with", {}).get("name", "").startswith("iam-pilot-e2e-")]
        self.assertEqual(len(handoff), 1)
        before_upload, _ = handoff[0]
        e2e_compile = next(i for i, step in enumerate(producer["steps"]) if " produce-e2e " in step.get("run", ""))
        self.assertLess(e2e_compile, before_upload)
        self.assertNotIn(" --matrix ", json.dumps(producer))
        steps = json.dumps(jobs["pilot-e2e"]["steps"])
        self.assertNotIn("cargo test", steps)
        self.assertNotIn("cargo build", steps)
        self.assertNotIn("build-service", steps)
        self.assertEqual(steps.count("iam_pilot.py run"), 1)
        self.assertNotIn("deploy-pages", raw)

    def test_runtime_gate_has_no_caller_selectable_bypass(self):
        env = dict(GITHUB_REPOSITORY="Djoe-Denne/IAMRusty", GITHUB_EVENT_NAME="push",
                   GITHUB_REF="refs/heads/" + pilot.BRANCH, GITHUB_JOB="pilot-e2e",
                   GITHUB_WORKFLOW_REF=f"Djoe-Denne/IAMRusty/{pilot.WORKFLOW}@refs/heads/{pilot.BRANCH}")
        self.assertTrue(runtime.pilot_hosted(env))
        for field in env:
            self.assertFalse(runtime.pilot_hosted({**env, field: "foreign"}))


if __name__ == "__main__":
    unittest.main()
