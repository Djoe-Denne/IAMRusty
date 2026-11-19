"""Static and data-only checks for the IAM compile-once experiment."""

from copy import deepcopy
from datetime import datetime
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
import iam_ac_inputs
import iam_ac_runtime
import iam_warm_compiler as warm
import runtime


class PilotTests(unittest.TestCase):
    def test_iam_benchmark_is_branch_and_workflow_bound(self):
        ref = f"refs/heads/{pilot.AC_BRANCH}"
        repo = "Djoe-Denne/IAMRusty"
        env = dict(GITHUB_ACTIONS="true", GITHUB_REPOSITORY=repo, GITHUB_REF=ref,
                   GITHUB_WORKFLOW_REF=f"{repo}/{pilot.AC_WORKFLOW}@{ref}", GITHUB_EVENT_NAME="push",
                    GITHUB_JOB="e2e", IAM_AC_ARM="c")
        self.assertEqual(pilot.run_lane(env), (pilot.AC_WORKFLOW, pilot.AC_BRANCH, "ac"))
        self.assertEqual(runtime.benchmark_arm(env), "c")
        self.assertEqual(runtime.benchmark_arm(dict(env, IAM_AC_ARM="d")), "d")
        for field, value in (("GITHUB_WORKFLOW_REF", f"{repo}/{pilot.CI_WORKFLOW}@{ref}"),
                             ("GITHUB_EVENT_NAME", "workflow_dispatch"), ("GITHUB_REPOSITORY", "other/IAMRusty"),
                             ("GITHUB_REF", "refs/heads/master"), ("GITHUB_REF", "refs/heads/ci-iam-cd")):
            foreign = {**env, field: value}
            self.assertIsNone(runtime.benchmark_arm(foreign))
            with self.assertRaises(ValueError):
                pilot.run_lane(foreign)
        self.assertIsNone(runtime.benchmark_arm(dict(env, GITHUB_JOB="build-a")))
        self.assertIsNone(runtime.benchmark_arm(dict(env, IAM_AC_ARM="b")))

    def test_ac_producer_and_partition_are_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / "binary"
            executable.write_bytes(b"\x7fELF\x02\x01" + bytes(10) + b"\x03\x00\x3e\x00" + bytes(70))
            known = {"lane": "ac", "producer_job": "build-c-tests"}
            d_source = pilot.part_identity(known, "e2e-d")
            self.assertEqual(pilot.part_identity(known, "e2e")["producer_job"], "build-c-server")
            self.assertEqual(d_source["producer_job"], "build-d-server")
            self.assertEqual(pilot.part_identity(known, "tests-0")["producer_job"], "build-c-tests")
            archive = root / "bundle.tar.gz"
            pilot.bundle(archive, d_source, "e2e-d", [("iam-service", ".", "iam-service", executable)])
            for identity, part in ((known, "e2e"), (known, "e2e-d"), (d_source, "e2e"),
                                   ({**d_source, "lane": "ci"}, "e2e-d")):
                with self.assertRaisesRegex(ValueError, "identity mismatch"):
                    pilot.unpack(archive, root / "refused", identity, part, {"iam-service"})
            self.assertFalse((root / "refused").exists())
            pilot.unpack(archive, root / "valid", d_source, "e2e-d", {"iam-service"})

    def test_ac_dag_cannot_delay_k6_for_test_compilation(self):
        config = yaml.load((ROOT / pilot.AC_WORKFLOW).read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(config["on"], {"push": {"branches": [pilot.AC_BRANCH]}})
        jobs = config["jobs"]
        for arm in ("c", "d"):
            server, tests = f"build-{arm}-server", f"build-{arm}-tests"
            self.assertEqual(jobs[f"coverage-{arm}"]["needs"], [tests])
            self.assertEqual(jobs[f"coverage-{arm}"]["with"]["benchmark-arm"], arm)
            self.assertEqual(jobs[f"e2e-{arm}"]["needs"], [server, "native-oauth", "native-ext-authz", "prepare-runtime"])
            self.assertEqual(jobs[f"e2e-{arm}"]["with"]["arm"], arm)
        self.assertEqual(set(config["jobs"]), {
            "prepare-cargo", "prepare-runtime", "build-c-server", "build-c-tests", "build-d-server", "build-d-tests",
            "native-oauth", "native-ext-authz", "coverage-c", "coverage-d", "e2e-c", "e2e-d",
        })
        for job in ("build-c-server", "build-c-tests", "build-d-server", "build-d-tests", "native-oauth", "native-ext-authz"):
            self.assertEqual(jobs[job]["needs"], ["prepare-cargo"])
            self.assertEqual(jobs[job]["runs-on"], "ubuntu-24.04")
            steps = jobs[job]["steps"]
            restores = [i for i, s in enumerate(steps) if "iam_warm_compiler.py" in s.get("run", "")]
            if job.startswith("build-d-"):
                self.assertEqual(len(restores), 1)
                build = next(i for i, s in enumerate(steps) if "iam_compile_once.py produce" in s.get("run", ""))
                setup = next(i for i, s in enumerate(steps) if s.get("uses") == "./.github/actions/setup-compiler-cache")
                self.assertLess(setup, restores[0])
                self.assertLess(restores[0], build)
            else:
                self.assertFalse(restores)
        self.assertNotIn("iam-cd-d-compiler-", (ROOT / pilot.AC_WORKFLOW).read_text())
        with self.assertRaisesRegex(ValueError, "Unknown pilot artifact"):
            pilot.fetch("compiler", None)

    def test_prior_seed_is_bound_to_exact_run_producer_and_artifact(self):
        repo = dict(id=warm.REPO_ID, full_name=warm.REPO)
        run = dict(id=warm.SEED_RUN, run_attempt=1, path=pilot.AC_WORKFLOW, event="push",
                   head_branch=warm.SEED_BRANCH, head_sha=warm.SEED_SOURCE, status="completed",
                   conclusion="failure", repository=repo, head_repository=repo.copy())
        job = dict(id=warm.SEED_JOB, name="D instrumented IAM with cache export", run_id=warm.SEED_RUN,
                   run_attempt=1, head_sha=warm.SEED_SOURCE, status="completed", conclusion="success",
                   steps=[dict(name="Upload only the compiler cache", conclusion="success",
                               started_at="2026-11-19T11:52:10Z", completed_at="2026-11-19T11:52:13Z")])
        artifact = dict(warm.SEED_ARTIFACT, expired=False, expires_at="2026-11-20T11:52:10Z",
                        created_at="2026-11-19T11:52:13Z",
                        workflow_run=dict(id=warm.SEED_RUN, head_sha=warm.SEED_SOURCE, head_branch=warm.SEED_BRANCH,
                                          repository_id=warm.REPO_ID, head_repository_id=warm.REPO_ID))
        expected = {"actions/runs/44518086358/attempts/1": run, "actions/jobs/135539506878": job,
                    "actions/artifacts/14527570584": artifact}
        now = datetime.fromisoformat("2026-11-19T12:00:00Z")
        api = mock.Mock()
        api.json.side_effect = expected.__getitem__
        self.assertEqual(warm.select_seed(api, now), warm.SEED_ARTIFACT)
        refusals = (
            (run, "run_attempt", 2), (run, "event", "pull_request"), (run, "path", pilot.CI_WORKFLOW),
            (run, "head_branch", "master"), (run, "head_sha", "0" * 40),
            (run, "head_repository", {**repo, "id": 5}), (job, "id", 5), (job, "run_attempt", 2),
            (job, "conclusion", "failure"), (job, "steps", []),
            (artifact, "workflow_run", dict(artifact["workflow_run"], head_repository_id=5)),
            (artifact, "workflow_run", dict(artifact["workflow_run"], id=5)),
            (artifact, "id", 5), (artifact, "name", "other"),
            (artifact, "digest", "sha256:" + "0" * 64), (artifact, "size_in_bytes", 1),
            (artifact, "created_at", "2026-11-19T11:52:14Z"), (artifact, "expired", True),
            (artifact, "expires_at", "2026-11-19T11:52:13Z"),
        )
        for obj, field, value in refusals:
            with self.subTest(field=field, value=value):
                mutated = deepcopy(expected)
                mutated[next(k for k, v in expected.items() if v is obj)][field] = value
                api.json.side_effect = mutated.__getitem__
                with self.assertRaises(ValueError):
                    warm.select_seed(api, now)

    def test_only_warm_producers_can_restore_and_seed_rejection_never_downloads(self):
        known = dict(lane="ac", repository_id=str(warm.REPO_ID), sdk=warm.SEED_SDK)
        for name in ("build-d-server", "build-d-tests"):
            warm.require_consumer(known, {"GITHUB_JOB": name})
        for name in ("build-c-server", "build-c-tests", "coverage", "e2e"):
            with self.assertRaises(ValueError):
                warm.require_consumer(known, {"GITHUB_JOB": name})
        for different in (dict(known, lane="ci"), dict(known, repository_id="1"), dict(known, sdk="foreign")):
            with self.assertRaises(ValueError):
                warm.require_consumer(different, {"GITHUB_JOB": "build-d-server"})
        with tempfile.TemporaryDirectory() as directory:
            cache = Path(directory) / "iam-ac-sccache"
            cache.mkdir()
            env = dict(GITHUB_JOB="build-d-server", GH_TOKEN="mock-token", RUNNER_TEMP=directory, SCCACHE_DIR=str(cache))
            with (mock.patch.object(pilot, "identity", return_value=known),
                  mock.patch.object(warm, "GitHub") as api,
                  mock.patch.object(warm, "select_seed", side_effect=ValueError("Expired"))):
                with self.assertRaisesRegex(ValueError, "Expired"):
                    warm.restore(env)
                api.return_value.download.assert_not_called()
                self.assertFalse(any(cache.iterdir()))

    def test_compiler_snapshot_accepts_only_bounded_local_cache_entries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            restored = root / "restored"
            key = "ab" + "f" * 62
            archive = root / "bundle.tar.gz"
            data = b"compiler output entry"
            with tarfile.open(archive, "w:gz") as bundle:
                entry = tarfile.TarInfo(f"a/b/{key}")
                entry.size = len(data)
                bundle.addfile(entry, io.BytesIO(data))
            restored.mkdir()
            warm.restore_snapshot(archive, restored)
            self.assertEqual((restored / "a/b" / key).read_bytes(), data)
            self.assertFalse((restored / "target").exists())
            with self.assertRaisesRegex(ValueError, "fresh compiler cache"):
                warm.restore_snapshot(archive, restored)
            for index, (name, mode) in enumerate((("../outside", tarfile.REGTYPE),
                                                 ("target/CACHEDIR.TAG", tarfile.REGTYPE),
                                                 (f"a/b/{key}", tarfile.SYMTYPE),
                                                 (f"x/b/{key}", tarfile.REGTYPE))):
                unsafe = root / f"unsafe-{index}.tar.gz"
                with tarfile.open(unsafe, "w:gz") as bundle:
                    entry = tarfile.TarInfo(name)
                    entry.type = mode
                    if mode == tarfile.REGTYPE:
                        entry.size = 1
                    bundle.addfile(entry, io.BytesIO(b"x") if entry.size else None)
                out = root / f"unsafe-{index}"
                out.mkdir()
                with self.assertRaisesRegex(ValueError, "Unexpected compiler-cache member"):
                    warm.restore_snapshot(unsafe, out)
                self.assertFalse(any(out.iterdir()))
            with mock.patch.object(warm, "MAX_BYTES", len(data) - 1):
                with self.assertRaisesRegex(ValueError, "Unexpected compiler-cache member"):
                    warm.restore_snapshot(archive, out)
                self.assertFalse(any(out.iterdir()))

    def test_runtime_staging_keeps_original_dockerfile_entrypoints_without_apt(self):
        for role, path in {"iam": "services/IAMRusty/Dockerfile", "oauth": "services/OAuthConnect/Dockerfile",
                           "ext-authz": "ops/deploy/build/Dockerfile.ext-authz"}.items():
            source = (ROOT / path).read_text(encoding="utf-8")
            staged = iam_ac_runtime.stage_recipe(role, source)
            for instruction in ("USER ", "ENTRYPOINT ", "HEALTHCHECK ", "COPY "):
                self.assertEqual([line for line in source.splitlines() if line.startswith(instruction)],
                                 [line for line in staged.splitlines() if line.startswith(instruction)])
            self.assertNotIn("apt-get", staged)
            with self.assertRaisesRegex(ValueError, "RECIPE_DRIFT"):
                iam_ac_runtime.stage_recipe(role, staged)

    def test_cargo_seed_has_no_toolchain_secrets_or_links(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original"
            for part in ("cache", "index"):
                path = original / "registry" / part
                path.mkdir(parents=True)
                (path / "locked").write_bytes(b"download")
            (original / "credentials.toml").write_text("not exported", encoding="utf-8")
            iam_ac_inputs.registry_archive(original, root / "bundle.tar.gz")
            result = root / "restored"
            iam_ac_inputs.restore_registry(root / "bundle.tar.gz", result)
            self.assertEqual((result / "registry/cache/locked").read_bytes(), b"download")
            self.assertFalse((result / "credentials.toml").exists())
            for index, (name, mode) in enumerate((("registry/cache/../../escaped", tarfile.REGTYPE),
                                                 ("registry/cache/linked", tarfile.SYMTYPE))):
                with tarfile.open(root / "unsafe.tar.gz", "w:gz") as archive:
                    member = tarfile.TarInfo(name)
                    member.type = mode
                    archive.addfile(member)
                with self.assertRaisesRegex(ValueError, "Unexpected Cargo seed member"):
                    iam_ac_inputs.restore_registry(root / "unsafe.tar.gz", root / f"unsafe-{index}")

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
