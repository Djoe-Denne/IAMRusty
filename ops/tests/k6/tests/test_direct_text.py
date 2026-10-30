"""Closed format families only: fixtures are not native diagnostics or causes."""
import unittest
from unittest import mock

from test_runner import runner
import test_direct_stop
import safety


FATAL = b"[123:456:1030/121314.123456:FATAL:base/sample.cc:12] sample fatal message\n"
ASSERTION = b"chrome: sample.c:12: sample_func: Assertion `a == b' failed.\n"


class TextFacetTests(unittest.TestCase):
    def classify(self, stdout=b"", stderr=b""):
        value = runner.direct_text_observation(stdout, stderr)
        self.assertEqual(set(value), {"version", "stdout", "stderr", "family"})
        self.assertEqual(value["version"], 1)
        self.assertEqual(value["stdout"], "PRESENT" if stdout else "EMPTY")
        self.assertEqual(value["stderr"], "PRESENT" if stderr else "EMPTY")
        return value["family"]

    def test_empty_presence_and_whitespace_are_distinct(self):
        self.assertEqual(self.classify(), "NO_TEXT")
        for stdout, stderr in ((b"noise\n", b""), (b"", b"noise\n"), (b"noise\n", b"noise\n"),
                               (b" \t\n", b""), (b"", b"\n")):
            self.assertEqual(self.classify(stdout, stderr), "OTHER_TEXT")

    def test_pinned_chromium_fatal_header_shapes_only(self):
        for line in (FATAL, b"[FATAL:base/sample.cc:12] sample fatal message\n",
                     b"[1030/121314.123456:FATAL:base/sample.cc:12] sample fatal message\n"):
            for stdout, stderr in ((line, b""), (b"", line)):
                self.assertEqual(self.classify(stdout, stderr), "CHROMIUM_FATAL_REPORTED")

    def test_glibc_assertion_full_format_with_optional_program_and_function(self):
        for line in (ASSERTION, b"sample.c:12: Assertion `a == b' failed.\n",
                     b"chrome: dir/sample.c:12: void ns::sample(): Assertion `a == b' failed.\n"):
            self.assertEqual(self.classify(stderr=line), "ASSERTION_FAILURE_REPORTED")

    def test_multiple_means_distinct_families_not_duplicate_lines_or_noise(self):
        self.assertEqual(self.classify(FATAL, ASSERTION), "MULTIPLE")
        self.assertEqual(self.classify(stderr=ASSERTION + FATAL), "MULTIPLE")
        self.assertEqual(self.classify(stderr=FATAL + FATAL), "CHROMIUM_FATAL_REPORTED")
        self.assertEqual(self.classify(FATAL, b"ordinary noise\n"), "CHROMIUM_FATAL_REPORTED")

    def test_words_error_lines_incomplete_formats_and_cross_stream_fragments_are_other(self):
        other = [b"FATAL\n", b"Assertion failed\n", b"abort SIGABRT assertion fatal\n",
                 FATAL.replace(b":FATAL:", b":ERROR:"), FATAL.rstrip(b"\n"), ASSERTION.rstrip(b"\n"),
                 b"[123:456:1030/121314.123456:FATAL:sample.cc(12)] old header\n",
                 b"prefix " + FATAL, b"sample.c:12: Assertion a == b failed.\n",
                 b"sample.c:12: Assertion `a == b' failed. suffix\n",
                 b"[FATAL:sample.cc:not-a-line] sample fatal message\n"]
        for raw in other:
            self.assertEqual(self.classify(stderr=raw), "OTHER_TEXT")
        for raw, split in ((FATAL, FATAL.index(b"FATAL") + 3), (ASSERTION, ASSERTION.index(b"Assertion") + 4)):
            self.assertEqual(self.classify(raw[:split], raw[split:]), "OTHER_TEXT")

    def test_both_streams_must_pass_privacy_before_any_recognition(self):
        for bad in (b"CONFIDENTIAL_SENTINEL", b"\xff", b"https://example.invalid/private", b"x" * (safety.LIMIT + 1)):
            for stdout, stderr in ((FATAL, bad), (bad, ASSERTION)):
                with mock.patch.object(runner, "DIRECT_FATAL_FORMAT") as recognize, self.assertRaises(safety.Rejected):
                    runner.direct_text_observation(stdout, stderr)
                recognize.fullmatch.assert_not_called()
        for forged in (None, "text", bytearray(b"text"), {"bad": "CONFIDENTIAL_SENTINEL"}):
            with self.assertRaises(safety.Rejected):
                runner.direct_text_observation(forged, b"")

    def test_closed_facet_rejects_forged_fields_labels_types_and_empty_inconsistency(self):
        good = {"version": 1, "stdout": "EMPTY", "stderr": "EMPTY", "family": "NO_TEXT"}
        self.assertEqual(runner.closed_direct_text(good), good)
        bad = [None, {}, {**good, "version": True}, {**good, "stdout": True},
               {**good, "stderr": "CONFIDENTIAL_SENTINEL"}, {**good, "family": "CAUSE_PROVED"},
               {**good, "stdout": "PRESENT"}, {**good, "family": "CHROMIUM_FATAL_REPORTED"}]
        bad += [{**good, key: "native-data"} for key in ("text", "site", "assertion", "line", "pid", "path", "hash")]
        for forged in bad:
            with self.assertRaises(safety.Rejected):
                runner.closed_direct_text(forged)

    def test_real_reader_adds_only_closed_direct_facet_and_never_qualifies(self):
        helper = test_direct_stop.StopFacetTests()
        for raw, family in ((b"", "NO_TEXT"), (FATAL, "CHROMIUM_FATAL_REPORTED"),
                            (ASSERTION, "ASSERTION_FAILURE_REPORTED"), (FATAL + ASSERTION, "MULTIPLE"),
                            (b"ordinary noise\n", "OTHER_TEXT")):
            for native in (-6, 0):
                child, _, result, _ = helper.launch(payload=raw, native_exit=native)
                self.assertEqual(result["direct_text"], {"version": 1, "stdout": "EMPTY",
                    "stderr": "PRESENT" if raw else "EMPTY", "family": family})
                self.assertEqual(result["direct_stop"]["path"], "NATIVE_OBSERVED_BEFORE_CLEANUP")
                self.assertEqual(child.code, native)

    def test_real_reader_privacy_utf8_and_overflow_withhold_whole_facet(self):
        helper = test_direct_stop.StopFacetTests()
        for raw in (FATAL + b"CONFIDENTIAL_SENTINEL", ASSERTION + b"\xff", FATAL + b"x" * safety.LIMIT):
            for cleanup in (True, False):
                _, _, result, _ = helper.launch(payload=raw, cleanup=cleanup)
                self.assertNotIn("direct_text", result)
                self.assertIn(result["diagnostic"], {"PRIVACY_FAILED", "OUTPUT_LIMIT", "CLEANUP_FAILED"})
                self.assertEqual(result["cleanup"], "PASS" if cleanup else "FAIL")

    def test_cleanup_and_timeout_priorities_survive_useful_format(self):
        helper = test_direct_stop.StopFacetTests()
        for kwargs, diagnostic in (({"cleanup": False}, "CLEANUP_FAILED"), ({"native_exit": None}, "CHILD_TIMEOUT")):
            _, _, result, _ = helper.launch(payload=ASSERTION, **kwargs)
            self.assertEqual(result["diagnostic"], diagnostic)
            self.assertEqual(result["direct_text"]["family"], "ASSERTION_FAILURE_REPORTED")

    def test_forged_exception_facet_is_withheld_without_changing_failure(self):
        failure = safety.Rejected("SANDBOX_UNPROVEN", True)
        failure.cleanup = "PASS"
        failure.direct_text = {"version": 1, "stdout": "EMPTY", "stderr": "PRESENT",
                               "family": "CONFIDENTIAL_SENTINEL"}
        with mock.patch.object(runner, "direct_chromium", side_effect=failure):
            _, _, result, _ = test_direct_stop.StopFacetTests().launch()
        self.assertNotIn("direct_text", result)
        self.assertEqual(result["diagnostic"], "SANDBOX_UNPROVEN")
        self.assertEqual(result["cleanup"], "PASS")
