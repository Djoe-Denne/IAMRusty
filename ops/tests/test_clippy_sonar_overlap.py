"""Synthetic, offline tests for conservative overlap accounting."""

import contextlib
import copy
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "clippy-sonar-overlap.py"
SPEC = importlib.util.spec_from_file_location("clippy_sonar_overlap", SCRIPT)
overlap = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(overlap)


def diagnostic(message="message", spans=None, rule="clippy::example"):
    if spans is None:
        spans = [span()]
    return {"reason": "compiler-message", "package_id": "package",
            "target": {"name": "example", "kind": ["lib"]},
            "message": {"level": "warning", "code": {"code": rule},
                        "message": message, "spans": spans}}


def span(file="src\\lib.rs", start=2, end=4):
    return {"file_name": file, "is_primary": True, "line_start": 10,
            "line_end": 11, "column_start": start, "column_end": end}


def issue(key="key", message="message", **changes):
    value = {"key": key, "rule": "external_clippy:example", "project": "project",
             "component": "project:src/lib.rs", "message": message, "line": 10,
             "textRange": {"startLine": 10, "endLine": 11, "startOffset": 1, "endOffset": 3}}
    value.update(changes)
    return value


def page(issues, total=None, index=1, size=100):
    return {"paging": {"pageIndex": index, "pageSize": size,
                       "total": len(issues) if total is None else total}, "issues": issues}


class OverlapTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.c = self.root / "clippy.jsonl"
        self.s = self.root / "sonar.json"

    def cargo(self, rows=None, path=None, finished=True):
        path = path or self.c
        rows = list(rows if rows is not None else [diagnostic()])
        if finished:
            rows.append({"reason": "build-finished", "success": True})
        path.write_text("\n".join(map(json.dumps, rows)), encoding="utf-8")
        return path

    def sonar(self, value=None):
        self.s.write_text(json.dumps(value if value is not None else page([issue()])), encoding="utf-8")
        return self.s

    def run_inventory(self, rows=None, value=None):
        return overlap.inventory([self.cargo(rows)], self.sonar(value))

    def test_full_range_column_conversion_and_links(self):
        result = self.run_inventory()
        self.assertEqual(result["counts"]["exact_intersection"], 1)
        self.assertEqual(result["counts"]["strict_union"], 1)
        self.assertEqual(result["clippy"][0]["primary_spans"][0]["range"], [10, 1, 11, 3])
        self.assertEqual(result["matches"][0]["sonar_issue_keys"], ["key"])
        self.assertEqual(result["matches"][0]["clippy_provenance"][0]["row"], 1)
        self.assertEqual(result["coverage_percent"]["clippy"], 100)

    def test_dedup_targets_and_lanes_preserves_provenance(self):
        row = diagnostic()
        other = copy.deepcopy(row)
        other["target"]["kind"] = ["test"]
        lane = self.root / "tests.jsonl"
        self.cargo([row, other])
        self.cargo([row], lane)
        result = overlap.inventory([lane, self.c], self.sonar())
        self.assertEqual(result["counts"]["clippy_unique"], 1)
        self.assertEqual(len(result["clippy"][0]["provenance"]), 3)
        self.assertEqual(result["source_stats"]["clippy"]["successful_files"], 2)
        self.assertEqual(result, overlap.inventory([self.c, lane], self.s))

    def test_sonar_duplicate_keys_and_sites(self):
        a = issue()
        result = self.run_inventory(value=page([a, a, issue("second")], total=2))
        self.assertEqual(result["counts"]["sonar_raw_keys"], 2)
        self.assertEqual(result["counts"]["sonar_unique_sites"], 1)
        self.assertEqual(result["sonar"][0]["issue_keys"], ["key", "second"])

    def test_paginated_input_and_order_independent_output(self):
        pages = [page([issue("a")], 2, 1, 1), page([issue("b")], 2, 2, 1)]
        first = self.run_inventory(value=pages)
        second = self.run_inventory(value=list(reversed(pages)))
        self.assertEqual(first, second)
        self.assertEqual(first["counts"]["sonar_raw_keys"], 2)

    def test_identical_key_can_repeat_between_complete_pages(self):
        pages = [page([issue("a"), issue("b")], 3, 1, 2),
                 page([issue("b"), issue("c")], 3, 2, 2)]
        result = self.run_inventory(value=pages)
        self.assertEqual(result["source_stats"]["sonar"]["raw_issue_rows"], 4)
        self.assertEqual(result["counts"]["sonar_raw_keys"], 3)
        self.assertEqual(result["counts"]["sonar_unique_sites"], 1)

    def test_empty_complete_inputs(self):
        result = self.run_inventory(rows=[], value=page([]))
        self.assertEqual(result["counts"]["strict_union"], 0)
        self.assertIsNone(result["coverage_percent"]["clippy"])

    def test_incomplete_or_incoherent_paging_fails(self):
        cases = [
            [], {}, {"issues": []}, page([], total=1),
            [page([issue()], 2, 1, 1)],
            [page([issue()], 2, 1, 1), page([issue("b")], 3, 2, 1)],
            [page([issue()], 1), page([issue()], 1)],
            page([issue()], index=2),
            dict(page([issue()]), total=2),
            page([issue()], size=True),
            dict(page([issue()]), p=True),
            page([issue(), issue("b")], size=1),
            [page([issue()], 2, 1, 1), page([issue()], 2, 2, 1)],
            [page([issue("a")], 3, 1, 2),
             page([issue("b"), issue("c")], 3, 2, 2)],
        ]
        for value in cases:
            with self.subTest(value=value), self.assertRaises(overlap.InputError):
                self.run_inventory(value=value)

    def test_conflicting_key_metadata_fails(self):
        for changed in [issue(message="different"), issue(severity="CRITICAL")]:
            with self.subTest(changed=changed), self.assertRaisesRegex(overlap.InputError, "conflicting"):
                self.run_inventory(value=page([issue(), changed], total=1))

    def test_every_clippy_input_needs_successful_terminal_build(self):
        self.cargo()
        second = self.root / "second.jsonl"
        for rows in [[], [diagnostic()], [{"reason": "build-finished", "success": False}],
                     [{"reason": "build-finished", "success": True}, diagnostic()]]:
            with self.subTest(rows=rows):
                self.cargo(rows, second, finished=False)
                with self.assertRaises(overlap.InputError):
                    overlap.inventory([self.c, second], self.sonar())

    def test_malformed_json_and_shapes_fail(self):
        for raw in ["{", "[]", "null", '{"reason": 5}', '{"reason":"compiler-message","message":[]}']:
            with self.subTest(raw=raw):
                self.c.write_text(raw, encoding="utf-8")
                with self.assertRaises(overlap.InputError):
                    overlap.inventory([self.c], self.sonar())
        self.cargo()
        for raw in ["{", "null", "5", "NaN"]:
            with self.subTest(raw=raw):
                self.s.write_text(raw, encoding="utf-8")
                with self.assertRaises(overlap.InputError):
                    overlap.inventory([self.c], self.s)

    def test_invalid_issue_and_span_shapes_fail(self):
        for changes in [{"key": ""}, {"component": None}, {"message": []},
                        {"textRange": {}}, {"line": 9},
                        {"textRange": {"startLine": 10, "endLine": 9, "startOffset": 0, "endOffset": 0}}]:
            with self.subTest(changes=changes), self.assertRaises(overlap.InputError):
                self.run_inventory(value=page([issue(**changes)]))
        bad_span = span()
        bad_span["column_start"] = 0
        with self.assertRaises(overlap.InputError):
            self.run_inventory(rows=[diagnostic(spans=[bad_span])])

    def test_missing_primary_and_sonar_range_stay_unmerged(self):
        result = self.run_inventory(rows=[diagnostic(spans=[])],
                                    value=page([issue(textRange=None)]))
        self.assertEqual(result["counts"]["exact_intersection"], 0)
        self.assertEqual(result["counts"]["strict_union"], 2)
        self.assertEqual(result["clippy"][0]["unresolved_reason"], "no_primary_span")
        self.assertEqual(result["sonar"][0]["unresolved_reason"], "missing_text_range")

    def test_secondary_only_span_and_missing_sonar_range_are_explicit(self):
        secondary = span()
        secondary["is_primary"] = False
        result = self.run_inventory(rows=[diagnostic(spans=[secondary])])
        self.assertEqual(result["clippy"][0]["unresolved_reason"], "no_primary_span")
        self.assertEqual(result["counts"]["exact_intersection"], 0)
        result = self.run_inventory(value=page([issue(textRange=None)]))
        self.assertEqual(result["counts"]["exact_intersection"], 0)
        self.assertEqual(len(result["unmerged_hints"]), 1)

    def test_multi_primary_stays_unmerged_with_hint(self):
        result = self.run_inventory(rows=[diagnostic(spans=[span(), span("src/other.rs")])])
        self.assertEqual(result["counts"]["exact_intersection"], 0)
        self.assertEqual(result["clippy"][0]["unresolved_reason"], "multi_primary_spans")
        self.assertEqual(len(result["unmerged_hints"]), 1)

    def test_ambiguous_exact_matches_stay_unmerged(self):
        # Distinct Sonar projects/components have the same declared relative path.
        result = self.run_inventory(value=page([
            issue(), issue("b", project="other", component="other:src/lib.rs")]))
        self.assertEqual(result["counts"]["exact_intersection"], 0)
        self.assertEqual(result["counts"]["strict_union"], 3)
        self.assertEqual(len(result["ambiguous_exact_matches"]), 1)
        self.assertEqual(result["clippy"][0]["unresolved_reason"], "ambiguous_exact_match")

    def test_dot_segments_are_not_resolved_or_folded(self):
        rows = [diagnostic(), diagnostic(spans=[span("src/other/../lib.rs")])]
        result = self.run_inventory(rows=rows)
        self.assertEqual(result["counts"]["clippy_unique"], 2)
        self.assertEqual(result["counts"]["exact_intersection"], 1)
        self.assertEqual(result["counts"]["strict_union"], 2)

    def test_matched_sites_do_not_generate_unresolved_hints(self):
        rows = [diagnostic(), diagnostic(spans=[span(end=5)])]
        second = issue("second")
        second["textRange"]["endOffset"] = 4
        result = self.run_inventory(rows=rows, value=page([issue(), second]))
        self.assertEqual(result["counts"]["exact_intersection"], 2)
        self.assertEqual(result["unmerged_hints"], [])

    def test_changed_message_is_separate_unmerged_hint(self):
        result = self.run_inventory(rows=[diagnostic(message="future is 17480 bytes")],
                                    value=page([issue(message="future is 17456 bytes")]))
        self.assertEqual(result["counts"]["exact_intersection"], 0)
        self.assertEqual(result["counts"]["strict_union"], 2)
        self.assertEqual(result["unmerged_hints"][0]["type"], "same_full_range_different_message")

    def test_changed_full_range_not_merged_even_with_same_start(self):
        result = self.run_inventory(rows=[diagnostic(spans=[span(end=5)])])
        self.assertEqual(result["counts"]["exact_intersection"], 0)
        self.assertEqual(len(result["unmerged_hints"]), 1)
        self.assertEqual(result["counts"]["clippy_unmatched"], 1)
        self.assertEqual(result["counts"]["sonar_unmatched"], 1)

    def test_no_substring_matching_or_native_rule_synonyms(self):
        for value in [issue(message="message longer"), issue(rule="rust:S107")]:
            with self.subTest(value=value):
                result = self.run_inventory(value=page([value]))
                self.assertEqual(result["counts"]["exact_intersection"], 0)

    def test_only_clippy_warning_error_diagnostics_count(self):
        rows = [diagnostic(), diagnostic(rule="dead_code"), {"reason": "compiler-artifact"}]
        note = diagnostic()
        note["message"]["level"] = "note"
        rows.append(note)
        result = self.run_inventory(rows=rows)
        self.assertEqual(result["counts"]["clippy_unique"], 1)
        self.assertEqual(result["source_stats"]["clippy"]["excluded_compiler_messages"], 2)
        self.assertEqual(result["source_stats"]["clippy"]["excluded_non_compiler_rows"], 1)

    def test_clippy_error_diagnostic_counts_without_becoming_failed_build(self):
        row = diagnostic()
        row["message"]["level"] = "error"
        result = self.run_inventory(rows=[row])
        self.assertEqual(result["counts"]["exact_intersection"], 1)
        self.assertEqual(result["clippy"][0]["provenance"][0]["level"], "error")

    def test_cli_failure_preserves_output_and_has_nonzero_status(self):
        self.cargo(finished=False)
        self.sonar()
        output = self.root / "output.json"
        output.write_text("preserve me", encoding="utf-8")
        err, out = io.StringIO(), io.StringIO()
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(out):
            status = overlap.main(["--clippy", str(self.c), "--sonar", str(self.s),
                                   "--output", str(output)])
        self.assertEqual(status, 2)
        self.assertIn("missing successful build-finished", err.getvalue())
        self.assertEqual(out.getvalue(), "")
        self.assertEqual(output.read_text(encoding="utf-8"), "preserve me")

    def test_cli_stdout_file_stability_and_input_protection(self):
        self.cargo()
        self.sonar()
        args = ["--clippy", str(self.c), "--sonar", str(self.s)]
        first, second = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(first):
            self.assertEqual(overlap.main(args), 0)
        with contextlib.redirect_stdout(second):
            self.assertEqual(overlap.main(args), 0)
        self.assertEqual(first.getvalue(), second.getvalue())
        output = self.root / "output.json"
        self.assertEqual(overlap.main(args + ["--output", str(output)]), 0)
        self.assertEqual(output.read_text(encoding="utf-8"), first.getvalue())
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(overlap.main(args + ["--output", str(self.c)]), 2)


if __name__ == "__main__":
    unittest.main()
