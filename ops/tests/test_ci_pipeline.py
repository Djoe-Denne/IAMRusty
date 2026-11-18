"""CI DAG, cache provenance, selection, partitions and report refusal regressions."""

import copy
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import Mock, patch

import yaml

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/ci"))
import pipeline as p
import compiler_cache as cache
import reports
import build


IDENTITY = {"GITHUB_SHA": "a" * 40, "GITHUB_RUN_ID": "123", "GITHUB_RUN_ATTEMPT": "1"}


def fixture_metadata(root):
    packages = []
    for group, (prefix, _) in p.GROUPS.items():
        names = p.INVENTORY["packages"][group]
        for name in names:
            packages.append({
                "name": name,
                "manifest_path": str(root / prefix / name / "Cargo.toml"),
                "dependencies": [], "features": {},
                "targets": [
                    {"name": name.replace("-", "_"), "kind": ["lib"], "test": True},
                    {"name": "case_a", "kind": ["test"], "test": True},
                    {"name": "case_b", "kind": ["test"], "test": True},
                    {"name": "live", "kind": ["test"], "test": True, "required-features": ["live"]},
                ],
            })
    telegraph = next(package for package in packages if package["name"] == "telegraph-service")
    telegraph["targets"].extend(
        {"name": name, "kind": ["test"], "test": True}
        for name in ("notification_created_routing_test", "user_email_verified_event_test")
    )
    return {"packages": packages}


class PlannerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "rust-toolchain.toml").write_text('[toolchain]\nchannel="1.99.1"\n')
        self.data = fixture_metadata(self.root)
        environment = patch.dict(os.environ, {"GITHUB_OUTPUT": ""})
        environment.start()
        self.addCleanup(environment.stop)

    def test_registered_inventory_preserves_baseline_not_all_workspace_coverage(self):
        p.validate_inventory(self.data, self.root)
        self.assertEqual(p.coverage_packages(self.data, "lazaret", self.root), [])
        self.assertEqual(set(p.INVENTORY["builds"]), {
            "iam", "telegraph", "hive", "manifesto", "monolith", "sentinel-sync", "ext-authz", "oauth"})
        self.assertEqual(build.build_arguments("iam")[:2], ["build", "--locked"])

    def test_unknown_workspace_package_is_not_silently_omitted(self):
        self.data["packages"].append({
            "name": "unknown", "manifest_path": str(self.root / "new/Cargo.toml")})
        with self.assertRaisesRegex(ValueError, "Unregistered"):
            p.validate_inventory(self.data, self.root)

    def test_new_package_under_known_owner_is_also_rejected(self):
        self.data["packages"].append({
            "name": "new-iam-crate",
            "manifest_path": str(self.root / "services/IAMRusty/new-crate/Cargo.toml")})
        with self.assertRaisesRegex(ValueError, "Unregistered"):
            p.validate_inventory(self.data, self.root)

    def test_master_unknown_shared_manifest_and_submodule_changes_select_everything(self):
        for event, paths in [
            ("push", ["docs/guide.md"]), ("pull_request", ["unknown/file"]),
            ("pull_request", ["crates/iam-events/src/lib.rs"]),
            ("pull_request", ["rustycog"]),
            ("pull_request", ["services/Hive/Cargo.toml"]),
            ("pull_request", ["AGENTS.md"]),
        ]:
            with self.subTest(event=event, paths=paths):
                self.assertEqual(p.selected_groups(paths, self.data, event=event, root=self.root), set(p.GROUPS))

    def test_plain_documentation_pr_can_be_selective_but_not_fixtures(self):
        self.assertEqual(p.selected_groups(["docs/guide.md"], self.data,
                                          event="pull_request", root=self.root), set())
        self.assertEqual(p.selected_groups(["docs/fixture/Cargo.toml"], self.data,
                                          event="pull_request", root=self.root), set(p.GROUPS))

    def test_source_change_includes_runtime_consumers(self):
        groups = p.selected_groups(["services/Telegraph/domain/src/lib.rs"],
                                   self.data, event="pull_request", root=self.root)
        self.assertTrue({"telegraph", "sentinel-sync", "monolith"}.issubset(groups))
        self.assertNotIn("readiness", groups)

    def test_cargo_reverse_dependency_is_included(self):
        producer = next(x for x in self.data["packages"] if x["name"] == "hive-domain")
        consumer = next(x for x in self.data["packages"] if x["name"] == "iam-domain")
        consumer["dependencies"] = [{"path": str(Path(producer["manifest_path"]).parent)}]
        groups = p.selected_groups(["services/Hive/domain/src/lib.rs"],
                                   self.data, event="pull_request", root=self.root)
        self.assertIn("iam", groups)

    def test_whole_targets_are_complete_disjoint_stable_and_live_features_stay_off(self):
        for group in p.INVENTORY["coverage"]:
            targets = p.test_targets(self.data, group, self.root)
            rows = [p.test_partition(targets, group, i) for i in range(p.GROUPS[group][1])]
            flattened = [target for row in rows for target in row]
            self.assertCountEqual(flattened, targets)
            self.assertEqual(len(flattened), len(set(flattened)))
            self.assertFalse(any(name == "live" for _, name in targets))
            for row in p.matrix_for(self.data, group, self.root)["include"]:
                commands = p.coverage_commands(self.data, group, row["lane"], row["shard"], self.root)
                self.assertTrue(commands)
                for command in commands:
                    self.assertNotIn("--tests", command)
                    self.assertIn("--no-report", command)
                    if row["lane"] == "tests":
                        self.assertNotIn("--lib", command)
                if row["lane"] == "tests":
                    executed = [
                        f"{command[command.index('--package') + 1]}|test|{command[index + 1]}"
                        for command in commands for index, value in enumerate(command)
                        if value == "--test"
                    ]
                    self.assertCountEqual(
                        executed, p.target_assignment(self.data, group, "tests", row["shard"], self.root))
        with self.assertRaises(ValueError):
            p.partition([], 2, 2)

    def test_telegraph_preserves_legacy_first_shard_and_splits_only_the_second(self):
        targets = p.test_targets(self.data, "telegraph", self.root)
        rows = [p.test_partition(targets, "telegraph", index) for index in range(4)]
        self.assertEqual(rows[0], p.partition(targets, 2, 0))
        self.assertEqual(rows[1], [("telegraph-service", "notification_created_routing_test")])
        self.assertEqual(rows[3], [("telegraph-service", "user_email_verified_event_test")])
        self.assertCountEqual(rows[1] + rows[2] + rows[3], p.partition(targets, 2, 1))
        self.assertEqual(p.matrix_for(self.data, "telegraph", self.root)["include"], [
            {"lane": "unit", "shard": 0},
            *[{"lane": "tests", "shard": index} for index in range(4)],
        ])
        with self.assertRaises(ValueError):
            p.test_partition(targets, "telegraph", 4)

    def test_failed_tests_are_not_masked_by_successful_reporting(self):
        output = self.root / "out/lcov.info"
        calls = []

        def run(command, **kwargs):
            calls.append(command)
            if "report" in command:
                output.write_text("TN:\nSF:source.rs\nDA:1,1\nend_of_record\n")
                return type("Result", (), {"returncode": 0})()
            return type("Result", (), {"returncode": 7})()

        status = p.run_coverage(self.data, "iam", "unit", 0, output, self.root, run,
                                {**IDENTITY, "CI_COVERAGE_SCOPE": "full"})
        self.assertEqual(status, 7)
        receipt = json.loads(output.with_suffix(".json").read_text())
        self.assertEqual(receipt["test_status"], 7)
        self.assertEqual(calls[-1][0:3], ["cargo", "llvm-cov", "report"])
        with self.assertRaisesRegex(ValueError, "stale"):
            p.run_coverage(self.data, "iam", "unit", 0, output, self.root, run,
                           {**IDENTITY, "CI_COVERAGE_SCOPE": "full"})

    def complete_receipts(self):
        directory = self.root / "coverage"
        directory.mkdir()
        for group in p.INVENTORY["coverage"]:
            for row in p.matrix_for(self.data, group, self.root)["include"]:
                name = f"lcov-{group}-{row['lane']}-{row['shard']}.info"
                report = directory / name
                report.write_text("TN:\nSF:source.rs\nDA:1,1\nend_of_record\n")
                value = {
                    "schema": "aiforall.ci-coverage/v1", "identity": IDENTITY, "scope": "full",
                    "inventory_sha256": hashlib.sha256(p.INVENTORY_PATH.read_bytes()).hexdigest(),
                    "toolchain_sha256": hashlib.sha256((self.root / "rust-toolchain.toml").read_bytes()).hexdigest(),
                    "service": group, "lane": row["lane"], "shard": row["shard"],
                    "features": p.FEATURES.get(group, []),
                    "targets": p.target_assignment(self.data, group, row["lane"], row["shard"], self.root),
                    "test_status": 0, "report_status": 0, "report": name,
                    "report_sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
                }
                report.with_suffix(".json").write_text(json.dumps(value))
        return directory

    def test_coverage_requires_complete_compatible_successful_and_disjoint_receipts(self):
        directory = self.complete_receipts()
        self.assertTrue(p.validate_reports(self.data, directory, identity=IDENTITY, root=self.root))
        receipt = next(directory.glob("*.json"))
        original = receipt.read_text()
        for key, value in [
            ("scope", "partial"), ("identity", {**IDENTITY, "GITHUB_RUN_ATTEMPT": "2"}),
            ("targets", []), ("test_status", 1), ("report_status", 1),
            ("report_sha256", "f" * 64), ("report", "../outside.info"),
            ("toolchain_sha256", "f" * 64), ("inventory_sha256", "f" * 64),
        ]:
            changed = json.loads(original)
            changed[key] = value
            receipt.write_text(json.dumps(changed))
            with self.subTest(key=key), self.assertRaises(ValueError):
                p.validate_reports(self.data, directory, identity=IDENTITY, root=self.root)
        receipt.write_text(original)
        duplicate = directory / "duplicate.json"
        duplicate.write_text(original)
        with self.assertRaises(ValueError):
            p.validate_reports(self.data, directory, identity=IDENTITY, root=self.root)
        duplicate.unlink()
        receipt.unlink()
        with self.assertRaises(ValueError):
            p.validate_reports(self.data, directory, identity=IDENTITY, root=self.root)

    def test_failed_telegraph_receipt_names_the_test_failure_not_the_successful_report(self):
        directory = self.complete_receipts()
        receipt = directory / "lcov-telegraph-tests-0.json"
        value = json.loads(receipt.read_text())
        value["test_status"] = 101
        receipt.write_text(json.dumps(value))
        with self.assertRaisesRegex(
                ValueError,
                r"lcov-telegraph-tests-0\.json.*test_status \(test_status=101, report_status=0\)"):
            p.validate_reports(self.data, directory, identity=IDENTITY, root=self.root)

    def test_receipt_identity_failure_is_distinguished_from_test_failure(self):
        directory = self.complete_receipts()
        receipt = directory / "lcov-iam-unit-0.json"
        value = json.loads(receipt.read_text())
        value["identity"]["GITHUB_RUN_ATTEMPT"] = "2"
        receipt.write_text(json.dumps(value))
        with self.assertRaisesRegex(ValueError, r"lcov-iam-unit-0\.json.*: identity$"):
            p.validate_reports(self.data, directory, identity=IDENTITY, root=self.root)

    def merge_tracefiles(self, command, **kwargs):
        self.assertEqual(command[0], "lcov")
        self.assertTrue(all(Path(command[index + 1]).is_file()
                            for index, value in enumerate(command) if value == "--add-tracefile"))
        Path(command[command.index("--output-file") + 1]).write_text(
            "TN:\nSF:source.rs\nDA:1,1\nend_of_record\n")
        return type("Result", (), {"returncode": 0})()

    def test_failed_test_command_still_merges_diagnostically_without_rewriting_evidence(self):
        directory = self.complete_receipts()
        receipt = directory / "lcov-telegraph-tests-3.json"
        value = json.loads(receipt.read_text())
        value["test_status"] = 101
        receipt.write_text(json.dumps(value))
        original = receipt.read_bytes()
        runner = Mock(side_effect=self.merge_tracefiles)
        output = self.root / "merged"
        self.assertEqual(p.aggregate_coverage(self.data, directory, output, identity=IDENTITY,
                                             root=self.root, run=runner), 0)
        runner.assert_called_once()
        self.assertEqual(receipt.read_bytes(), original)
        self.assertFalse((output / "authoritative").exists())
        summary = json.loads((output / "diagnostic/coverage-summary.json").read_text())
        self.assertFalse(summary["authoritative"])
        self.assertTrue(summary["reports_complete"])
        self.assertFalse(summary["all_test_commands_succeeded"])
        self.assertEqual(summary["execution_completeness"], "unproven")
        self.assertEqual(summary["failed_partitions"], [
            dict(service="telegraph", lane="tests", shard=3, test_status=101, report_status=0)])
        with self.assertRaises(ValueError):
            p.verify_merged_coverage(output / "diagnostic", identity=IDENTITY, root=self.root)
        with self.assertRaisesRegex(ValueError, "test_status"):
            p.validate_reports(self.data, directory, identity=IDENTITY, root=self.root)

    def test_successful_complete_merge_publishes_and_verifies_authoritative_coverage(self):
        directory = self.complete_receipts()
        output = self.root / "merged"
        self.assertEqual(p.aggregate_coverage(self.data, directory, output, identity=IDENTITY,
                                             root=self.root, run=self.merge_tracefiles), 0)
        canonical = output / "authoritative"
        summary = p.verify_merged_coverage(canonical, identity=IDENTITY, root=self.root)
        self.assertTrue(summary["authoritative"])
        self.assertEqual(summary["failed_partitions"], [])
        self.assertEqual((canonical / "lcov.info").read_bytes(),
                         (output / "diagnostic/diagnostic-lcov.info").read_bytes())
        manifest = canonical / "coverage-summary.json"
        original = manifest.read_text()
        for field, changed in (
            ("authoritative", False), ("authoritative", 1),
            ("identity", {**IDENTITY, "GITHUB_RUN_ATTEMPT": "2"}),
            ("report_sha256", "f" * 64), ("expected_partitions", []),
            ("failed_partitions", [{"test_status": 101}]),
        ):
            bad = json.loads(original)
            bad[field] = changed
            manifest.write_text(json.dumps(bad))
            with self.subTest(field=field, changed=changed), self.assertRaises(ValueError):
                p.verify_merged_coverage(canonical, identity=IDENTITY, root=self.root)
        bad = json.loads(original)
        bad["partitions"][0]["test_status"] = 101
        manifest.write_text(json.dumps(bad))
        with self.assertRaisesRegex(ValueError, "unsuccessful"):
            p.verify_merged_coverage(canonical, identity=IDENTITY, root=self.root)

    def test_diagnostic_merge_still_rejects_invalid_outcomes_and_reports_before_running_lcov(self):
        directory = self.complete_receipts()
        receipt = next(directory.glob("*.json"))
        original = receipt.read_text()
        for field, changed in (
            ("report_status", 1), ("report_status", False), ("test_status", False),
            ("test_status", "101"), ("report_sha256", "f" * 64),
            ("identity", {**IDENTITY, "GITHUB_RUN_ID": "999"}),
            ("targets", []),
        ):
            bad = json.loads(original)
            bad[field] = changed
            receipt.write_text(json.dumps(bad))
            runner = Mock()
            with self.subTest(field=field, changed=changed), self.assertRaises(ValueError):
                p.aggregate_coverage(self.data, directory, self.root / "merged",
                                     identity=IDENTITY, root=self.root, run=runner)
            runner.assert_not_called()
        receipt.write_text(original)
        duplicate = directory / "duplicate.json"
        duplicate.write_text(original)
        with self.assertRaises(ValueError):
            p.aggregate_coverage(self.data, directory, self.root / "merged",
                                 identity=IDENTITY, root=self.root, run=Mock())
        duplicate.unlink()
        receipt.unlink()
        with self.assertRaises(ValueError):
            p.aggregate_coverage(self.data, directory, self.root / "merged",
                                 identity=IDENTITY, root=self.root, run=Mock())

    def test_merger_failure_or_missing_output_does_not_publish_qualification(self):
        directory = self.complete_receipts()
        output = self.root / "failed-merge"
        runner = Mock(return_value=type("Result", (), {"returncode": 9})())
        self.assertEqual(p.aggregate_coverage(self.data, directory, output, identity=IDENTITY,
                                             root=self.root, run=runner), 9)
        self.assertFalse((output / "diagnostic/coverage-summary.json").exists())
        self.assertFalse((output / "authoritative").exists())
        with self.assertRaisesRegex(ValueError, "nonempty"):
            p.aggregate_coverage(self.data, directory, self.root / "missing-output",
                                 identity=IDENTITY, root=self.root,
                                 run=Mock(return_value=type("Result", (), {"returncode": 0})()))

    def test_sonar_verification_cli_never_runs_cargo_metadata(self):
        with patch.dict(os.environ, IDENTITY), patch.object(p, "metadata") as metadata, patch.object(
                p, "verify_merged_coverage") as verify:
            self.assertEqual(p.main(["verify-merged", "--directory", str(self.root)]), 0)
            verify.assert_called_once()
            metadata.assert_not_called()

    def test_merge_qualification_output_is_emitted_only_after_successful_aggregation(self):
        directory = self.complete_receipts()
        output_file = self.root / "github-output"
        with patch.dict(os.environ, {"GITHUB_OUTPUT": str(output_file)}):
            failed = Mock(return_value=type("Result", (), {"returncode": 9})())
            self.assertEqual(p.aggregate_coverage(
                self.data, directory, self.root / "failed", identity=IDENTITY,
                root=self.root, run=failed), 9)
            self.assertFalse(output_file.exists())
            receipt = directory / "lcov-telegraph-tests-3.json"
            value = json.loads(receipt.read_text())
            value["test_status"] = 101
            receipt.write_text(json.dumps(value))
            self.assertEqual(p.aggregate_coverage(
                self.data, directory, self.root / "diagnostic-result", identity=IDENTITY,
                root=self.root, run=self.merge_tracefiles), 0)
            self.assertEqual(output_file.read_text(), "authoritative=false\n")
            value["test_status"] = 0
            receipt.write_text(json.dumps(value))
            self.assertEqual(p.aggregate_coverage(
                self.data, directory, self.root / "canonical-result", identity=IDENTITY,
                root=self.root, run=self.merge_tracefiles), 0)
            self.assertEqual(output_file.read_text(), "authoritative=false\nauthoritative=true\n")


