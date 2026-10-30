"""Stop facets from real Child.execute under fake OS boundaries; no native process."""
import contextlib
import io
import json
from pathlib import Path
import unittest
from unittest import mock

from test_boundaries import scratch
from test_direct_chromium import fake_child_reads, host
from test_runner import runner
import safety


class StopFacetTests(unittest.TestCase):
    def launch(self, *, payload=b"", cleanup=True, native_exit=-5, args=None, interrupt=False, read_error=False):
        events = []
        with scratch() as root:
            (root / "private").mkdir()
            with fake_child_reads(payload, 1, cleanup, native_exit=native_exit, args=args,
                                  interrupt=interrupt, events=events) as child, \
                    mock.patch.object(runner, "linux_platform"), mock.patch.object(runner, "new_output", return_value=root), \
                    mock.patch.object(runner, "pinned_tools", return_value=({"k6": Path("/pinned/k6"), "chromium": Path("/pinned/chrome")}, {})), \
                    mock.patch.object(runner, "version_checks", return_value={}), \
                    mock.patch.object(runner, "host_metadata", return_value=host()), \
                    mock.patch.object(runner, "verify_tools"), mock.patch.object(runner, "Child", return_value=child), \
                    mock.patch.object(runner, "publish") as publish, contextlib.ExitStack() as stack, \
                    contextlib.redirect_stdout(io.StringIO()) as log:
                if read_error:
                    stack.enter_context(mock.patch.object(safety.os, "read", side_effect=OSError("CONFIDENTIAL_SENTINEL")))
                code = runner.main(["diagnose-chromium", "--output", str(root)])
            publish.assert_not_called()
            self.assertFalse((root / "result.json").exists())
            self.assertFalse((root / "provenance.json").exists())
            result = json.loads(log.getvalue())
            self.assertNotEqual(code, 0)
            self.assertEqual(result["HTTP"], "INCONCLUSIVE")
            self.assertEqual(result["Chromium"], "INCONCLUSIVE")
            self.assertEqual(result["parent_runtime_cleanup"], "INCONCLUSIVE")
            self.assertNotIn("k6_exit", result)
            self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue())
            self.assertLessEqual(len(log.getvalue().encode()), safety.ARTIFACT_LIMIT)
            return child, code, result, events

    def test_native_negative_observed_before_cleanup_is_not_supervisor_term(self):
        child, code, result, events = self.launch()
        self.assertEqual(code, 2)
        self.assertEqual(child.stop_path, "NATIVE_OBSERVED_BEFORE_CLEANUP")
        self.assertEqual(child.code, -5)
        self.assertFalse(any(event[0] == "term" for event in events))
        self.assertTrue(any(event == ("poll", -5, "NOT_OBSERVED") for event in events))
        self.assertTrue(all(event[1] == child.stop_path for event in events if event[0] == "close"))
        self.assertEqual(result["direct_stop"], {"version": 1, "path": child.stop_path, "signal": "SIGTRAP"})
        self.assertEqual(result["direct_observation"]["termination"], "EXITED")

    def test_real_forbidden_flag_guard_records_path_before_cleanup_term(self):
        child, code, result, events = self.launch(native_exit=None, args=[b"--headless", b"--no-sandbox"])
        self.assertEqual(code, 1)
        self.assertEqual(result["diagnostic"], "SANDBOX_UNPROVEN")
        self.assertEqual(child.stop_path, "GUARD_REJECTED")
        self.assertEqual(child.code, -15)
        self.assertEqual([event for event in events if event[0] == "term"], [("term", "GUARD_REJECTED", None)])
        self.assertEqual(result["direct_stop"], {"version": 1, "path": "GUARD_REJECTED", "signal": "SIGTERM"})

    def test_timeout_and_interruption_keep_path_despite_final_negative_status(self):
        for interrupt, path, diagnostic in ((False, "TIMEOUT", "CHILD_TIMEOUT"), (True, "INTERRUPTED", "INTERRUPTED")):
            child, _, result, events = self.launch(native_exit=None, interrupt=interrupt)
            self.assertEqual(child.stop_path, path)
            self.assertEqual(child.code, -15)
            self.assertEqual(result["diagnostic"], diagnostic)
            self.assertEqual(result["direct_stop"], {"version": 1, "path": path, "signal": "SIGTERM"})
            self.assertTrue(any(event == ("term", path, None) for event in events))
            self.assertEqual(result["cleanup"], "PASS")

    def test_privacy_supersedes_error_but_does_not_erase_internal_path(self):
        for native, expected in ((-6, "NATIVE_OBSERVED_BEFORE_CLEANUP"), (None, "TIMEOUT")):
            child, _, result, _ = self.launch(payload=b"CONFIDENTIAL_SENTINEL", native_exit=native)
            self.assertEqual(child.stop_path, expected)
            self.assertEqual(result["diagnostic"], "PRIVACY_FAILED")
            self.assertEqual(result["confidentiality"], "FAIL")
            self.assertNotIn("direct_stop", result)

    def test_overflow_and_cleanup_failure_keep_origin_but_withhold_projection(self):
        for cleanup in (False, True):
            child, _, result, events = self.launch(payload=b"x" * (safety.LIMIT + 1), native_exit=None, cleanup=cleanup)
            self.assertTrue(child.output_overflow)
            self.assertEqual(child.stop_path, "OUTPUT_LIMIT")
            self.assertEqual(result["diagnostic"], "OUTPUT_LIMIT" if cleanup else "CLEANUP_FAILED")
            self.assertEqual(result["cleanup"], "PASS" if cleanup else "FAIL")
            self.assertNotIn("direct_stop", result)
            self.assertNotIn("direct_observation", result)
            self.assertTrue(any(event == ("term", "OUTPUT_LIMIT", None) for event in events))

    def test_native_fact_survives_cleanup_failure_and_late_poll(self):
        child, _, result, events = self.launch(cleanup=False)
        self.assertEqual(child.code, -5)
        self.assertEqual(child.stop_path, "NATIVE_OBSERVED_BEFORE_CLEANUP")
        self.assertEqual(result["diagnostic"], "CLEANUP_FAILED")
        self.assertEqual(result["cleanup"], "FAIL")
        self.assertEqual(result["direct_stop"]["signal"], "SIGTRAP")
        self.assertTrue(any(event == ("term", child.stop_path, -5) for event in events))

    def test_other_error_native_zero_and_unknown_signal_stay_nonqualifying(self):
        child, _, result, events = self.launch(native_exit=None, read_error=True)
        self.assertEqual(child.stop_path, "ERROR")
        self.assertEqual(result["diagnostic"], "INTERNAL_ERROR")
        self.assertEqual(result["direct_stop"]["path"], "ERROR")
        self.assertTrue(any(event == ("term", "ERROR", None) for event in events))
        for native, expected in ((0, "NO_SIGNAL"), (-63, "UNKNOWN_SIGNAL")):
            child, _, result, _ = self.launch(native_exit=native)
            self.assertEqual(child.code, native)
            self.assertEqual(result["direct_stop"]["signal"], expected)

    def test_linux_signal_table_is_not_windows_enum_and_rejects_forged_types(self):
        expected = {None: "ABSENT", 0: "NO_SIGNAL", 107: "NO_SIGNAL", -4: "SIGILL", -5: "SIGTRAP",
                    -6: "SIGABRT", -9: "SIGKILL", -11: "SIGSEGV", -15: "SIGTERM", -31: "SIGSYS", -63: "UNKNOWN_SIGNAL"}
        for code, label in expected.items():
            self.assertEqual(safety.linux_signal_category(code), label)
        for forged in (False, True, "-5", -5.0, {"text": "CONFIDENTIAL_SENTINEL"}):
            with self.assertRaises(safety.Rejected):
                safety.linux_signal_category(forged)

    def test_forged_native_type_from_poll_does_not_become_a_signal(self):
        child, _, result, _ = self.launch(native_exit=True)
        self.assertEqual(child.stop_path, "NATIVE_OBSERVED_BEFORE_CLEANUP")
        self.assertEqual(result["diagnostic"], "INTERNAL_ERROR")
        self.assertNotIn("direct_stop", result)

    def test_forged_exception_facet_is_withheld_without_replacing_failure(self):
        failure = safety.Rejected("SANDBOX_UNPROVEN", True)
        failure.cleanup = "PASS"
        failure.direct_stop = {"version": 1, "path": "CONFIDENTIAL_SENTINEL", "signal": "SIGTRAP"}
        with scratch() as root, mock.patch.object(runner, "linux_platform"), \
                mock.patch.object(runner, "new_output", return_value=root), \
                mock.patch.object(runner, "pinned_tools", return_value=({}, {})), \
                mock.patch.object(runner, "version_checks", return_value={}), \
                mock.patch.object(runner, "host_metadata", return_value=host()), \
                mock.patch.object(runner, "direct_chromium", side_effect=failure), \
                mock.patch.object(runner, "publish") as publish, contextlib.redirect_stdout(io.StringIO()) as log:
            self.assertEqual(runner.main(["diagnose-chromium", "--output", str(root)]), 2)
        result = json.loads(log.getvalue())
        self.assertEqual(result["diagnostic"], "SANDBOX_UNPROVEN")
        self.assertEqual(result["cleanup"], "PASS")
        self.assertEqual(result["Chromium"], "INCONCLUSIVE")
        self.assertNotIn("direct_stop", result)
        self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue())
        publish.assert_not_called()

    def test_facet_rejects_unknown_fields_labels_versions_and_types(self):
        value = {"version": 1, "path": "NOT_OBSERVED", "signal": "ABSENT"}
        self.assertEqual(runner.closed_direct_stop(value), value)
        for forged in (None, {}, {**value, "emitter": "parent"}, {**value, "path": "CONFIDENTIAL_SENTINEL"},
                       {**value, "signal": -5}, {**value, "signal": "SIG_NEW"}, {**value, "version": True}):
            with self.assertRaises(safety.Rejected):
                runner.closed_direct_stop(forged)
