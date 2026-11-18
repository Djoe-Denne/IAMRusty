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
from unittest.mock import patch

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
    return {"packages": packages}


class PlannerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "rust-toolchain.toml").write_text('[toolchain]\nchannel="1.99.1"\n')
        self.data = fixture_metadata(self.root)

    def test_registered_inventory_preserves_baseline_not_all_workspace_coverage(self):
        p.validate_inventory(self.data, self.root)
        self.assertEqual(p.coverage_packages(self.data, "lazaret", self.root), [])
        self.assertEqual(set(p.INVENTORY["builds"]), {
            "iam", "telegraph", "hive", "manifesto", "monolith", "sentinel-sync", "ext-authz", "oauth"})
        self.assertEqual(build.build_arguments("iam")[:3], ["build", "--locked", "--all-targets"])

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
            rows = [p.partition(targets, p.GROUPS[group][1], i) for i in range(p.GROUPS[group][1])]
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
        with self.assertRaises(ValueError):
            p.partition([], 2, 2)

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
