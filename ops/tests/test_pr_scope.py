"""Regression tests for advisory service scope and changed-code evidence."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from test_ci_pipeline import fixture_metadata
import pr_scope as scope


class ServiceScopeTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name).resolve()
        self.data = fixture_metadata(self.root)

    def dependency(self, consumer, producer, kind=None):
        packages = {item["name"]: item for item in self.data["packages"]}
        packages[consumer]["dependencies"].append({
            "path": str(Path(packages[producer]["manifest_path"]).parent), "kind": kind,
        })

    def test_cargo_build_edges_not_runtime_or_dev_edges_expand_builds(self):
        self.dependency("oodhive-monolith", "telegraph-domain")
        self.dependency("hive-service", "telegraph-domain", "dev")
        plan = scope.shadow_plan(
            ["services/Telegraph/telegraph-domain/src/lib.rs"], self.data, self.root
        )
        self.assertEqual(set(plan["build"]), {"telegraph", "monolith"})
        self.assertIn("hive", plan["integration"])
        self.assertIn("Conservative runtime dependency on telegraph", plan["integration"]["sentinel-sync"])

    def test_fixture_rust_change_only_selects_own_integration_lane(self):
        plan = scope.shadow_plan(
            ["services/Telegraph/telegraph-service/tests/fixtures/data.rs"], self.data, self.root
        )
        self.assertEqual(plan["build"], {})
        self.assertEqual(set(plan["integration"]), {"telegraph"})

    def test_unknown_global_and_unmapped_inputs_default_to_full(self):
        for paths in (None, [], ["Cargo.lock"], ["unexpected/file.rs"],
                      ["services/Hive/hive-service/config/default.toml"],
                      ["crates/iam-events/src/lib.rs"], ["rustycog"]):
            with self.subTest(paths=paths):
                self.assertTrue(scope.shadow_plan(paths, self.data, self.root)["full"])

    def test_docs_only_keep_build_and_integration_lists_empty(self):
        plan = scope.shadow_plan(["docs/guide.md"], self.data, self.root)
        self.assertEqual(plan, {"full": False, "build": {}, "integration": {}})

    def test_changes_in_two_owners_are_unioned(self):
        plan = scope.shadow_plan(
            ["services/Telegraph/telegraph-service/src/old.rs",
             "services/Hive/hive-service/src/new.rs"], self.data, self.root,
        )
        self.assertEqual(set(plan["build"]), {"telegraph", "hive"})


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name).resolve()
        self.ranges = {"src/lib.rs": [(2, 4)]}

    def test_docs_fast_path_requires_allowlisted_regular_markdown(self):
        guide = "docs/guides/ci-cache-and-service-lanes.md"
        with patch.object(scope, "git", return_value=b"100644 blob sha\tfile\0"):
            self.assertTrue(scope.docs_only([guide, "README.md"], "base", "merge", self.root))
            for paths in ([], [guide, "Cargo.lock"], ["docs/fixtures/README.md"],
                          ["docs/guides/example.rs"], ["services/Telegraph/README.md"]):
                with self.subTest(paths=paths):
                    self.assertFalse(scope.docs_only(paths, "base", "merge", self.root))
        for mode in (b"120000 blob ", b"100755 blob ", b"160000 commit "):
            with patch.object(scope, "git", side_effect=[b"100644 blob sha\tfile\0", mode]):
                self.assertFalse(scope.docs_only([guide], "base", "merge", self.root))
        with patch.object(scope, "git", side_effect=[b"100644 blob sha\tfile\0", b""]):
            self.assertTrue(scope.docs_only([guide], "base", "merge", self.root))

    def test_lcov_counts_zero_and_unknown_separately(self):
        report = [
            f"SF:{(self.root / 'src/lib.rs').as_posix()}\n", "DA:1,200\n",
            "DA:2,0\n", "DA:3,1\n", "end_of_record\n",
            "SF:src/lib.rs\n", "DA:3,0\n", "end_of_record\n",
            "SF:src/foreign.rs\n", "DA:4,99\n", "end_of_record\n",
        ]
        self.assertEqual(scope.patch_coverage(report, self.ranges, self.root), {
            "src/lib.rs": {"instrumented": 2, "covered": 1, "unmapped": 1},
        })

    def test_clippy_import_only_retains_warnings_with_a_changed_primary_span(self):
        source = self.root / "src/lib.rs"
        source.parent.mkdir()
        source.touch()
        external = self.root.parent / "external.rs"

        def warning(path, start, end, *, primary=True, code="clippy::fixture"):
            return json.dumps({
                "reason": "compiler-message", "message": {
                    "level": "warning", "code": {"code": code},
                    "spans": [{"file_name": str(path), "line_start": start,
                               "line_end": end, "is_primary": primary}],
                },
            }) + "\n"

        wanted = warning(source, 1, 2)
        lines = [wanted, warning("src/lib.rs", 5, 5), warning("src/lib.rs", 3, 3, primary=False),
                 warning(external, 3, 3), warning("src/lib.rs", 3, 3, code="unused_variables"),
                 '{"reason":"build-finished","success":true}\n']
        self.assertEqual(list(scope.filter_clippy(lines, self.ranges, self.root)), [wanted])

    def test_exact_merge_tree_and_added_line_ranges(self):
        def git(*args):
            return subprocess.check_output(
                ["git", "-c", "core.autocrlf=false", *args], cwd=self.root, text=True
            ).strip()

        def commit(message):
            git("add", "--all")
            git("-c", "user.name=CI Test", "-c", "user.email=ci@example.invalid",
                "-c", "commit.gpgsign=false", "-c", "core.hooksPath=" + str(self.root / "no-hooks"),
                "commit", "-m", message)
            return git("rev-parse", "HEAD")

        git("init", "-b", "master")
        file = self.root / "code[1].rs"
        old = self.root / "removed.rs"
        file.write_text("a\nb\nc\nd\n", encoding="utf-8")
        old.write_text("only\n", encoding="utf-8")
        base = commit("base")
        git("switch", "-c", "topic")
        file.write_text("a\nB\nc\n", encoding="utf-8")
        old.unlink()
        head = commit("topic")
        git("switch", "master")
        git("-c", "user.name=CI Test", "-c", "user.email=ci@example.invalid",
            "-c", "commit.gpgsign=false", "-c", "core.hooksPath=" + str(self.root / "no-hooks"),
            "merge", "--no-ff", "--no-edit", head)
        tested = git("rev-parse", "HEAD")
        paths = scope.pr_paths(base, head, tested, self.root)
        self.assertCountEqual(paths, ["code[1].rs", "removed.rs"])
        self.assertEqual(scope.changed_rust_ranges(base, tested, paths, self.root),
                         {"code[1].rs": [(2, 2)]})
        with self.assertRaisesRegex(ValueError, "merge parents"):
            scope.pr_paths(head, base, tested, self.root)


if __name__ == "__main__":
    unittest.main()
