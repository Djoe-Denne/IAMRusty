"""Static and data-only checks for the IAM compile-once experiment."""

import io
import json
import os
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
    def test_iam_benchmark_is_branch_and_workflow_bound(self):
        ref = f"refs/heads/{pilot.AB_BRANCH}"
        repo = "Djoe-Denne/IAMRusty"
        env = dict(GITHUB_ACTIONS="true", GITHUB_REPOSITORY=repo, GITHUB_REF=ref,
                   GITHUB_WORKFLOW_REF=f"{repo}/{pilot.AB_WORKFLOW}@{ref}", GITHUB_EVENT_NAME="push",
                   GITHUB_JOB="e2e", IAM_AB_ARM="a")
        self.assertEqual(pilot.run_lane(env), (pilot.AB_WORKFLOW, pilot.AB_BRANCH, "ab"))
        self.assertEqual(runtime.benchmark_arm(env), "a")
        for field, value in (("GITHUB_WORKFLOW_REF", f"{repo}/{pilot.CI_WORKFLOW}@{ref}"),
                             ("GITHUB_EVENT_NAME", "workflow_dispatch"), ("GITHUB_REPOSITORY", "other/IAMRusty"),
                             ("GITHUB_REF", "refs/heads/master")):
            foreign = {**env, field: value}
            self.assertIsNone(runtime.benchmark_arm(foreign))
            with self.assertRaises(ValueError):
                pilot.run_lane(foreign)
        self.assertIsNone(runtime.benchmark_arm(dict(env, GITHUB_JOB="build-a")))
        self.assertIsNone(runtime.benchmark_arm(dict(env, IAM_AB_ARM="other")))

    def test_ab_producer_and_partition_are_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / "binary"
            executable.write_bytes(b"\x7fELF\x02\x01" + bytes(10) + b"\x03\x00\x3e\x00" + bytes(70))
            known = {"lane": "ab", "producer_job": "build-b"}
            a_source = pilot.part_identity(known, "native-e2e")
            archive = root / "bundle.tar.gz"
            pilot.bundle(archive, a_source, "native-e2e", [("iam-service", ".", "iam-service", executable)])
            for identity, part in ((known, "e2e"), (known, "native-e2e"), (a_source, "e2e"),
                                   ({**a_source, "lane": "ci"}, "native-e2e")):
                with self.assertRaisesRegex(ValueError, "identity mismatch"):
                    pilot.unpack(archive, root / "refused", identity, part, {"iam-service"})
            self.assertFalse((root / "refused").exists())
            pilot.unpack(archive, root / "valid", a_source, "native-e2e", {"iam-service"})

    def test_ab_dag_delays_only_b_for_the_shared_test_producer(self):
        config = yaml.load((ROOT / pilot.AB_WORKFLOW).read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(config["on"], {"push": {"branches": [pilot.AB_BRANCH]}})
        for arm in ("a", "b"):
            jobs = config["jobs"]
            self.assertEqual(jobs[f"coverage-{arm}"]["needs"], [f"build-{arm}"])
            self.assertEqual(jobs[f"coverage-{arm}"]["with"]["benchmark-arm"], arm)
            self.assertEqual(jobs[f"e2e-{arm}"]["needs"], [f"build-{arm}", "native-oauth", "native-ext-authz"])
            self.assertEqual(jobs[f"e2e-{arm}"]["with"]["arm"], arm)
        self.assertEqual(set(config["jobs"]), {
            "build-a", "build-b", "native-oauth", "native-ext-authz", "coverage-a", "coverage-b", "e2e-a", "e2e-b",
        })

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

    def test_current_ci_refuses_untrusted_events_and_foreign_workflows(self):
        repo = "Djoe-Denne/IAMRusty"
        ref = "refs/heads/master"
        env = {"GITHUB_ACTIONS": "true", "GITHUB_REPOSITORY": repo,
               "GITHUB_EVENT_NAME": "push", "GITHUB_REF": ref,
               "GITHUB_WORKFLOW_REF": f"{repo}/{pilot.CI_WORKFLOW}@{ref}"}
        self.assertEqual(pilot.run_lane(env), (pilot.CI_WORKFLOW, "master", "ci"))
        merge = "refs/pull/123/merge"
        pr = dict(env, GITHUB_EVENT_NAME="pull_request", GITHUB_REF=merge,
                  GITHUB_HEAD_REF="contributor", GITHUB_BASE_REF="master",
                  GITHUB_WORKFLOW_REF=f"{repo}/{pilot.CI_WORKFLOW}@{merge}")
        self.assertEqual(pilot.run_lane(pr), (pilot.CI_WORKFLOW, "contributor", "ci"))
        for field, value in (("GITHUB_EVENT_NAME", "pull_request_target"), ("GITHUB_BASE_REF", "foreign"),
                             ("GITHUB_WORKFLOW_REF", env["GITHUB_WORKFLOW_REF"]),
                             ("GITHUB_REPOSITORY", "other/IAMRusty")):
            with self.subTest(field=field), self.assertRaises(ValueError):
                pilot.run_lane({**pr, field: value})

    def test_current_ci_schema_cannot_be_read_as_pilot_or_another_attempt(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "binary").write_bytes(b"\x7fELF\x02\x01" + bytes(10) + b"\x03\x00\x3e\x00" + bytes(70))
            archive = root / "bundle.tar.gz"
            provenance = {"lane": "ci", "GITHUB_RUN_ATTEMPT": "1"}
            expected = {"iam-service|test|example"}
            pilot.bundle(archive, provenance, "tests-0",
                         [("iam-service|test|example", "services/IAMRusty", "test-000", root / "binary")])
            for wrong in ({**provenance, "lane": "pilot"}, {**provenance, "GITHUB_RUN_ATTEMPT": "2"}):
                with self.assertRaisesRegex(ValueError, "identity mismatch"):
                    pilot.unpack(archive, root / "refused", wrong, "tests-0", expected)
            self.assertFalse((root / "refused").exists())

    def test_canonical_replay_cannot_hide_failed_execution_or_export(self):
        known = {"lane": "ci", "GITHUB_SHA": "a" * 40, "GITHUB_RUN_ID": "42", "GITHUB_RUN_ATTEMPT": "1"}
        for result, error, expected in (((7, 0), None, (7, 0)), ((7, 1), None, (7, 1)),
                                         (None, OSError("cannot execute"), (1, 1))):
            with self.subTest(error=error), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                settings = dict(known, RUNNER_TEMP=directory, CI_COVERAGE_SCOPE="full")
                settings.pop("lane")
                with mock.patch.dict(os.environ, settings), \
                        mock.patch.object(pilot, "identity", return_value=known), \
                        mock.patch.object(pilot.ci, "target_assignment", return_value=["iam-domain|lib|iam_domain"]), \
                        mock.patch.object(pilot, "fetch"), \
                        mock.patch.object(pilot, "replay", return_value=result, side_effect=error), \
                        mock.patch.object(pilot.ci, "write_coverage_receipt", return_value=expected[0] or expected[1]) as write:
                    self.assertNotEqual(pilot.canonical_coverage({}, "unit", 0, root / "lcov.info", root), 0)
                self.assertEqual(write.call_args.args[-3:-1], expected)

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

    def test_hosted_pilot_creates_the_private_parent_before_provisioning(self):
        config = yaml.load((ROOT / pilot.WORKFLOW).read_text(), Loader=yaml.BaseLoader)
        steps = config["jobs"]["pilot-e2e"]["steps"]
        commands = "\n".join(step.get("run", "") for step in steps)
        root = "$RUNNER_TEMP/iam-pilot-private"
        setup = f'mkdir -m 700 "{root}"'
        self.assertIn(setup, commands)
        for path in ("acquired", "images", "cli", "browser", "campaign"):
            use = commands.index(f'"{root}/{path}')
            self.assertLess(commands.index(setup), use)
        self.assertNotRegex(commands, r"\$RUNNER_TEMP/iam-pilot-(cli|browser|campaign|images|acquired)")

    def test_runtime_gate_has_no_caller_selectable_bypass(self):
        env = dict(GITHUB_REPOSITORY="Djoe-Denne/IAMRusty", GITHUB_EVENT_NAME="push",
                   GITHUB_REF="refs/heads/" + pilot.BRANCH, GITHUB_JOB="pilot-e2e",
                   GITHUB_WORKFLOW_REF=f"Djoe-Denne/IAMRusty/{pilot.WORKFLOW}@refs/heads/{pilot.BRANCH}")
        self.assertTrue(runtime.pilot_hosted(env))
        for field in env:
            self.assertFalse(runtime.pilot_hosted({**env, field: "foreign"}))


if __name__ == "__main__":
    unittest.main()