class BuildTests(unittest.TestCase):
    def test_all_builds_keep_their_packages_without_precompiling_test_targets(self):
        for service, packages in p.INVENTORY["builds"].items():
            with self.subTest(service=service):
                self.assertEqual(build.build_arguments(service), [
                    "build", "--locked",
                    *[arg for package in packages for arg in ("--package", package)],
                ])

    def test_only_a_successful_build_exports_the_exact_command_for_native_provenance(self):
        with tempfile.TemporaryDirectory() as temporary:
            environment = Path(temporary) / "github-env"
            for status in (101, 0):
                with self.subTest(status=status), patch.dict(os.environ, {"GITHUB_ENV": str(environment)}), \
                        patch.object(sys, "argv", ["build.py", "--service", "iam"]), \
                        patch.object(build.subprocess, "run",
                                     return_value=Mock(returncode=status)) as runner:
                    self.assertEqual(build.main(), status)
                    actual = runner.call_args.args[0]
                    self.assertEqual(actual, ["cargo", *build.build_arguments("iam")])
                    if status:
                        self.assertFalse(environment.exists())
                    else:
                        text = environment.read_text()
                        self.assertTrue(text.startswith("K6_BUILD_ARGS_JSON="))
                        self.assertEqual(json.loads(text.split("=", 1)[1]), actual[1:])
                        self.assertNotIn("--all-targets", text)


class WorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = yaml.load((ROOT / ".github/workflows/ci.yml").read_text(), Loader=yaml.BaseLoader)
        cls.jobs = cls.workflow["jobs"]

    def test_each_test_waits_only_for_its_own_build(self):
        for service in ("iam", "telegraph", "hive", "manifesto", "sentinel"):
            self.assertEqual(self.jobs[f"test-{service}"]["needs"], [f"build-{service}", "plan"])
        self.assertEqual(self.jobs["k6-e2e"]["needs"], ["build-iam", "build-oauth", "build-ext-authz"])
        self.assertEqual(self.jobs["clippy"]["needs"], ["fmt"])
        self.assertEqual(self.jobs["sonar"]["needs"], ["merge-coverage", "clippy"])
        self.assertNotIn("build", self.jobs)

    def test_diagnostic_merge_and_canonical_sonar_admission_are_separate(self):
        merge = self.jobs["merge-coverage"]
        self.assertIn("authoritative", merge["outputs"])
        uploads = {step["with"]["name"]: step for step in merge["steps"]
                   if "upload-artifact@" in step.get("uses", "")}
        self.assertIn("coverage-diagnostic", uploads)
        self.assertNotIn("if", uploads["coverage-diagnostic"])
        self.assertIn("authoritative == 'true'", uploads["lcov"]["if"])
        self.assertIn("authoritative == 'true'", self.jobs["sonar"]["if"])
        self.assertTrue(any("verify-merged" in step.get("run", "")
                            for step in self.jobs["sonar"]["steps"]))
        coverage = yaml.load((ROOT / ".github/workflows/ci-coverage.yml").read_text(),
                             Loader=yaml.BaseLoader)
        self.assertNotIn("continue-on-error", str(coverage))

    def test_cache_is_shared_without_target_archives_and_installer_is_pinned(self):
        action = yaml.load((ROOT / ".github/actions/setup-compiler-cache/action.yml").read_text(),
                           Loader=yaml.BaseLoader)
        steps = action["runs"]["steps"]
        registry = steps[0]["with"]
        self.assertEqual(registry["cache-targets"], "false")
        self.assertEqual(registry["cache-bin"], "false")
        self.assertIn("build-iam", registry["save-if"])
        self.assertFalse(any("mozilla-actions" in x.get("uses", "") for x in steps))
        self.assertTrue(any("compiler_cache.py install" in x.get("run", "") for x in steps))
        bridge = yaml.load((ROOT / ".github/actions/export-cache-runtime/action.yml").read_text(),
                           Loader=yaml.BaseLoader)
        self.assertNotIn("post", bridge["runs"])
        source = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertNotIn("Swatinem/rust-cache@", source)
        self.assertIn("CI_SELECTIVE_PR_TESTS", source)
        self.assertFalse(any("cargo " in step.get("run", "") for step in self.jobs["sonar"]["steps"]))
        self.assertNotIn("continue-on-error", str(self.jobs["sonar"]))


class ReportTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("node"), "Node is required for the credential bridge boundary")
    def test_runtime_bridge_selects_v2_with_real_capabilities_even_without_inherited_flag(self):
        with tempfile.TemporaryDirectory() as temporary:
            env_file = Path(temporary) / "github-env"
            token = "fixture-runtime-token-not-a-secret"
            environment = dict(os.environ, GITHUB_ENV=str(env_file),
                               ACTIONS_RESULTS_URL="https://results.example.invalid/",
                               ACTIONS_RUNTIME_TOKEN=token)
            environment.pop("ACTIONS_CACHE_SERVICE_V2", None)
            result = subprocess.run(
                [shutil.which("node"), str(ROOT / ".github/actions/export-cache-runtime/index.js")],
                env=environment, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            exported = env_file.read_text()
            self.assertRegex(exported, r"ACTIONS_CACHE_SERVICE_V2<<[^\n]+\ntrue\n")
            self.assertIn("ACTIONS_RESULTS_URL", exported)
            self.assertIn("ACTIONS_RUNTIME_TOKEN", exported)
            self.assertNotIn("SCCACHE_GHA_ENABLED", exported)
            self.assertEqual(result.stdout, f"::add-mask::{token}\n")

    @unittest.skipUnless(shutil.which("node"), "Node is required for the credential bridge boundary")
    def test_runtime_bridge_disables_remote_cache_without_credentials(self):
        with tempfile.TemporaryDirectory() as temporary:
            env_file = Path(temporary) / "github-env"
            environment = dict(os.environ, GITHUB_ENV=str(env_file),
                               SCCACHE_GHA_VERSION="aiforall-rust-v1")
            environment.pop("ACTIONS_RESULTS_URL", None)
            environment.pop("ACTIONS_RUNTIME_TOKEN", None)
            result = subprocess.run(
                [shutil.which("node"), str(ROOT / ".github/actions/export-cache-runtime/index.js")],
                env=environment, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("SCCACHE_GHA_ENABLED", env_file.read_text())
            self.assertIn("\nfalse\n", env_file.read_text())
            self.assertRegex(env_file.read_text(), r"SCCACHE_GHA_VERSION<<[^\n]+\n\n")
            self.assertNotIn("ACTIONS_RUNTIME_TOKEN", env_file.read_text())

    def archive(self, binary=b"approved fixture", extra=False):
        buffer = io.BytesIO()
        name = f"sccache-v{cache.POLICY['version']}-{cache.POLICY['platform']}/sccache"
        with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
            info = tarfile.TarInfo(name)
            info.size = len(binary)
            archive.addfile(info, io.BytesIO(binary))
            if extra:
                info = tarfile.TarInfo(str(Path(name).parent / "python3").replace("\\", "/"))
                info.size = 1
                archive.addfile(info, io.BytesIO(b"x"))
        return buffer.getvalue()

    def test_altered_archive_is_rejected_before_any_file_or_execution(self):
        approved = self.archive()
        altered = self.archive(extra=True)
        policy = {**cache.POLICY, "archive_sha256": hashlib.sha256(approved).hexdigest(),
                  "executable_sha256": hashlib.sha256(b"approved fixture").hexdigest()}
        with tempfile.TemporaryDirectory() as temporary, patch.object(
                cache.subprocess, "run") as execute:
            destination = Path(temporary) / "not-created"
            with self.assertRaisesRegex(ValueError, "archive digest"):
                cache.install_archive(altered, destination, policy)
            execute.assert_not_called()
            self.assertFalse(destination.exists())

    def test_approved_archive_exposes_only_the_verified_executable(self):
        raw = self.archive()
        policy = {**cache.POLICY, "archive_sha256": hashlib.sha256(raw).hexdigest(),
                  "executable_sha256": hashlib.sha256(b"approved fixture").hexdigest()}
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "installed"
            value, executable = cache.install_archive(
                raw, directory, policy, lambda path, selected: cache.expected_attestation(selected))
            self.assertEqual(list(directory.iterdir()), [executable])
            self.assertEqual(executable.read_bytes(), b"approved fixture")
            self.assertEqual(value, cache.expected_attestation(policy))

    def test_rejected_executable_is_never_run_for_statistics(self):
        with tempfile.TemporaryDirectory() as temporary:
            exe = Path(temporary) / "sccache"
            exe.write_bytes(b"rejected executable")
            with patch.object(cache.shutil, "which", return_value=str(exe)), patch.object(
                    cache.subprocess, "run") as execute:
                with self.assertRaisesRegex(ValueError, "digest"):
                    cache.statistics()
                execute.assert_not_called()

    def full_needs(self):
        workflow = yaml.load((ROOT / ".github/workflows/ci.yml").read_text(), Loader=yaml.BaseLoader)
        needs = {name: {"result": "success"} for name in workflow["jobs"]["ci-status"]["needs"]}
        needs["plan"]["outputs"] = {"selection": json.dumps(
            {**{name: True for name in p.GROUPS}, "full": True, "any": True})}
        needs["merge-coverage"]["outputs"] = {"authoritative": "true"}
        return needs

    def test_summary_does_not_accept_skipped_selected_jobs_or_an_unevaluated_sonar(self):
        needs = self.full_needs()
        self.assertIn("Complete", reports.status(needs, event="push", ref="refs/heads/master"))
        for name in ("test-iam", "clippy", "sonar", "k6-e2e", "plan"):
            changed = copy.deepcopy(needs)
            changed[name]["result"] = "skipped"
            with self.subTest(name=name), self.assertRaises(ValueError):
                reports.status(changed, event="push", ref="refs/heads/master")

    def test_partial_pr_is_explicit_and_cannot_qualify_master_or_sonar(self):
        needs = self.full_needs()
        selection = {name: False for name in p.GROUPS}
        selection.update(full=False, any=False)
        needs["plan"]["outputs"]["selection"] = json.dumps(selection)
        for name in needs:
            if name not in ("fmt", "plan", "infra-static", "clippy", "apparatus-p0"):
                needs[name]["result"] = "skipped"
        self.assertIn("NOT evaluated", reports.status(needs, event="pull_request", ref="refs/pull/1/merge"))
        with self.assertRaises(ValueError):
            reports.status(needs, event="push", ref="refs/heads/master")
        needs["sonar"]["result"] = "success"
        with self.assertRaises(ValueError):
            reports.status(needs, event="pull_request", ref="refs/pull/1/merge")

    def test_diagnostic_merge_does_not_become_a_failed_merge_or_a_passed_sonar(self):
        needs = self.full_needs()
        needs["merge-coverage"]["outputs"]["authoritative"] = "false"
        needs["test-telegraph"]["result"] = "failure"
        needs["sonar"]["result"] = "skipped"
        with self.assertRaisesRegex(
                ValueError, "CI jobs did not succeed: test-telegraph.*merged for diagnosis"):
            reports.status(needs, event="push", ref="refs/heads/master")
        needs["test-telegraph"]["result"] = "success"
        with self.assertRaisesRegex(ValueError, "not authoritative despite successful"):
            reports.status(needs, event="push", ref="refs/heads/master")
        needs["sonar"]["result"] = "success"
        with self.assertRaisesRegex(ValueError, "must not qualify Sonar"):
            reports.status(needs, event="push", ref="refs/heads/master")
        needs["sonar"]["result"] = "skipped"
        needs["merge-coverage"]["outputs"] = {}
        with self.assertRaisesRegex(ValueError, "no qualification decision"):
            reports.status(needs, event="push", ref="refs/heads/master")

    def test_clippy_requires_both_successful_terminal_records_and_exact_concatenation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            terminal = b'{"reason":"build-finished","success":true}\n'
            for lane in ("production", "tests"):
                (root / f"clippy-{lane}.jsonl").write_bytes(terminal)
            (root / "clippy.json").write_bytes(terminal + terminal)
            self.assertEqual(reports.clippy_evidence(root, IDENTITY)["identity"], IDENTITY)
            for bad in (b"", b"not json", b'{"reason":"build-finished","success":false}\n'):
                (root / "clippy-tests.jsonl").write_bytes(bad)
                with self.assertRaises((ValueError, json.JSONDecodeError)):
                    reports.clippy_evidence(root, IDENTITY)

    def test_cache_executable_digest_and_version_are_both_checked(self):
        with tempfile.TemporaryDirectory() as temporary:
            exe = Path(temporary) / "sccache"
            exe.write_bytes(b"fixture")
            with patch.object(cache.shutil, "which", return_value=str(exe)):
                with self.assertRaisesRegex(ValueError, "digest"):
                    cache.attest()
                with patch.dict(cache.POLICY, executable_sha256=hashlib.sha256(b"fixture").hexdigest()):
                    with patch.object(cache.subprocess, "run", return_value=type(
                            "Result", (), {"stdout": "sccache wrong"})()):
                        with self.assertRaisesRegex(ValueError, "version"):
                            cache.attest()
                    with patch.object(cache.subprocess, "run", return_value=type(
                            "Result", (), {"stdout": "sccache " + cache.POLICY["version"]})()):
                        self.assertEqual(cache.attest(), cache.expected_attestation())


if __name__ == "__main__":
    unittest.main()
