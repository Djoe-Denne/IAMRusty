"""Fixture-only tests for the read-only GitHub issue inventory."""

from contextlib import redirect_stderr, redirect_stdout
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "github-lint-inventory.py"
SPEC = importlib.util.spec_from_file_location("github_lint_inventory", SCRIPT)
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)


def issue(number=1, title="An issue", body=None, labels=None, **kwargs):
    return {
        "number": number, "title": title, "body": body,
        "labels": [] if labels is None else labels, "state": "open",
        "html_url": f"https://github.com/example/repo/issues/{number}", **kwargs,
    }


def report(data):
    return inventory.inventory(json.dumps(data), {"mode": "offline", "input": "fixture.json"})


class InventoryTests(unittest.TestCase):
    def test_multi_page_filter_deduplicate_and_retain_evidence(self):
        result = report([
            [issue(3, "clippy::ptr_arg", body="services/Hive/src/lib.rs"),
             issue(2, state="closed"), issue(4, pull_request={})],
            [issue(1), issue(3, "clippy::ptr_arg", body="workers/sync/src/main.rs"),
             issue(5, state="closed", pull_request={})],
        ])
        self.assertEqual([item["number"] for item in result["issues"]], [1, 3])
        self.assertEqual(result["source"]["page_count"], 2)
        self.assertEqual(result["counts"], {
            "retrieved_records": 6, "excluded_pull_requests": 2, "excluded_closed_issues": 1,
            "open_issue_records": 3, "duplicate_open_issue_records": 1, "unique_open_issues": 2,
            "by_source_family": {"clippy": 1, "sonar": 0, "mixed": 0, "other": 1},
        })
        lint = result["issues"][1]
        self.assertEqual(lint["areas"], ["services/Hive", "workers/sync"])
        self.assertEqual(lint["dispersion"], "multiple_areas")
        self.assertEqual(lint["observed_pages"], [1, 2])
        self.assertEqual(len([e for e in lint["evidence"] if e["kind"] == "path"]), 2)

    def test_empty_arrays_are_valid_not_errors(self):
        for data in ([], [[]], [[], []]):
            with self.subTest(data=data):
                result = report(data)
                self.assertEqual(result["issues"], [])
                self.assertEqual(result["counts"]["unique_open_issues"], 0)
                self.assertTrue(result["completeness"]["all_records_in_retrieved_pages_processed"])
                self.assertEqual(result["completeness"]["repository_snapshot"], "not_proven")

    def test_source_families_and_exact_anchors(self):
        cases = [
            (issue(title="clippy::unknown_future_lint"), "clippy"),
            (issue(title="rust:S2208"), "sonar"),
            (issue(title="secrets:S6706 and clippy::ptr_arg"), "mixed"),
            (issue(title="SonarQube report"), "sonar"),
            (issue(labels=[{"name": "lint:clippy"}]), "clippy"),
            (issue(labels=["sonarcloud"]), "sonar"),
            (issue(labels=["clippy::ptr_arg"]), "clippy"),
            (issue(labels=["rust:S2208"]), "sonar"),
            (issue(title="unclippy and sonarization", labels=["not-clippy", "sonarish"]), "other"),
            (issue(title="Regular feature request"), "other"),
        ]
        for fixture, family in cases:
            with self.subTest(fixture=fixture):
                self.assertEqual(report([fixture])["issues"][0]["source_family"], family)

    def test_unknowns_stay_in_inventory_without_body_or_commands(self):
        body = "An unknown diagnostic. SECRET_FIXTURE_TOKEN; $(do-not-run)"
        result = report([issue(body=body)])
        item = result["issues"][0]
        self.assertEqual(item["rules"], [])
        self.assertEqual(item["rules_status"], "unknown")
        self.assertEqual(item["paths"], [])
        self.assertEqual(item["paths_status"], "unknown")
        self.assertEqual(item["dispersion"], "unknown")
        self.assertNotIn("SECRET_FIXTURE_TOKEN", inventory.render(result))
        self.assertNotIn("body", item)

    def test_public_api_and_single_file_never_prove_local_impact(self):
        item = report([issue(
            title="Local public API fix clippy::needless_pass_by_value",
            body="Change pub fn in services/IAMRusty/src/lib.rs",
        )])["issues"][0]
        self.assertEqual(item["dispersion"], "single_area")
        self.assertEqual(item["impact"], "needs_review")

    def test_paths_are_explicit_relative_hints_only(self):
        text = (
            "`./services/Hive/src/lib.rs:42` crates/core/src/lib.rs "
            "runtime/host/src/main.rs ops/scripts/tool.py Cargo.toml "
            "../services/Bad/src/a.rs /services/Bad/src/b.rs "
            "C:/services/Bad/src/c.rs https://host/services/Bad/src/d.rs "
            "services/Bad/../secret.rs services/AreaOnly"
        )
        item = report([issue(body=text)])["issues"][0]
        self.assertEqual(item["paths"], [
            "Cargo.toml", "crates/core/src/lib.rs", "ops/scripts/tool.py",
            "runtime/host/src/main.rs", "services/Hive/src/lib.rs",
        ])
        self.assertEqual(item["areas"], ["crates/core", "runtime/host", "services/Hive"])

    def test_non_area_path_does_not_manufacture_area(self):
        item = report([issue(body="docs/README.md")])["issues"][0]
        self.assertEqual(item["paths"], ["docs/README.md"])
        self.assertEqual(item["areas"], [])
        self.assertEqual(item["dispersion"], "unknown")

    def test_reject_malformed_json_and_unsupported_shapes(self):
        for text in ("", "{", "null", "{}", '{"message":"denied"}', "[1]", '[[],{}]', "[NaN]",
                     json.dumps([issue(extra=float("inf"))]),
                     "[[[{}]]]", '[{"number":1}]'):
            with self.subTest(text=text):
                with self.assertRaises(inventory.InventoryError):
                    inventory.parse_pages(text)

    def test_reject_malformed_issue_fields_even_if_closed(self):
        for changes in (
            {"number": True}, {"number": -1}, {"title": None}, {"html_url": 12},
            {"state": "unknown"}, {"body": {}}, {"labels": None},
            {"labels": [{}]}, {"labels": [12]}, {"pull_request": None},
        ):
            with self.subTest(changes=changes):
                fixture = issue(state="closed")
                fixture.update(changes)
                with self.assertRaises(inventory.InventoryError):
                    report([fixture])

    def test_read_only_gh_argv_and_live_metadata(self):
        completed = subprocess.CompletedProcess([], 0, stdout="[[]]", stderr="")
        with patch.object(inventory.subprocess, "run", return_value=completed) as run:
            text, metadata = inventory.fetch("Djoe-Denne/IAMRusty")
        run.assert_called_once_with(
            ["gh", "api", "--method", "GET", "repos/Djoe-Denne/IAMRusty/issues",
             "--paginate", "--slurp", "-f", "state=open", "-f", "per_page=100"],
            shell=False, check=False, capture_output=True, text=True, encoding="utf-8",
        )
        self.assertEqual(inventory.inventory(text, metadata)["completeness"]["pagination"],
                         "gh_paginate_completed")

    def test_repo_injection_rejected_before_subprocess(self):
        for repo in ("", "--help", "a/b;whoami", "a/b/c", "https://github.com/a/b",
                     "a/b\n", "a/..", "../b", "a/b?state=all", "a/b c"):
            with self.subTest(repo=repo), patch.object(inventory.subprocess, "run") as run:
                with self.assertRaises(inventory.InventoryError):
                    inventory.fetch(repo)
                run.assert_not_called()

    def test_rule_label_evidence_is_exact_not_substring(self):
        item = report([issue(labels=["CLIPPY::PTR_ARG", "RUST:s2208", "not-clippy::made_up"])])["issues"][0]
        self.assertEqual(item["rules"], ["clippy::ptr_arg", "rust:S2208"])
        self.assertEqual(item["source_family"], "mixed")
        self.assertTrue(all(e["field"] == "label" for e in item["evidence"]))

    def test_fetch_failure_not_zero_and_no_raw_stderr(self):
        with patch.object(inventory.subprocess, "run", return_value=subprocess.CompletedProcess(
            [], 7, stdout="[]", stderr="SECRET_FIXTURE_TOKEN",
        )):
            err, out = io.StringIO(), io.StringIO()
            with redirect_stderr(err), redirect_stdout(out):
                code = inventory.main(["--repo", "owner/repo"])
        self.assertEqual(code, 1)
        self.assertIn("exit 7", err.getvalue())
        self.assertNotIn("SECRET_FIXTURE_TOKEN", err.getvalue())
        self.assertEqual(out.getvalue(), "")

    def test_missing_gh_fails_clearly(self):
        with patch.object(inventory.subprocess, "run", side_effect=FileNotFoundError):
            with self.assertRaisesRegex(inventory.InventoryError, "gh could not run"):
                inventory.fetch("owner/repo")

    def test_malformed_fetch_success_fails_at_cli(self):
        with patch.object(inventory, "fetch", return_value=('{"message":"bad"}', {"mode": "gh"})):
            with redirect_stderr(io.StringIO()), redirect_stdout(io.StringIO()) as out:
                self.assertEqual(inventory.main(["--repo", "owner/repo"]), 1)
            self.assertEqual(out.getvalue(), "")

    def test_deterministic_sorted_output_and_duplicate_metadata_conflict(self):
        fixtures = [issue(2, labels=["z", "a"]), issue(1, "B"), issue(1, "A")]
        one = report(fixtures)
        two = report(list(reversed(fixtures)))
        # Raw source digest intentionally reflects input bytes, not semantic ordering.
        one["source"].pop("sha256")
        two["source"].pop("sha256")
        self.assertEqual(inventory.render(one), inventory.render(two))
        self.assertEqual(one["issues"][0]["title"], "A")
        self.assertTrue(one["issues"][0]["display_metadata_conflict"])
        self.assertEqual(inventory.render(report(fixtures)), inventory.render(report(fixtures)))

    def test_cli_offline_stdout_output_and_failure_preserves_output(self):
        with tempfile.TemporaryDirectory() as directory:
            source, output = Path(directory) / "pages.json", Path(directory) / "out.json"
            source.write_text("[[]]", encoding="utf-8")
            with redirect_stdout(io.StringIO()) as stdout:
                self.assertEqual(inventory.main(["--input", str(source)]), 0)
            self.assertEqual(json.loads(stdout.getvalue())["issues"], [])
            self.assertEqual(inventory.main(["--input", str(source), "--output", str(output)]), 0)
            original = output.read_bytes()
            source.write_text("invalid", encoding="utf-8")
            with redirect_stderr(io.StringIO()):
                self.assertEqual(inventory.main(["--input", str(source), "--output", str(output)]), 1)
            self.assertEqual(output.read_bytes(), original)

    def test_cli_requires_exactly_one_source(self):
        for args in ([], ["--repo", "owner/repo", "--input", "fixture.json"]):
            with self.subTest(args=args), redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as error:
                    inventory.main(args)
                self.assertEqual(error.exception.code, 2)

    def test_empty_repo_cli_fails_without_fetching(self):
        with patch.object(inventory.subprocess, "run") as run:
            with redirect_stderr(io.StringIO()) as stderr:
                self.assertEqual(inventory.main(["--repo", ""]), 1)
            self.assertIn("invalid repository", stderr.getvalue())
            run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
